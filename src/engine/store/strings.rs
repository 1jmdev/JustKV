use std::time::Duration;

use crate::engine::value::Entry;

use super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired};
use super::Store;

impl Store {
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        shard
            .get(key)
            .filter(|entry| !entry.is_expired(now_ms))
            .map(|entry| entry.value.to_vec())
    }

    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) {
        let idx = self.shard_index(&key);
        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        self.shards[idx]
            .write()
            .insert(key.into_boxed_slice(), Entry::new(value, expires_at_ms));
    }

    pub fn setnx(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if !purge_if_expired(&mut shard, &key, now_ms) && shard.contains_key(key.as_slice()) {
            return false;
        }

        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        shard.insert(key.into_boxed_slice(), Entry::new(value, expires_at_ms));
        true
    }

    pub fn setxx(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, &key, now_ms) || !shard.contains_key(key.as_slice()) {
            return false;
        }

        let expires_at_ms = ttl.map(deadline_from_ttl).unwrap_or(0);
        shard.insert(key.into_boxed_slice(), Entry::new(value, expires_at_ms));
        true
    }

    pub fn getset(&self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        let idx = self.shard_index(&key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let old_value = if purge_if_expired(&mut shard, &key, now_ms) {
            None
        } else {
            shard.get(key.as_slice()).map(|entry| entry.value.to_vec())
        };

        shard.insert(key.into_boxed_slice(), Entry::new(value, 0));
        old_value
    }

    pub fn getdel(&self, key: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return None;
        }
        shard.remove(key).map(|entry| entry.value.into_vec())
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
                .get(key_slice)
                .map(|entry| entry.value.to_vec())
                .unwrap_or_default()
        };

        base.extend_from_slice(suffix);
        let size = base.len();
        shard.insert(key.into_boxed_slice(), Entry::new(base, 0));
        size
    }

    pub fn strlen(&self, key: &[u8]) -> usize {
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        shard
            .get(key)
            .filter(|entry| !entry.is_expired(now_ms))
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
            match shard.get(key) {
                Some(entry) => {
                    let text = std::str::from_utf8(&entry.value).map_err(|_| ())?;
                    text.parse::<i64>().map_err(|_| ())?
                }
                None => 0,
            }
        };

        let next = current.checked_add(delta).ok_or(())?;
        shard.insert(
            key.to_vec().into_boxed_slice(),
            Entry::new(next.to_string().into_bytes(), 0),
        );
        Ok(next)
    }

    pub fn mget(&self, keys: &[Vec<u8>]) -> Vec<Option<Vec<u8>>> {
        keys.iter().map(|key| self.get(key)).collect()
    }

    pub fn mset(&self, pairs: &[(Vec<u8>, Vec<u8>)]) {
        for (key, value) in pairs {
            self.set(key.clone(), value.clone(), None);
        }
    }

    pub fn msetnx(&self, pairs: &[(Vec<u8>, Vec<u8>)]) -> bool {
        let now_ms = monotonic_now_ms();
        for (key, _) in pairs {
            let idx = self.shard_index(key);
            let mut shard = self.shards[idx].write();
            if !purge_if_expired(&mut shard, key, now_ms) && shard.contains_key(key.as_slice()) {
                return false;
            }
        }

        for (key, value) in pairs {
            self.set(key.clone(), value.clone(), None);
        }
        true
    }
}
