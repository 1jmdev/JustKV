mod algebra;
mod core;
mod random;
mod scan;

use rapidhash::fast::RandomState;

use types::value::{CompactKey, Entry, SetValue};

fn get_set(entry: &Entry) -> Option<&SetValue> {
    entry.as_set()
}

fn get_set_mut(entry: &mut Entry) -> Option<&mut SetValue> {
    entry.as_set_mut()
}

fn new_set() -> SetValue {
    SetValue::with_hasher(RandomState::new())
}

fn new_set_with_capacity(capacity: usize) -> SetValue {
    SetValue::with_capacity_and_hasher(capacity, RandomState::new())
}

fn collect_members(set: &SetValue) -> Vec<CompactKey> {
    set.iter().cloned().collect()
}
