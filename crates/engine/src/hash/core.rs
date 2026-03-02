use crate::store::Store;
use crate::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_pairs, get_hash_map, get_hash_map_mut};

impl Store {
    pub fn hset(&self, key: &[u8], pairs: &[(CompactArg, CompactArg)]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hset");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), Entry::empty_hash);

        let map = get_hash_map_mut(entry).ok_or(())?;
        let mut created = 0;
        for (field, value) in pairs {
            if map
                .insert(
                    CompactKey::from_slice(field.as_slice()),
                    CompactValue::from_slice(value.as_slice()),
                )
                .is_none()
            {
                created += 1;
            }
        }
        Ok(created)
    }

    pub fn hsetnx(&self, key: &[u8], field: &[u8], value: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hsetnx");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), Entry::empty_hash);
        let map = get_hash_map_mut(entry).ok_or(())?;

        let field_key = CompactKey::from_slice(field);
        if map.contains_key(field_key.as_slice()) {
            return Ok(0);
        }
        map.insert(field_key, CompactValue::from_slice(value));
        Ok(1)
    }

    pub fn hget(&self, key: &[u8], field: &[u8]) -> Result<Option<CompactValue>, ()> {
        let _trace = profiler::scope("engine::hash::core::hget");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(map.get(field).cloned())
    }

    pub fn hmget(
        &self,
        key: &[u8],
        fields: &[CompactArg],
    ) -> Result<Vec<Option<CompactValue>>, ()> {
        let _trace = profiler::scope("engine::hash::core::hmget");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(vec![None; fields.len()]);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(vec![None; fields.len()]);
        };
        let map = get_hash_map(entry).ok_or(())?;

        Ok(fields
            .iter()
            .map(|field| map.get(field.as_slice()).cloned())
            .collect())
    }

    pub fn hgetall(&self, key: &[u8]) -> Result<Vec<(CompactKey, CompactValue)>, ()> {
        let _trace = profiler::scope("engine::hash::core::hgetall");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(collect_pairs(map))
    }

    pub fn hdel(&self, key: &[u8], fields: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hdel");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let map = get_hash_map_mut(entry).ok_or(())?;

        let mut removed = 0;
        for field in fields {
            if map.remove(field.as_slice()).is_some() {
                removed += 1;
            }
        }
        if map.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(removed)
    }

    pub fn hexists(&self, key: &[u8], field: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hexists");
        Ok(self.hget(key, field)?.is_some() as i64)
    }

    pub fn hkeys(&self, key: &[u8]) -> Result<Vec<CompactKey>, ()> {
        let _trace = profiler::scope("engine::hash::core::hkeys");
        let pairs = self.hgetall(key)?;
        Ok(pairs.into_iter().map(|(field, _)| field).collect())
    }

    pub fn hvals(&self, key: &[u8]) -> Result<Vec<CompactValue>, ()> {
        let _trace = profiler::scope("engine::hash::core::hvals");
        let pairs = self.hgetall(key)?;
        Ok(pairs.into_iter().map(|(_, value)| value).collect())
    }

    pub fn hlen(&self, key: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hlen");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(map.len() as i64)
    }

    pub fn hstrlen(&self, key: &[u8], field: &[u8]) -> Result<usize, ()> {
        let _trace = profiler::scope("engine::hash::core::hstrlen");
        Ok(self.hget(key, field)?.map(|value| value.len()).unwrap_or(0))
    }
}
