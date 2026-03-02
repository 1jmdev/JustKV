use crate::store::Store;
use crate::value::{CompactArg, CompactKey, CompactValue, Entry};

use super::super::helpers::{monotonic_now_ms, purge_if_expired};

impl Store {
    pub fn mget<K: AsRef<[u8]>>(&self, keys: &[K]) -> Result<Vec<Option<CompactValue>>, ()> {
        let _trace = profiler::scope("engine::strings::multi::mget");
        keys.iter().map(|key| self.get(key.as_ref())).collect()
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
            for (key, entry) in entries {
                let _ = shard.clear_ttl(key.as_slice());
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
