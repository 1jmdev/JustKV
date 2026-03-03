use crate::store::Store;
use crate::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};

impl Store {
    pub fn mget<K: AsRef<[u8]>>(&self, keys: &[K]) -> Result<Vec<Option<CompactValue>>, ()> {
        let _trace = profiler::scope("engine::strings::multi::mget");
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
                if !shard.ttl.is_empty()
                    && shard
                        .ttl
                        .get(key)
                        .copied()
                        .is_some_and(|deadline| now_ms >= deadline)
                {
                    continue;
                }
                match entry.as_string() {
                    Some(value) => out[pos] = Some(value.clone()),
                    None => return Err(()),
                }
            }
            return Ok(out);
        }

        let shard_count = self.shards.len();
        let mut grouped = vec![Vec::new(); shard_count];
        for (pos, key) in keys.iter().enumerate() {
            grouped[self.shard_index(key.as_ref())].push(pos);
        }

        for (idx, positions) in grouped.into_iter().enumerate() {
            if positions.is_empty() {
                continue;
            }

            let shard = self.shards[idx].read();
            let has_ttl = !shard.ttl.is_empty();

            for pos in positions {
                let key = keys[pos].as_ref();
                let Some(entry) = shard.entries.get::<[u8]>(key) else {
                    continue;
                };
                if has_ttl
                    && shard
                        .ttl
                        .get(key)
                        .copied()
                        .is_some_and(|deadline| now_ms >= deadline)
                {
                    continue;
                }
                match entry.as_string() {
                    Some(value) => out[pos] = Some(value.clone()),
                    None => return Err(()),
                }
            }
        }

        Ok(out)
    }

    pub fn mset_args(&self, pairs: &[CompactArg]) {
        let _trace = profiler::scope("engine::strings::multi::mset_args");
        let shard_count = self.shards.len();
        let pair_count = pairs.len() / 2;

        if pair_count <= 2 {
            for chunk in pairs.chunks_exact(2) {
                let key = chunk[0].as_slice();
                let value = chunk[1].as_slice();
                let idx = self.shard_index(key);
                let mut shard = self.shards[idx].write();
                if !shard.ttl.is_empty() {
                    let _ = shard.clear_ttl(key);
                }
                shard
                    .entries
                    .insert(CompactKey::from_slice(key), Entry::from_slice(value));
            }
            return;
        }

        // Pre-size per-shard buffers to avoid repeated growth when MSET carries
        // many keys.
        let mut per_shard_counts = vec![0usize; shard_count];
        for chunk in pairs.chunks_exact(2) {
            let idx = self.shard_index(chunk[0].as_slice());
            per_shard_counts[idx] += 1;
        }

        let mut grouped: Vec<Vec<(CompactKey, Entry)>> = per_shard_counts
            .into_iter()
            .map(Vec::with_capacity)
            .collect();

        for chunk in pairs.chunks_exact(2) {
            let key = &chunk[0];
            let value = &chunk[1];
            let idx = self.shard_index(key.as_slice());
            grouped[idx].push((
                CompactKey::from_slice(key.as_slice()),
                Entry::from_slice(value.as_slice()),
            ));
        }

        for (idx, entries) in grouped.into_iter().enumerate() {
            if entries.is_empty() {
                continue;
            }

            let mut shard = self.shards[idx].write();
            let has_ttl = !shard.ttl.is_empty();

            if has_ttl {
                for (key, _) in &entries {
                    let _ = shard.clear_ttl(key.as_slice());
                }
            }

            shard.entries.insert_batch(entries);
        }
    }

    pub fn mset(&self, pairs: Vec<(CompactArg, CompactArg)>) {
        let _trace = profiler::scope("engine::strings::multi::mset");
        let shard_count = self.shards.len();
        let mut grouped = vec![Vec::new(); shard_count];

        for (key, value) in pairs {
            let idx = self.shard_index(&key);
            grouped[idx].push((CompactKey::from_slice(&key), Entry::from_slice(&value)));
        }

        for (idx, entries) in grouped.into_iter().enumerate() {
            if entries.is_empty() {
                continue;
            }

            let mut shard = self.shards[idx].write();
            let has_ttl = !shard.ttl.is_empty();

            if has_ttl {
                for (key, _) in &entries {
                    let _ = shard.clear_ttl(key.as_slice());
                }
            }

            shard.entries.insert_batch(entries);
        }
    }

    pub fn msetnx(&self, pairs: Vec<(CompactArg, CompactArg)>) -> bool {
        let _trace = profiler::scope("engine::strings::multi::msetnx");
        let now_ms = monotonic_now_ms();
        for (key, _) in &pairs {
            let idx = self.shard_index(key);
            let mut shard = self.shards[idx].write();
            if !purge_if_expired(&mut shard, key, now_ms) && shard.entries.contains_key(key) {
                return false;
            }
        }

        self.mset(pairs);
        true
    }
}
