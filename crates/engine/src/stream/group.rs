use ahash::RandomState;
use hashbrown::HashMap;

use crate::Store;
use types::value::{CompactKey, Entry, StreamGroup, StreamId, StreamValue};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_stream_mut;

impl Store {
    pub fn xgroup_create(
        &self,
        key: &[u8],
        group: &[u8],
        id: StreamId,
        mkstream: bool,
    ) -> Result<bool, ()> {
        let _trace = profiler::scope("engine::stream::group::xgroup_create");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        if !shard.entries.contains_key(key) {
            if !mkstream {
                return Ok(false);
            }
            shard.insert_entry(
                CompactKey::from_slice(key),
                Entry::Stream(Box::new(StreamValue::new())),
                None,
            );
        }

        let stream = get_stream_mut(shard.entries.get_mut(key).ok_or(())?).ok_or(())?;
        let group_key = CompactKey::from_slice(group);
        if stream.groups.contains_key(group_key.as_slice()) {
            return Ok(false);
        }

        stream.groups.insert(
            group_key,
            StreamGroup {
                last_delivered: id,
                pending: HashMap::with_hasher(RandomState::new()),
            },
        );
        Ok(true)
    }

    pub fn xgroup_setid(&self, key: &[u8], group: &[u8], id: StreamId) -> Result<bool, ()> {
        let _trace = profiler::scope("engine::stream::group::xgroup_setid");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(false);
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(false);
        };
        let stream = get_stream_mut(entry).ok_or(())?;
        let Some(group_state) = stream.groups.get_mut(group) else {
            return Ok(false);
        };
        group_state.last_delivered = id;
        Ok(true)
    }

    pub fn xgroup_destroy(&self, key: &[u8], group: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::stream::group::xgroup_destroy");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let stream = get_stream_mut(entry).ok_or(())?;
        Ok(i64::from(stream.groups.remove(group).is_some()))
    }
}
