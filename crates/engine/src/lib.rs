#![allow(clippy::result_unit_err)]

mod geo;
mod hash;
pub mod helpers;
mod json;
mod keyspace;
mod list;
mod pattern;
pub mod pubsub;
mod script;
mod set;
mod stream;
mod strings;
pub mod transaction;
pub mod ttl;
mod zset;

// Re-export as `store` so that `engine::store::Store` etc. still works
pub mod store {
    pub use super::geo::GeoSearchMatch;
    pub use super::json::{
        JsonError, JsonPath, JsonPathToken, JsonSetMode, JsonSetResult, JsonType,
    };
    pub use super::keyspace::{
        PreDecodedRestoreEntry, RestoreError, SortError, SortOptions, SortOrder, SortResult,
    };
    pub use super::pattern::wildcard_match;
    pub use super::stream::XDelexPolicy;
    pub use super::stream::{StreamRangeItem, StreamWriteError, XPendingSummary};
    pub use super::strings::{
        HyperLogLogError, MSetExExistCondition, SharedTtl, StringDigestCondition, StringIntOpError,
    };
    pub use super::zset::LexBound;
    pub use super::{
        BitFieldEncoding, BitFieldOp, BitFieldOverflow, BitOp, GetExMode, HashFloatOpError,
        HashIntOpError, ListInsertPosition, ListSetError, ListSide, Shard, Store, XAddId,
        XTrimMode,
    };
}

use std::hash::BuildHasher;
use std::hint::spin_loop;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::yield_now;

use parking_lot::RwLock;
use rapidhash::fast::RandomState;

use rehash::RehashingMap;
use types::value::{CompactKey, CompactValue, Entry};

#[derive(Clone, Debug)]
pub(crate) struct StoredEntry {
    pub(crate) entry: Entry,
}

impl StoredEntry {
    pub(crate) fn new(entry: Entry) -> Self {
        Self { entry }
    }

    #[inline]
    pub(crate) fn hash_getall_cache(&self) -> Option<&bytes::Bytes> {
        self.entry.hash_getall_cache()
    }

    #[inline]
    pub(crate) fn set_hash_getall_cache(&mut self, encoded: bytes::Bytes) {
        let _ = self.entry.set_hash_getall_cache(encoded);
    }

    #[inline]
    pub(crate) fn invalidate_hash_getall_cache(&mut self) {
        self.entry.invalidate_hash_getall_cache();
    }
}

impl Deref for StoredEntry {
    type Target = Entry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl DerefMut for StoredEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

type StoreMap = RehashingMap<CompactKey, StoredEntry>;
type ScriptMap = RehashingMap<CompactKey, CompactValue>;

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
    pub(crate) expirations: RehashingMap<CompactKey, u64>,
    /// Tracks the smallest known deadline so sweep can skip shards that have
    /// no expired keys without rebuilding the full sorted structure.
    pub(crate) ttl_min_deadline: u64,
    pub(crate) ttl_count: usize,
}

impl Shard {
    fn new() -> Self {
        Self {
            entries: RehashingMap::with_capacity(256),
            expirations: RehashingMap::with_capacity(64),
            ttl_min_deadline: u64::MAX,
            ttl_count: 0,
        }
    }

    #[inline]
    fn track_deadline(&mut self, deadline: u64) {
        if deadline < self.ttl_min_deadline {
            self.ttl_min_deadline = deadline;
        }
    }

    #[inline]
    pub fn has_ttls(&self) -> bool {
        self.ttl_count != 0
    }

    #[inline]
    pub fn ttl_deadline(&self, key: &[u8]) -> Option<u64> {
        self.expirations.get(key).copied()
    }

    #[inline]
    pub fn is_expired(&self, key: &[u8], now_ms: u64) -> bool {
        self.ttl_deadline(key)
            .is_some_and(|deadline| now_ms >= deadline)
    }

    #[inline]
    pub fn set_ttl(&mut self, key: &[u8], deadline: u64) -> bool {
        if !self.entries.contains_key(key) {
            return false;
        }
        if self
            .expirations
            .insert(CompactKey::from_slice(key), deadline)
            .is_none()
        {
            self.ttl_count += 1;
        }
        self.track_deadline(deadline);
        true
    }

    #[inline]
    pub fn set_ttl_existing(&mut self, key: &[u8], deadline: u64) -> bool {
        if self
            .expirations
            .insert(CompactKey::from_slice(key), deadline)
            .is_none()
        {
            self.ttl_count += 1;
        }
        self.track_deadline(deadline);
        true
    }

    pub fn clear_ttl(&mut self, key: &[u8]) -> Option<u64> {
        if self.ttl_count == 0 {
            return None;
        }
        let previous = self.expirations.remove(key);
        if previous.is_some() {
            self.ttl_count -= 1;
            if self.ttl_count == 0 {
                self.ttl_min_deadline = u64::MAX;
            }
        }
        previous
    }

    #[inline]
    fn update_deadline_for_existing_entry(&mut self, key: &[u8], deadline: Option<u64>) {
        match deadline {
            Some(deadline) => {
                if self
                    .expirations
                    .insert(CompactKey::from_slice(key), deadline)
                    .is_none()
                {
                    self.ttl_count += 1;
                }
                self.track_deadline(deadline);
            }
            None => {
                let _ = self.clear_ttl(key);
                if self.ttl_count == 0 {
                    self.ttl_min_deadline = u64::MAX;
                }
            }
        }
    }

    #[inline]
    pub(crate) fn upsert_string(&mut self, key: &[u8], value: &[u8], deadline: Option<u64>) {
        if let Some(entry) = self.entries.get_mut::<[u8]>(key) {
            entry.entry = Entry::String(CompactValue::from_slice(value));
            self.update_deadline_for_existing_entry(key, deadline);
            return;
        }

        self.insert_entry(
            CompactKey::from_slice(key),
            Entry::String(CompactValue::from_slice(value)),
            deadline,
        );
    }

    pub fn insert_entry(&mut self, key: CompactKey, entry: Entry, deadline: Option<u64>) {
        if let Some(deadline) = deadline {
            if self
                .entries
                .insert(key.clone(), StoredEntry::new(entry))
                .is_some()
                && self.ttl_count != 0
                && self.expirations.remove(key.as_slice()).is_some()
            {
                self.ttl_count -= 1;
            }
            if self.expirations.insert(key, deadline).is_none() {
                self.ttl_count += 1;
            }
            self.track_deadline(deadline);
        } else {
            if self.ttl_count == 0 {
                let _ = self.entries.insert(key, StoredEntry::new(entry));
                self.ttl_min_deadline = u64::MAX;
            } else if self
                .entries
                .insert(key.clone(), StoredEntry::new(entry))
                .is_some()
                && self.expirations.remove(key.as_slice()).is_some()
            {
                self.ttl_count -= 1;
            }
        }
    }

    pub fn remove_key(&mut self, key: &[u8]) -> Option<Entry> {
        let entry = self.entries.remove(key)?;
        if self.ttl_count != 0 && self.expirations.remove(key).is_some() {
            self.ttl_count -= 1;
            if self.ttl_count == 0 {
                self.ttl_min_deadline = u64::MAX;
            }
        }
        Some(entry.entry)
    }
}

#[derive(Clone)]
pub struct Store {
    pub(crate) shards: Arc<Vec<RwLock<Shard>>>,
    pub(crate) shard_mask: usize,
    pub(crate) hash_builder: RandomState,
    pub(crate) scripts: Arc<RwLock<ScriptMap>>,
    pub(crate) transaction_gate: Arc<RwLock<()>>,
    pub(crate) writer_pending: Arc<AtomicBool>,
    pub(crate) active_commands: Arc<AtomicUsize>,
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
            scripts: Arc::new(RwLock::new(RehashingMap::with_capacity(64))),
            transaction_gate: Arc::new(RwLock::new(())),
            writer_pending: Arc::new(AtomicBool::new(false)),
            active_commands: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[inline]
    pub fn with_command_gate<T>(&self, operation: impl FnOnce() -> T) -> T {
        const SPIN_LIMIT: usize = 64;
        loop {
            let mut spins = 0usize;
            while self.writer_pending.load(Ordering::Acquire) {
                if spins < SPIN_LIMIT {
                    spin_loop();
                    spins += 1;
                } else {
                    yield_now();
                    spins = 0;
                }
            }

            self.active_commands.fetch_add(1, Ordering::Acquire);
            if !self.writer_pending.load(Ordering::Acquire) {
                let result = operation();
                self.active_commands.fetch_sub(1, Ordering::Release);
                return result;
            }

            self.active_commands.fetch_sub(1, Ordering::Release);
        }
    }

    #[inline]
    pub fn with_watch_gate<T>(&self, operation: impl FnOnce() -> T) -> T {
        self.with_command_gate(operation)
    }

    #[inline]
    pub fn with_transaction_gate<T>(&self, operation: impl FnOnce() -> T) -> T {
        const SPIN_LIMIT: usize = 64;
        let _guard = self.transaction_gate.write();
        self.writer_pending.store(true, Ordering::Release);
        let mut spins = 0usize;
        while self.active_commands.load(Ordering::Acquire) != 0 {
            if spins < SPIN_LIMIT {
                spin_loop();
                spins += 1;
            } else {
                yield_now();
                spins = 0;
            }
        }
        let result = operation();
        self.writer_pending.store(false, Ordering::Release);
        result
    }

    pub fn sweep_expired(&self) -> usize {
        let now_ms = helpers::monotonic_now_ms();
        let mut removed = 0;
        for shard in self.shards.iter() {
            let mut guard = shard.write();
            // Fast check: if the minimum-deadline hint is in the future, skip.
            if !guard.has_ttls() {
                guard.ttl_min_deadline = u64::MAX;
                continue;
            }
            if guard.ttl_min_deadline > now_ms {
                continue;
            }

            let mut new_min = u64::MAX;
            let mut expired_count = 0usize;
            for (_key, deadline) in guard.expirations.iter() {
                if *deadline <= now_ms {
                    expired_count += 1;
                } else {
                    new_min = new_min.min(*deadline);
                }
            }

            if expired_count == 0 {
                guard.ttl_min_deadline = new_min;
                continue;
            }

            let mut expired_keys = Vec::with_capacity(expired_count);
            for (key, deadline) in guard.expirations.iter() {
                if *deadline <= now_ms {
                    expired_keys.push(key.clone());
                }
            }

            for key in &expired_keys {
                if guard.remove_key(key.as_slice()).is_some() {
                    removed += 1;
                }
            }

            guard.ttl_min_deadline = new_min;
        }
        removed
    }

    pub fn refresh_cached_time(&self) {
        helpers::refresh_monotonic_now_ms();
    }

    pub(crate) fn shard_index(&self, key: &[u8]) -> usize {
        let hash = self.hash_builder.hash_one(key);
        (hash as usize) & self.shard_mask
    }
}
