use crate::engine::store::Store;
use crate::engine::value::{CompactArg, CompactKey};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_zset_mut;

impl Store {
    pub fn zpopmin(&self, key: &[u8], count: usize) -> Result<Option<Vec<(CompactKey, f64)>>, ()> {
        self.zpop_edge(key, count, false)
    }

    pub fn zpopmax(&self, key: &[u8], count: usize) -> Result<Option<Vec<(CompactKey, f64)>>, ()> {
        self.zpop_edge(key, count, true)
    }

    pub fn bzpop_edge(
        &self,
        keys: &[CompactArg],
        max: bool,
    ) -> Result<Option<(CompactKey, CompactKey, f64)>, ()> {
        for key in keys {
            let popped = if max {
                self.zpopmax(key.as_slice(), 1)?
            } else {
                self.zpopmin(key.as_slice(), 1)?
            };
            if let Some(mut items) = popped
                && let Some((member, score)) = items.pop()
            {
                return Ok(Some((
                    CompactKey::from_slice(key.as_slice()),
                    member,
                    score,
                )));
            }
        }
        Ok(None)
    }

    pub fn zmpop(
        &self,
        keys: &[CompactArg],
        max: bool,
        count: usize,
    ) -> Result<Option<(CompactKey, Vec<(CompactKey, f64)>)>, ()> {
        let take = count.max(1);
        for key in keys {
            let popped = if max {
                self.zpopmax(key.as_slice(), take)?
            } else {
                self.zpopmin(key.as_slice(), take)?
            };
            if let Some(items) = popped
                && !items.is_empty()
            {
                return Ok(Some((CompactKey::from_slice(key.as_slice()), items)));
            }
        }
        Ok(None)
    }

    fn zpop_edge(
        &self,
        key: &[u8],
        count: usize,
        max: bool,
    ) -> Result<Option<Vec<(CompactKey, f64)>>, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(None);
        };
        let zset = get_zset_mut(entry).ok_or(())?;
        if zset.is_empty() {
            return Ok(None);
        }

        let mut out: Vec<(CompactKey, f64)> = zset.iter_ordered(max).take(count).map(|(member, score)| (member.clone(), score)).collect();
        out.retain(|(member, _)| zset.remove(member.as_slice()).is_some());

        if zset.is_empty() {
            shard.entries.remove(key);
            shard.ttl.remove(key);
        }
        Ok(Some(out))
    }
}
