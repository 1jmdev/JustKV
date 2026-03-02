pub(super) struct Node<K, V> {
    pub(super) hash: u64,
    pub(super) key: K,
    pub(super) value: V,
    pub(super) next: u32,
}
