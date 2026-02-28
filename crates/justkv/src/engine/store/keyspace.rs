use std::collections::VecDeque;

use super::helpers::{monotonic_now_ms, purge_if_expired};
use super::pattern::wildcard_match;
use super::Store;
use crate::engine::value::{CompactKey, CompactValue, Entry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub struct SortOptions {
    pub alpha: bool,
    pub order: SortOrder,
    pub limit: Option<(usize, usize)>,
    pub store: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub enum SortResult {
    Values(Vec<Vec<u8>>),
    Stored(i64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreError {
    BusyKey,
    InvalidPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortError {
    WrongType,
    InvalidNumber,
}

impl Store {
    pub fn del<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        let mut removed = 0;
        for key in keys {
            let key = key.as_ref();
            let idx = self.shard_index(key);
            let mut shard = self.shards[idx].write();
            shard.ttl.remove(key);
            if shard.entries.remove(key).is_some() {
                removed += 1;
            }
        }
        removed
    }

    pub fn exists<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        let now_ms = monotonic_now_ms();
        let mut count = 0;
        for key in keys {
            let key = key.as_ref();
            let idx = self.shard_index(key);
            let shard = self.shards[idx].read();
            if shard.entries.get(key).is_some_and(|_| {
                shard
                    .ttl
                    .get(key)
                    .copied()
                    .is_none_or(|deadline| now_ms < deadline)
            }) {
                count += 1;
            }
        }
        count
    }

    pub fn touch<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        self.exists(keys)
    }

    pub fn unlink<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        self.del(keys)
    }

    pub fn rename(&self, from: &[u8], to: &[u8]) -> bool {
        let from_idx = self.shard_index(from);
        let mut source = self.shards[from_idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut source, from, now_ms) {
            return false;
        }

        let Some(entry) = source.entries.remove(from) else {
            return false;
        };
        let deadline = source.ttl.remove(from);
        drop(source);

        let to_idx = self.shard_index(to);
        let mut destination = self.shards[to_idx].write();
        let key = CompactKey::from_slice(to);
        destination.entries.insert(key.clone(), entry);
        if let Some(deadline) = deadline {
            destination.ttl.insert(key, deadline);
        } else {
            destination.ttl.remove(key.as_slice());
        }
        true
    }

    pub fn renamenx(&self, from: &[u8], to: &[u8]) -> Result<i64, ()> {
        let from_idx = self.shard_index(from);
        let mut source = self.shards[from_idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut source, from, now_ms) {
            return Err(());
        }

        let Some(entry) = source.entries.get(from).cloned() else {
            return Err(());
        };
        let deadline = source.ttl.get(from).copied();
        drop(source);

        let to_idx = self.shard_index(to);
        let mut destination = self.shards[to_idx].write();
        if !purge_if_expired(&mut destination, to, now_ms) && destination.entries.contains_key(to) {
            return Ok(0);
        }
        let key = CompactKey::from_slice(to);
        destination.entries.insert(key.clone(), entry);
        if let Some(deadline) = deadline {
            destination.ttl.insert(key, deadline);
        } else {
            destination.ttl.remove(key.as_slice());
        }
        drop(destination);

        let mut source = self.shards[from_idx].write();
        source.entries.remove(from);
        source.ttl.remove(from);
        Ok(1)
    }

    pub fn copy(&self, from: &[u8], to: &[u8], replace: bool) -> i64 {
        let from_idx = self.shard_index(from);
        let now_ms = monotonic_now_ms();
        let mut source = self.shards[from_idx].write();
        if purge_if_expired(&mut source, from, now_ms) {
            return 0;
        }

        let Some(entry) = source.entries.get(from).cloned() else {
            return 0;
        };
        let deadline = source.ttl.get(from).copied();
        drop(source);

        let to_idx = self.shard_index(to);
        let mut destination = self.shards[to_idx].write();
        let exists =
            !purge_if_expired(&mut destination, to, now_ms) && destination.entries.contains_key(to);
        if exists && !replace {
            return 0;
        }

        let key = CompactKey::from_slice(to);
        destination.entries.insert(key.clone(), entry);
        if let Some(deadline) = deadline {
            destination.ttl.insert(key, deadline);
        } else {
            destination.ttl.remove(key.as_slice());
        }
        1
    }

    pub fn move_key(&self, _key: &[u8], db: i64) -> Result<i64, ()> {
        if db != 0 {
            return Err(());
        }
        Ok(0)
    }

    pub fn key_type(&self, key: &[u8]) -> &'static str {
        self.value_kind(key).unwrap_or("none")
    }

    pub fn value_kind(&self, key: &[u8]) -> Option<&'static str> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let entry = shard.entries.get(key)?;
        if shard
            .ttl
            .get(key)
            .copied()
            .is_some_and(|deadline| now_ms >= deadline)
        {
            return None;
        }
        Some(entry.kind())
    }

    pub fn dbsize(&self) -> i64 {
        let now_ms = monotonic_now_ms();
        let mut total = 0;
        for shard in self.shards.iter() {
            let guard = shard.read();
            total += guard
                .entries
                .iter()
                .filter(|(key, _)| {
                    guard
                        .ttl
                        .get(key.as_slice())
                        .copied()
                        .is_none_or(|deadline| now_ms < deadline)
                })
                .count() as i64;
        }
        total
    }

    pub fn keys(&self, pattern: &[u8]) -> Vec<Vec<u8>> {
        let now_ms = monotonic_now_ms();
        let mut out = Vec::new();
        for shard in self.shards.iter() {
            let guard = shard.read();
            for (key, _) in guard.entries.iter() {
                if guard
                    .ttl
                    .get(key.as_slice())
                    .copied()
                    .is_none_or(|deadline| now_ms < deadline)
                    && wildcard_match(pattern, key.as_slice())
                {
                    out.push(key.to_vec());
                }
            }
        }
        out
    }

    pub fn scan(
        &self,
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
        value_type: Option<&[u8]>,
    ) -> (u64, Vec<CompactKey>) {
        let now_ms = monotonic_now_ms();
        let mut all = Vec::new();
        for shard in self.shards.iter() {
            let guard = shard.read();
            for (key, entry) in guard.entries.iter() {
                if !guard
                    .ttl
                    .get(key.as_slice())
                    .copied()
                    .is_none_or(|deadline| now_ms < deadline)
                {
                    continue;
                }
                if pattern.is_some_and(|matcher| !wildcard_match(matcher, key.as_slice())) {
                    continue;
                }
                if value_type
                    .is_some_and(|kind| !entry.kind().as_bytes().eq_ignore_ascii_case(kind))
                {
                    continue;
                }
                all.push(key.clone());
            }
        }

        all.sort_unstable_by(|left, right| left.as_slice().cmp(right.as_slice()));
        if all.is_empty() {
            return (0, Vec::new());
        }

        let mut index = usize::try_from(cursor).unwrap_or(usize::MAX).min(all.len());
        let target = count.max(1);
        let mut out = Vec::with_capacity(target);
        while index < all.len() && out.len() < target {
            out.push(all[index].clone());
            index += 1;
        }

        let next = if index >= all.len() { 0 } else { index as u64 };
        (next, out)
    }

    pub fn dump(&self, key: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return None;
        }
        let entry = shard.entries.get(key)?;
        Some(serialize_entry(entry))
    }

    pub fn restore(
        &self,
        key: &[u8],
        ttl_ms: u64,
        payload: &[u8],
        replace: bool,
    ) -> Result<(), RestoreError> {
        let entry = deserialize_entry(payload).ok_or(RestoreError::InvalidPayload)?;
        let deadline = if ttl_ms == 0 {
            None
        } else {
            Some(monotonic_now_ms().saturating_add(ttl_ms))
        };

        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let exists = !purge_if_expired(&mut shard, key, now_ms) && shard.entries.contains_key(key);
        if exists && !replace {
            return Err(RestoreError::BusyKey);
        }

        let compact_key = CompactKey::from_slice(key);
        shard.entries.insert(compact_key.clone(), entry);
        if let Some(value) = deadline {
            shard.ttl.insert(compact_key, value);
        } else {
            shard.ttl.remove(compact_key.as_slice());
        }
        Ok(())
    }

    pub fn sort(&self, key: &[u8], options: &SortOptions) -> Result<SortResult, SortError> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            if let Some(destination) = &options.store {
                drop(shard);
                return Ok(SortResult::Stored(
                    self.store_sorted_list(destination, Vec::new()),
                ));
            }
            return Ok(SortResult::Values(Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            if let Some(destination) = &options.store {
                drop(shard);
                return Ok(SortResult::Stored(
                    self.store_sorted_list(destination, Vec::new()),
                ));
            }
            return Ok(SortResult::Values(Vec::new()));
        };

        let mut values = match entry {
            Entry::List(list) => list.iter().map(CompactValue::to_vec).collect::<Vec<_>>(),
            Entry::Set(set) => set.iter().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::ZSet(zset) => zset.keys().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::String(_) | Entry::Hash(_) => return Err(SortError::WrongType),
        };

        if options.alpha {
            values.sort_unstable();
        } else {
            values.sort_unstable_by(|left, right| {
                parse_sort_number(left)
                    .zip(parse_sort_number(right))
                    .map(|(left, right)| left.total_cmp(&right))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if values
                .iter()
                .any(|value| parse_sort_number(value).is_none())
            {
                return Err(SortError::InvalidNumber);
            }
        }

        if options.order == SortOrder::Desc {
            values.reverse();
        }

        if let Some((offset, count)) = options.limit {
            values = values.into_iter().skip(offset).take(count).collect();
        }

        if let Some(destination) = &options.store {
            drop(shard);
            return Ok(SortResult::Stored(
                self.store_sorted_list(destination, values),
            ));
        }

        Ok(SortResult::Values(values))
    }

    fn store_sorted_list(&self, destination: &[u8], values: Vec<Vec<u8>>) -> i64 {
        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();
        let list: VecDeque<CompactValue> = values
            .iter()
            .map(|value| CompactValue::from_vec(value.clone()))
            .collect();
        let key = CompactKey::from_slice(destination);
        shard.entries.insert(key.clone(), Entry::List(list));
        shard.ttl.remove(key.as_slice());
        values.len() as i64
    }

    pub fn flushdb(&self) -> i64 {
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            removed += guard.entries.len() as i64;
            guard.entries.clear();
            guard.ttl.clear();
        }
        removed
    }
}

fn parse_sort_number(raw: &[u8]) -> Option<f64> {
    let value = std::str::from_utf8(raw).ok()?;
    value.parse::<f64>().ok()
}

fn serialize_entry(entry: &Entry) -> Vec<u8> {
    let mut out = vec![1u8];
    match entry {
        Entry::String(value) => {
            out.push(0);
            write_bytes(&mut out, value.as_slice());
        }
        Entry::Hash(map) => {
            out.push(1);
            write_u32(&mut out, map.len() as u32);
            for (field, value) in map {
                write_bytes(&mut out, field.as_slice());
                write_bytes(&mut out, value.as_slice());
            }
        }
        Entry::List(list) => {
            out.push(2);
            write_u32(&mut out, list.len() as u32);
            for value in list {
                write_bytes(&mut out, value.as_slice());
            }
        }
        Entry::Set(set) => {
            out.push(3);
            write_u32(&mut out, set.len() as u32);
            for member in set {
                write_bytes(&mut out, member.as_slice());
            }
        }
        Entry::ZSet(map) => {
            out.push(4);
            write_u32(&mut out, map.len() as u32);
            for (member, score) in map {
                write_bytes(&mut out, member.as_slice());
                out.extend_from_slice(&score.to_le_bytes());
            }
        }
    }
    out
}

fn deserialize_entry(payload: &[u8]) -> Option<Entry> {
    if payload.len() < 2 || payload[0] != 1 {
        return None;
    }

    let mut input = &payload[2..];
    match payload[1] {
        0 => {
            let value = read_bytes(&mut input)?;
            if !input.is_empty() {
                return None;
            }
            Some(Entry::String(CompactValue::from_vec(value)))
        }
        1 => {
            let count = read_u32(&mut input)? as usize;
            let mut map = hashbrown::HashMap::with_hasher(ahash::RandomState::new());
            for _ in 0..count {
                let field = CompactKey::from_vec(read_bytes(&mut input)?);
                let value = CompactValue::from_vec(read_bytes(&mut input)?);
                map.insert(field, value);
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Hash(map))
        }
        2 => {
            let count = read_u32(&mut input)? as usize;
            let mut list = VecDeque::with_capacity(count);
            for _ in 0..count {
                list.push_back(CompactValue::from_vec(read_bytes(&mut input)?));
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::List(list))
        }
        3 => {
            let count = read_u32(&mut input)? as usize;
            let mut set = hashbrown::HashSet::with_hasher(ahash::RandomState::new());
            for _ in 0..count {
                set.insert(CompactKey::from_vec(read_bytes(&mut input)?));
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Set(set))
        }
        4 => {
            let count = read_u32(&mut input)? as usize;
            let mut zset = hashbrown::HashMap::with_hasher(ahash::RandomState::new());
            for _ in 0..count {
                let member = CompactKey::from_vec(read_bytes(&mut input)?);
                if input.len() < 8 {
                    return None;
                }
                let mut score_bytes = [0u8; 8];
                score_bytes.copy_from_slice(&input[..8]);
                input = &input[8..];
                zset.insert(member, f64::from_le_bytes(score_bytes));
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::ZSet(zset))
        }
        _ => None,
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(input: &mut &[u8]) -> Option<u32> {
    if input.len() < 4 {
        return None;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&input[..4]);
    *input = &input[4..];
    Some(u32::from_le_bytes(bytes))
}

fn write_bytes(out: &mut Vec<u8>, value: &[u8]) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value);
}

fn read_bytes(input: &mut &[u8]) -> Option<Vec<u8>> {
    let len = read_u32(input)? as usize;
    if input.len() < len {
        return None;
    }
    let value = input[..len].to_vec();
    *input = &input[len..];
    Some(value)
}
