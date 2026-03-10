use std::cell::Cell;

use crate::small::central;
use crate::small::class::{CLASS_COUNT, LOCAL_FLUSH_COUNT, LOCAL_REFILL_COUNT, SizeClass};
use crate::small::freelist::FreeList;

thread_local! {
    static LOCAL_CACHE: LocalCache = const { LocalCache::new() };
}

pub fn alloc(class: SizeClass) -> *mut u8 {
    LOCAL_CACHE.with(|cache| {
        let mut local_list = cache.list(class.index);
        let slot_ptr = local_list.pop();
        if !slot_ptr.is_null() {
            cache.store_list(class.index, local_list);
            return slot_ptr;
        }

        central::refill(class, &mut local_list, LOCAL_REFILL_COUNT);
        let slot_ptr = local_list.pop();
        cache.store_list(class.index, local_list);
        slot_ptr
    })
}

pub fn dealloc(class: SizeClass, slot_ptr: *mut u8) {
    LOCAL_CACHE.with(|cache| {
        let mut local_list = cache.list(class.index);
        unsafe {
            local_list.push(slot_ptr);
        }

        if local_list.len() >= LOCAL_FLUSH_COUNT {
            central::drain(class, &mut local_list, LOCAL_REFILL_COUNT);
        }

        cache.store_list(class.index, local_list);
    });
}

struct LocalCache {
    heads: [Cell<usize>; CLASS_COUNT],
    lens: [Cell<u16>; CLASS_COUNT],
}

impl LocalCache {
    const fn new() -> Self {
        Self {
            heads: [const { Cell::new(0) }; CLASS_COUNT],
            lens: [const { Cell::new(0) }; CLASS_COUNT],
        }
    }

    #[inline(always)]
    fn list(&self, index: usize) -> FreeList {
        FreeList::from_raw(self.heads[index].get(), self.lens[index].get())
    }

    #[inline(always)]
    fn store_list(&self, index: usize, list: FreeList) {
        self.heads[index].set(list.head());
        self.lens[index].set(list.len());
    }
}
