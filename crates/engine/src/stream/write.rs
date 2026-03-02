use crate::store::{Store, XAddId, XTrimMode};
use crate::value::{CompactArg, CompactKey, Entry, StreamId, StreamValue};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::types::{apply_trim, xadd_into_stream};
use super::{get_stream, get_stream_mut};

impl Store {
    pub fn xadd(
        &self,
        key: &[u8],
        id: XAddId,
        fields: &[(CompactArg, CompactArg)],
        trim: Option<(XTrimMode, StreamId, Option<usize>)>,
        nomkstream: bool,
    ) -> Result<Option<StreamId>, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::write::xadd");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let expired = purge_if_expired(&mut shard, key, now_ms);
        if expired && nomkstream {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            if nomkstream {
                return Ok(None);
            }
            shard.entries.insert(
                CompactKey::from_slice(key),
                Entry::Stream(Box::new(StreamValue::new())),
            );
            let created = shard.entries.get_mut(key).expect("stream created");
            let stream = get_stream_mut(created).ok_or(())?;
            return xadd_into_stream(stream, id, fields, trim);
        };

        let stream = get_stream_mut(entry).ok_or(())?;
        xadd_into_stream(stream, id, fields, trim)
    }

    pub fn xlen(&self, key: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::write::xlen");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }
        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let stream = get_stream(entry).ok_or(())?;
        Ok(stream.entries.len() as i64)
    }

    pub fn xdel(&self, key: &[u8], ids: &[StreamId]) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::write::xdel");
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

        let mut removed = 0i64;
        for id in ids {
            if stream.entries.remove(id).is_some() {
                removed += 1;
            }
            for group in stream.groups.values_mut() {
                let _ = group.pending.remove(id);
            }
        }
        Ok(removed)
    }

    pub fn xtrim(
        &self,
        key: &[u8],
        mode: XTrimMode,
        threshold: StreamId,
        limit: Option<usize>,
    ) -> Result<i64, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::write::xtrim");
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
        let before = stream.entries.len();
        apply_trim(stream, mode, threshold, limit);
        Ok((before.saturating_sub(stream.entries.len())) as i64)
    }
}
