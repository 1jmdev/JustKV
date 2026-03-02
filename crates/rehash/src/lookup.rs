use std::borrow::Borrow;
use std::hash::Hash;

use super::constants::REHASH_STEPS_PER_WRITE;
use super::index::{bucket_index_from_hash, find_in_chain, hash_key};
use super::types::RehashingMap;

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _trace = profiler::scope("crates::rehash::src::lookup::contains_key");
        self.find_index(key).is_some()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _trace = profiler::scope("crates::rehash::src::lookup::get");
        let idx = self.find_index(key)?;
        Some(&self.nodes[idx as usize].as_ref().unwrap().value)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _trace = profiler::scope("crates::rehash::src::lookup::get_mut");
        self.rehash_step(REHASH_STEPS_PER_WRITE);
        let idx = self.find_index(key)?;
        Some(&mut self.nodes[idx as usize].as_mut().unwrap().value)
    }

    #[inline(always)]
    pub fn find_index<Q>(&self, key: &Q) -> Option<u32>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let _trace = profiler::scope("crates::rehash::src::lookup::find_index");
        let hash = hash_key(&self.hash_builder, key);
        self.find_index_hashed(key, hash)
    }

    #[inline(always)]
    pub fn find_index_hashed<Q>(&self, key: &Q, hash: u64) -> Option<u32>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let _trace = profiler::scope("crates::rehash::src::lookup::find_index_hashed");
        if let Some(table) = self.rehash_table.as_ref() {
            let bucket = bucket_index_from_hash(hash, table.mask);
            if let Some(idx) = find_in_chain(&self.nodes, table.heads[bucket], hash, key) {
                return Some(idx);
            }
        }

        let bucket = bucket_index_from_hash(hash, self.table.mask);
        find_in_chain(&self.nodes, self.table.heads[bucket], hash, key)
    }
}
