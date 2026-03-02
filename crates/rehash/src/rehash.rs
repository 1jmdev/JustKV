use std::hash::Hash;

use super::constants::{MAX_LOAD_FACTOR, NIL};
use super::index::bucket_index_from_hash;
use super::table::Table;
use super::types::RehashingMap;

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub(crate) fn maybe_start_rehash(&mut self) {
        let _trace = profiler::scope("crates::rehash::src::rehash::maybe_start_rehash");
        if self.rehash_table.is_some() || self.len < self.table.len() * MAX_LOAD_FACTOR {
            return;
        }
        self.rehash_table = Some(Table::with_buckets(self.table.len() * 2));
        self.rehash_index = 0;
    }

    pub(crate) fn rehash_step(&mut self, mut steps: usize) {
        let _trace = profiler::scope("crates::rehash::src::rehash::rehash_step");
        while steps > 0 {
            let Some(new_table) = self.rehash_table.as_mut() else {
                return;
            };

            if self.rehash_index >= self.table.len() {
                let finished = self.rehash_table.take().unwrap();
                self.table = finished;
                self.rehash_index = 0;
                return;
            }

            let mut node_idx = self.table.heads[self.rehash_index];
            self.table.heads[self.rehash_index] = NIL;

            while node_idx != NIL {
                let node = self.nodes[node_idx as usize].as_mut().unwrap();
                let next = node.next;
                let bucket = bucket_index_from_hash(node.hash, new_table.mask);
                let head = new_table.heads[bucket];
                node.next = head;
                new_table.heads[bucket] = node_idx;
                node_idx = next;
            }

            self.rehash_index += 1;
            steps -= 1;
        }
    }
}
