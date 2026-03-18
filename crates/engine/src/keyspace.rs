use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use super::Store;
use super::helpers::{get_live_entry, is_expired, monotonic_now_ms, purge_if_expired};
use super::pattern::{CompiledPattern, wildcard_match};
use types::value::{
    CompactKey, CompactValue, Entry, HashValue, StreamId, StreamValue, ZSetValueMap,
};

static RANDOM_COUNTER: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);

#[inline(always)]
fn random_seed(len: usize) -> u64 {
    RANDOM_COUNTER
        .fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed)
        .wrapping_add((len as u64).wrapping_mul(0xbf58476d1ce4e5b9))
}

#[inline(always)]
fn random_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58476d1ce4e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn is_integer_encoded_string(value: &[u8]) -> bool {
    if value.is_empty() {
        return false;
    }

    let mut index = 0usize;
    let negative = matches!(value[0], b'-');
    if negative || matches!(value[0], b'+') {
        index = 1;
    }

    let digits = &value[index..];
    if digits.is_empty() {
        return false;
    }
    if digits.len() > 1 && digits[0] == b'0' {
        return false;
    }
    if negative && digits == b"0" {
        return false;
    }
    if !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }

    let text = match std::str::from_utf8(value) {
        Ok(text) => text,
        Err(_) => return false,
    };

    text.parse::<i64>().is_ok()
}

fn object_encoding(entry: &Entry) -> &'static str {
    match entry {
        Entry::String(value) => {
            if is_integer_encoded_string(value.as_slice()) {
                "int"
            } else {
                "raw"
            }
        }
        Entry::Hash(_) => "hashtable",
        Entry::List(_) => "quicklist",
        Entry::Set(_) => "hashtable",
        Entry::ZSet(_) | Entry::Geo(_) => "skiplist",
        Entry::Stream(_) => "stream",
        Entry::Json(_) => "json",
    }
}

#[derive(Clone, Debug)]
pub struct PreDecodedRestoreEntry {
    pub key: Vec<u8>,
    pub ttl_ms: u64,
    pub entry: Entry,
}

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
            if shard.remove_key(key).is_some() {
                removed += 1;
            }
        }
        removed
    }

    pub fn exists<K: AsRef<[u8]>>(&self, keys: &[K]) -> i64 {
        let mut count = 0;
        let now_ms = monotonic_now_ms();
        for key in keys {
            let key = key.as_ref();
            let idx = self.shard_index(key);
            let shard = self.shards[idx].read();
            let present = if shard.has_ttls() {
                get_live_entry(&shard, key, now_ms).is_some()
            } else {
                shard.entries.get(key).is_some()
            };
            if present {
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
        let now_ms = monotonic_now_ms();
        let to_idx = self.shard_index(to);
        if from_idx == to_idx {
            let mut shard = self.shards[from_idx].write();
            if purge_if_expired(&mut shard, from, now_ms) {
                return false;
            }

            let deadline = shard.ttl_deadline(from);
            let Some(entry) = shard.remove_key(from) else {
                return false;
            };
            let key = CompactKey::from_slice(to);
            shard.insert_entry(key, entry, deadline);
            return true;
        }

        let mut source = self.shards[from_idx].write();
        if purge_if_expired(&mut source, from, now_ms) {
            return false;
        }

        let deadline = source.ttl_deadline(from);
        let Some(entry) = source.remove_key(from) else {
            return false;
        };
        drop(source);

        let mut destination = self.shards[to_idx].write();
        let key = CompactKey::from_slice(to);
        destination.insert_entry(key, entry, deadline);
        true
    }

    pub fn renamenx(&self, from: &[u8], to: &[u8]) -> Result<i64, ()> {
        let from_idx = self.shard_index(from);
        let now_ms = monotonic_now_ms();
        let to_idx = self.shard_index(to);
        if from_idx == to_idx {
            let mut shard = self.shards[from_idx].write();
            if purge_if_expired(&mut shard, from, now_ms) {
                return Err(());
            }

            if !purge_if_expired(&mut shard, to, now_ms) && shard.entries.get(to).is_some() {
                return Ok(0);
            }

            let deadline = shard.ttl_deadline(from);
            let Some(entry) = shard.remove_key(from) else {
                return Err(());
            };
            let key = CompactKey::from_slice(to);
            shard.insert_entry(key, entry, deadline);
            return Ok(1);
        }

        if from_idx < to_idx {
            let mut source = self.shards[from_idx].write();
            let mut destination = self.shards[to_idx].write();
            if purge_if_expired(&mut source, from, now_ms) {
                return Err(());
            }
            if !purge_if_expired(&mut destination, to, now_ms)
                && destination.entries.get(to).is_some()
            {
                return Ok(0);
            }
            let deadline = source.ttl_deadline(from);
            let Some(entry) = source.remove_key(from) else {
                return Err(());
            };
            let key = CompactKey::from_slice(to);
            destination.insert_entry(key, entry, deadline);
            return Ok(1);
        }

        let mut destination = self.shards[to_idx].write();
        let mut source = self.shards[from_idx].write();
        if purge_if_expired(&mut source, from, now_ms) {
            return Err(());
        }
        if !purge_if_expired(&mut destination, to, now_ms) && destination.entries.get(to).is_some()
        {
            return Ok(0);
        }
        let deadline = source.ttl_deadline(from);
        let Some(entry) = source.remove_key(from) else {
            return Err(());
        };
        let key = CompactKey::from_slice(to);
        destination.insert_entry(key, entry, deadline);
        Ok(1)
    }

    pub fn copy(&self, from: &[u8], to: &[u8], replace: bool) -> i64 {
        let from_idx = self.shard_index(from);
        let now_ms = monotonic_now_ms();
        let to_idx = self.shard_index(to);
        if from_idx == to_idx {
            let mut shard = self.shards[from_idx].write();
            if purge_if_expired(&mut shard, from, now_ms) {
                return 0;
            }

            let deadline = shard.ttl_deadline(from);
            let Some(entry) = shard.entries.get(from).cloned() else {
                return 0;
            };
            let exists = if from == to {
                true
            } else {
                !purge_if_expired(&mut shard, to, now_ms) && shard.entries.get(to).is_some()
            };
            if exists && !replace {
                return 0;
            }

            let key = CompactKey::from_slice(to);
            shard.insert_entry(key, entry.entry, deadline);
            return 1;
        }

        let mut source = self.shards[from_idx].write();
        if purge_if_expired(&mut source, from, now_ms) {
            return 0;
        }

        let deadline = source.ttl_deadline(from);
        let Some(entry) = source.entries.get(from).cloned() else {
            return 0;
        };
        drop(source);

        let mut destination = self.shards[to_idx].write();
        let exists = !purge_if_expired(&mut destination, to, now_ms)
            && destination.entries.get(to).is_some();
        if exists && !replace {
            return 0;
        }

        let key = CompactKey::from_slice(to);
        destination.insert_entry(key, entry.entry, deadline);
        1
    }

    pub fn move_key(&self, key: &[u8], db: i64) -> Result<i64, ()> {
        if !(0..=15).contains(&db) {
            return Err(());
        }
        Ok(i64::from(self.dump(key).is_some()))
    }

    pub fn key_type(&self, key: &[u8]) -> &'static str {
        self.value_kind(key).unwrap_or("none")
    }

    pub fn object_encoding(&self, key: &[u8]) -> Option<&'static str> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let entry = get_live_entry(&shard, key, now_ms)?;
        Some(object_encoding(entry))
    }

    pub fn object_freq(&self, key: &[u8]) -> Result<Option<i64>, ()> {
        if self.object_encoding(key).is_none() {
            return Ok(None);
        }
        Err(())
    }

    pub fn object_idletime(&self, key: &[u8]) -> Option<i64> {
        self.object_encoding(key).map(|_| 0)
    }

    pub fn object_refcount(&self, key: &[u8]) -> Option<i64> {
        self.object_encoding(key).map(|_| 1)
    }

    pub fn value_kind(&self, key: &[u8]) -> Option<&'static str> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let entry = shard.entries.get(key)?;
        if shard.is_expired(key, now_ms) {
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
                .filter(|(key, _entry)| !guard.is_expired(key.as_slice(), now_ms))
                .count() as i64;
        }
        total
    }

    pub fn keys(&self, pattern: &[u8]) -> Vec<Vec<u8>> {
        let now_ms = monotonic_now_ms();
        let pattern = CompiledPattern::new((pattern != b"*").then_some(pattern));
        let mut out = Vec::new();
        for shard in self.shards.iter() {
            let guard = shard.read();
            for (key, _entry) in guard.entries.iter() {
                let key_bytes = key.slice();
                let pattern_matches = match &pattern {
                    CompiledPattern::Any => true,
                    CompiledPattern::Exact(pattern) => key_bytes == *pattern,
                    CompiledPattern::Prefix(prefix) => key_bytes.starts_with(prefix),
                    CompiledPattern::Suffix(suffix) => key_bytes.ends_with(suffix),
                    CompiledPattern::Contains(needle) => {
                        needle.is_empty()
                            || key_bytes
                                .windows(needle.len())
                                .any(|window| window == *needle)
                    }
                    CompiledPattern::PrefixSuffix { prefix, suffix } => {
                        key_bytes.len() >= prefix.len() + suffix.len()
                            && key_bytes.starts_with(prefix)
                            && key_bytes.ends_with(suffix)
                    }
                    CompiledPattern::Wildcard(pattern) => wildcard_match(pattern, key_bytes),
                };
                if !guard.is_expired(key.as_slice(), now_ms) && pattern_matches {
                    out.push(key.to_vec());
                }
            }
        }
        out
    }

    pub fn randomkey(&self) -> Option<Vec<u8>> {
        let now_ms = monotonic_now_ms();
        let shard_count = self.shards.len();
        if shard_count == 0 {
            return None;
        }

        let mut state = random_seed(shard_count);
        let start_shard = (random_next(&mut state) as usize) % shard_count;

        for shard_offset in 0..shard_count {
            let shard_index = (start_shard + shard_offset) % shard_count;
            let shard = self.shards[shard_index].read();
            let (keys, _entries) = shard.entries.slices();
            let len = keys.len();
            if len == 0 {
                continue;
            }

            let start_entry = (random_next(&mut state) as usize) % len;
            for entry_offset in 0..len {
                let entry_index = (start_entry + entry_offset) % len;
                if !shard.is_expired(keys[entry_index].as_slice(), now_ms) {
                    return Some(keys[entry_index].to_vec());
                }
            }
        }

        None
    }

    pub fn scan(
        &self,
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
        value_type: Option<&[u8]>,
    ) -> (u64, Vec<CompactKey>) {
        let cursor = usize::try_from(cursor).unwrap_or(usize::MAX);
        let now_ms = monotonic_now_ms();
        let pattern = CompiledPattern::new(pattern.filter(|pattern| *pattern != b"*"));
        let target = count.max(1);
        let mut seen = 0usize;
        let mut out = Vec::with_capacity(target);

        for shard in self.shards.iter() {
            let guard = shard.read();
            let (keys, entries) = guard.entries.slices();

            // Fast path: no per-entry filters, so the cursor can skip entire shards.
            if !guard.has_ttls() && matches!(pattern, CompiledPattern::Any) && value_type.is_none()
            {
                if cursor >= seen + keys.len() {
                    seen += keys.len();
                    continue;
                }

                let start = cursor.saturating_sub(seen);
                let remaining = target - out.len();
                let available = keys.len().saturating_sub(start);
                let take = available.min(remaining);

                out.extend(keys[start..start + take].iter().cloned());
                seen += start + take;

                if out.len() == target {
                    return (seen as u64, out);
                }

                continue;
            }

            for i in 0..keys.len() {
                let key = &keys[i];
                let entry = &entries[i];
                let key_bytes = key.slice();
                let pattern_matches = match &pattern {
                    CompiledPattern::Any => true,
                    CompiledPattern::Exact(pattern) => key_bytes == *pattern,
                    CompiledPattern::Prefix(prefix) => key_bytes.starts_with(prefix),
                    CompiledPattern::Suffix(suffix) => key_bytes.ends_with(suffix),
                    CompiledPattern::Contains(needle) => {
                        needle.is_empty()
                            || key_bytes
                                .windows(needle.len())
                                .any(|window| window == *needle)
                    }
                    CompiledPattern::PrefixSuffix { prefix, suffix } => {
                        key_bytes.len() >= prefix.len() + suffix.len()
                            && key_bytes.starts_with(prefix)
                            && key_bytes.ends_with(suffix)
                    }
                    CompiledPattern::Wildcard(pattern) => wildcard_match(pattern, key_bytes),
                };

                if guard.is_expired(key.as_slice(), now_ms) {
                    continue;
                }
                if !pattern_matches {
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
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return None;
        }
        let entry = shard.entries.get(key)?;
        serialize_entry(entry)
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

        self.restore_decoded_with_deadline(key, deadline, entry, replace)
    }

    pub fn restore_decoded(
        &self,
        key: &[u8],
        ttl_ms: u64,
        entry: Entry,
        replace: bool,
    ) -> Result<(), RestoreError> {
        let deadline = if ttl_ms == 0 {
            None
        } else {
            Some(monotonic_now_ms().saturating_add(ttl_ms))
        };

        self.restore_decoded_with_deadline(key, deadline, entry, replace)
    }

    pub fn restore_predecoded_unchecked(&self, entries: Vec<PreDecodedRestoreEntry>) {
        if entries.is_empty() {
            return;
        }

        let now_ms = monotonic_now_ms();

        let mut by_shard = vec![Vec::new(); self.shards.len()];
        for entry in entries {
            let idx = self.shard_index(&entry.key);
            by_shard[idx].push(entry);
        }

        for (idx, shard_entries) in by_shard.into_iter().enumerate() {
            if shard_entries.is_empty() {
                continue;
            }
            let mut shard = self.shards[idx].write();
            for entry in shard_entries {
                let compact_key = CompactKey::from_vec(entry.key);
                let deadline = (entry.ttl_ms > 0).then(|| now_ms.saturating_add(entry.ttl_ms));
                shard.insert_entry(compact_key, entry.entry, deadline);
            }
        }
    }

    fn restore_decoded_with_deadline(
        &self,
        key: &[u8],
        deadline: Option<u64>,
        entry: Entry,
        replace: bool,
    ) -> Result<(), RestoreError> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let exists = !purge_if_expired(&mut shard, key, now_ms) && shard.entries.contains_key(key);
        if exists && !replace {
            return Err(RestoreError::BusyKey);
        }

        let compact_key = CompactKey::from_slice(key);
        shard.insert_entry(compact_key, entry, deadline);
        Ok(())
    }

    pub fn sort(&self, key: &[u8], options: &SortOptions) -> Result<SortResult, SortError> {
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

        let mut values = match &entry.entry {
            Entry::List(list) => list.iter().map(CompactValue::to_vec).collect::<Vec<_>>(),
            Entry::Set(set) => set.iter().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::ZSet(zset) => zset
                .iter_member_scores()
                .map(|(member, _)| member.to_vec())
                .collect::<Vec<_>>(),
            Entry::Geo(geo) => geo.keys().map(CompactKey::to_vec).collect::<Vec<_>>(),
            Entry::String(_) | Entry::Hash(_) | Entry::Stream(_) | Entry::Json(_) => {
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
        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();
        let len = values.len() as i64;
        let list: VecDeque<CompactValue> = values.into_iter().map(CompactValue::from_vec).collect();
        let key = CompactKey::from_slice(destination);
        shard.insert_entry(key, Entry::List(Box::new(list)), None);
        len
    }

    pub fn flushdb(&self) -> i64 {
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            removed += guard.entries.len() as i64;
            guard.entries.clear();
            guard.ttl_min_deadline = u64::MAX;
            guard.ttl_count = 0;
        }
        removed
    }
}

fn parse_sort_number(raw: &[u8]) -> Option<f64> {
    let value = std::str::from_utf8(raw).ok()?;
    value.parse::<f64>().ok()
}

fn serialize_entry(entry: &Entry) -> Option<Vec<u8>> {
    let mut out = vec![1u8];
    match entry {
        Entry::String(value) => {
            out.push(0);
            write_bytes(&mut out, value.as_slice());
        }
        Entry::Hash(hash_value) => {
            let map = hash_value.map();
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
        Entry::Json(value) => {
            out.push(7);
            let encoded = serde_json::to_vec(value).ok()?;
            write_bytes(&mut out, &encoded);
        }
    }
    Some(out)
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
            let mut hash_value = HashValue::with_capacity(count);
            let map = hash_value.map_mut();
            for _ in 0..count {
                let field = CompactKey::from_vec(read_bytes(&mut input)?);
                let value = CompactValue::from_vec(read_bytes(&mut input)?);
                map.insert(field, value);
            }
            if !input.is_empty() {
                return None;
            }
            Some(Entry::Hash(Box::new(hash_value)))
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
            let mut set: indexmap::IndexSet<CompactKey, rapidhash::fast::RandomState> =
                indexmap::IndexSet::with_capacity_and_hasher(
                    count,
                    rapidhash::fast::RandomState::new(),
                );
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
            let mut zset = ZSetValueMap::new();
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
            let mut geo = hashbrown::HashMap::with_capacity_and_hasher(
                count,
                rapidhash::fast::RandomState::new(),
            );
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
        7 => {
            let value = read_bytes(&mut input)?;
            if !input.is_empty() {
                return None;
            }
            let value = serde_json::from_slice(&value).ok()?;
            Some(Entry::Json(Box::new(value)))
        }
        6 => {
            let count = read_u32(&mut input)? as usize;
            let mut stream = StreamValue::new();
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
                let id = StreamId {
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
