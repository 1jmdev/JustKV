use std::collections::BTreeMap;

use ahash::RandomState;
use hashbrown::HashMap;

use super::{CompactKey, CompactValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId {
    pub ms: u64,
    pub seq: u64,
}

#[derive(Clone, Debug)]
pub struct StreamPendingEntry {
    pub consumer: CompactKey,
    pub deliveries: u64,
}

#[derive(Clone, Debug)]
pub struct StreamGroup {
    pub last_delivered: StreamId,
    pub pending: HashMap<StreamId, StreamPendingEntry, RandomState>,
}

#[derive(Clone, Debug)]
pub struct StreamValue {
    pub entries: BTreeMap<StreamId, Vec<(CompactKey, CompactValue)>>,
    pub groups: HashMap<CompactKey, StreamGroup, RandomState>,
    pub last_id: StreamId,
}

impl StreamValue {
    pub fn new() -> Self {
        let _trace = profiler::scope("crates::types::src::value::new");
        Self {
            entries: BTreeMap::new(),
            groups: HashMap::with_hasher(RandomState::new()),
            last_id: StreamId { ms: 0, seq: 0 },
        }
    }
}

impl Default for StreamValue {
    fn default() -> Self {
        Self::new()
    }
}
