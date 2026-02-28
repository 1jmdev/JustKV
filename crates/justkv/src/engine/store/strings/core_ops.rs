use std::time::Duration;

use crate::engine::store::Store;
use crate::engine::value::{CompactValue, Entry};

use super::super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired};
use super::write_entry;

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

    pub fn set(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        write_entry(
            &mut shard,
            key,
            Entry::from_slice(value),
            ttl.map(deadline_from_ttl),
        );
    }

    pub fn setnx(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if !purge_if_expired(&mut shard, key, now_ms) && shard.entries.contains_key(key) {
            return false;
        }

        write_entry(
            &mut shard,
            key,
            Entry::from_slice(value),
            ttl.map(deadline_from_ttl),
        );
        true
    }

    pub fn setxx(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) || !shard.entries.contains_key(key) {
            return false;
        }

        write_entry(
            &mut shard,
            key,
            Entry::from_slice(value),
            ttl.map(deadline_from_ttl),
        );
        true
    }

    pub fn getset(&self, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let old_value = if purge_if_expired(&mut shard, key, now_ms) {
            None
        } else {
            shard.entries.get(key).map(|entry| entry.value.to_vec())
        };

        write_entry(&mut shard, key, Entry::from_slice(value), None);
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

    pub fn append(&self, key: &[u8], suffix: &[u8]) -> usize {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let mut base = if purge_if_expired(&mut shard, key, now_ms) {
            Vec::new()
        } else {
            shard
                .entries
                .get(key)
                .map(|entry| entry.value.to_vec())
                .unwrap_or_default()
        };
        let ttl_deadline = shard.ttl.get(key).copied();

        base.extend_from_slice(suffix);
        let size = base.len();
        write_entry(&mut shard, key, Entry::new(base), ttl_deadline);
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
}
