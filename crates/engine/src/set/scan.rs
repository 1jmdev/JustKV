use crate::store::Store;
use crate::value::CompactKey;

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::super::pattern::wildcard_match;
use super::get_set;

impl Store {
    pub fn sscan(
        &self,
        key: &[u8],
        cursor: u64,
        pattern: Option<&[u8]>,
        count: usize,
    ) -> Result<(u64, Vec<CompactKey>), ()> {
        let _trace = profiler::scope("crates::engine::src::set::scan::sscan");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok((0, Vec::new()));
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok((0, Vec::new()));
        };
        let set = get_set(entry).ok_or(())?;

        if set.is_empty() {
            return Ok((0, Vec::new()));
        }

        let total_len = set.len();
        let mut index = usize::try_from(cursor).unwrap_or(usize::MAX).min(total_len);
        let target = count.max(1);
        let mut out = Vec::with_capacity(target);
        let mut iter = set.iter().skip(index);
        while out.len() < target {
            let Some(member) = iter.next() else {
                break;
            };
            if pattern.is_none_or(|matcher| wildcard_match(matcher, member.as_slice())) {
                out.push(member.clone());
            }
            index += 1;
        }

        let next = if index >= total_len { 0 } else { index as u64 };
        Ok((next, out))
    }
}
