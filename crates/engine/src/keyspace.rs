use std::collections::VecDeque;

use super::Store;
use super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::pattern::wildcard_match;
use crate::value::{CompactKey, CompactValue, Entry};

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
        let _trace = profiler::scope("crates::engine::src::keyspace::del");
        let mut removed = 0;
        for key in keys {
            let key = key.as_ref();
            let idx = self.shard_index(key);
            let mut shard = self.shards[idx].write();
            if shard.remove_key(key).is_some() {
                removed += 1;
            }
        }
        removed
    }

    pub fn exists<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        let _trace = profiler::scope("crates::engine::src::keyspace::exists");
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
        let _trace = profiler::scope("crates::engine::src::keyspace::touch");
        self.exists(keys)
    }

    pub fn unlink<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        let _trace = profiler::scope("crates::engine::src::keyspace::unlink");
        self.del(keys)
    }

    pub fn rename(&self, from: &[u8], to: &[u8]) -> bool {
        let _trace = profiler::scope("crates::engine::src::keyspace::rename");
        let from_idx = self.shard_index(from);
        let mut source = self.shards[from_idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut source, from, now_ms) {
            return false;
        }

        let Some(entry) = source.entries.remove(from) else {
            return false;
        };
        let deadline = source.clear_ttl(from);
        drop(source);

        let to_idx = self.shard_index(to);
        let mut destination = self.shards[to_idx].write();
        let key = CompactKey::from_slice(to);
        destination.entries.insert(key.clone(), entry);
        if let Some(deadline) = deadline {
            destination.set_ttl(key, deadline);
        } else {
            let _ = destination.clear_ttl(key.as_slice());
        }
        true
    }

    pub fn renamenx(&self, from: &[u8], to: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::keyspace::renamenx");
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
            destination.set_ttl(key, deadline);
        } else {
            let _ = destination.clear_ttl(key.as_slice());
        }
        drop(destination);

        let mut source = self.shards[from_idx].write();
        let _ = source.remove_key(from);
        Ok(1)
    }

    pub fn copy(&self, from: &[u8], to: &[u8], replace: bool) -> i64 {
        let _trace = profiler::scope("crates::engine::src::keyspace::copy");
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
            destination.set_ttl(key, deadline);
        } else {
            let _ = destination.clear_ttl(key.as_slice());
        }
        1
    }

    pub fn move_key(&self, _key: &[u8], db: i64) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::keyspace::move_key");
        if db != 0 {
            return Err(());
        }
        Ok(0)
    }

    pub fn key_type(&self, key: &[u8]) -> &'static str {
        let _trace = profiler::scope("crates::engine::src::keyspace::key_type");
        self.value_kind(key).unwrap_or("none")
    }

    pub fn value_kind(&self, key: &[u8]) -> Option<&'static str> {
        let _trace = profiler::scope("crates::engine::src::keyspace::value_kind");
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
        let _trace = profiler::scope("crates::engine::src::keyspace::dbsize");
        let now_ms = monotonic_now_ms();
        let mut total = 0;
        for shard in self.shards.iter() {
            let guard = shard.read();
            total += guard
                .entries
                .iter()
                .filter(|(key, _entry): &(&CompactKey, &Entry)| {
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
        let _trace = profiler::scope("crates::engine::src::keyspace::keys");
        let now_ms = monotonic_now_ms();
        let mut out = Vec::new();
        for shard in self.shards.iter() {
            let guard = shard.read();
            for (key, _) in guard.entries.iter() {
                if guard
                    .ttl
                    .get::<[u8]>(key.as_slice())
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
        let _trace = profiler::scope("crates::engine::src::keyspace::scan");
        let cursor = usize::try_from(cursor).unwrap_or(usize::MAX);
        let now_ms = monotonic_now_ms();
        let target = count.max(1);
        let mut seen = 0usize;
        let mut out = Vec::with_capacity(target);

        for shard in self.shards.iter() {
            let guard = shard.read();
            for (key, entry) in guard.entries.iter() {
                if !guard
                    .ttl
                    .get::<[u8]>(key.as_slice())
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

                if seen < cursor {
                    seen += 1;
                    continue;
                }

                if out.len() < target {
                    out.push(key.clone());
                    seen += 1;
                    continue;
                }

                return (seen as u64, out);
            }
        }

        (0, out)
    }

    pub fn dump(&self, key: &[u8]) -> Option<Vec<u8>> {
        let _trace = profiler::scope("crates::engine::src::keyspace::dump");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
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
        let _trace = profiler::scope("crates::engine::src::keyspace::restore");
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
            shard.set_ttl(compact_key, value);
        } else {
            let _ = shard.clear_ttl(compact_key.as_slice());
        }
        Ok(())
    }

    pub fn sort(&self, key: &[u8], options: &SortOptions) -> Result<SortResult, SortError> {
        let _trace = profiler::scope("crates::engine::src::keyspace::sort");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            if let Some(destination) = &options.store {
                return Ok(SortResult::Stored(
                    self.store_sorted_list(destination, Vec::new()),
                ));
            }
            return Ok(SortResult::Values(Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            if let Some(destination) = &options.store {
                return Ok(SortResult::Stored(
                    self.store_sorted_list(destination, Vec::new()),
                ));
            }
            return Ok(SortResult::Values(Vec::new()));
        };

        let mut values = match entry {
            Entry::List(list) => list.iter().map(CompactValue::to_vec).collect::<Vec<_>>(),
            Entry::Set(set) => set.iter().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::ZSet(zset) => zset
                .iter_member_scores()
                .map(|(member, _)| member.to_vec())
                .collect::<Vec<_>>(),
            Entry::Geo(geo) => geo.keys().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::String(_) | Entry::Hash(_) | Entry::Stream(_) => {
                return Err(SortError::WrongType);
            }
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
            return Ok(SortResult::Stored(
                self.store_sorted_list(destination, values),
            ));
        }

        Ok(SortResult::Values(values))
    }

    fn store_sorted_list(&self, destination: &[u8], values: Vec<Vec<u8>>) -> i64 {
        let _trace = profiler::scope("crates::engine::src::keyspace::store_sorted_list");
        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();
        let len = values.len() as i64;
        let list: VecDeque<CompactValue> = values.into_iter().map(CompactValue::from_vec).collect();
        let key = CompactKey::from_slice(destination);
        shard
            .entries
            .insert(key.clone(), Entry::List(Box::new(list)));
        let _ = shard.clear_ttl(key.as_slice());
        len
    }

    pub fn flushdb(&self) -> i64 {
        let _trace = profiler::scope("crates::engine::src::keyspace::flushdb");
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            removed += guard.entries.len() as i64;
            guard.entries.clear();
            guard.ttl.clear();
            guard.ttl_deadlines.clear();
        }
        removed
    }
}

fn parse_sort_number(raw: &[u8]) -> Option<f64> {
    let _trace = profiler::scope("crates::engine::src::keyspace::parse_sort_number");
    let value = std::str::from_utf8(raw).ok()?;
    value.parse::<f64>().ok()
}

fn serialize_entry(entry: &Entry) -> Vec<u8> {
    let _trace = profiler::scope("crates::engine::src::keyspace::serialize_entry");
    let mut out = vec![1u8];
    match entry {
        Entry::String(value) => {
            out.push(0);
            write_bytes(&mut out, value.as_slice());
        }
        Entry::Hash(map) => {
            out.push(1);
            write_u32(&mut out, map.len() as u32);
            for (field, value) in map.iter() {
                write_bytes(&mut out, field.as_slice());
                write_bytes(&mut out, value.as_slice());
            }
        }
        Entry::List(list) => {
            out.push(2);
            write_u32(&mut out, list.len() as u32);
            for value in list.iter() {
                write_bytes(&mut out, value.as_slice());
            }
        }
        Entry::Set(set) => {
            out.push(3);
            write_u32(&mut out, set.len() as u32);
            for member in set.iter() {
                write_bytes(&mut out, member.as_slice());
            }
        }
        Entry::ZSet(map) => {
            out.push(4);
            write_u32(&mut out, map.len() as u32);
            for (member, score) in map.iter_member_scores() {
                write_bytes(&mut out, member.as_slice());
                out.extend_from_slice(&score.to_le_bytes());
            }
        }
        Entry::Geo(map) => {
            out.push(5);
            write_u32(&mut out, map.len() as u32);
            for (member, (lon, lat)) in map.iter() {
                write_bytes(&mut out, member.as_slice());
                out.extend_from_slice(&lon.to_le_bytes());
                out.extend_from_slice(&lat.to_le_bytes());
            }
        }
        Entry::Stream(stream) => {
            out.push(6);
            write_u32(&mut out, stream.entries.len() as u32);
            for (id, fields) in &stream.entries {
                out.extend_from_slice(&id.ms.to_le_bytes());
                out.extend_from_slice(&id.seq.to_le_bytes());
                write_u32(&mut out, fields.len() as u32);
                for (field, value) in fields {
                    write_bytes(&mut out, field.as_slice());
                    write_bytes(&mut out, value.as_slice());
                }
            }
        }
    }
    out
}

fn deserialize_entry(payload: &[u8]) -> Option<Entry> {
    let _trace = profiler::scope("crates::engine::src::keyspace::deserialize_entry");
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
            let mut map: hashbrown::HashMap<CompactKey, CompactValue, ahash::RandomState> =
                hashbrown::HashMap::with_capacity_and_hasher(count, ahash::RandomState::new());
            for _ in 0..count {
                let field = CompactKey::from_vec(read_bytes(&mut input)?);
                let value = CompactValue::from_vec(read_bytes(&mut input)?);
                map.insert(field, value);
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Hash(Box::new(map)))
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
            Some(Entry::List(Box::new(list)))
        }
        3 => {
            let count = read_u32(&mut input)? as usize;
            let mut set: hashbrown::HashSet<CompactKey, ahash::RandomState> =
                hashbrown::HashSet::with_capacity_and_hasher(count, ahash::RandomState::new());
            for _ in 0..count {
                set.insert(CompactKey::from_vec(read_bytes(&mut input)?));
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Set(Box::new(set)))
        }
        4 => {
            let count = read_u32(&mut input)? as usize;
            let mut zset = crate::value::ZSetValueMap::new();
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
            Some(Entry::ZSet(Box::new(zset)))
        }
        5 => {
            let count = read_u32(&mut input)? as usize;
            let mut geo =
                hashbrown::HashMap::with_capacity_and_hasher(count, ahash::RandomState::new());
            for _ in 0..count {
                let member = CompactKey::from_vec(read_bytes(&mut input)?);
                if input.len() < 16 {
                    return None;
                }
                let mut lon_bytes = [0u8; 8];
                lon_bytes.copy_from_slice(&input[..8]);
                let mut lat_bytes = [0u8; 8];
                lat_bytes.copy_from_slice(&input[8..16]);
                input = &input[16..];
                geo.insert(
                    member,
                    (f64::from_le_bytes(lon_bytes), f64::from_le_bytes(lat_bytes)),
                );
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Geo(Box::new(geo)))
        }
        6 => {
            let count = read_u32(&mut input)? as usize;
            let mut stream = crate::value::StreamValue::new();
            for _ in 0..count {
                if input.len() < 16 {
                    return None;
                }
                let mut ms_bytes = [0u8; 8];
                ms_bytes.copy_from_slice(&input[..8]);
                let mut seq_bytes = [0u8; 8];
                seq_bytes.copy_from_slice(&input[8..16]);
                input = &input[16..];
                let field_count = read_u32(&mut input)? as usize;
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    let field = CompactKey::from_vec(read_bytes(&mut input)?);
                    let value = CompactValue::from_vec(read_bytes(&mut input)?);
                    fields.push((field, value));
                }
                let id = crate::value::StreamId {
                    ms: u64::from_le_bytes(ms_bytes),
                    seq: u64::from_le_bytes(seq_bytes),
                };
                stream.last_id = id;
                stream.entries.insert(id, fields);
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Stream(Box::new(stream)))
        }
        _ => None,
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    let _trace = profiler::scope("crates::engine::src::keyspace::write_u32");
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(input: &mut &[u8]) -> Option<u32> {
    let _trace = profiler::scope("crates::engine::src::keyspace::read_u32");
    if input.len() < 4 {
        return None;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&input[..4]);
    *input = &input[4..];
    Some(u32::from_le_bytes(bytes))
}

fn write_bytes(out: &mut Vec<u8>, value: &[u8]) {
    let _trace = profiler::scope("crates::engine::src::keyspace::write_bytes");
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value);
}

fn read_bytes(input: &mut &[u8]) -> Option<Vec<u8>> {
    let _trace = profiler::scope("crates::engine::src::keyspace::read_bytes");
    let len = read_u32(input)? as usize;
    if input.len() < len {
        return None;
    }
    let value = input[..len].to_vec();
    *input = &input[len..];
    Some(value)
}
