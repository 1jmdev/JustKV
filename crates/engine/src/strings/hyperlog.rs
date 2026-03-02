use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ahash::RandomState;
use hashbrown::HashSet;

use crate::store::Store;
use crate::value::{CompactArg, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};
use super::write_entry;

const HLL_MAGIC: &[u8] = b"JKVHLL1";

impl Store {
    pub fn pfadd(&self, key: &[u8], elements: &[CompactArg]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let now_ms = monotonic_now_ms();

        let mut entries = if purge_if_expired(&mut shard, key, now_ms) {
            HashSet::with_hasher(RandomState::new())
        } else {
            match shard.entries.get::<[u8]>(key) {
                Some(entry) => {
                    let data = entry.as_string().ok_or(())?;
                    decode_hll(data.as_slice()).ok_or(())?
                }
                None => HashSet::with_hasher(RandomState::new()),
            }
        };
        let ttl_deadline = shard.ttl.get(key).copied();

        let mut changed = false;
        for element in elements {
            changed |= entries.insert(hash_element(element.as_slice()));
        }

        if changed {
            let encoded = encode_hll(&entries);
            write_entry(&mut shard, key, Entry::new(encoded), ttl_deadline);
            return Ok(1);
        }

        Ok(0)
    }

    pub fn pfcount(&self, keys: &[CompactArg]) -> Result<i64, ()> {
        let mut union = HashSet::with_hasher(RandomState::new());
        for key in keys {
            let Some(value) = self.get(key.as_slice())? else {
                continue;
            };
            let decoded = decode_hll(value.as_slice()).ok_or(())?;
            union.extend(decoded);
        }

        Ok(union.len() as i64)
    }

    pub fn pfmerge(&self, destination: &[u8], keys: &[CompactArg]) -> Result<(), ()> {
        let mut union = HashSet::with_hasher(RandomState::new());
        for key in keys {
            let Some(value) = self.get(key.as_slice())? else {
                continue;
            };
            let decoded = decode_hll(value.as_slice()).ok_or(())?;
            union.extend(decoded);
        }

        let encoded = encode_hll(&union);
        self.set(destination, &encoded, None);
        Ok(())
    }
}

fn hash_element(value: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn encode_hll(entries: &HashSet<u64, RandomState>) -> Vec<u8> {
    let mut values: Vec<u64> = entries.iter().copied().collect();
    values.sort_unstable();

    let mut out = Vec::with_capacity(HLL_MAGIC.len() + 4 + values.len() * 8);
    out.extend_from_slice(HLL_MAGIC);
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn decode_hll(raw: &[u8]) -> Option<HashSet<u64, RandomState>> {
    if raw.is_empty() {
        return Some(HashSet::with_hasher(RandomState::new()));
    }
    if raw.len() < HLL_MAGIC.len() + 4 || !raw.starts_with(HLL_MAGIC) {
        return None;
    }

    let count_offset = HLL_MAGIC.len();
    let count = u32::from_le_bytes(raw[count_offset..count_offset + 4].try_into().ok()?) as usize;
    let body = &raw[count_offset + 4..];
    if body.len() != count.saturating_mul(8) {
        return None;
    }

    let mut out = HashSet::with_capacity_and_hasher(count, RandomState::new());
    for chunk in body.chunks_exact(8) {
        out.insert(u64::from_le_bytes(chunk.try_into().ok()?));
    }
    Some(out)
}
