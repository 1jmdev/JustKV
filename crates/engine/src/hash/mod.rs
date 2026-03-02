mod core;
mod counter;
mod random;
mod scan;

use crate::value::{CompactKey, CompactValue, Entry, HashValueMap};

fn get_hash_map_mut(entry: &mut Entry) -> Option<&mut HashValueMap> {
    entry.as_hash_mut()
}

fn get_hash_map(entry: &Entry) -> Option<&HashValueMap> {
    entry.as_hash()
}

fn collect_pairs(map: &HashValueMap) -> Vec<(CompactKey, CompactValue)> {
    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
