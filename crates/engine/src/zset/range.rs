use crate::store::Store;
use crate::value::CompactKey;

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::{get_zset, normalize_range, sorted_by_score};

impl Store {
    pub fn zrange(
        &self,
        key: &[u8],
        start: i64,
        stop: i64,
        reverse: bool,
    ) -> Result<Vec<(CompactKey, f64)>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let zset = get_zset(entry).ok_or(())?;
        let ordered = sorted_by_score(zset, reverse);

        let Some((from, to_exclusive)) = normalize_range(start, stop, ordered.len()) else {
            return Ok(Vec::new());
        };
        Ok(ordered[from..to_exclusive].to_vec())
    }

    pub fn zrange_by_score(
        &self,
        key: &[u8],
        min: f64,
        max: f64,
        reverse: bool,
        offset: usize,
        count: Option<usize>,
    ) -> Result<Vec<(CompactKey, f64)>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let zset = get_zset(entry).ok_or(())?;

        let filtered: Vec<_> = zset
            .iter_ordered(reverse)
            .filter(|(_, score)| *score >= min && *score <= max)
            .map(|(member, score)| (member.clone(), score))
            .collect();

        if offset >= filtered.len() {
            return Ok(Vec::new());
        }

        let mut sliced = filtered.into_iter().skip(offset);
        let out = if let Some(limit) = count {
            sliced.by_ref().take(limit).collect()
        } else {
            sliced.collect()
        };
        Ok(out)
    }
}
