use std::borrow::Borrow;
use std::hash::Hash;

use super::constants::{NIL, REHASH_STEPS_PER_WRITE};
use super::index::{bucket_index_from_hash, hash_key};
use super::types::{RehashingMap, TargetTable};

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.rehash_step(REHASH_STEPS_PER_WRITE);
        let hash = hash_key(&self.hash_builder, key);

        if let Some(value) = self.remove_from_table_hashed(TargetTable::New, key, hash) {
            self.len -= 1;
            return Some(value);
        }

        if let Some(value) = self.remove_from_table_hashed(TargetTable::Old, key, hash) {
            self.len -= 1;
            return Some(value);
        }

        None
    }

    #[inline(always)]
    pub(crate) fn remove_from_table_hashed<Q>(
        &mut self,
        target: TargetTable,
        key: &Q,
        hash: u64,
    ) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let bucket = match target {
            TargetTable::Old => bucket_index_from_hash(hash, self.table.mask),
            TargetTable::New => {
                let table = self.rehash_table.as_ref()?;
                bucket_index_from_hash(hash, table.mask)
            }
        };

        let mut head = match target {
            TargetTable::Old => self.table.heads[bucket],
            TargetTable::New => self.rehash_table.as_ref()?.heads[bucket],
        };

        let mut prev = NIL;
        while head != NIL {
            let node = self.nodes[head as usize].as_ref().unwrap();
            let next = node.next;
            let hit = node.hash == hash && node.key.borrow() == key;

            if hit {
                if prev == NIL {
                    match target {
                        TargetTable::Old => self.table.heads[bucket] = next,
                        TargetTable::New => {
                            if let Some(table) = self.rehash_table.as_mut() {
                                table.heads[bucket] = next;
                            }
                        }
                    }
                } else {
                    self.nodes[prev as usize].as_mut().unwrap().next = next;
                }

                let node = self.nodes[head as usize].take().unwrap();
                self.free.push(head);
                return Some(node.value);
            }

            prev = head;
            head = next;
        }

        None
    }
}
