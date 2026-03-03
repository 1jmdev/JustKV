use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use ahash::RandomState;
use hashbrown::HashMap;
use indexmap::IndexSet;

const INLINE_BYTES_CAPACITY: usize = 22;
const INLINE_VALUE_CAPACITY: usize = 22;

#[derive(Clone, Debug)]
pub enum CompactBytes<const INLINE_CAPACITY: usize> {
    Inline {
        len: u8,
        data: [u8; INLINE_CAPACITY],
    },
    Heap(Box<[u8]>),
}

impl<const INLINE_CAPACITY: usize> CompactBytes<INLINE_CAPACITY> {
    pub fn from_slice(value: &[u8]) -> Self {
        let _trace = profiler::scope("engine::value::from_slice");
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..value.len()].copy_from_slice(value);
            Self::Inline {
                len: value.len() as u8,
                data,
            }
        } else {
            Self::Heap(value.to_vec().into_boxed_slice())
        }
    }

    pub fn from_vec(value: Vec<u8>) -> Self {
        let _trace = profiler::scope("engine::value::from_vec");
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..value.len()].copy_from_slice(&value);
            Self::Inline {
                len: value.len() as u8,
                data,
            }
        } else {
            Self::Heap(value.into_boxed_slice())
        }
    }

    /// Internal non-instrumented slice accessor used by trait impls (eq, hash,
    /// borrow, deref, cmp) and hot-path code so they don't pay a nested
    /// profiler scope on top of their own.
    #[inline(always)]
    pub(crate) fn slice(&self) -> &[u8] {
        match self {
            Self::Inline { len, data } => &data[..*len as usize],
            Self::Heap(value) => value,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        let _trace = profiler::scope("engine::value::as_slice");
        self.slice()
    }

    pub fn len(&self) -> usize {
        let _trace = profiler::scope("engine::value::len");
        self.as_slice().len()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let _trace = profiler::scope("engine::value::to_vec");
        self.as_slice().to_vec()
    }

    pub fn into_vec(self) -> Vec<u8> {
        let _trace = profiler::scope("engine::value::into_vec");
        match self {
            Self::Inline { len, data } => data[..len as usize].to_vec(),
            Self::Heap(value) => value.into_vec(),
        }
    }

    pub fn make_ascii_uppercase(&mut self) {
        let _trace = profiler::scope("engine::value::make_ascii_uppercase");
        match self {
            Self::Inline { len, data } => {
                data[..*len as usize].make_ascii_uppercase();
            }
            Self::Heap(value) => {
                value.make_ascii_uppercase();
            }
        }
    }
}

impl<const INLINE_CAPACITY: usize> PartialEq for CompactBytes<INLINE_CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        let _trace = profiler::scope("engine::value::eq");
        match (self, other) {
            (Self::Inline { len: la, data: da }, Self::Inline { len: lb, data: db }) => {
                la == lb && da[..*la as usize] == db[..*lb as usize]
            }
            _ => self.slice() == other.slice(),
        }
    }
}

impl<const INLINE_CAPACITY: usize> Eq for CompactBytes<INLINE_CAPACITY> {}

impl<const INLINE_CAPACITY: usize> Ord for CompactBytes<INLINE_CAPACITY> {
    fn cmp(&self, other: &Self) -> Ordering {
        let _trace = profiler::scope("engine::value::cmp");
        self.slice().cmp(other.slice())
    }
}

impl<const INLINE_CAPACITY: usize> PartialOrd for CompactBytes<INLINE_CAPACITY> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _trace = profiler::scope("engine::value::partial_cmp");
        Some(self.cmp(other))
    }
}

impl<const INLINE_CAPACITY: usize> Hash for CompactBytes<INLINE_CAPACITY> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let _trace = profiler::scope("engine::value::hash");
        self.slice().hash(state);
    }
}

impl<const INLINE_CAPACITY: usize> Borrow<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn borrow(&self) -> &[u8] {
        let _trace = profiler::scope("engine::value::borrow");
        self.slice()
    }
}

impl<const INLINE_CAPACITY: usize> AsRef<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn as_ref(&self) -> &[u8] {
        let _trace = profiler::scope("engine::value::as_ref");
        self.slice()
    }
}

impl<const INLINE_CAPACITY: usize> Deref for CompactBytes<INLINE_CAPACITY> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let _trace = profiler::scope("engine::value::deref");
        self.slice()
    }
}

pub type CompactKey = CompactBytes<INLINE_BYTES_CAPACITY>;
pub type CompactValue = CompactBytes<INLINE_VALUE_CAPACITY>;
pub type CompactArg = CompactBytes<INLINE_BYTES_CAPACITY>;

pub type HashValueMap = HashMap<CompactKey, CompactValue, RandomState>;
pub type ListValue = VecDeque<CompactValue>;
pub type SetValue = IndexSet<CompactKey, RandomState>;
pub type GeoValue = HashMap<CompactKey, (f64, f64), RandomState>;

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
        let _trace = profiler::scope("engine::value::new");
        Self {
            entries: BTreeMap::new(),
            groups: HashMap::with_hasher(RandomState::new()),
            last_id: StreamId { ms: 0, seq: 0 },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ZSetOrderEntry {
    score: f64,
    member: CompactKey,
}

impl ZSetOrderEntry {
    fn new(score: f64, member: CompactKey) -> Self {
        let _trace = profiler::scope("engine::value::new");
        Self { score, member }
    }
}

impl PartialEq for ZSetOrderEntry {
    fn eq(&self, other: &Self) -> bool {
        let _trace = profiler::scope("engine::value::eq");
        self.score.total_cmp(&other.score) == Ordering::Equal && self.member == other.member
    }
}

impl Eq for ZSetOrderEntry {}

impl Ord for ZSetOrderEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let _trace = profiler::scope("engine::value::cmp");
        self.score
            .total_cmp(&other.score)
            .then_with(|| self.member.as_slice().cmp(other.member.as_slice()))
    }
}

impl PartialOrd for ZSetOrderEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _trace = profiler::scope("engine::value::partial_cmp");
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct ZSetValue {
    member_scores: HashMap<CompactKey, f64, RandomState>,
    ordered: BTreeSet<ZSetOrderEntry>,
}

impl ZSetValue {
    pub fn new() -> Self {
        let _trace = profiler::scope("engine::value::new");
        Self {
            member_scores: HashMap::with_hasher(RandomState::new()),
            ordered: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        let _trace = profiler::scope("engine::value::len");
        self.member_scores.len()
    }

    pub fn is_empty(&self) -> bool {
        let _trace = profiler::scope("engine::value::is_empty");
        self.member_scores.is_empty()
    }

    pub fn get(&self, member: &[u8]) -> Option<f64> {
        let _trace = profiler::scope("engine::value::get");
        self.member_scores.get(member).copied()
    }

    pub fn contains_key(&self, member: &[u8]) -> bool {
        let _trace = profiler::scope("engine::value::contains_key");
        self.member_scores.contains_key(member)
    }

    pub fn insert(&mut self, member: CompactKey, score: f64) -> Option<f64> {
        let _trace = profiler::scope("engine::value::insert");
        if let Some(old_score) = self.member_scores.insert(member.clone(), score) {
            let _ = self
                .ordered
                .remove(&ZSetOrderEntry::new(old_score, member.clone()));
            let _ = self.ordered.insert(ZSetOrderEntry::new(score, member));
            Some(old_score)
        } else {
            let _ = self.ordered.insert(ZSetOrderEntry::new(score, member));
            None
        }
    }

    pub fn remove(&mut self, member: &[u8]) -> Option<f64> {
        let _trace = profiler::scope("engine::value::remove");
        let old_score = self.member_scores.remove(member)?;
        let _ = self.ordered.remove(&ZSetOrderEntry::new(
            old_score,
            CompactKey::from_slice(member),
        ));
        Some(old_score)
    }

    pub fn iter_member_scores(&self) -> impl Iterator<Item = (&CompactKey, f64)> {
        let _trace = profiler::scope("engine::value::iter_member_scores");
        self.member_scores
            .iter()
            .map(|(member, score)| (member, *score))
    }

    pub fn iter_ordered(&self, reverse: bool) -> impl Iterator<Item = (&CompactKey, f64)> {
        let _trace = profiler::scope("engine::value::iter_ordered");
        if reverse {
            EitherIter::Rev(self.ordered.iter().rev())
        } else {
            EitherIter::Fwd(self.ordered.iter())
        }
        .map(|entry| (&entry.member, entry.score))
    }
}

impl Default for ZSetValue {
    fn default() -> Self {
        let _trace = profiler::scope("engine::value::default");
        Self::new()
    }
}

enum EitherIter<'a> {
    Fwd(std::collections::btree_set::Iter<'a, ZSetOrderEntry>),
    Rev(std::iter::Rev<std::collections::btree_set::Iter<'a, ZSetOrderEntry>>),
}

impl<'a> Iterator for EitherIter<'a> {
    type Item = &'a ZSetOrderEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let _trace = profiler::scope("engine::value::next");
        match self {
            Self::Fwd(iter) => iter.next(),
            Self::Rev(iter) => iter.next(),
        }
    }
}

pub type ZSetValueMap = ZSetValue;

#[derive(Clone, Debug)]
pub enum Entry {
    String(CompactValue),
    Hash(Box<HashValueMap>),
    List(Box<ListValue>),
    Set(Box<SetValue>),
    ZSet(Box<ZSetValueMap>),
    Geo(Box<GeoValue>),
    Stream(Box<StreamValue>),
}

impl Entry {
    pub fn from_slice(value: &[u8]) -> Self {
        let _trace = profiler::scope("engine::value::from_slice");
        Self::String(CompactValue::from_slice(value))
    }

    pub fn new(value: Vec<u8>) -> Self {
        let _trace = profiler::scope("engine::value::new");
        Self::String(CompactValue::from_vec(value))
    }

    pub fn empty_hash() -> Self {
        let _trace = profiler::scope("engine::value::empty_hash");
        Self::Hash(Box::new(HashMap::with_hasher(RandomState::new())))
    }

    pub fn as_string(&self) -> Option<&CompactValue> {
        let _trace = profiler::scope("engine::value::as_string");
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn into_string(self) -> Option<CompactValue> {
        let _trace = profiler::scope("engine::value::into_string");
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_hash(&self) -> Option<&HashValueMap> {
        let _trace = profiler::scope("engine::value::as_hash");
        match self {
            Self::Hash(value) => Some(value),
            Self::String(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_hash_mut(&mut self) -> Option<&mut HashValueMap> {
        let _trace = profiler::scope("engine::value::as_hash_mut");
        match self {
            Self::Hash(value) => Some(value),
            Self::String(_) => None,
            Self::List(_) => None,
            Self::Set(_) => None,
            Self::ZSet(_) => None,
            Self::Geo(_) => None,
            Self::Stream(_) => None,
        }
    }

    pub fn as_list(&self) -> Option<&ListValue> {
        let _trace = profiler::scope("engine::value::as_list");
        match self {
            Self::List(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListValue> {
        let _trace = profiler::scope("engine::value::as_list_mut");
        match self {
            Self::List(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_set(&self) -> Option<&SetValue> {
        let _trace = profiler::scope("engine::value::as_set");
        match self {
            Self::Set(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_set_mut(&mut self) -> Option<&mut SetValue> {
        let _trace = profiler::scope("engine::value::as_set_mut");
        match self {
            Self::Set(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::ZSet(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_zset(&self) -> Option<&ZSetValueMap> {
        let _trace = profiler::scope("engine::value::as_zset");
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_zset_mut(&mut self) -> Option<&mut ZSetValueMap> {
        let _trace = profiler::scope("engine::value::as_zset_mut");
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::Geo(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_geo(&self) -> Option<&GeoValue> {
        let _trace = profiler::scope("engine::value::as_geo");
        match self {
            Self::Geo(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_geo_mut(&mut self) -> Option<&mut GeoValue> {
        let _trace = profiler::scope("engine::value::as_geo_mut");
        match self {
            Self::Geo(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Stream(_) => None,
        }
    }

    pub fn as_stream(&self) -> Option<&StreamValue> {
        let _trace = profiler::scope("engine::value::as_stream");
        match self {
            Self::Stream(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_) => None,
        }
    }

    pub fn as_stream_mut(&mut self) -> Option<&mut StreamValue> {
        let _trace = profiler::scope("engine::value::as_stream_mut");
        match self {
            Self::Stream(value) => Some(value),
            Self::String(_)
            | Self::Hash(_)
            | Self::List(_)
            | Self::Set(_)
            | Self::ZSet(_)
            | Self::Geo(_) => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        let _trace = profiler::scope("engine::value::kind");
        match self {
            Self::String(_) => "string",
            Self::Hash(_) => "hash",
            Self::List(_) => "list",
            Self::Set(_) => "set",
            Self::ZSet(_) => "zset",
            Self::Geo(_) => "zset",
            Self::Stream(_) => "stream",
        }
    }
}
