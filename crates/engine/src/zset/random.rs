use crate::store::Store;
use crate::value::CompactKey;

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::{get_zset, sorted_by_score};

impl Store {
    pub fn zrandmember(
        &self,
        key: &[u8],
        count: i64,
    ) -> Result<Option<Vec<(CompactKey, f64)>>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let zset = get_zset(entry).ok_or(())?;
        let members = sorted_by_score(zset, false);
        if members.is_empty() {
            return Ok(None);
        }

        let start = (monotonic_now_ms() as usize) % members.len();
        if count >= 0 {
            let take = (count as usize).min(members.len());
            let mut out = Vec::with_capacity(take);
            for i in 0..take {
                out.push(members[(start + i) % members.len()].clone());
            }
            Ok(Some(out))
        } else {
            let take = count.unsigned_abs() as usize;
            let mut out = Vec::with_capacity(take);
            for i in 0..take {
                out.push(members[(start + i) % members.len()].clone());
            }
            Ok(Some(out))
        }
    }
}
