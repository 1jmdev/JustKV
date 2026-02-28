mod helpers;
mod keyspace;
mod pattern;
mod strings;
mod ttl;

use std::sync::Arc;

use ahash::RandomState;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::engine::value::Entry;

type StoreMap = HashMap<Box<[u8]>, Entry, RandomState>;

#[derive(Clone)]
pub struct Store {
    shards: Arc<Vec<RwLock<StoreMap>>>,
    shard_mask: usize,
    hash_builder: RandomState,
}

impl Store {
    pub fn new(shards: usize) -> Self {
        let shard_count = shards.max(1).next_power_of_two();
        let mut shard_vec = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shard_vec.push(RwLock::new(HashMap::with_hasher(RandomState::new())));
        }

        Self {
            shards: Arc::new(shard_vec),
            shard_mask: shard_count - 1,
            hash_builder: RandomState::new(),
        }
    }

    pub fn sweep_expired(&self) -> usize {
        let now_ms = helpers::monotonic_now_ms();
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            let before = guard.len();
            guard.retain(|_, entry| !entry.is_expired(now_ms));
            removed += before - guard.len();
        }
        removed
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}
