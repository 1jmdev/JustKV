use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};

use crate::store::Store;
use types::value::{CompactValue, Entry};

use super::super::helpers::{
    deadline_from_ttl, get_live_entry, monotonic_now_ms, purge_if_expired,
};
use super::write_entry;

impl Store {
    pub fn get_preencoded(&self, key: &[u8]) -> Result<Bytes, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let entry = if shard.has_ttls() {
            let now_ms = monotonic_now_ms();
            get_live_entry(&shard, key, now_ms)
        } else {
            shard.entries.get(key)
        };
        let Some(entry) = entry else {
            return Ok(Bytes::from_static(b"$-1\r\n"));
        };
        let Some(value) = entry.as_string() else {
            return Err(());
        };

        let value_bytes = value.as_slice();
        let mut buf = BytesMut::with_capacity(1 + 20 + 2 + value_bytes.len() + 2);
        let mut len_buf = itoa::Buffer::new();
        buf.put_u8(b'$');
        buf.put_slice(len_buf.format(value_bytes.len()).as_bytes());
        buf.put_slice(b"\r\n");
        buf.put_slice(value_bytes);
        buf.put_slice(b"\r\n");
        Ok(buf.freeze())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<CompactValue>, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let entry = if shard.has_ttls() {
            let now_ms = monotonic_now_ms();
            get_live_entry(&shard, key, now_ms)
        } else {
            shard.entries.get(key)
        };
        let Some(entry) = entry else {
            return Ok(None);
        };
        match entry.as_string() {
            Some(value) => Ok(Some(value.clone())),
            None => Err(()),
        }
    }

    pub fn set(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        shard.upsert_string(key, value, ttl.map(deadline_from_ttl));
    }

    pub fn setnx(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if !shard.has_ttls() {
            if shard.entries.contains_key(key) {
                return false;
            }
        } else {
            let now_ms = monotonic_now_ms();
            if !purge_if_expired(&mut shard, key, now_ms) && shard.entries.contains_key(key) {
                return false;
            }
        }

        shard.upsert_string(key, value, ttl.map(deadline_from_ttl));
        true
    }

    pub fn setxx(&self, key: &[u8], value: &[u8], ttl: Option<Duration>) -> bool {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if !shard.has_ttls() {
            if !shard.entries.contains_key(key) {
                return false;
            }
        } else {
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key, now_ms) || !shard.entries.contains_key(key) {
                return false;
            }
        }

        shard.upsert_string(key, value, ttl.map(deadline_from_ttl));
        true
    }

    pub fn set_with_options(
        &self,
        key: &[u8],
        value: &[u8],
        ttl: Option<Duration>,
        must_exist: Option<bool>,
        return_old: bool,
    ) -> Result<(bool, Option<CompactValue>), ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if shard.has_ttls() {
            let now_ms = monotonic_now_ms();
            let _ = purge_if_expired(&mut shard, key, now_ms);
        }

        let mut key_exists = false;
        let previous = match shard.entries.get::<[u8]>(key) {
            Some(entry) => {
                let Some(current) = entry.as_string() else {
                    return Err(());
                };
                key_exists = true;
                if return_old {
                    Some(current.clone())
                } else {
                    None
                }
            }
            None => None,
        };

        if let Some(must_exist) = must_exist
            && must_exist != key_exists
        {
            return Ok((false, None));
        }

        shard.upsert_string(key, value, ttl.map(deadline_from_ttl));
        Ok((true, if return_old { previous } else { None }))
    }

    pub fn getset(&self, key: &[u8], value: &[u8]) -> Result<Option<CompactValue>, ()> {
        let (written, previous) = self.set_with_options(key, value, None, None, true)?;
        debug_assert!(written);
        Ok(previous)
    }

    pub fn getdel(&self, key: &[u8]) -> Result<Option<CompactValue>, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if shard.has_ttls() {
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key, now_ms) {
                return Ok(None);
            }
        }

        match shard.remove_key(key) {
            Some(entry) => match entry.into_string() {
                Some(value) => Ok(Some(value)),
                None => Err(()),
            },
            None => Ok(None),
        }
    }

    pub fn append(&self, key: &[u8], suffix: &[u8]) -> Result<usize, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();

        let mut base = if !shard.has_ttls() {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => match entry.as_string() {
                    Some(value) => value.to_vec(),
                    None => return Err(()),
                },
                None => Vec::new(),
            }
        } else {
            let now_ms = monotonic_now_ms();
            if purge_if_expired(&mut shard, key, now_ms) {
                Vec::new()
            } else {
                match shard.entries.get::<[u8]>(key) {
                    Some(entry) => match entry.as_string() {
                        Some(value) => value.to_vec(),
                        None => return Err(()),
                    },
                    None => Vec::new(),
                }
            }
        };
        let ttl_deadline = shard.ttl_deadline(key);

        base.extend_from_slice(suffix);
        let size = base.len();
        write_entry(&mut shard, key, Entry::new(base), ttl_deadline);
        Ok(size)
    }

    pub fn strlen(&self, key: &[u8]) -> Result<usize, ()> {
        let idx = self.shard_index(key);
        let shard = self.shards[idx].read();
        let entry = if shard.has_ttls() {
            let now_ms = monotonic_now_ms();
            get_live_entry(&shard, key, now_ms)
        } else {
            shard.entries.get(key)
        };
        let Some(entry) = entry else {
            return Ok(0);
        };
        match entry.as_string() {
            Some(value) => Ok(value.len()),
            None => Err(()),
        }
    }
}
