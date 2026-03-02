use crate::store::Store;
use crate::value::{CompactKey, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_members, get_set};

impl Store {
    pub fn spop(&self, key: &[u8], count: usize) -> Result<Option<Vec<CompactKey>>, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get_mut::<[u8]>(key) else {
            return Ok(None);
        };
        let Entry::Set(set) = entry else {
            return Err(());
        };
        if set.is_empty() {
            return Ok(None);
        }

        let members = collect_members(set);
        if members.is_empty() {
            return Ok(None);
        }

        let take = count.min(members.len());
        let start = (monotonic_now_ms() as usize) % members.len();
        let mut out = Vec::with_capacity(take);
        for i in 0..take {
            let member = members[(start + i) % members.len()].clone();
            if set.remove(member.as_slice()) {
                out.push(member);
            }
        }

        if set.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(Some(out))
    }

    pub fn srandmember(&self, key: &[u8], count: i64) -> Result<Option<Vec<CompactKey>>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get::<[u8]>(key) else {
            return Ok(None);
        };
        let Some(set) = get_set(entry) else {
            return Err(());
        };
        let members = collect_members(set);
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
