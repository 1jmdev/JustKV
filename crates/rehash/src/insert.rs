use super::constants::{BULK_RESERVE_CAP, NIL};
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
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);
        let mut bucket = (hash as usize) & self.table.mask;

        unsafe {
            let mut idx = *self.table.heads.as_ptr().add(bucket);
            let metas_ptr = self.metas.as_ptr();
            let keys_ptr = self.keys.as_ptr();

            while idx != NIL {
                let meta = &*metas_ptr.add(idx as usize);
                if meta.hash == hash && (*keys_ptr.add(idx as usize)).as_ref() == key_bytes {
                    let val_ptr = self.values.as_mut_ptr().add(idx as usize);
                    return Some(std::ptr::replace(val_ptr, value));
                }
                idx = meta.next;
            }
        }

        self.maybe_grow();
        bucket = (hash as usize) & self.table.mask;

        unsafe {
            let head = *self.table.heads.as_ptr().add(bucket);
            let idx = self.metas.len() as u32;

            self.metas.push(NodeMeta { hash, next: head });
            self.keys.push(key);
            self.values.push(value);

            *self.table.heads.as_mut_ptr().add(bucket) = idx;
        }
        None
    }

    pub fn get_or_insert_with<F>(&mut self, key: K, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        let _trace = profiler::scope("rehash::insert::get_or_insert_with");
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);
        let mut bucket = (hash as usize) & self.table.mask;

        unsafe {
            let mut idx = *self.table.heads.as_ptr().add(bucket);
            let metas_ptr = self.metas.as_ptr();
            let keys_ptr = self.keys.as_ptr();

            while idx != NIL {
                let meta = &*metas_ptr.add(idx as usize);
                if meta.hash == hash && (*keys_ptr.add(idx as usize)).as_ref() == key_bytes {
                    return &mut *self.values.as_mut_ptr().add(idx as usize);
                }
                idx = meta.next;
            }
        }

        self.maybe_grow();
        bucket = (hash as usize) & self.table.mask;

        unsafe {
            let head = *self.table.heads.as_ptr().add(bucket);
            let idx = self.metas.len() as u32;

            self.metas.push(NodeMeta { hash, next: head });
            self.keys.push(key);
            self.values.push(default());

            *self.table.heads.as_mut_ptr().add(bucket) = idx;
            &mut *self.values.as_mut_ptr().add(idx as usize)
        }
    }
}
