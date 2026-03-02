use ahash::RandomState;
use hashbrown::HashMap;

use crate::engine::store::Store;
use crate::engine::value::{CompactArg, CompactKey, StreamId};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::types::{ensure_pending_entry, StreamRangeItem, XPendingSummary};
use super::{get_stream, get_stream_mut};

impl Store {
    pub fn xreadgroup(
        &self,
        group: &[u8],
        consumer: &[u8],
        streams: &[(CompactArg, StreamId)],
        count: Option<usize>,
        noack: bool,
    ) -> Result<Vec<(CompactArg, Vec<StreamRangeItem>)>, ()> {
        let mut out = Vec::new();
        for (key, start_id) in streams {
            let idx = self.shard_index(key.as_slice());
            let mut shard = self.shards[idx].write();
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key.as_slice(), now_ms) {
                continue;
            }

            let Some(entry) = shard.entries.get_mut(key.as_slice()) else {
                continue;
            };
            let stream = get_stream_mut(entry).ok_or(())?;
            let Some(group_state) = stream.groups.get_mut(group) else {
                continue;
            };

            let start = if start_id.ms == u64::MAX && start_id.seq == u64::MAX {
                StreamId {
                    ms: group_state.last_delivered.ms,
                    seq: group_state.last_delivered.seq.saturating_add(1),
                }
            } else {
                *start_id
            };

            let mut items: Vec<StreamRangeItem> = stream
                .entries
                .range(
                    start..=StreamId {
                        ms: u64::MAX,
                        seq: u64::MAX,
                    },
                )
                .map(|(id, fields)| StreamRangeItem {
                    id: *id,
                    fields: fields.clone(),
                })
                .collect();
            if let Some(limit) = count {
                items.truncate(limit);
            }

            if let Some(last) = items.last() {
                group_state.last_delivered = last.id;
            }

            if !noack {
                for item in &items {
                    ensure_pending_entry(&mut group_state.pending, item.id, consumer);
                }
            }

            if !items.is_empty() {
                out.push((key.clone(), items));
            }
        }
        Ok(out)
    }

    pub fn xack(&self, key: &[u8], group: &[u8], ids: &[StreamId]) -> Result<i64, ()> {
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
        let Some(group_state) = stream.groups.get_mut(group) else {
            return Ok(0);
        };

        let mut removed = 0i64;
        for id in ids {
            if group_state.pending.remove(id).is_some() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn xpending_summary(
        &self,
        key: &[u8],
        group: &[u8],
    ) -> Result<Option<XPendingSummary>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let stream = get_stream(entry).ok_or(())?;
        let Some(group_state) = stream.groups.get(group) else {
            return Ok(None);
        };

        let total = group_state.pending.len() as i64;
        let min = group_state.pending.keys().min().copied();
        let max = group_state.pending.keys().max().copied();
        let mut consumers = HashMap::with_hasher(RandomState::new());
        for pending in group_state.pending.values() {
            *consumers.entry(pending.consumer.clone()).or_insert(0) += 1;
        }

        Ok(Some(XPendingSummary {
            total,
            min,
            max,
            consumers,
        }))
    }

    pub fn xclaim(
        &self,
        key: &[u8],
        group: &[u8],
        consumer: &[u8],
        ids: &[StreamId],
    ) -> Result<Vec<StreamRangeItem>, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(Vec::new());
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(Vec::new());
        };
        let stream = get_stream_mut(entry).ok_or(())?;
        let Some(group_state) = stream.groups.get_mut(group) else {
            return Ok(Vec::new());
        };

        let mut out = Vec::new();
        for id in ids {
            let Some(fields) = stream.entries.get(id) else {
                continue;
            };
            if let Some(pending) = group_state.pending.get_mut(id) {
                pending.consumer = CompactKey::from_slice(consumer);
                pending.deliveries = pending.deliveries.saturating_add(1);
                out.push(StreamRangeItem {
                    id: *id,
                    fields: fields.clone(),
                });
            }
        }
        Ok(out)
    }

    pub fn xautoclaim(
        &self,
        key: &[u8],
        group: &[u8],
        consumer: &[u8],
        start: StreamId,
        count: usize,
    ) -> Result<(StreamId, Vec<StreamRangeItem>), ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok((start, Vec::new()));
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok((start, Vec::new()));
        };
        let stream = get_stream_mut(entry).ok_or(())?;
        let Some(group_state) = stream.groups.get_mut(group) else {
            return Ok((start, Vec::new()));
        };

        let mut ids: Vec<StreamId> = group_state
            .pending
            .keys()
            .copied()
            .filter(|id| *id >= start)
            .collect();
        ids.sort_unstable();
        ids.truncate(count);

        let mut out = Vec::new();
        let mut next = start;
        for id in ids {
            if let Some(pending) = group_state.pending.get_mut(&id) {
                pending.consumer = CompactKey::from_slice(consumer);
                pending.deliveries = pending.deliveries.saturating_add(1);
            }
            if let Some(fields) = stream.entries.get(&id) {
                out.push(StreamRangeItem {
                    id,
                    fields: fields.clone(),
                });
                next = StreamId {
                    ms: id.ms,
                    seq: id.seq.saturating_add(1),
                };
            }
        }
        Ok((next, out))
    }
}
