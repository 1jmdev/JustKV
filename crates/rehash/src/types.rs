use std::mem;

use super::constants::{INITIAL_BUCKETS, MAX_LOAD_FACTOR, NIL, REHASH_BUCKETS_PER_STEP};
use super::iter::Iter;
use super::node::NodeMeta;
use super::table::Table;

pub struct RehashingMap<K, V> {
    pub(super) seed: u64,
    pub(super) table: Table,
    pub(super) old_table: Option<Table>,
    pub(super) rehash_cursor: usize,
    // SoA (Structure of Arrays) Layout:
    pub(super) metas: Vec<NodeMeta>,
    pub(super) keys: Vec<K>,
    pub(super) values: Vec<V>,
}

impl<K, V> RehashingMap<K, V>
where
    K: Eq + AsRef<[u8]>,
{
    pub fn new() -> Self {
        let _trace = profiler::scope("rehash::types::new");
        Self {
            seed: random_seed(),
            table: Table::with_buckets(INITIAL_BUCKETS),
            old_table: None,
            rehash_cursor: 0,
            metas: Vec::with_capacity(INITIAL_BUCKETS),
            keys: Vec::with_capacity(INITIAL_BUCKETS),
            values: Vec::with_capacity(INITIAL_BUCKETS),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        let _trace = profiler::scope("rehash::types::len");
        self.metas.len()
    }

    pub fn clear(&mut self) {
        let _trace = profiler::scope("rehash::types::clear");
        self.table = Table::with_buckets(INITIAL_BUCKETS);
        self.old_table = None;
        self.rehash_cursor = 0;
        self.metas.clear();
        self.keys.clear();
        self.values.clear();
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        let _trace = profiler::scope("rehash::types::iter");
        Iter::new(&self.keys, &self.values)
    }

    pub fn slices(&self) -> (&[K], &[V]) {
        let _trace = profiler::scope("rehash::types::slices");
        (&self.keys, &self.values)
    }

    #[inline(always)]
    pub(super) fn maybe_grow(&mut self) {
        let _trace = profiler::scope("rehash::types::maybe_grow");
        if self.old_table.is_some() {
            return;
        }
        if self.metas.len() < self.table.len() * MAX_LOAD_FACTOR {
            return;
        }
        self.start_rehash(self.table.len() * 2);
    }

    pub(super) fn reserve_for_batch(&mut self, additional: usize) {
        let _trace = profiler::scope("rehash::types::reserve_for_batch");
        if additional == 0 {
            return;
        }
        let required = self.metas.len().saturating_add(additional);
        let bucket_need = required.div_ceil(MAX_LOAD_FACTOR).next_power_of_two();
        if self.old_table.is_none() && bucket_need > self.table.len() {
            self.start_rehash(bucket_need);
        }
        self.metas.reserve(additional);
        self.keys.reserve(additional);
        self.values.reserve(additional);
    }

    pub(super) fn rehash_step(&mut self, steps: usize) {
        let _trace = profiler::scope("rehash::types::rehash_step");
        let Some(old_table) = self.old_table.as_mut() else {
            return;
        };

        let steps = steps.max(1);
        let old_len = old_table.len();
        let metas_ptr = self.metas.as_mut_ptr();
        let heads_ptr = self.table.heads.as_mut_ptr();

        for _ in 0..steps {
            if self.rehash_cursor >= old_len {
                self.old_table = None;
                self.rehash_cursor = 0;
                return;
            }

            let bucket = self.rehash_cursor;
            self.rehash_cursor += 1;

            unsafe {
                let mut idx = old_table.heads[bucket];
                old_table.heads[bucket] = NIL;

                while idx != NIL {
                    let meta = &mut *metas_ptr.add(idx as usize);
                    let next = meta.next;
                    let new_bucket = self.table.bucket(meta.hash);
                    meta.next = *heads_ptr.add(new_bucket);
                    *heads_ptr.add(new_bucket) = idx;
                    idx = next;
                }
            }
        }

        if self.rehash_cursor >= old_len {
            self.old_table = None;
            self.rehash_cursor = 0;
        }
    }

    pub(super) fn rehash_write_step(&mut self) {
        self.rehash_step(REHASH_BUCKETS_PER_STEP);
    }

    fn start_rehash(&mut self, new_bucket_count: usize) {
        let _trace = profiler::scope("rehash::types::start_rehash");
        let new_bucket_count = new_bucket_count.next_power_of_two();
        if new_bucket_count <= self.table.len() {
            return;
        }

        let new_table = Table::with_buckets(new_bucket_count);
        let old_table = mem::replace(&mut self.table, new_table);
        self.old_table = Some(old_table);
        self.rehash_cursor = 0;
    }

    pub(super) fn patch_swapped(&mut self, old_idx: u32, new_idx: u32) {
        let _trace = profiler::scope("rehash::types::patch_swapped");
        if old_idx == new_idx {
            return;
        }

        let hash = self.metas[new_idx as usize].hash;
        if Self::patch_swapped_in_table_impl(
            &mut self.table,
            &mut self.metas,
            hash,
            old_idx,
            new_idx,
        ) {
            return;
        }
        if let Some(old_table) = self.old_table.as_mut() {
            let _ = Self::patch_swapped_in_table_impl(
                old_table,
                &mut self.metas,
                hash,
                old_idx,
                new_idx,
            );
        }
    }

    fn patch_swapped_in_table_impl(
        table: &mut Table,
        metas: &mut [NodeMeta],
        hash: u32,
        old_idx: u32,
        new_idx: u32,
    ) -> bool {
        unsafe {
            let bucket = table.bucket(hash);
            let heads_ptr = table.heads.as_mut_ptr();
            let metas_ptr = metas.as_mut_ptr();

            let mut cur = *heads_ptr.add(bucket);
            let mut prev = NIL;

            while cur != NIL {
                if cur == old_idx {
                    if prev == NIL {
                        *heads_ptr.add(bucket) = new_idx;
                    } else {
                        (*metas_ptr.add(prev as usize)).next = new_idx;
                    }
                    return true;
                }
                prev = cur;
                cur = (*metas_ptr.add(cur as usize)).next;
            }
        }

        false
    }
}

fn random_seed() -> u64 {
    let _trace = profiler::scope("rehash::types::random_seed");
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}
