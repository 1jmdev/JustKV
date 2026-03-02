use std::time::Duration;

use crate::store::Store;
use crate::value::{CompactValue, Entry};

use super::super::helpers::{deadline_from_ttl, monotonic_now_ms, purge_if_expired};
use super::write_entry;

impl Store {
    pub fn get(&self, key: &[u8]) -> Result<Option<CompactValue>, ()> {
        let _trace = profiler::scope("engine::strings::core::get");
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let Some(entry) = shard.entries.get::<[u8]>(key) else {
            return Ok(None);
        };
        if shard
            .ttl
            .get(key)
            .copied()
            .is_some_and(|deadline| now_ms >= deadline)
        {
            return Ok(None);
        }
        match entry.as_string() {
            Some(value) => Ok(Some(value.clone())),
            None => Err(()),
        }
    }

    pub fn set(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) {
        let _trace = profiler::scope("engine::strings::core::set");
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
        let _trace = profiler::scope("engine::strings::core::setnx");
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
        let _trace = profiler::scope("engine::strings::core::setxx");
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

    pub fn getset(&self, key: &[u8], value: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        let _trace = profiler::scope("engine::strings::core::getset");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let old_value = if purge_if_expired(&mut shard, key, now_ms) {
            None
        } else {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => match entry.as_string() {
                    Some(value) => Some(value.to_vec()),
                    None => return Err(()),
                },
                None => None,
            }
        };

        write_entry(&mut shard, key, Entry::from_slice(value), None);
        Ok(old_value)
    }

    pub fn getdel(&self, key: &[u8]) -> Result<Option<Vec<u8>>, ()> {
        let _trace = profiler::scope("engine::strings::core::getdel");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let _ = shard.clear_ttl(key);
        match shard.entries.remove::<[u8]>(key) {
            Some(entry) => match entry.into_string() {
                Some(value) => Ok(Some(value.into_vec())),
                None => Err(()),
            },
            None => Ok(None),
        }
    }

    pub fn append(&self, key: &[u8], suffix: &[u8]) -> Result<usize, ()> {
        let _trace = profiler::scope("engine::strings::core::append");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let mut base = if purge_if_expired(&mut shard, key, now_ms) {
            Vec::new()
        } else {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => match entry.as_string() {
                    Some(value) => value.to_vec(),
                    None => return Err(()),
                },
                None => Vec::new(),
            }
        };
        let ttl_deadline = shard.ttl.get(key).copied();

        base.extend_from_slice(suffix);
        let size = base.len();
        write_entry(&mut shard, key, Entry::new(base), ttl_deadline);
        Ok(size)
    }

    pub fn strlen(&self, key: &[u8]) -> Result<usize, ()> {
        let _trace = profiler::scope("engine::strings::core::strlen");
        let idx = self.shard_index(key);
        let now_ms = monotonic_now_ms();
        let shard = self.shards[idx].read();
        let Some(entry) = shard.entries.get::<[u8]>(key) else {
            return Ok(0);
        };
        if shard
            .ttl
            .get(key)
            .copied()
            .is_some_and(|deadline| now_ms >= deadline)
        {
            return Ok(0);
        }
        match entry.as_string() {
            Some(value) => Ok(value.len()),
            None => Err(()),
        }
    }
}
