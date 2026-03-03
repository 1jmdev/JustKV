use crate::store::Store;
use crate::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};

impl Store {
    pub fn mget<K: AsRef<[u8]>>(&self, keys: &[K]) -> Result<Vec<Option<CompactValue>>, ()> {
        let _trace = profiler::scope("engine::strings::multi::mget");
        keys.iter().map(|key| self.get(key.as_ref())).collect()
    }

    pub fn mset_args(&self, pairs: &[CompactArg]) {
        let _trace = profiler::scope("engine::strings::multi::mset_args");
        let shard_count = self.shards.len();

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

            for (key, entry) in entries {
                if has_ttl {
                    let _ = shard.clear_ttl(key.as_slice());
                }
                shard.entries.insert(key, entry);
            }
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
            for (key, entry) in entries {
                if has_ttl {
                    let _ = shard.clear_ttl(key.as_slice());
                }
                shard.entries.insert(key, entry);
            }
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
