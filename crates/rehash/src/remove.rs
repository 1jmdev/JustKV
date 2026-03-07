use super::constants::NIL;
use super::index::hash_key;
use super::node::NodeMeta;
use super::table::Table;
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
        self.rehash_write_step();
        let key_bytes = key.as_ref();
        let hash = hash_key(self.seed, key_bytes);

        let removed_idx = unsafe {
            unlink_from_table(
                &mut self.table,
                self.metas.as_mut_ptr(),
                self.keys.as_ptr(),
                key_bytes,
                hash,
            )
        }
        .or_else(|| {
            self.old_table.as_mut().and_then(|table| unsafe {
                unlink_from_table(
                    table,
                    self.metas.as_mut_ptr(),
                    self.keys.as_ptr(),
                    key_bytes,
                    hash,
                )
            })
        });

        if let Some(cur) = removed_idx {
            self.metas.swap_remove(cur as usize);
            self.keys.swap_remove(cur as usize);
            let removed_val = self.values.swap_remove(cur as usize);

            if (cur as usize) < self.metas.len() {
                let old_last_idx = self.metas.len() as u32;
                self.patch_swapped(old_last_idx, cur);
            }

            return Some(removed_val);
        }

        None
    }
}

unsafe fn unlink_from_table<K>(
    table: &mut Table,
    metas_ptr: *mut NodeMeta,
    keys_ptr: *const K,
    key_bytes: &[u8],
    hash: u32,
) -> Option<u32>
where
    K: Eq + AsRef<[u8]>,
{
    unsafe {
        let bucket = table.bucket(hash);
        let heads_ptr = table.heads.as_mut_ptr();
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
                return Some(cur);
            }
            prev = cur;
            cur = meta.next;
        }
    }

    None
}
