use std::borrow::Borrow;
use std::hash::Hash;

use ahash::RandomState;
use hashbrown::HashMap;

pub(in crate::engine::store) struct RehashingMap<K, V> {
    map: HashMap<K, V, RandomState>,
}

impl<K, V> RehashingMap<K, V>
where
    K: Eq + Hash,
{
    pub(in crate::engine::store) fn new() -> Self {
        Self {
            map: HashMap::with_hasher(RandomState::new()),
        }
    }

    pub(in crate::engine::store) fn len(&self) -> usize {
        self.map.len()
    }

    pub(in crate::engine::store) fn clear(&mut self) {
        self.map.clear();
    }

    pub(in crate::engine::store) fn iter(&self) -> hashbrown::hash_map::Iter<'_, K, V> {
        self.map.iter()
    }

    pub(in crate::engine::store) fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    pub(in crate::engine::store) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }

    pub(in crate::engine::store) fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_mut(key)
    }

    pub(in crate::engine::store) fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove(key)
    }

    pub(in crate::engine::store) fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.map.insert(key, value)
    }

    pub(in crate::engine::store) fn get_or_insert_with<F>(&mut self, key: K, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.map.entry(key).or_insert_with(default)
    }
}
