mod algebra;
mod core;
mod random;
mod scan;

use ahash::RandomState;

use crate::value::{CompactKey, Entry, SetValue};

fn get_set(entry: &Entry) -> Option<&SetValue> {
    let _trace = profiler::scope("engine::set::get_set");
    entry.as_set()
}

fn get_set_mut(entry: &mut Entry) -> Option<&mut SetValue> {
    let _trace = profiler::scope("engine::set::get_set_mut");
    entry.as_set_mut()
}

fn new_set() -> SetValue {
    let _trace = profiler::scope("engine::set::new_set");
    SetValue::with_hasher(RandomState::new())
}

fn collect_members(set: &SetValue) -> Vec<CompactKey> {
    let _trace = profiler::scope("engine::set::collect_members");
    set.iter().cloned().collect()
}
