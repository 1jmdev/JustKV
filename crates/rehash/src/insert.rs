use std::hash::Hash;

use super::constants::{NIL, REHASH_STEPS_PER_WRITE};
use super::index::{bucket_index_from_hash, hash_key};
use super::node::Node;
use super::types::{RehashingMap, TargetTable};

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let _trace = profiler::scope("crates::rehash::src::insert::insert");
        self.rehash_step(REHASH_STEPS_PER_WRITE);

        let hash = hash_key(&self.hash_builder, &key);
        if let Some(idx) = self.find_index_hashed(&key, hash) {
            let node = self.nodes[idx as usize].as_mut().unwrap();
            return Some(std::mem::replace(&mut node.value, value));
        }

        let target = if self.rehash_table.is_some() {
            TargetTable::New
        } else {
            TargetTable::Old
        };
        self.insert_new(target, hash, key, value);
        self.len += 1;
        self.maybe_start_rehash();
        None
    }

    pub fn get_or_insert_with<F>(&mut self, key: K, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        let _trace = profiler::scope("crates::rehash::src::insert::get_or_insert_with");
        self.rehash_step(REHASH_STEPS_PER_WRITE);

        let hash = hash_key(&self.hash_builder, &key);
        if let Some(idx) = self.find_index_hashed(&key, hash) {
            return &mut self.nodes[idx as usize].as_mut().unwrap().value;
        }

        let target = if self.rehash_table.is_some() {
            TargetTable::New
        } else {
            TargetTable::Old
        };
        let idx = self.insert_new(target, hash, key, default());
        self.len += 1;
        self.maybe_start_rehash();
        &mut self.nodes[idx as usize].as_mut().unwrap().value
    }

    #[inline(always)]
    pub(super) fn insert_new(&mut self, target: TargetTable, hash: u64, key: K, value: V) -> u32 {
        let _trace = profiler::scope("crates::rehash::src::insert::insert_new");
        let idx = self.alloc_node(Node {
            hash,
            key,
            value,
            next: NIL,
        });

        match target {
            TargetTable::Old => {
                let bucket = bucket_index_from_hash(hash, self.table.mask);
                let head = self.table.heads[bucket];
                self.nodes[idx as usize].as_mut().unwrap().next = head;
                self.table.heads[bucket] = idx;
            }
            TargetTable::New => {
                let table = self.rehash_table.as_mut().expect("rehash table missing");
                let bucket = bucket_index_from_hash(hash, table.mask);
                let head = table.heads[bucket];
                self.nodes[idx as usize].as_mut().unwrap().next = head;
                table.heads[bucket] = idx;
            }
        }

        idx
    }

    #[inline(always)]
    pub(super) fn alloc_node(&mut self, node: Node<K, V>) -> u32 {
        let _trace = profiler::scope("crates::rehash::src::insert::alloc_node");
        if let Some(idx) = self.free.pop() {
            self.nodes[idx as usize] = Some(node);
            idx
        } else {
            let idx = self.nodes.len() as u32;
            self.nodes.push(Some(node));
            idx
        }
    }
}
