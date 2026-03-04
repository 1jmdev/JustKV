mod geo;
mod hash;
pub mod helpers;
mod keyspace;
mod list;
mod pattern;
mod script;
mod set;
mod stream;
mod strings;
pub mod ttl;
mod zset;

pub mod value;

// Re-export as `store` so that `engine::store::Store` etc. still works
pub mod store {
    pub use super::geo::GeoSearchMatch;
    pub use super::keyspace::{RestoreError, SortError, SortOptions, SortOrder, SortResult};
    pub use super::stream::{StreamRangeItem, XPendingSummary};
    pub use super::{
        BitFieldEncoding, BitFieldOp, BitFieldOverflow, BitOp, GetExMode, HashFloatOpError,
        HashIntOpError, ListInsertPosition, ListSetError, ListSide, Shard, Store, XAddId,
        XTrimMode,
    };
}

use std::sync::Arc;

use ahash::RandomState;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::value::{CompactKey, Entry};
use rehash::RehashingMap;

type StoreMap = RehashingMap<CompactKey, Entry>;
type TtlMap = HashMap<CompactKey, u64, RandomState>;
type ScriptMap = HashMap<CompactKey, crate::value::CompactValue, RandomState>;

#[derive(Clone, Copy, Debug)]
pub enum GetExMode {
    KeepTtl,
    Persist,
    Ex(u64),
    Px(u64),
    ExAt(u64),
    PxAt(u64),
}

#[derive(Clone, Copy, Debug)]
pub enum HashIntOpError {
    WrongType,
    InvalidInteger,
    Overflow,
}

#[derive(Clone, Copy, Debug)]
pub enum HashFloatOpError {
    WrongType,
    InvalidFloat,
}

#[derive(Clone, Copy, Debug)]
pub enum ListSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum ListInsertPosition {
    Before,
    After,
}

#[derive(Clone, Copy, Debug)]
pub enum ListSetError {
    NoSuchKey,
    OutOfRange,
    WrongType,
}

#[derive(Clone, Copy, Debug)]
pub enum BitOp {
    And,
    Or,
    Xor,
    Not,
}

#[derive(Clone, Copy, Debug)]
pub enum BitFieldEncoding {
    Signed { bits: u8 },
    Unsigned { bits: u8 },
}

#[derive(Clone, Copy, Debug)]
pub enum BitFieldOverflow {
    Wrap,
    Sat,
    Fail,
}

#[derive(Clone, Copy, Debug)]
pub enum BitFieldOp {
    Get {
        encoding: BitFieldEncoding,
        offset: usize,
    },
    Set {
        encoding: BitFieldEncoding,
        offset: usize,
        value: i64,
    },
    IncrBy {
        encoding: BitFieldEncoding,
        offset: usize,
        increment: i64,
        overflow: BitFieldOverflow,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum XAddId {
    Auto,
    Explicit { ms: u64, seq: u64 },
    AutoSeqAtMs { ms: u64 },
}

#[derive(Clone, Copy, Debug)]
pub enum XTrimMode {
    MaxLen,
    MinId,
}

pub struct Shard {
    pub(crate) entries: StoreMap,
    pub(crate) ttl: TtlMap,
    /// Tracks the smallest known deadline so sweep can skip shards that have
    /// no expired keys without rebuilding the full sorted structure.
    pub(crate) ttl_min_deadline: u64,
}

impl Shard {
    fn new() -> Self {
        let _trace = profiler::scope("engine::lib::new");
        Self {
            entries: RehashingMap::new(),
            ttl: HashMap::with_hasher(RandomState::new()),
            ttl_min_deadline: u64::MAX,
        }
    }

    /// Fast O(1) amortized TTL set — only a single HashMap insert.
    /// No heap push, no key clone beyond what the HashMap needs.
    #[inline]
    pub fn set_ttl(&mut self, key: CompactKey, deadline: u64) {
        let _trace = profiler::scope("engine::lib::set_ttl");
        let _ = self.ttl.insert(key, deadline);
        // Maintain the minimum-deadline hint cheaply.
        if deadline < self.ttl_min_deadline {
            self.ttl_min_deadline = deadline;
        }
    }

    /// Variant that avoids allocating a CompactKey when the caller only has a
    /// byte slice and the key is already present in the TTL map.
    #[inline]
    pub fn set_ttl_existing(&mut self, key: &[u8], deadline: u64) {
        let _trace = profiler::scope("engine::lib::set_ttl_existing");
        // RawEntryMut lets us look up by &[u8] and only allocate a CompactKey
        // on the (rare) insert-new path.
        use hashbrown::hash_map::RawEntryMut;
        let hash = self.ttl.hasher().hash_one(key);
        match self.ttl.raw_entry_mut().from_key_hashed_nocheck(hash, key) {
            RawEntryMut::Occupied(mut occ) => {
                *occ.get_mut() = deadline;
            }
            RawEntryMut::Vacant(vac) => {
                vac.insert_hashed_nocheck(hash, CompactKey::from_slice(key), deadline);
            }
        }
        if deadline < self.ttl_min_deadline {
            self.ttl_min_deadline = deadline;
        }
    }

    pub fn clear_ttl(&mut self, key: &[u8]) -> Option<u64> {
        let _trace = profiler::scope("engine::lib::clear_ttl");
        self.ttl.remove(key)
        // Note: we intentionally do NOT update ttl_min_deadline here.
        // It stays as a lower bound; sweep will simply find nothing to
        // remove for that timestamp and the hint gets refreshed during sweep.
    }

    pub fn remove_key(&mut self, key: &[u8]) -> Option<Entry> {
        let _trace = profiler::scope("engine::lib::remove_key");
        let _ = self.clear_ttl(key);
        self.entries.remove(key)
    }
}

#[derive(Clone)]
pub struct Store {
    pub(crate) shards: Arc<Vec<RwLock<Shard>>>,
    pub(crate) shard_mask: usize,
    pub(crate) hash_builder: RandomState,
    pub(crate) scripts: Arc<RwLock<ScriptMap>>,
}

impl Store {
    pub fn new(shards: usize) -> Self {
        let _trace = profiler::scope("engine::lib::new");
        let shard_count = shards.max(1).next_power_of_two();
        let mut shard_vec = Vec::with_capacity(shard_count);

        for _ in 0..shard_count {
            shard_vec.push(RwLock::new(Shard::new()));
        }

        Self {
            shards: Arc::new(shard_vec),
            shard_mask: shard_count - 1,
            hash_builder: RandomState::new(),
            scripts: Arc::new(RwLock::new(HashMap::with_hasher(RandomState::new()))),
        }
    }

    pub fn sweep_expired(&self) -> usize {
        let _trace = profiler::scope("engine::lib::sweep_expired");
        let now_ms = helpers::monotonic_now_ms();
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            // Fast check: if the minimum-deadline hint is in the future, skip.
            if guard.ttl_min_deadline > now_ms {
                continue;
            }

            let mut new_min = u64::MAX;
            let expired_keys: Vec<CompactKey> = guard
                .ttl
                .iter()
                .filter_map(|(key, &deadline)| {
                    if deadline <= now_ms {
                        Some(key.clone())
                    } else {
                        new_min = new_min.min(deadline);
                        None
                    }
                })
                .collect();

            for key in &expired_keys {
                guard.ttl.remove(key.as_slice());
                if guard.entries.remove(key.as_slice()).is_some() {
                    removed += 1;
                }
            }

            guard.ttl_min_deadline = new_min;
        }
        removed
    }

    pub fn refresh_cached_time(&self) {
        let _trace = profiler::scope("engine::lib::refresh_cached_time");
        helpers::refresh_monotonic_now_ms();
    }

    pub(crate) fn shard_index(&self, key: &[u8]) -> usize {
        let _trace = profiler::scope("engine::lib::shard_index");
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}
