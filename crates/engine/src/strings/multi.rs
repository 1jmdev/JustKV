use crate::store::Store;
use bytes::{BufMut, BytesMut};
use types::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};

impl Store {
    pub fn mget<K: AsRef<[u8]>>(&self, keys: &[K]) -> Result<Vec<Option<CompactValue>>, ()> {
        let count = keys.len();
        if count == 0 {
            return Ok(Vec::new());
        }

        let now_ms = monotonic_now_ms();
        let mut out = vec![None; count];

        if count <= 4 {
            for (pos, key) in keys.iter().enumerate() {
                let key = key.as_ref();
                let idx = self.shard_index(key);
                let shard = self.shards[idx].read();
                let Some(entry) = shard.entries.get::<[u8]>(key) else {
                    continue;
                };
                if shard.is_expired(key, now_ms) {
                    continue;
                }
                match entry.as_string() {
                    Some(value) => out[pos] = Some(value.clone()),
                    None => return Err(()),
                }
            }
            return Ok(out);
        }

        let key_bytes: Vec<&[u8]> = keys.iter().map(|key| key.as_ref()).collect();

        let shard_count = self.shards.len();
        let mut grouped: Vec<Vec<usize>> = vec![Vec::new(); shard_count];
        let mut touched = Vec::with_capacity(shard_count);
        for (pos, key) in key_bytes.iter().copied().enumerate() {
            let idx = self.shard_index(key);
            if grouped[idx].is_empty() {
                touched.push(idx);
            }
            grouped[idx].push(pos);
        }

        for idx in touched {
            let positions = std::mem::take(&mut grouped[idx]);
            let shard = self.shards[idx].read();
            let has_ttl = shard.has_ttls();

            if has_ttl {
                let mut chunks = positions.chunks_exact(8);
                for batch in &mut chunks {
                    let key_batch: [&[u8]; 8] = std::array::from_fn(|i| key_bytes[batch[i]]);
                    let entry_batch = shard.entries.get_batch::<8, [u8]>(&key_batch);

                    for i in 0..8 {
                        let Some(entry) = entry_batch[i] else {
                            continue;
                        };
                        let pos = batch[i];
                        if shard.is_expired(key_batch[i], now_ms) {
                            continue;
                        }
                        match entry.as_string() {
                            Some(value) => out[pos] = Some(value.clone()),
                            None => return Err(()),
                        }
                    }
                }

                for &pos in chunks.remainder() {
                    let key = key_bytes[pos];
                    let Some(entry) = shard.entries.get::<[u8]>(key) else {
                        continue;
                    };
                    if shard.is_expired(key, now_ms) {
                        continue;
                    }
                    match entry.as_string() {
                        Some(value) => out[pos] = Some(value.clone()),
                        None => return Err(()),
                    }
                }
            } else {
                let mut chunks = positions.chunks_exact(8);
                for batch in &mut chunks {
                    let key_batch: [&[u8]; 8] = std::array::from_fn(|i| key_bytes[batch[i]]);
                    let entry_batch = shard.entries.get_batch::<8, [u8]>(&key_batch);

                    for i in 0..8 {
                        let Some(entry) = entry_batch[i] else {
                            continue;
                        };
                        match entry.as_string() {
                            Some(value) => out[batch[i]] = Some(value.clone()),
                            None => return Err(()),
                        }
                    }
                }

                for &pos in chunks.remainder() {
                    let key = key_bytes[pos];
                    let Some(entry) = shard.entries.get::<[u8]>(key) else {
                        continue;
                    };
                    match entry.as_string() {
                        Some(value) => out[pos] = Some(value.clone()),
                        None => return Err(()),
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn mget_encode<K: AsRef<[u8]>>(&self, keys: &[K]) -> Result<bytes::Bytes, ()> {
        let count = keys.len();
        if count == 0 {
            return Ok(bytes::Bytes::from_static(b"*0\r\n"));
        }

        let values = self.mget(keys)?;
        let mut buf = BytesMut::with_capacity(16 + count * 12);
        let mut header_buf = itoa::Buffer::new();
        let mut len_buf = itoa::Buffer::new();

        buf.put_u8(b'*');
        buf.put_slice(header_buf.format(count).as_bytes());
        buf.put_slice(b"\r\n");

        for value in values {
            match value {
                Some(value) => encode_bulk_value(&value, &mut buf, &mut len_buf),
                None => buf.put_slice(b"$-1\r\n"),
            }
        }

        Ok(buf.freeze())
    }

    pub fn mset_args(&self, pairs: &[CompactArg]) {
        let shard_count = self.shards.len();
        let pair_count = pairs.len() / 2;

        if pair_count <= 2 {
            for chunk in pairs.chunks_exact(2) {
                let key = chunk[0].as_slice();
                let idx = self.shard_index(key);
                let mut shard = self.shards[idx].write();
                shard.insert_entry(chunk[0].clone(), Entry::String(chunk[1].clone()), None);
            }
            return;
        }

        let mut grouped: Vec<Vec<(CompactKey, Entry)>> = vec![Vec::new(); shard_count];
        let mut touched = Vec::with_capacity(pair_count.min(shard_count));

        for chunk in pairs.chunks_exact(2) {
            let key = &chunk[0];
            let value = &chunk[1];
            let idx = self.shard_index(key.as_slice());
            if grouped[idx].is_empty() {
                touched.push(idx);
            }
            grouped[idx].push((key.clone(), Entry::String(value.clone())));
        }

        for idx in touched {
            let entries = std::mem::take(&mut grouped[idx]);

            let mut shard = self.shards[idx].write();
            for (key, entry) in entries {
                shard.insert_entry(key, entry, None);
            }
        }
    }

    pub fn mset(&self, pairs: Vec<(CompactArg, CompactArg)>) {
        let shard_count = self.shards.len();
        let mut grouped = vec![Vec::new(); shard_count];
        let mut touched = Vec::with_capacity(pairs.len().min(shard_count));

        for (key, value) in pairs {
            let idx = self.shard_index(&key);
            if grouped[idx].is_empty() {
                touched.push(idx);
            }
            grouped[idx].push((CompactKey::from_slice(&key), Entry::from_slice(&value)));
        }

        for idx in touched {
            let entries = std::mem::take(&mut grouped[idx]);
            let mut shard = self.shards[idx].write();
            for (key, entry) in entries {
                shard.insert_entry(key, entry, None);
            }
        }
    }

    pub fn msetnx(&self, pairs: Vec<(CompactArg, CompactArg)>) -> bool {
        if pairs.is_empty() {
            return true;
        }

        let now_ms = monotonic_now_ms();
        let shard_count = self.shards.len();
        let mut grouped: Vec<Vec<(CompactArg, CompactArg)>> = vec![Vec::new(); shard_count];
        let mut touched = Vec::with_capacity(pairs.len().min(shard_count));

        for (key, value) in pairs {
            let idx = self.shard_index(&key);
            if grouped[idx].is_empty() {
                touched.push(idx);
            }
            grouped[idx].push((key, value));
        }

        touched.sort_unstable();

        let mut guards = Vec::with_capacity(touched.len());
        for &idx in &touched {
            guards.push((idx, self.shards[idx].write()));
        }

        for (idx, shard) in &mut guards {
            for (key, _) in &grouped[*idx] {
                if !purge_if_expired(shard, key, now_ms) && shard.entries.contains_key(key) {
                    return false;
                }
            }
        }

        for (idx, shard) in &mut guards {
            for (key, value) in grouped[*idx].drain(..) {
                shard.insert_entry(
                    CompactKey::from_slice(&key),
                    Entry::from_slice(&value),
                    None,
                );
            }
        }

        true
    }
}

fn encode_bulk_value(value: &CompactValue, buf: &mut BytesMut, len_buf: &mut itoa::Buffer) {
    let bytes = value.as_slice();
    buf.put_u8(b'$');
    buf.put_slice(len_buf.format(bytes.len()).as_bytes());
    buf.put_slice(b"\r\n");
    buf.put_slice(bytes);
    buf.put_slice(b"\r\n");
}
