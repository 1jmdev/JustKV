use crate::Store;
use crate::store::{XAddId, XTrimMode};
use types::value::{CompactArg, CompactKey, Entry, StreamId, StreamValue};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::stream_types::{apply_trim, xadd_into_stream};
use super::{get_stream, get_stream_mut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamWriteError {
    WrongType,
    InternalInvariant,
}

impl Store {
    pub fn xadd(
        &self,
        key: &[u8],
        id: XAddId,
        fields: &[(CompactArg, CompactArg)],
        trim: Option<(XTrimMode, StreamId, Option<usize>)>,
        nomkstream: bool,
    ) -> Result<Option<StreamId>, StreamWriteError> {
        let _trace = profiler::scope("engine::stream::write::xadd");
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
            shard.insert_entry(
                CompactKey::from_slice(key),
                Entry::Stream(Box::new(StreamValue::new())),
                None,
            );
            let Some(created) = shard.entries.get_mut(key) else {
                return Err(StreamWriteError::InternalInvariant);
            };
            let stream = get_stream_mut(created).ok_or(StreamWriteError::WrongType)?;
            return xadd_into_stream(stream, id, fields, trim)
                .map_err(|()| StreamWriteError::WrongType);
        };

        let stream = get_stream_mut(entry).ok_or(StreamWriteError::WrongType)?;
        xadd_into_stream(stream, id, fields, trim).map_err(|()| StreamWriteError::WrongType)
    }

    pub fn xlen(&self, key: &[u8]) -> Result<i64, StreamWriteError> {
        let _trace = profiler::scope("engine::stream::write::xlen");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }
        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let stream = get_stream(entry).ok_or(StreamWriteError::WrongType)?;
        Ok(stream.entries.len() as i64)
    }

    pub fn xtrim(
        &self,
        key: &[u8],
        mode: XTrimMode,
        threshold: StreamId,
        limit: Option<usize>,
    ) -> Result<i64, StreamWriteError> {
        let _trace = profiler::scope("engine::stream::write::xtrim");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let stream = get_stream_mut(entry).ok_or(StreamWriteError::WrongType)?;
        let before = stream.entries.len();
        apply_trim(stream, mode, threshold, limit);
        Ok((before.saturating_sub(stream.entries.len())) as i64)
    }
}
