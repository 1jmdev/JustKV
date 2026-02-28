use std::sync::Arc;
use std::time::{Duration, Instant};

use ahash::RandomState;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::engine::value::Entry;

type StoreMap = HashMap<Vec<u8>, Entry, RandomState>;

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

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        let expired = shard.get(key).map(Entry::is_expired).unwrap_or(false);
        if expired {
            shard.remove(key);
            return None;
        }
        shard.get(key).map(|entry| entry.value.clone())
    }

    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, ttl: Option<Duration>) {
        let idx = self.shard_index(&key);
        let expires_at = ttl.map(|timeout| Instant::now() + timeout);
        let entry = Entry { value, expires_at };
        self.shards[idx].write().insert(key, entry);
    }

    pub fn del(&self, keys: &[Vec<u8>]) -> i64 {
        let mut removed = 0;
        for key in keys {
            let idx = self.shard_index(key);
            if self.shards[idx].write().remove(key).is_some() {
                removed += 1;
            }
        }
        removed
    }

    pub fn exists(&self, keys: &[Vec<u8>]) -> i64 {
        let mut count = 0;
        for key in keys {
            if self.get(key).is_some() {
                count += 1;
            }
        }
        count
    }

    pub fn incr(&self, key: &[u8]) -> Result<i64, ()> {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();

        let expired = shard.get(key).map(Entry::is_expired).unwrap_or(false);
        if expired {
            shard.remove(key);
        }

        let next_value = match shard.get(key) {
            Some(entry) => {
                let text = std::str::from_utf8(&entry.value).map_err(|_| ())?;
                text.parse::<i64>().map_err(|_| ())? + 1
            }
            None => 1,
        };

        shard.insert(
            key.to_vec(),
            Entry {
                value: next_value.to_string().into_bytes(),
                expires_at: None,
            },
        );

        Ok(next_value)
    }

    pub fn mget(&self, keys: &[Vec<u8>]) -> Vec<Option<Vec<u8>>> {
        keys.iter().map(|key| self.get(key)).collect()
    }

    pub fn mset(&self, pairs: &[(Vec<u8>, Vec<u8>)]) {
        for (key, value) in pairs {
            self.set(key.clone(), value.clone(), None);
        }
    }

    pub fn expire(&self, key: &[u8], seconds: u64) -> i64 {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        if let Some(entry) = shard.get_mut(key) {
            if entry.is_expired() {
                shard.remove(key);
                return 0;
            }
            entry.expires_at = Some(Instant::now() + Duration::from_secs(seconds));
            return 1;
        }
        0
    }

    pub fn ttl(&self, key: &[u8]) -> i64 {
        let idx = self.shard_index(key);
        let mut shard = self.shards[idx].write();
        match shard.get(key) {
            Some(entry) => {
                if entry.is_expired() {
                    shard.remove(key);
                    return -2;
                }

                match entry.expires_at {
                    Some(deadline) => {
                        deadline.saturating_duration_since(Instant::now()).as_secs() as i64
                    }
                    None => -1,
                }
            }
            None => -2,
        }
    }

    pub fn sweep_expired(&self) -> usize {
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            let before = guard.len();
            guard.retain(|_, entry| !entry.is_expired());
            removed += before - guard.len();
        }
        removed
    }

    fn shard_index(&self, key: &[u8]) -> usize {
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}
