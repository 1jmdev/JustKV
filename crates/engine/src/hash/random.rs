use crate::store::Store;
use crate::value::{CompactKey, CompactValue};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::{collect_pairs, get_hash_map};

impl Store {
    pub fn hrandfield_one(&self, key: &[u8]) -> Result<Option<CompactKey>, ()> {
        let selected = self.hrandfield_pairs(key, 1)?;
        Ok(selected.into_iter().next().map(|(field, _)| field))
    }

    pub fn hrandfield_pairs(
        &self,
        key: &[u8],
        count: i64,
    ) -> Result<Vec<(CompactKey, CompactValue)>, ()> {
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
        let pairs = collect_pairs(map);
        if pairs.is_empty() {
            return Ok(Vec::new());
        }

        let target = count.unsigned_abs() as usize;
        if count >= 0 {
            let take = target.min(pairs.len());
            let start = (monotonic_now_ms() as usize) % pairs.len();
            let mut out = Vec::with_capacity(take);
            for i in 0..take {
                out.push(pairs[(start + i) % pairs.len()].clone());
            }
            Ok(out)
        } else {
            let mut out = Vec::with_capacity(target);
            let base = monotonic_now_ms() as usize;
            for i in 0..target {
                let index = (base + i) % pairs.len();
                out.push(pairs[index].clone());
            }
            Ok(out)
        }
    }
}
