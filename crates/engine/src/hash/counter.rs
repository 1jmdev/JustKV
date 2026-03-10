use crate::store::{HashFloatOpError, HashIntOpError};
use crate::{Store, StoredEntry};
use types::value::{CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::get_hash_map_mut;

#[inline(always)]
fn parse_i64_bytes(raw: &[u8]) -> Option<i64> {
    if raw.is_empty() {
        return None;
    }

    let mut index = 0;
    let mut negative = false;
    match raw[0] {
        b'-' => {
            negative = true;
            index = 1;
        }
        b'+' => index = 1,
        _ => {}
    }

    if index == raw.len() {
        return None;
    }

    let mut value: i64 = 0;
    while index < raw.len() {
        let digit = raw[index].wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(i64::from(digit))?;
        index += 1;
    }

    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}

impl Store {
    pub fn hincrby(&self, key: &[u8], field: &[u8], delta: i64) -> Result<i64, HashIntOpError> {
        let _trace = profiler::scope("engine::hash::counter::hincrby");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if shard.has_ttls() {
            let _ = purge_if_expired(&mut shard, key, monotonic_now_ms());
        }

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                StoredEntry::new(Entry::empty_hash())
            });
        entry.invalidate_hash_getall_cache();
        let map = get_hash_map_mut(entry).ok_or(HashIntOpError::WrongType)?;

        let current = match map.get(field) {
            Some(value) => {
                parse_i64_bytes(value.as_slice()).ok_or(HashIntOpError::InvalidInteger)?
            }
            None => 0,
        };
        let next = current.checked_add(delta).ok_or(HashIntOpError::Overflow)?;
        let mut buffer = itoa::Buffer::new();
        let encoded = buffer.format(next);

        map.insert(
            CompactKey::from_slice(field),
            CompactValue::from_slice(encoded.as_bytes()),
        );
        Ok(next)
    }

    pub fn hincrbyfloat(
        &self,
        key: &[u8],
        field: &[u8],
        delta: f64,
    ) -> Result<f64, HashFloatOpError> {
        let _trace = profiler::scope("engine::hash::counter::hincrbyfloat");
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if shard.has_ttls() {
            let _ = purge_if_expired(&mut shard, key, monotonic_now_ms());
        }

        let entry = shard
            .entries
            .get_or_insert_with(CompactKey::from_slice(key), || {
                StoredEntry::new(Entry::empty_hash())
            });
        entry.invalidate_hash_getall_cache();
        let map = get_hash_map_mut(entry).ok_or(HashFloatOpError::WrongType)?;

        let current = match map.get(field) {
            Some(value) => {
                let text = std::str::from_utf8(value.as_slice())
                    .map_err(|_| HashFloatOpError::InvalidFloat)?;
                text.parse::<f64>()
                    .map_err(|_| HashFloatOpError::InvalidFloat)?
            }
            None => 0.0,
        };
        let next = current + delta;
        if !next.is_finite() {
            return Err(HashFloatOpError::InvalidFloat);
        }
        let mut buffer = ryu::Buffer::new();
        let encoded = buffer.format(next);

        map.insert(
            CompactKey::from_slice(field),
            CompactValue::from_slice(encoded.as_bytes()),
        );
        Ok(next)
    }
}
