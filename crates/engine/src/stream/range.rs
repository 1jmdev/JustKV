use crate::store::Store;
use crate::value::{CompactArg, StreamId};

use super::super::helpers::{is_expired, monotonic_now_ms};
use super::get_stream;
use super::types::{StreamRangeItem, push_items};

impl Store {
    pub fn xrange(
        &self,
        key: &[u8],
        start: StreamId,
        end: StreamId,
        reverse: bool,
        count: Option<usize>,
    ) -> Result<Vec<StreamRangeItem>, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::range::xrange");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let stream = get_stream(entry).ok_or(())?;
        Ok(push_items(stream, start, end, reverse, count))
    }

    pub fn xread(
        &self,
        streams: &[(CompactArg, StreamId)],
        count: Option<usize>,
    ) -> Result<Vec<(CompactArg, Vec<StreamRangeItem>)>, ()> {
        let _trace = profiler::scope("crates::engine::src::stream::range::xread");
        let mut out = Vec::new();
        for (key, last_seen) in streams {
            let next = StreamId {
                ms: last_seen.ms,
                seq: last_seen.seq.saturating_add(1),
            };
            let items = self.xrange(
                key.as_slice(),
                next,
                StreamId {
                    ms: u64::MAX,
                    seq: u64::MAX,
                },
                false,
                count,
            )?;
            if !items.is_empty() {
                out.push((key.clone(), items));
            }
        }
        Ok(out)
    }
}
