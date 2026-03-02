use std::borrow::Borrow;
use std::hash::Hash;

use ahash::RandomState;

use super::constants::NIL;
use super::node::Node;

#[inline(always)]
pub(super) fn hash_key<Q: Hash + ?Sized>(hash_builder: &RandomState, key: &Q) -> u64 {
    let _trace = profiler::scope("crates::rehash::src::index::hash_key");
    hash_builder.hash_one(key)
}

#[inline(always)]
pub(super) fn bucket_index_from_hash(hash: u64, mask: usize) -> usize {
    let _trace = profiler::scope("crates::rehash::src::index::bucket_index_from_hash");
    (hash as usize) & mask
}

#[inline(always)]
pub(super) fn find_in_chain<K, V, Q>(
    nodes: &[Option<Node<K, V>>],
    mut head: u32,
    hash: u64,
    key: &Q,
) -> Option<u32>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    let _trace = profiler::scope("crates::rehash::src::index::find_in_chain");
    while head != NIL {
        let node = nodes[head as usize].as_ref().unwrap();
        if node.hash == hash && node.key.borrow() == key {
            return Some(head);
        }
        head = node.next;
    }
    None
}
