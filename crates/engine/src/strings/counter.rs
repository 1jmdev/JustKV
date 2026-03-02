use crate::store::Store;
use crate::value::Entry;

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

impl Store {
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
                    let Some(value) = entry.as_string() else {
                        return Err(());
                    };
                    let text = std::str::from_utf8(value.as_slice()).map_err(|_| ())?;
                    text.parse::<i64>().map_err(|_| ())?
                }
                None => 0,
            }
        };

        let ttl_deadline = shard.ttl.get(key).copied();
        let next = current.checked_add(delta).ok_or(())?;
        let mut buffer = itoa::Buffer::new();
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
