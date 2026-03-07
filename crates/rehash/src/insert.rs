use super::constants::BULK_RESERVE_CAP;
use super::index::hash_key;
use super::node::NodeMeta;
use super::types::RehashingMap;

impl<K, V> RehashingMap<K, V>
where
    K: Eq + AsRef<[u8]>,
{
    pub fn insert_batch<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let _trace = profiler::scope("rehash::insert::insert_batch");
        let iter = entries.into_iter();
        let (lower_bound, _) = iter.size_hint();
        self.reserve_for_batch(lower_bound.min(BULK_RESERVE_CAP));

        for (key, value) in iter {
            self.insert(key, value);
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let _trace = profiler::scope("rehash::insert::insert");
        self.rehash_write_step();
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);

        if let Some(idx) = self.find_index_hashed(key_bytes, hash) {
            unsafe {
                let val_ptr = self.values.as_mut_ptr().add(idx as usize);
                return Some(std::ptr::replace(val_ptr, value));
            }
        }

        self.maybe_grow();
        self.insert_new(key, value, hash);
        None
    }

    pub fn get_or_insert_with<F>(&mut self, key: K, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        let _trace = profiler::scope("rehash::insert::get_or_insert_with");
        self.rehash_write_step();
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);

        if let Some(idx) = self.find_index_hashed(key_bytes, hash) {
            unsafe {
                return &mut *self.values.as_mut_ptr().add(idx as usize);
            }
        }

        self.maybe_grow();
        let idx = self.insert_new(key, default(), hash);
        unsafe { &mut *self.values.as_mut_ptr().add(idx as usize) }
    }

    fn insert_new(&mut self, key: K, value: V, hash: u32) -> u32 {
        let bucket = self.table.bucket(hash);

        unsafe {
            let head = *self.table.heads.as_ptr().add(bucket);
            let idx = self.metas.len() as u32;

            self.metas.push(NodeMeta { hash, next: head });
            self.keys.push(key);
            self.values.push(value);

            *self.table.heads.as_mut_ptr().add(bucket) = idx;
            idx
        }
    }
}
