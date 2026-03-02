mod geo;
mod hash;
pub mod helpers;
mod keyspace;
mod list;
mod pattern;
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

use std::collections::BTreeSet;
use std::sync::Arc;

use ahash::RandomState;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::value::{CompactKey, Entry};
use rehash::RehashingMap;

type StoreMap = RehashingMap<CompactKey, Entry>;
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
    pub(crate) ttl_deadlines: BTreeSet<(u64, CompactKey)>,
}

impl Shard {
    fn new() -> Self {
        let _trace = profiler::scope("engine::lib::new");
        Self {
            entries: RehashingMap::new(),
            ttl: HashMap::with_hasher(RandomState::new()),
            ttl_deadlines: BTreeSet::new(),
        }
    }

    pub fn set_ttl(&mut self, key: CompactKey, deadline: u64) {
        let _trace = profiler::scope("engine::lib::set_ttl");
        if let Some(previous_deadline) = self.ttl.insert(key.clone(), deadline) {
            let _ = self.ttl_deadlines.remove(&(previous_deadline, key.clone()));
        }
        let _ = self.ttl_deadlines.insert((deadline, key));
    }

    pub fn clear_ttl(&mut self, key: &[u8]) -> Option<u64> {
        let _trace = profiler::scope("engine::lib::clear_ttl");
        let deadline = self.ttl.remove(key)?;
        let _ = self
            .ttl_deadlines
            .remove(&(deadline, CompactKey::from_slice(key)));
        Some(deadline)
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
        }
    }

    pub fn sweep_expired(&self) -> usize {
        let _trace = profiler::scope("engine::lib::sweep_expired");
        let now_ms = helpers::monotonic_now_ms();
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            while let Some((deadline, key)) = guard.ttl_deadlines.first().cloned() {
                if deadline > now_ms {
                    break;
                }

                let _ = guard.ttl_deadlines.pop_first();
                let current = guard.ttl.get(key.as_slice()).copied();
                if current != Some(deadline) {
                    continue;
                }

                guard.ttl.remove(key.as_slice());
                if guard.entries.remove(key.as_slice()).is_some() {
                    removed += 1;
                }
            }
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
