mod algebra;
mod core;
mod random;
mod scan;

use ahash::RandomState;
use hashbrown::HashSet;

use crate::value::{CompactKey, Entry, SetValue};

fn get_set(entry: &Entry) -> Option<&SetValue> {
    entry.as_set()
}

fn get_set_mut(entry: &mut Entry) -> Option<&mut SetValue> {
    entry.as_set_mut()
}

fn new_set() -> SetValue {
    HashSet::with_hasher(RandomState::new())
}

fn collect_members(set: &SetValue) -> Vec<CompactKey> {
    set.iter().cloned().collect()
}
