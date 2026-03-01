use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use ahash::RandomState;
use hashbrown::{HashMap, HashSet};

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

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Inline { len, data } => &data[..*len as usize],
            Self::Heap(value) => value,
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Inline { len, data } => data[..len as usize].to_vec(),
            Self::Heap(value) => value.into_vec(),
        }
    }
}

impl<const INLINE_CAPACITY: usize> PartialEq for CompactBytes<INLINE_CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Eq for CompactBytes<INLINE_CAPACITY> {}

impl<const INLINE_CAPACITY: usize> Hash for CompactBytes<INLINE_CAPACITY> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<const INLINE_CAPACITY: usize> Borrow<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> AsRef<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Deref for CompactBytes<INLINE_CAPACITY> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

pub type CompactKey = CompactBytes<INLINE_BYTES_CAPACITY>;
pub type CompactValue = CompactBytes<INLINE_VALUE_CAPACITY>;
pub type CompactArg = CompactBytes<INLINE_BYTES_CAPACITY>;

pub type HashValueMap = HashMap<CompactKey, CompactValue, RandomState>;
pub type ListValue = VecDeque<CompactValue>;
pub type SetValue = HashSet<CompactKey, RandomState>;

#[derive(Clone, Debug)]
pub struct ZSetOrderEntry {
    score: f64,
    member: CompactKey,
}

impl ZSetOrderEntry {
    fn new(score: f64, member: CompactKey) -> Self {
        Self { score, member }
    }
}

impl PartialEq for ZSetOrderEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score.total_cmp(&other.score) == Ordering::Equal && self.member == other.member
    }
}

impl Eq for ZSetOrderEntry {}

impl Ord for ZSetOrderEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| self.member.as_slice().cmp(other.member.as_slice()))
    }
}

impl PartialOrd for ZSetOrderEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
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
        Self {
            member_scores: HashMap::with_hasher(RandomState::new()),
            ordered: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.member_scores.len()
    }

    pub fn is_empty(&self) -> bool {
        self.member_scores.is_empty()
    }

    pub fn get(&self, member: &[u8]) -> Option<f64> {
        self.member_scores.get(member).copied()
    }

    pub fn contains_key(&self, member: &[u8]) -> bool {
        self.member_scores.contains_key(member)
    }

    pub fn insert(&mut self, member: CompactKey, score: f64) -> Option<f64> {
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
        let old_score = self.member_scores.remove(member)?;
        let _ = self.ordered.remove(&ZSetOrderEntry::new(
            old_score,
            CompactKey::from_slice(member),
        ));
        Some(old_score)
    }

    pub fn iter_member_scores(&self) -> impl Iterator<Item = (&CompactKey, f64)> {
        self.member_scores
            .iter()
            .map(|(member, score)| (member, *score))
    }

    pub fn iter_ordered(&self, reverse: bool) -> impl Iterator<Item = (&CompactKey, f64)> {
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
}

impl Entry {
    pub fn from_slice(value: &[u8]) -> Self {
        Self::String(CompactValue::from_slice(value))
    }

    pub fn new(value: Vec<u8>) -> Self {
        Self::String(CompactValue::from_vec(value))
    }

    pub fn empty_hash() -> Self {
        Self::Hash(Box::new(HashMap::with_hasher(RandomState::new())))
    }

    pub fn as_string(&self) -> Option<&CompactValue> {
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_) | Self::List(_) | Self::Set(_) | Self::ZSet(_) => None,
        }
    }

    pub fn into_string(self) -> Option<CompactValue> {
        match self {
            Self::String(value) => Some(value),
            Self::Hash(_) | Self::List(_) | Self::Set(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_hash(&self) -> Option<&HashValueMap> {
        match self {
            Self::Hash(value) => Some(value),
            Self::String(_) | Self::List(_) | Self::Set(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_hash_mut(&mut self) -> Option<&mut HashValueMap> {
        match self {
            Self::Hash(value) => Some(value),
            Self::String(_) => None,
            Self::List(_) => None,
            Self::Set(_) => None,
            Self::ZSet(_) => None,
        }
    }

    pub fn as_list(&self) -> Option<&ListValue> {
        match self {
            Self::List(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::Set(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_list_mut(&mut self) -> Option<&mut ListValue> {
        match self {
            Self::List(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::Set(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_set(&self) -> Option<&SetValue> {
        match self {
            Self::Set(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::List(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_set_mut(&mut self) -> Option<&mut SetValue> {
        match self {
            Self::Set(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::List(_) | Self::ZSet(_) => None,
        }
    }

    pub fn as_zset(&self) -> Option<&ZSetValueMap> {
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::List(_) | Self::Set(_) => None,
        }
    }

    pub fn as_zset_mut(&mut self) -> Option<&mut ZSetValueMap> {
        match self {
            Self::ZSet(value) => Some(value),
            Self::String(_) | Self::Hash(_) | Self::List(_) | Self::Set(_) => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Hash(_) => "hash",
            Self::List(_) => "list",
            Self::Set(_) => "set",
            Self::ZSet(_) => "zset",
        }
    }
}
