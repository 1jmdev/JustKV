use crate::store::{ListSide, Store};
use crate::value::{CompactArg, CompactKey, CompactValue};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_list_mut;

impl Store {
    pub fn list_pop_first(
        &self,
        keys: &[CompactArg],
        side: ListSide,
    ) -> Result<Option<(CompactKey, CompactValue)>, ()> {
        for key in keys {
            let idx = self.shard_index(key.as_slice());
            let mut shard = self.shards[idx].write();
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key.as_slice(), now_ms) {
                continue;
            }

            let Some(entry) = shard.entries.get_mut(key.as_slice()) else {
                continue;
            };
            let list = get_list_mut(entry).ok_or(())?;
            let popped = match side {
                ListSide::Left => list.pop_front(),
                ListSide::Right => list.pop_back(),
            };

            if let Some(value) = popped {
                if list.is_empty() {
                    let _ = shard.remove_key(key.as_slice());
                }
                return Ok(Some((CompactKey::from_slice(key.as_slice()), value)));
            }
        }

        Ok(None)
    }

    pub fn lmpop(
        &self,
        keys: &[CompactArg],
        side: ListSide,
        count: usize,
    ) -> Result<Option<(CompactKey, Vec<CompactValue>)>, ()> {
        let take = count.max(1);
        for key in keys {
            let idx = self.shard_index(key.as_slice());
            let mut shard = self.shards[idx].write();
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key.as_slice(), now_ms) {
                continue;
            }

            let Some(entry) = shard.entries.get_mut(key.as_slice()) else {
                continue;
            };
            let list = get_list_mut(entry).ok_or(())?;
            if list.is_empty() {
                let _ = shard.remove_key(key.as_slice());
                continue;
            }

            let mut values = Vec::with_capacity(take.min(list.len()));
            for _ in 0..take {
                let popped = match side {
                    ListSide::Left => list.pop_front(),
                    ListSide::Right => list.pop_back(),
                };
                if let Some(value) = popped {
                    values.push(value);
                } else {
                    break;
                }
            }

            if values.is_empty() {
                continue;
            }
            if list.is_empty() {
                let _ = shard.remove_key(key.as_slice());
            }

            return Ok(Some((CompactKey::from_slice(key.as_slice()), values)));
        }

        Ok(None)
    }
}
