use ahash::RandomState;
use hashbrown::HashMap;

use crate::store::Store;
use crate::value::{CompactArg, CompactKey, ZSetValueMap};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::{compare_member_score, get_zset};

impl Store {
    pub fn zinter(&self, keys: &[CompactArg]) -> Result<Vec<(CompactKey, f64)>, ()> {
        let _trace = profiler::scope("crates::engine::src::zset::algebra::zinter");
        let snapshots = self.zset_snapshots(keys)?;
        if snapshots.is_empty() || snapshots.iter().any(|set| set.is_empty()) {
            return Ok(Vec::new());
        }

        let mut out = HashMap::with_hasher(RandomState::new());
        let (first, rest) = snapshots.split_first().expect("checked non-empty");
        for (member, score) in first.iter_member_scores() {
            if rest.iter().all(|set| set.contains_key(member.as_slice())) {
                let total = rest.iter().fold(score, |acc, set| {
                    acc + set.get(member.as_slice()).unwrap_or(0.0)
                });
                out.insert(member.clone(), total);
            }
        }
        Ok(sort_snapshot(&out, false))
    }

    pub fn zunion(&self, keys: &[CompactArg]) -> Result<Vec<(CompactKey, f64)>, ()> {
        let _trace = profiler::scope("crates::engine::src::zset::algebra::zunion");
        let snapshots = self.zset_snapshots(keys)?;
        let mut out = HashMap::with_hasher(RandomState::new());
        for set in snapshots {
            for (member, score) in set.iter_member_scores() {
                let next = out.get(member.as_slice()).copied().unwrap_or(0.0) + score;
                out.insert(member.clone(), next);
            }
        }
        Ok(sort_snapshot(&out, false))
    }

    pub fn zdiff(&self, keys: &[CompactArg]) -> Result<Vec<(CompactKey, f64)>, ()> {
        let _trace = profiler::scope("crates::engine::src::zset::algebra::zdiff");
        let snapshots = self.zset_snapshots(keys)?;
        let Some((first, rest)) = snapshots.split_first() else {
            return Ok(Vec::new());
        };

        let mut out = HashMap::with_hasher(RandomState::new());
        for (member, score) in first.iter_member_scores() {
            if rest.iter().all(|set| !set.contains_key(member.as_slice())) {
                out.insert(member.clone(), score);
            }
        }
        Ok(sort_snapshot(&out, false))
    }

    fn zset_snapshots(&self, keys: &[CompactArg]) -> Result<Vec<ZSetValueMap>, ()> {
        let _trace = profiler::scope("crates::engine::src::zset::algebra::zset_snapshots");
        let mut snapshots = Vec::with_capacity(keys.len());
        let now_ms = monotonic_now_ms();
        for key in keys {
            let idx = self.shard_index(key.as_slice());
            let shard = self.shards[idx].read();
            if is_expired(&shard, key.as_slice(), now_ms) {
                snapshots.push(ZSetValueMap::new());
                continue;
            }

            match shard.entries.get(key.as_slice()) {
                None => snapshots.push(ZSetValueMap::new()),
                Some(entry) => {
                    let zset = get_zset(entry).ok_or(())?;
                    snapshots.push(zset.clone());
                }
            }
        }
        Ok(snapshots)
    }
}

fn sort_snapshot(
    values: &HashMap<CompactKey, f64, RandomState>,
    reverse: bool,
) -> Vec<(CompactKey, f64)> {
    let _trace = profiler::scope("crates::engine::src::zset::algebra::sort_snapshot");
    let mut out: Vec<_> = values
        .iter()
        .map(|(member, score)| (member.clone(), *score))
        .collect();
    out.sort_by(|left, right| compare_member_score(left, right, reverse));
    out
}
