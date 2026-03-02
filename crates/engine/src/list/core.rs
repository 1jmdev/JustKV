use bytes::{BufMut, BytesMut};
use itoa;

use crate::store::{ListInsertPosition, ListSetError, ListSide, Store};
use crate::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{get_list, get_list_mut};

impl Store {
    pub fn lpush(&self, key: &[u8], values: &[CompactArg]) -> Result<i64, ()> {
        self.push_with_side(key, values, ListSide::Left)
    }

    pub fn rpush(&self, key: &[u8], values: &[CompactArg]) -> Result<i64, ()> {
        self.push_with_side(key, values, ListSide::Right)
    }

    pub fn lpop(&self, key: &[u8], count: usize) -> Result<Option<Vec<CompactValue>>, ()> {
        self.pop_with_side(key, count, ListSide::Left)
    }

    pub fn rpop(&self, key: &[u8], count: usize) -> Result<Option<Vec<CompactValue>>, ()> {
        self.pop_with_side(key, count, ListSide::Right)
    }

    pub fn llen(&self, key: &[u8]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let list = get_list(entry).ok_or(())?;
        Ok(list.len() as i64)
    }

    pub fn lindex(&self, key: &[u8], index: i64) -> Result<Option<CompactValue>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let list = get_list(entry).ok_or(())?;
        let Some(offset) = normalize_index(index, list.len()) else {
            return Ok(None);
        };
        Ok(list.get(offset).cloned())
    }

    pub fn lrange(&self, key: &[u8], start: i64, stop: i64) -> Result<Vec<CompactValue>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let list = get_list(entry).ok_or(())?;
        let Some((from, to_exclusive)) = normalize_range(start, stop, list.len()) else {
            return Ok(Vec::new());
        };

        let count = to_exclusive - from;
        let mut out = Vec::with_capacity(count);
        let (a, b) = list.as_slices();

        // VecDeque exposes two contiguous slices. Collect from them directly
        // using index arithmetic so we never skip element-by-element.
        if from < a.len() {
            let end_in_a = to_exclusive.min(a.len());
            out.extend_from_slice(&a[from..end_in_a]);
            if to_exclusive > a.len() {
                let b_end = to_exclusive - a.len();
                out.extend_from_slice(&b[..b_end]);
            }
        } else {
            let b_from = from - a.len();
            let b_end = to_exclusive - a.len();
            out.extend_from_slice(&b[b_from..b_end]);
        }

        Ok(out)
    }

    /// Encode the LRANGE response directly into a `BytesMut` while holding the
    /// shard read lock, avoiding a heap-allocated `Vec<CompactValue>` clone.
    pub fn lrange_encode(&self, key: &[u8], start: i64, stop: i64) -> Result<bytes::Bytes, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();

        if is_expired(&shard, key, now_ms) {
            // Empty array: *0\r\n
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        };
        let list = get_list(entry).ok_or(())?;
        let Some((from, to_exclusive)) = normalize_range(start, stop, list.len()) else {
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        };

        let count = to_exclusive - from;
        // Pre-allocate: header + count * (avg element overhead).
        // Each element costs "$N\r\n<data>\r\n" – estimate 8 bytes overhead.
        let mut buf = BytesMut::with_capacity(16 + count * 10);

        // Write array header.
        buf.put_u8(b'*');
        write_usize(&mut buf, count);
        buf.put_slice(b"\r\n");

        let (a, b) = list.as_slices();

        let encode_slice = |buf: &mut BytesMut, slice: &[CompactValue]| {
            for value in slice {
                let bytes = value.as_slice();
                buf.put_u8(b'$');
                write_usize(buf, bytes.len());
                buf.put_slice(b"\r\n");
                buf.put_slice(bytes);
                buf.put_slice(b"\r\n");
            }
        };

        if from < a.len() {
            let end_in_a = to_exclusive.min(a.len());
            encode_slice(&mut buf, &a[from..end_in_a]);
            if to_exclusive > a.len() {
                let b_end = to_exclusive - a.len();
                encode_slice(&mut buf, &b[..b_end]);
            }
        } else {
            let b_from = from - a.len();
            let b_end = to_exclusive - a.len();
            encode_slice(&mut buf, &b[b_from..b_end]);
        }

        Ok(buf.freeze())
    }

    pub fn lset(&self, key: &[u8], index: i64, value: &[u8]) -> Result<(), ListSetError> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Err(ListSetError::NoSuchKey);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Err(ListSetError::NoSuchKey);
        };
        let list = get_list_mut(entry).ok_or(ListSetError::WrongType)?;
        let Some(offset) = normalize_index(index, list.len()) else {
            return Err(ListSetError::OutOfRange);
        };

        list[offset] = CompactValue::from_slice(value);
        Ok(())
    }

    pub fn ltrim(&self, key: &[u8], start: i64, stop: i64) -> Result<(), ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(());
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(());
        };
        let list = get_list_mut(entry).ok_or(())?;
        let range = normalize_range(start, stop, list.len());

        if let Some((from, to_exclusive)) = range {
            let mut trimmed = std::collections::VecDeque::with_capacity(to_exclusive - from);
            trimmed.extend(list.iter().skip(from).take(to_exclusive - from).cloned());
            *list = trimmed;
        } else {
            list.clear();
        }

        if list.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(())
    }

    pub fn linsert(
        &self,
        key: &[u8],
        position: ListInsertPosition,
        pivot: &[u8],
        element: &[u8],
    ) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        let list = get_list_mut(entry).ok_or(())?;
        let Some(pivot_pos) = list.iter().position(|value| value.as_slice() == pivot) else {
            return Ok(-1);
        };

        let insert_at = match position {
            ListInsertPosition::Before => pivot_pos,
            ListInsertPosition::After => pivot_pos + 1,
        };
        list.insert(insert_at, CompactValue::from_slice(element));
        Ok(list.len() as i64)
    }

    pub fn lpos(
        &self,
        key: &[u8],
        element: &[u8],
        rank: i64,
        count: Option<usize>,
        maxlen: Option<usize>,
    ) -> Result<Option<Vec<i64>>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let list = get_list(entry).ok_or(())?;
        if list.is_empty() {
            return Ok(None);
        }

        let scan_len = maxlen.unwrap_or(list.len()).min(list.len());
        if scan_len == 0 {
            return Ok(None);
        }

        let mut matches = Vec::new();
        if rank > 0 {
            for (idx, value) in list.iter().take(scan_len).enumerate() {
                if value.as_slice() == element {
                    matches.push(idx as i64);
                }
            }
        } else {
            for (idx, value) in list.iter().rev().take(scan_len).enumerate() {
                if value.as_slice() == element {
                    matches.push((list.len() - 1 - idx) as i64);
                }
            }
        }

        if matches.is_empty() {
            return Ok(None);
        }

        let rank_index = rank.unsigned_abs() as usize;
        if rank_index == 0 || matches.len() < rank_index {
            return Ok(None);
        }

        let start = rank_index - 1;
        let positions: Vec<i64> = if let Some(limit) = count {
            if limit == 0 {
                matches.into_iter().skip(start).collect()
            } else {
                matches.into_iter().skip(start).take(limit).collect()
            }
        } else {
            vec![matches[start]]
        };

        if positions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(positions))
        }
    }

    fn push_with_side(&self, key: &[u8], values: &[CompactArg], side: ListSide) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                Entry::List(Box::new(std::collections::VecDeque::new()))
            });
        let list = get_list_mut(entry).ok_or(())?;

        match side {
            ListSide::Left => {
                for value in values {
                    list.push_front(CompactValue::from_slice(value.as_slice()));
                }
            }
            ListSide::Right => {
                for value in values {
                    list.push_back(CompactValue::from_slice(value.as_slice()));
                }
            }
        }

        Ok(list.len() as i64)
    }

    fn pop_with_side(
        &self,
        key: &[u8],
        count: usize,
        side: ListSide,
    ) -> Result<Option<Vec<CompactValue>>, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(None);
        };
        let list = get_list_mut(entry).ok_or(())?;

        let take = count.min(list.len());
        let mut out = Vec::with_capacity(take);
        for _ in 0..take {
            let value = match side {
                ListSide::Left => list.pop_front(),
                ListSide::Right => list.pop_back(),
            };
            if let Some(value) = value {
                out.push(value);
            }
        }

        if list.is_empty() {
            let _ = shard.remove_key(key);
        }

        Ok(Some(out))
    }
}

fn write_usize(buf: &mut BytesMut, value: usize) {
    let mut tmp = itoa::Buffer::new();
    buf.put_slice(tmp.format(value).as_bytes());
}

fn normalize_index(index: i64, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let len_i64 = len as i64;
    let normalized = if index < 0 { len_i64 + index } else { index };
    if !(0..len_i64).contains(&normalized) {
        None
    } else {
        Some(normalized as usize)
    }
}

fn normalize_range(start: i64, stop: i64, len: usize) -> Option<(usize, usize)> {
    if len == 0 {
        return None;
    }

    let len_i64 = len as i64;
    let mut from = if start < 0 { len_i64 + start } else { start };
    let mut to = if stop < 0 { len_i64 + stop } else { stop };

    if from < 0 {
        from = 0;
    }
    if to < 0 {
        return None;
    }
    if from >= len_i64 {
        return None;
    }
    if to >= len_i64 {
        to = len_i64 - 1;
    }
    if from > to {
        return None;
    }

    Some((from as usize, (to as usize) + 1))
}
