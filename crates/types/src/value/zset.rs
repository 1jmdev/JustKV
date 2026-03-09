use std::cmp::Ordering;
use std::collections::BTreeSet;

use ahash::RandomState;
use hashbrown::HashMap;

use super::CompactKey;

#[derive(Clone, Debug)]
struct ZSetOrderEntry {
    score: f64,
    member: CompactKey,
}

impl ZSetOrderEntry {
    fn new(score: f64, member: CompactKey) -> Self {
        let _trace = profiler::scope("crates::types::src::value::new");
        Self { score, member }
    }
}

impl PartialEq for ZSetOrderEntry {
    fn eq(&self, other: &Self) -> bool {
        let _trace = profiler::scope("crates::types::src::value::eq");
        self.score.total_cmp(&other.score) == Ordering::Equal && self.member == other.member
    }
}

impl Eq for ZSetOrderEntry {}

impl Ord for ZSetOrderEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let _trace = profiler::scope("crates::types::src::value::cmp");
        self.score
            .total_cmp(&other.score)
            .then_with(|| self.member.as_slice().cmp(other.member.as_slice()))
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for ZSetOrderEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _trace = profiler::scope("crates::types::src::value::partial_cmp");
        Some(Ord::cmp(self, other))
    }
}

#[derive(Clone, Debug)]
pub struct ZSetValue {
    member_scores: HashMap<CompactKey, f64, RandomState>,
    ordered: BTreeSet<ZSetOrderEntry>,
}

impl ZSetValue {
    pub fn new() -> Self {
        let _trace = profiler::scope("crates::types::src::value::new");
        Self {
            member_scores: HashMap::with_hasher(RandomState::new()),
            ordered: BTreeSet::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let _trace = profiler::scope("crates::types::src::value::with_capacity");
        Self {
            member_scores: HashMap::with_capacity_and_hasher(capacity, RandomState::new()),
            ordered: BTreeSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        let _trace = profiler::scope("crates::types::src::value::len");
        self.member_scores.len()
    }

    pub fn is_empty(&self) -> bool {
        let _trace = profiler::scope("crates::types::src::value::is_empty");
        self.member_scores.is_empty()
    }

    pub fn get(&self, member: &[u8]) -> Option<f64> {
        let _trace = profiler::scope("crates::types::src::value::get");
        self.member_scores.get(member).copied()
    }

    pub fn contains_key(&self, member: &[u8]) -> bool {
        let _trace = profiler::scope("crates::types::src::value::contains_key");
        self.member_scores.contains_key(member)
    }

    pub fn reserve(&mut self, additional: usize) {
        let _trace = profiler::scope("crates::types::src::value::reserve");
        self.member_scores.reserve(additional);
    }

    pub fn insert(&mut self, member: CompactKey, score: f64) -> Option<f64> {
        let _trace = profiler::scope("crates::types::src::value::insert");
        match self.member_scores.entry(member) {
            hashbrown::hash_map::Entry::Occupied(mut occ) => {
                let old_score = *occ.get();
                if old_score != score {
                    let _ = self.ordered.remove(&ZSetOrderEntry {
                        score: old_score,
                        member: occ.key().clone(),
                    });
                    *occ.get_mut() = score;
                    let _ = self.ordered.insert(ZSetOrderEntry {
                        score,
                        member: occ.key().clone(),
                    });
                }
                Some(old_score)
            }
            hashbrown::hash_map::Entry::Vacant(vac) => {
                let member_clone = vac.key().clone();
                vac.insert(score);
                let _ = self.ordered.insert(ZSetOrderEntry {
                    score,
                    member: member_clone,
                });
                None
            }
        }
    }

    pub fn remove(&mut self, member: &[u8]) -> Option<f64> {
        let _trace = profiler::scope("crates::types::src::value::remove");
        let old_score = self.member_scores.remove(member)?;
        let _ = self.ordered.remove(&ZSetOrderEntry::new(
            old_score,
            CompactKey::from_slice(member),
        ));
        Some(old_score)
    }

    pub fn iter_member_scores(&self) -> impl Iterator<Item = (&CompactKey, f64)> {
        let _trace = profiler::scope("crates::types::src::value::iter_member_scores");
        self.member_scores
            .iter()
            .map(|(member, score)| (member, *score))
    }

    pub fn iter_ordered(&self, reverse: bool) -> impl Iterator<Item = (&CompactKey, f64)> {
        let _trace = profiler::scope("crates::types::src::value::iter_ordered");
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
        let _trace = profiler::scope("crates::types::src::value::default");
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
        let _trace = profiler::scope("crates::types::src::value::next");
        match self {
            Self::Fwd(iter) => iter.next(),
            Self::Rev(iter) => iter.next(),
        }
    }
}

pub type ZSetValueMap = ZSetValue;
