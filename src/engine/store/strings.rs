use std::time::Duration;

use crate::engine::value::{CompactKey, CompactValue, Entry};

use super::Store;
use super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired};

impl Store {
    pub fn get(&self, key: &[u8]) -> Option<CompactValue> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        shard
            .entries
            .get(key)
            .filter(|_| {
                shard
                    .ttl
                    .get(key)
                    .copied()
                    .is_none_or(|deadline| now_ms < deadline)
            })
            .map(|entry| entry.value.clone())
    }

    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) {
        let idx = self.shard_index(&key);
        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        let mut shard = self.shards[idx].write();
        let compact_key = CompactKey::from_vec(key);
        shard.entries.insert(compact_key.clone(), Entry::new(value));
        if expires_at_ms == 0 {
            shard.ttl.remove(compact_key.as_slice());
        } else {
            shard.ttl.insert(compact_key, expires_at_ms);
        }
    }

    pub fn setnx(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if !purge_if_expired(&mut shard, &key, now_ms) && shard.entries.contains_key(key.as_slice())
        {
            return false;
        }

        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        let compact_key = CompactKey::from_vec(key);
        shard.entries.insert(compact_key.clone(), Entry::new(value));
        if expires_at_ms == 0 {
            shard.ttl.remove(compact_key.as_slice());
        } else {
            shard.ttl.insert(compact_key, expires_at_ms);
        }
        true
    }

    pub fn setxx(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, &key, now_ms) || !shard.entries.contains_key(key.as_slice())
        {
            return false;
        }

        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        let compact_key = CompactKey::from_vec(key);
        shard.entries.insert(compact_key.clone(), Entry::new(value));
        if expires_at_ms == 0 {
            shard.ttl.remove(compact_key.as_slice());
        } else {
            shard.ttl.insert(compact_key, expires_at_ms);
        }
        true
    }

    pub fn getset(&self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let old_value = if purge_if_expired(&mut shard, &key, now_ms) {
            None
        } else {
            shard
                .entries
                .get(key.as_slice())
                .map(|entry| entry.value.to_vec())
        };

        let compact_key = CompactKey::from_vec(key);
        shard.entries.insert(compact_key.clone(), Entry::new(value));
        shard.ttl.remove(compact_key.as_slice());
        old_value
    }

    pub fn getdel(&self, key: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return None;
        }
        shard.ttl.remove(key);
        shard
            .entries
            .remove(key)
            .map(|entry| entry.value.into_vec())
    }

    pub fn append(&self, key: Vec<u8>, suffix: &[u8]) -> usize {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let key_slice = key.as_slice();

        let mut base = if purge_if_expired(&mut shard, key_slice, now_ms) {
            Vec::new()
        } else {
            shard
                .entries
                .get(key_slice)
                .map(|entry| entry.value.to_vec())
                .unwrap_or_default()
        };

        base.extend_from_slice(suffix);
        let size = base.len();
        let compact_key = CompactKey::from_vec(key);
        shard.entries.insert(compact_key.clone(), Entry::new(base));
        shard.ttl.remove(compact_key.as_slice());
        size
    }

    pub fn strlen(&self, key: &[u8]) -> usize {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        shard
            .entries
            .get(key)
            .filter(|_| {
                shard
                    .ttl
                    .get(key)
                    .copied()
                    .is_none_or(|deadline| now_ms < deadline)
            })
            .map_or(0, |entry| entry.value.len())
    }

    pub fn incr(&self, key: &[u8]) -> Result<i64, ()> {
        self.incr_by(key, 1)
    }

    pub fn incr_by(&self, key: &[u8], delta: i64) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let current = if purge_if_expired(&mut shard, key, now_ms) {
            0
        } else {
            match shard.entries.get(key) {
                Some(entry) => {
                    let text = std::str::from_utf8(entry.value.as_slice()).map_err(|_| ())?;
                    text.parse::<i64>().map_err(|_| ())?
                }
                None => 0,
            }
        };

        let next = current.checked_add(delta).ok_or(())?;
        let compact_key = CompactKey::from_vec(key.to_vec());
        shard.entries.insert(
            compact_key.clone(),
            Entry::new(next.to_string().into_bytes()),
        );
        shard.ttl.remove(compact_key.as_slice());
        Ok(next)
    }

    pub fn mget(&self, keys: &[Vec<u8>]) -> Vec<Option<CompactValue>> {
        keys.iter().map(|key| self.get(key)).collect()
    }

    pub fn mset(&self, pairs: Vec<(Vec<u8>, Vec<u8>)>) {
        let shard_count = self.shards.len();
        let mut grouped = vec![Vec::new(); shard_count];

        for (key, value) in pairs {
            let idx = self.shard_index(&key);
            grouped[idx].push((CompactKey::from_vec(key), Entry::new(value)));
        }

        for (idx, entries) in grouped.into_iter().enumerate() {
            if entries.is_empty() {
                continue;
            }

            let mut shard = self.shards[idx].write();
            for (key, entry) in entries {
                shard.ttl.remove(key.as_slice());
                shard.entries.insert(key, entry);
            }
        }
    }

    pub fn msetnx(&self, pairs: Vec<(Vec<u8>, Vec<u8>)>) -> bool {
        let now_ms = monotonic_now_ms();
        for (key, _) in &pairs {
            let idx = self.shard_index(key);
            let mut shard = self.shards[idx].write();
            if !purge_if_expired(&mut shard, key, now_ms)
                && shard.entries.contains_key(key.as_slice())
            {
                return false;
            }
        }

        self.mset(pairs);
        true
    }
}
