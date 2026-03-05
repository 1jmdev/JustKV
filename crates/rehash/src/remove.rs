use super::constants::NIL;
use super::index::hash_key;
use super::types::RehashingMap;

impl<K, V> RehashingMap<K, V>
where
    K: Eq + AsRef<[u8]>,
{
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::remove::remove");
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);
        let bucket = (hash as usize) & self.table.mask;

        unsafe {
            let heads_ptr = self.table.heads.as_mut_ptr();
            let metas_ptr = self.metas.as_mut_ptr();
            let keys_ptr = self.keys.as_ptr();

            let mut cur = *heads_ptr.add(bucket);
            let mut prev = NIL;

            while cur != NIL {
                let meta = &*metas_ptr.add(cur as usize);
                if meta.hash == hash && (*keys_ptr.add(cur as usize)).as_ref() == key_bytes {
                    let next = meta.next;

                    if prev == NIL {
                        *heads_ptr.add(bucket) = next;
                    } else {
                        (*metas_ptr.add(prev as usize)).next = next;
                    }

                    // Remove from all 3 SoA vectors
                    self.metas.swap_remove(cur as usize);
                    self.keys.swap_remove(cur as usize);
                    let removed_val = self.values.swap_remove(cur as usize);

                    if (cur as usize) < self.metas.len() {
                        let old_last_idx = self.metas.len() as u32;
                        self.patch_swapped(old_last_idx, cur);
                    }

                    return Some(removed_val);
                }
                prev = cur;
                cur = meta.next;
            }
        }
        None
    }
}
