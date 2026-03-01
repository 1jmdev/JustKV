use std::hash::Hash;

use ahash::RandomState;

use super::constants::{INITIAL_BUCKETS, MAX_LOAD_FACTOR};
use super::iter::Iter;
use super::node::Node;
use super::table::Table;

pub(in crate::engine::store) struct RehashingMap<K, V> {
    pub(super) hash_builder: RandomState,
    pub(super) len: usize,
    pub(super) table: Table,
    pub(super) rehash_table: Option<Table>,
    pub(super) rehash_index: usize,
    pub(super) nodes: Vec<Option<Node<K, V>>>,
    pub(super) free: Vec<u32>,
}

#[derive(Clone, Copy)]
pub(super) enum TargetTable {
    Old,
    New,
}

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub(in crate::engine::store) fn new() -> Self {
        let table = Table::with_buckets(INITIAL_BUCKETS);
        let node_cap = table.len() * MAX_LOAD_FACTOR;
        Self {
            hash_builder: RandomState::new(),
            len: 0,
            table,
            rehash_table: None,
            rehash_index: 0,
            nodes: Vec::with_capacity(node_cap),
            free: Vec::new(),
        }
    }

    pub(in crate::engine::store) fn len(&self) -> usize {
        self.len
    }

    pub(in crate::engine::store) fn clear(&mut self) {
        self.len = 0;
        self.table = Table::with_buckets(INITIAL_BUCKETS);
        self.rehash_table = None;
        self.rehash_index = 0;
        self.nodes.clear();
        self.free.clear();
    }

    pub(in crate::engine::store) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            nodes: &self.nodes,
            index: 0,
        }
    }
}
