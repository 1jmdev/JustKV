use crate::engine::store::Store;
use crate::engine::value::{CompactKey, CompactValue};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::super::pattern::wildcard_match;
use super::{collect_pairs, get_hash_map};

impl Store {
    pub fn hscan(
        &self,
        key: &[u8],
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
    ) -> Result<(u64, Vec<(CompactKey, CompactValue)>), ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok((0, Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok((0, Vec::new()));
        };
        let map = get_hash_map(entry).ok_or(())?;

        let pairs = collect_pairs(map);
        if pairs.is_empty() {
            return Ok((0, Vec::new()));
        }

        let mut index = usize::try_from(cursor)
            .unwrap_or(usize::MAX)
            .min(pairs.len());
        let target = count.max(1);
        let mut out = Vec::with_capacity(target);
        while index < pairs.len() && out.len() < target {
            let pair = &pairs[index];
            if pattern.is_none_or(|matcher| wildcard_match(matcher, pair.0.as_slice())) {
                out.push(pair.clone());
            }
            index += 1;
        }

        let next_cursor = if index >= pairs.len() {
            0
        } else {
            index as u64
        };
        Ok((next_cursor, out))
    }
}
