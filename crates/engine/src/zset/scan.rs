use crate::store::Store;
use crate::value::CompactKey;

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::super::pattern::wildcard_match;
use super::{get_zset, sorted_by_score_refs};

impl Store {
    pub fn zscan(
        &self,
        key: &[u8],
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
    ) -> Result<(u64, Vec<(CompactKey, f64)>), ()> {
        let _trace = profiler::scope("engine::zset::scan::zscan");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok((0, Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok((0, Vec::new()));
        };
        let zset = get_zset(entry).ok_or(())?;
        let ordered = sorted_by_score_refs(zset, false);
        if ordered.is_empty() {
            return Ok((0, Vec::new()));
        }

        let mut index = usize::try_from(cursor)
            .unwrap_or(usize::MAX)
            .min(ordered.len());
        let target = count.max(1);
        let mut out = Vec::with_capacity(target);
        while index < ordered.len() && out.len() < target {
            let item = &ordered[index];
            if pattern.is_none_or(|matcher| wildcard_match(matcher, item.0.as_slice())) {
                out.push((item.0.clone(), item.1));
            }
            index += 1;
        }

        let next = if index >= ordered.len() {
            0
        } else {
            index as u64
        };
        Ok((next, out))
    }
}
