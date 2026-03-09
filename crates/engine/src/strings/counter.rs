use crate::store::Store;
use types::value::Entry;

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringIntOpError {
    WrongType,
    InvalidInteger,
    Overflow,
}

impl Store {
    pub fn incr(&self, key: &[u8]) -> Result<i64, StringIntOpError> {
        let _trace = profiler::scope("engine::strings::counter::incr");
        self.incr_by(key, 1)
    }

    pub fn incr_by(&self, key: &[u8], delta: i64) -> Result<i64, StringIntOpError> {
        let _trace = profiler::scope("engine::strings::counter::incr_by");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let expired = purge_if_expired(&mut shard, key, now_ms);

        let mut buffer = itoa::Buffer::new();

        // Fast path: key exists and is a string — update in-place with get_mut,
        // avoiding a second full hash lookup that insert() would do.
        if !expired {
            if let Some(entry) = shard.entries.get_mut::<[u8]>(key) {
                let Some(value) = entry.as_string() else {
                    return Err(StringIntOpError::WrongType);
                };
                let text = std::str::from_utf8(value.slice())
                    .map_err(|_| StringIntOpError::InvalidInteger)?;
                let current = text
                    .parse::<i64>()
                    .map_err(|_| StringIntOpError::InvalidInteger)?;
                let next = current
                    .checked_add(delta)
                    .ok_or(StringIntOpError::Overflow)?;
                let encoded = buffer.format(next);
                entry.entry = Entry::from_slice(encoded.as_bytes());
                return Ok(next);
            }
        }

        let ttl_deadline = shard.ttl_deadline(key);
        let next = 0i64.checked_add(delta).ok_or(StringIntOpError::Overflow)?;
        let encoded = buffer.format(next);
        write_entry(
            &mut shard,
            key,
            Entry::from_slice(encoded.as_bytes()),
            ttl_deadline,
        );
        Ok(next)
    }

    pub fn incr_by_float(&self, key: &[u8], delta: f64) -> Result<f64, ()> {
        let _trace = profiler::scope("engine::strings::counter::incr_by_float");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let expired = purge_if_expired(&mut shard, key, now_ms);

        if !expired {
            if let Some(entry) = shard.entries.get_mut::<[u8]>(key) {
                let Some(value) = entry.as_string() else {
                    return Err(());
                };
                let text = std::str::from_utf8(value.slice()).map_err(|_| ())?;
                let current = text.parse::<f64>().map_err(|_| ())?;
                let next = current + delta;
                if !next.is_finite() {
                    return Err(());
                }
                let mut buffer = ryu::Buffer::new();
                let encoded = buffer.format(next);
                entry.entry = Entry::from_slice(encoded.as_bytes());
                return Ok(next);
            }
        }

        let next = delta;
        if !next.is_finite() {
            return Err(());
        }
        let ttl_deadline = shard.ttl_deadline(key);
        let mut buffer = ryu::Buffer::new();
        let encoded = buffer.format(next);
        write_entry(
            &mut shard,
            key,
            Entry::from_slice(encoded.as_bytes()),
            ttl_deadline,
        );
        Ok(next)
    }
}
