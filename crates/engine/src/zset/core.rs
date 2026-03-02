use crate::store::Store;
use crate::value::{CompactArg, CompactKey, Entry, ZSetValueMap};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{get_zset, get_zset_mut, sorted_by_score};

impl Store {
    pub fn zadd(&self, key: &[u8], pairs: &[(f64, CompactArg)]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::zset::core::zadd");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                Entry::ZSet(Box::new(new_zset()))
            });
        let zset = get_zset_mut(entry).ok_or(())?;

        let mut added = 0;
        for (score, member) in pairs {
            let member_key = CompactKey::from_slice(member.as_slice());
            if zset.insert(member_key, *score).is_none() {
                added += 1;
            }
        }
        Ok(added)
    }

    pub fn zrem(&self, key: &[u8], members: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::zset::core::zrem");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let zset = get_zset_mut(entry).ok_or(())?;

        let mut removed = 0;
        for member in members {
            if zset.remove(member.as_slice()).is_some() {
                removed += 1;
            }
        }
        if zset.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(removed)
    }

    pub fn zcard(&self, key: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::zset::core::zcard");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let zset = get_zset(entry).ok_or(())?;
        Ok(zset.len() as i64)
    }

    pub fn zcount(&self, key: &[u8], min: f64, max: f64) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::zset::core::zcount");
        if min > max {
            return Ok(0);
        }
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let zset = get_zset(entry).ok_or(())?;
        Ok(zset
            .iter_member_scores()
            .filter(|(_, score)| *score >= min && *score <= max)
            .count() as i64)
    }

    pub fn zscore(&self, key: &[u8], member: &[u8]) -> Result<Option<f64>, ()> {
        let _trace = profiler::scope("engine::zset::core::zscore");
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
        Ok(zset.get(member))
    }

    pub fn zincrby(&self, key: &[u8], increment: f64, member: &[u8]) -> Result<f64, ()> {
        let _trace = profiler::scope("engine::zset::core::zincrby");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                Entry::ZSet(Box::new(new_zset()))
            });
        let zset = get_zset_mut(entry).ok_or(())?;

        let member_key = CompactKey::from_slice(member);
        let current = zset.get(member).unwrap_or(0.0);
        let next = current + increment;
        zset.insert(member_key, next);
        Ok(next)
    }

    pub fn zmscore(&self, key: &[u8], members: &[CompactArg]) -> Result<Vec<Option<f64>>, ()> {
        let _trace = profiler::scope("engine::zset::core::zmscore");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(vec![None; members.len()]);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(vec![None; members.len()]);
        };
        let zset = get_zset(entry).ok_or(())?;
        Ok(members
            .iter()
            .map(|member| zset.get(member.as_slice()))
            .collect())
    }

    pub fn zrank(&self, key: &[u8], member: &[u8], reverse: bool) -> Result<Option<i64>, ()> {
        let _trace = profiler::scope("engine::zset::core::zrank");
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
        let ordered = sorted_by_score(zset, reverse);
        Ok(ordered
            .iter()
            .position(|(current, _)| current.as_slice() == member)
            .map(|index| index as i64))
    }

    pub fn zremrangebyrank(&self, key: &[u8], start: i64, stop: i64) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::zset::core::zremrangebyrank");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let zset = get_zset_mut(entry).ok_or(())?;
        let ordered = sorted_by_score(zset, false);
        let Some((from, to_exclusive)) = super::normalize_range(start, stop, ordered.len()) else {
            return Ok(0);
        };

        let mut removed = 0i64;
        for (member, _) in &ordered[from..to_exclusive] {
            if zset.remove(member.as_slice()).is_some() {
                removed += 1;
            }
        }
        if zset.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(removed)
    }
}

fn new_zset() -> ZSetValueMap {
    let _trace = profiler::scope("engine::zset::core::new_zset");
    ZSetValueMap::new()
}
