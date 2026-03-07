use super::constants::NIL;
use super::index::hash_key;
use super::table::Table;
use super::types::RehashingMap;

impl<K, V> RehashingMap<K, V>
where
    K: Eq + AsRef<[u8]>,
{
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::contains_key");
        self.find_index(key).is_some()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::get");
        let idx = self.find_index(key)?;
        Some(unsafe { &*self.values.as_ptr().add(idx as usize) })
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::get_mut");
        let idx = self.find_index(key)?;
        Some(unsafe { &mut *self.values.as_mut_ptr().add(idx as usize) })
    }

    pub fn get_batch<const P: usize, Q>(&self, keys: &[&Q; P]) -> [Option<&V>; P]
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::get_batch");
        keys.map(|key| self.get(key))
    }

    #[inline(always)]
    pub fn find_index<Q>(&self, key: &Q) -> Option<u32>
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::find_index");
        let hash = hash_key(self.seed, key.as_ref());
        self.find_index_hashed(key, hash)
    }

    #[inline(always)]
    pub fn find_index_hashed<Q>(&self, key: &Q, hash: u32) -> Option<u32>
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::find_index_hashed");
        let key_bytes = key.as_ref();
        if let Some(idx) = self.find_index_in_table(&self.table, key_bytes, hash) {
            return Some(idx);
        }
        self.old_table
            .as_ref()
            .and_then(|table| self.find_index_in_table(table, key_bytes, hash))
    }

    fn find_index_in_table(&self, table: &Table, key_bytes: &[u8], hash: u32) -> Option<u32> {
        let bucket = table.bucket(hash);

        unsafe {
            let mut idx = *table.heads.as_ptr().add(bucket);
            let metas_ptr = self.metas.as_ptr();
            let keys_ptr = self.keys.as_ptr();

            while idx != NIL {
                let meta = &*metas_ptr.add(idx as usize);
                let next = meta.next;

                if meta.hash == hash {
                    let k = &*keys_ptr.add(idx as usize);
                    if k.as_ref() == key_bytes {
                        return Some(idx);
                    }
                }
                idx = next;
            }
        }

        None
    }
}
