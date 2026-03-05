use super::constants::NIL;
use super::index::hash_key;
use super::types::RehashingMap;

#[inline(always)]
unsafe fn prefetch_read<T>(ptr: *const T) {
    let _trace = profiler::scope("rehash::lookup::prefetch_read");
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{_MM_HINT_T0, _mm_prefetch};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};

        unsafe {
            _mm_prefetch(ptr.cast::<i8>(), _MM_HINT_T0);
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        let _ = ptr;
    }
}

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

    /// Extreme Batching Lookup (from Section 3.1: Batching & Out-of-order execution)
    /// Allows the CPU to fetch multiple keys dynamically by calculating arrays of heads concurrently.
    pub fn get_batch<const P: usize, Q>(&self, keys: &[&Q; P]) -> [Option<&V>; P]
    where
        Q: AsRef<[u8]> + ?Sized,
    {
        let _trace = profiler::scope("rehash::lookup::get_batch");
        let mut hashes = [0u32; P];
        let key_bytes = keys.map(|key| key.as_ref());
        let mut heads = [NIL; P];

        unsafe {
            let heads_ptr = self.table.heads.as_ptr();
            let metas_ptr = self.metas.as_ptr();

            for i in 0..P {
                let key = key_bytes[i];
                hashes[i] = hash_key(self.seed, key);
                let bucket = (hashes[i] as usize) & self.table.mask;
                heads[i] = *heads_ptr.add(bucket);
                if heads[i] != NIL {
                    prefetch_read(metas_ptr.add(heads[i] as usize));
                }
            }
        }

        let mut results = [None; P];
        unsafe {
            let metas_ptr = self.metas.as_ptr();
            let keys_ptr = self.keys.as_ptr();
            let values_ptr = self.values.as_ptr();
            let mut remaining = heads.iter().filter(|&&idx| idx != NIL).count();

            while remaining != 0 {
                for i in 0..P {
                    let idx = heads[i];
                    if idx == NIL {
                        continue;
                    }

                    let meta = &*metas_ptr.add(idx as usize);
                    let next = meta.next;

                    if next != NIL {
                        prefetch_read(metas_ptr.add(next as usize));
                    }

                    if meta.hash == hashes[i] {
                        let key_ptr = keys_ptr.add(idx as usize);
                        prefetch_read(key_ptr);
                        let stored_key = &*key_ptr;

                        if stored_key.as_ref() == key_bytes[i] {
                            prefetch_read(values_ptr.add(idx as usize));
                            results[i] = Some(&*values_ptr.add(idx as usize));
                            heads[i] = NIL;
                            remaining -= 1;
                            continue;
                        }
                    }

                    heads[i] = next;
                    if next == NIL {
                        remaining -= 1;
                    }
                }
            }
        }
        results
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
        let bucket = (hash as usize) & self.table.mask;

        unsafe {
            let mut idx = *self.table.heads.as_ptr().add(bucket);
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
