use hashbrown::HashMap;
use rapidhash::fast::RandomState;

use crate::store::Store;

#[derive(Default)]
pub struct WatchState {
    watched: HashMap<Vec<u8>, Option<Vec<u8>>, RandomState>,
}

impl WatchState {
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.watched.is_empty()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.watched.clear();
    }

    #[inline]
    pub fn watch(&mut self, store: &Store, key: &[u8]) {
        self.watched.insert(key.to_vec(), store.dump(key));
    }

    pub fn is_dirty(&self, store: &Store) -> bool {
        self.watched
            .iter()
            .any(|(key, expected)| store.dump(key) != *expected)
    }
}
