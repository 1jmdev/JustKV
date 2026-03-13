use rapidhash::fast::RandomState;
use hashbrown::HashMap;

use crate::helpers::purge_if_expired;
use crate::store::Store;
use types::value::{CompactArg, CompactKey, ZSetValueMap};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::{compare_member_score, get_zset};

impl Store {
    pub fn zinter(&self, keys: &[CompactArg]) -> Result<Vec<(CompactKey, f64)>, ()> {
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

    pub fn zstore_items(&self, destination: &[u8], items: &[(CompactKey, f64)]) -> Result<i64, ()> {
        let idx = self.shard_index(destination);
        let mut shard = self.shards[idx].write();
        let _ = purge_if_expired(&mut shard, destination, monotonic_now_ms());

        if items.is_empty() {
            let _ = shard.remove_key(destination);
            return Ok(0);
        }

        let mut zset = ZSetValueMap::with_capacity(items.len());
        for (member, score) in items {
            zset.insert(member.clone(), *score);
        }
        shard.insert_entry(
            CompactKey::from_slice(destination),
            types::value::Entry::ZSet(Box::new(zset)),
            None,
        );
        Ok(items.len() as i64)
    }

    fn zset_snapshots(&self, keys: &[CompactArg]) -> Result<Vec<ZSetValueMap>, ()> {
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
    let mut out: Vec<_> = values
        .iter()
        .map(|(member, score)| (member.clone(), *score))
        .collect();
    out.sort_by(|left, right| compare_member_score(left, right, reverse));
    out
}
