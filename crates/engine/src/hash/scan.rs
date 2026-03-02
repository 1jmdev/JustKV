use crate::store::Store;
use crate::value::{CompactKey, CompactValue};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::super::pattern::wildcard_match;
use super::get_hash_map;

impl Store {
    pub fn hscan(
        &self,
        key: &[u8],
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
    ) -> Result<(u64, Vec<(CompactKey, CompactValue)>), ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok((0, Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok((0, Vec::new()));
        };
        let map = get_hash_map(entry).ok_or(())?;

        if map.is_empty() {
            return Ok((0, Vec::new()));
        }

        let total_len = map.len();
        let mut index = usize::try_from(cursor).unwrap_or(usize::MAX).min(total_len);
        let target = count.max(1);
        let mut out = Vec::with_capacity(target);
        let mut iter = map.iter().skip(index);
        while out.len() < target {
            let Some((field, value)) = iter.next() else {
                break;
            };
            if pattern.is_none_or(|matcher| wildcard_match(matcher, field.as_slice())) {
                out.push((field.clone(), value.clone()));
            }
            index += 1;
        }

        let next_cursor = if index >= total_len { 0 } else { index as u64 };
        Ok((next_cursor, out))
    }
}
