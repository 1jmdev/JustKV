use crate::store::{HashFloatOpError, HashIntOpError, Store};
use crate::value::{CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_hash_map_mut;

impl Store {
    pub fn hincrby(&self, key: &[u8], field: &[u8], delta: i64) -> Result<i64, HashIntOpError> {
        let _trace = profiler::scope("crates::engine::src::hash::counter::hincrby");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), Entry::empty_hash);
        let map = get_hash_map_mut(entry).ok_or(HashIntOpError::WrongType)?;

        let current = match map.get(field) {
            Some(value) => {
                let text = std::str::from_utf8(value.as_slice())
                    .map_err(|_| HashIntOpError::InvalidInteger)?;
                text.parse::<i64>()
                    .map_err(|_| HashIntOpError::InvalidInteger)?
            }
            None => 0,
        };
        let next = current.checked_add(delta).ok_or(HashIntOpError::Overflow)?;
        let mut buffer = itoa::Buffer::new();
        let encoded = buffer.format(next);

        map.insert(
            CompactKey::from_slice(field),
            CompactValue::from_slice(encoded.as_bytes()),
        );
        Ok(next)
    }

    pub fn hincrbyfloat(
        &self,
        key: &[u8],
        field: &[u8],
        delta: f64,
    ) -> Result<f64, HashFloatOpError> {
        let _trace = profiler::scope("crates::engine::src::hash::counter::hincrbyfloat");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), Entry::empty_hash);
        let map = get_hash_map_mut(entry).ok_or(HashFloatOpError::WrongType)?;

        let current = match map.get(field) {
            Some(value) => {
                let text = std::str::from_utf8(value.as_slice())
                    .map_err(|_| HashFloatOpError::InvalidFloat)?;
                text.parse::<f64>()
                    .map_err(|_| HashFloatOpError::InvalidFloat)?
            }
            None => 0.0,
        };
        let next = current + delta;
        if !next.is_finite() {
            return Err(HashFloatOpError::InvalidFloat);
        }
        let mut buffer = ryu::Buffer::new();
        let encoded = buffer.format(next);

        map.insert(
            CompactKey::from_slice(field),
            CompactValue::from_slice(encoded.as_bytes()),
        );
        Ok(next)
    }
}
