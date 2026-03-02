use crate::store::Store;
use crate::value::Entry;

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

impl Store {
    pub fn incr(&self, key: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::strings::counter::incr");
        self.incr_by(key, 1)
    }

    pub fn incr_by(&self, key: &[u8], delta: i64) -> Result<i64, ()> {
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
                    return Err(());
                };
                let text = std::str::from_utf8(value.slice()).map_err(|_| ())?;
                let current = text.parse::<i64>().map_err(|_| ())?;
                let next = current.checked_add(delta).ok_or(())?;
                let encoded = buffer.format(next);
                // Replace string value in-place — no re-hash needed.
                *entry = Entry::from_slice(encoded.as_bytes());
                return Ok(next);
            }
        }

        // Slow path: key absent or just expired — insert new entry.
        let ttl_deadline = shard.ttl.get(key).copied();
        let next = 0i64.checked_add(delta).ok_or(())?;
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
