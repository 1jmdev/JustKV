use bytes::{BufMut, BytesMut};

use crate::{Store, StoredEntry};
use ahash::RandomState;
use types::value::{CompactArg, CompactKey, CompactValue, Entry, HashValueMap};

use super::super::helpers::{is_expired, monotonic_now_ms, purge_if_expired};
use super::{collect_pairs, get_hash_map, get_hash_map_mut};

#[inline(always)]
fn bulk_entry_len(len: usize) -> usize {
    1 + decimal_len(len) + 2 + len + 2
}

#[inline(always)]
fn decimal_len(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn encode_hgetall_map(map: &HashValueMap) -> bytes::Bytes {
    let count = map.len() * 2;
    let mut total_len = 1 + decimal_len(count) + 2;
    for (field, value) in map.iter() {
        total_len += bulk_entry_len(field.slice().len());
        total_len += bulk_entry_len(value.slice().len());
    }

    let mut buf = BytesMut::with_capacity(total_len);
    let mut header_buf = itoa::Buffer::new();
    let mut len_buf = itoa::Buffer::new();

    buf.put_u8(b'*');
    buf.put_slice(header_buf.format(count).as_bytes());
    buf.put_slice(b"\r\n");

    for (field, value) in map.iter() {
        let field_bytes = field.slice();
        buf.put_u8(b'$');
        buf.put_slice(len_buf.format(field_bytes.len()).as_bytes());
        buf.put_slice(b"\r\n");
        buf.put_slice(field_bytes);
        buf.put_slice(b"\r\n");

        let value_bytes = value.slice();
        buf.put_u8(b'$');
        buf.put_slice(len_buf.format(value_bytes.len()).as_bytes());
        buf.put_slice(b"\r\n");
        buf.put_slice(value_bytes);
        buf.put_slice(b"\r\n");
    }

    buf.freeze()
}

impl Store {
    pub fn hset_args(&self, key: &[u8], pairs: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hset_args");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let pair_count = pairs.len() / 2;
        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                StoredEntry::new(
                    Entry::Hash(Box::new(HashValueMap::with_capacity_and_hasher(
                        pair_count,
                        RandomState::new(),
                    ))),
                    None,
                )
            });

        entry.invalidate_hash_getall_cache();
        let map = get_hash_map_mut(entry).ok_or(())?;
        if map.is_empty() {
            map.reserve(pair_count);
        }

        let mut created = 0;
        for chunk in pairs.chunks_exact(2) {
            if map.insert(chunk[0].clone(), chunk[1].clone()).is_none() {
                created += 1;
            }
        }

        Ok(created)
    }

    pub fn hset(&self, key: &[u8], pairs: &[(CompactArg, CompactArg)]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hset");
        if pairs.is_empty() {
            return Ok(0);
        }

        let mut flat = Vec::with_capacity(pairs.len() * 2);
        for (field, value) in pairs {
            flat.push(field.clone());
            flat.push(value.clone());
        }

        self.hset_args(key, &flat)
    }

    pub fn hsetnx(&self, key: &[u8], field: &[u8], value: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hsetnx");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        let _ = purge_if_expired(&mut shard, key, now_ms);

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                StoredEntry::new(Entry::empty_hash(), None)
            });
        let field_key = CompactKey::from_slice(field);
        if get_hash_map(entry)
            .ok_or(())?
            .contains_key(field_key.as_slice())
        {
            return Ok(0);
        }
        entry.invalidate_hash_getall_cache();
        let map = get_hash_map_mut(entry).ok_or(())?;
        map.insert(field_key, CompactValue::from_slice(value));
        Ok(1)
    }

    pub fn hget(&self, key: &[u8], field: &[u8]) -> Result<Option<CompactValue>, ()> {
        let _trace = profiler::scope("engine::hash::core::hget");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(None);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(None);
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(map.get(field).cloned())
    }

    pub fn hmget(
        &self,
        key: &[u8],
        fields: &[CompactArg],
    ) -> Result<Vec<Option<CompactValue>>, ()> {
        let _trace = profiler::scope("engine::hash::core::hmget");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(vec![None; fields.len()]);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(vec![None; fields.len()]);
        };
        let map = get_hash_map(entry).ok_or(())?;

        Ok(fields
            .iter()
            .map(|field| map.get(field.as_slice()).cloned())
            .collect())
    }

    pub fn hgetall(&self, key: &[u8]) -> Result<Vec<(CompactKey, CompactValue)>, ()> {
        let _trace = profiler::scope("engine::hash::core::hgetall");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(Vec::new());
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(Vec::new());
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(collect_pairs(map))
    }

    pub fn hgetall_encode(&self, key: &[u8]) -> Result<bytes::Bytes, ()> {
        let _trace = profiler::scope("engine::hash::core::hgetall_encode");
        let idx = self.shard_index(key);
        {
            let shard = self.shards[idx].read();
            if shard.has_ttls() && is_expired(&shard, key, monotonic_now_ms()) {
                return Ok(bytes::Bytes::from_static(b"*0\r\n"));
            }

            let Some(entry) = shard.entries.get(key) else {
                return Ok(bytes::Bytes::from_static(b"*0\r\n"));
            };
            if let Some(encoded) = entry.hash_getall_cache() {
                return Ok(encoded.clone());
            }
        }

        let mut shard = self.shards[idx].write();
        if shard.has_ttls() && is_expired(&shard, key, monotonic_now_ms()) {
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        };
        if let Some(encoded) = entry.hash_getall_cache() {
            return Ok(encoded.clone());
        }
        let map = get_hash_map(entry).ok_or(())?;
        let encoded = encode_hgetall_map(map);
        entry.set_hash_getall_cache(encoded.clone());

        Ok(encoded)
    }

    pub fn hdel(&self, key: &[u8], fields: &[CompactArg]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hdel");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();
        if purge_if_expired(&mut shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get_mut(key) else {
            return Ok(0);
        };
        entry.invalidate_hash_getall_cache();
        let map = get_hash_map_mut(entry).ok_or(())?;

        let mut removed = 0;
        for field in fields {
            if map.remove(field.as_slice()).is_some() {
                removed += 1;
            }
        }
        if map.is_empty() {
            let _ = shard.remove_key(key);
        }
        Ok(removed)
    }

    pub fn hexists(&self, key: &[u8], field: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hexists");
        Ok(self.hget(key, field)?.is_some() as i64)
    }

    pub fn hkeys(&self, key: &[u8]) -> Result<Vec<CompactKey>, ()> {
        let _trace = profiler::scope("engine::hash::core::hkeys");
        let pairs = self.hgetall(key)?;
        Ok(pairs.into_iter().map(|(field, _)| field).collect())
    }

    pub fn hvals(&self, key: &[u8]) -> Result<Vec<CompactValue>, ()> {
        let _trace = profiler::scope("engine::hash::core::hvals");
        let pairs = self.hgetall(key)?;
        Ok(pairs.into_iter().map(|(_, value)| value).collect())
    }

    pub fn hlen(&self, key: &[u8]) -> Result<i64, ()> {
        let _trace = profiler::scope("engine::hash::core::hlen");
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let now_ms = monotonic_now_ms();
        if is_expired(&shard, key, now_ms) {
            return Ok(0);
        }

        let Some(entry) = shard.entries.get(key) else {
            return Ok(0);
        };
        let map = get_hash_map(entry).ok_or(())?;
        Ok(map.len() as i64)
    }

    pub fn hstrlen(&self, key: &[u8], field: &[u8]) -> Result<usize, ()> {
        let _trace = profiler::scope("engine::hash::core::hstrlen");
        Ok(self.hget(key, field)?.map(|value| value.len()).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hgetall_cache_is_reused_and_invalidated_on_write() {
        let store = Store::new(1);
        let key = b"hash";

        store
            .hset_args(
                key,
                &[
                    CompactArg::from_slice(b"field"),
                    CompactArg::from_slice(b"value"),
                ],
            )
            .unwrap();

        let first = store.hgetall_encode(key).unwrap();
        let second = store.hgetall_encode(key).unwrap();
        assert_eq!(first, second);

        store
            .hset_args(
                key,
                &[
                    CompactArg::from_slice(b"field"),
                    CompactArg::from_slice(b"next"),
                ],
            )
            .unwrap();

        let updated = store.hgetall_encode(key).unwrap();
        assert_ne!(first, updated);
        assert_eq!(updated.as_ref(), b"*2\r\n$5\r\nfield\r\n$4\r\nnext\r\n");
    }
}
