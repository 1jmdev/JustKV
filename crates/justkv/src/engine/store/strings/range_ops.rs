use crate::engine::store::Store;
use crate::engine::value::Entry;

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

impl Store {
    pub fn setrange(&self, key: &[u8], offset: usize, value: &[u8]) -> Result<usize, ()> {
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
        if value.is_empty() {
            return Ok(base.len());
        }

        let ttl_deadline = shard.ttl.get(key).copied();
        let required_len = offset.saturating_add(value.len());
        if base.len() < required_len {
            base.resize(required_len, 0);
        }
        base[offset..required_len].copy_from_slice(value);

        let size = base.len();
        write_entry(&mut shard, key, Entry::new(base), ttl_deadline);
        Ok(size)
    }

    pub fn getrange(&self, key: &[u8], start: i64, end: i64) -> Result<Vec<u8>, ()> {
        let Some(value) = self.get(key)? else {
            return Ok(Vec::new());
        };

        let data = value.as_slice();
        let len = data.len() as i64;
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut start_index = if start < 0 { len + start } else { start };
        let mut end_index = if end < 0 { len + end } else { end };

        if start_index < 0 {
            start_index = 0;
        }
        if end_index < 0 {
            return Ok(Vec::new());
        }
        if end_index >= len {
            end_index = len - 1;
        }
        if start_index >= len || start_index > end_index {
            return Ok(Vec::new());
        }

        Ok(data[start_index as usize..=end_index as usize].to_vec())
    }
}
