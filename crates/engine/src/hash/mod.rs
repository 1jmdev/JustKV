mod core;
mod counter;
mod random;
mod scan;

use crate::value::{CompactKey, CompactValue, Entry, HashValueMap};

fn get_hash_map_mut(entry: &mut Entry) -> Option<&mut HashValueMap> {
    let _trace = profiler::scope("crates::engine::src::hash::get_hash_map_mut");
    entry.as_hash_mut()
}

fn get_hash_map(entry: &Entry) -> Option<&HashValueMap> {
    let _trace = profiler::scope("crates::engine::src::hash::get_hash_map");
    entry.as_hash()
}

fn collect_pairs(map: &HashValueMap) -> Vec<(CompactKey, CompactValue)> {
    let _trace = profiler::scope("crates::engine::src::hash::collect_pairs");
    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
