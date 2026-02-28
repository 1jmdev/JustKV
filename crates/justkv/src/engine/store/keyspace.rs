use super::helpers::{monotonic_now_ms, purge_if_expired};
use super::pattern::wildcard_match;
use super::Store;
use crate::engine::value::CompactKey;

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
