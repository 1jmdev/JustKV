use crate::Store;
use types::value::StreamId;

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_stream_mut;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XDelexPolicy {
    KeepRef,
    DelRef,
    Acked,
}

impl Store {
    pub fn xdel(
        &self,
        key: &[u8],
        ids: &[StreamId],
    ) -> Result<i64, super::write::StreamWriteError> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let stream = get_stream_mut(entry).ok_or(super::write::StreamWriteError::WrongType)?;

        let mut removed = 0i64;
        for id in ids {
            if stream.entries.remove(id).is_some() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn xdelex(
        &self,
        key: &[u8],
        policy: XDelexPolicy,
        ids: &[StreamId],
    ) -> Result<Vec<i64>, super::write::StreamWriteError> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(vec![-1; ids.len()]);
        }
        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(vec![-1; ids.len()]);
        };
        let stream = get_stream_mut(entry).ok_or(super::write::StreamWriteError::WrongType)?;

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let result = match policy {
                XDelexPolicy::KeepRef => {
                    if stream.entries.remove(id).is_some() {
                        1
                    } else {
                        -1
                    }
                }
                XDelexPolicy::DelRef => {
                    let removed_entry = stream.entries.remove(id).is_some();
                    let mut removed_reference = false;
                    for group in stream.groups.values_mut() {
                        if group.pending.remove(id).is_some() {
                            removed_reference = true;
                        }
                    }
                    if removed_entry || removed_reference {
                        1
                    } else {
                        -1
                    }
                }
                XDelexPolicy::Acked => {
                    if !stream.entries.contains_key(id) {
                        -1
                    } else if stream.groups.is_empty()
                        || stream
                            .groups
                            .values()
                            .any(|group| group.pending.contains_key(id))
                    {
                        2
                    } else if stream.entries.remove(id).is_some() {
                        1
                    } else {
                        -1
                    }
                }
            };
            out.push(result);
        }
        Ok(out)
    }
}
