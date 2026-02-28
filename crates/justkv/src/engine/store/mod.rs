mod helpers;
mod keyspace;
mod pattern;
mod strings;
mod ttl;

use std::sync::Arc;

use ahash::RandomState;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::engine::value::{CompactKey, Entry};

type StoreMap = HashMap<CompactKey, Entry, RandomState>;
type TtlMap = HashMap<CompactKey, u64, RandomState>;

#[derive(Clone, Copy, Debug)]
pub enum GetExMode {
    KeepTtl,
    Persist,
    Ex(u64),
    Px(u64),
    ExAt(u64),
    PxAt(u64),
}

pub(super) struct Shard {
    entries: StoreMap,
    ttl: TtlMap,
}

impl Shard {
    fn new() -> Self {
        Self {
            entries: HashMap::with_hasher(RandomState::new()),
            ttl: HashMap::with_hasher(RandomState::new()),
        }
    }
}

#[derive(Clone)]
pub struct Store {
    shards: Arc<Vec<RwLock<Shard>>>,
    shard_mask: usize,
    hash_builder: RandomState,
}

impl Store {
    pub fn new(shards: usize) -> Self {
        let shard_count = shards.max(1).next_power_of_two();
        let mut shard_vec = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shard_vec.push(RwLock::new(Shard::new()));
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
            let expired_keys: Vec<_> = guard
                .ttl
                .iter()
                .filter_map(|(key, &deadline)| (deadline <= now_ms).then(|| key.to_vec()))
                .collect();

            for key in expired_keys {
                guard.ttl.remove(key.as_slice());
                if guard.entries.remove(key.as_slice()).is_some() {
                    removed += 1;
                }
            }
        }
        removed
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}
