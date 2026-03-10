use std::alloc::Layout;
use std::sync::Mutex;

use crate::header::{SMALL_OFFSET, write_small_header};
use crate::small::class::{CLASS_COUNT, SizeClass};
use crate::small::freelist::FreeList;
use crate::system;

static CENTRAL_POOLS: [CentralPool; CLASS_COUNT] = [const { CentralPool::new() }; CLASS_COUNT];

pub fn refill(class: SizeClass, local: &mut FreeList, target_count: u16) {
    CENTRAL_POOLS[class.index].refill(class, local, target_count);
}

pub fn drain(class: SizeClass, local: &mut FreeList, retain: u16) {
    CENTRAL_POOLS[class.index].drain(local, retain);
}

struct CentralPool {
    state: Mutex<CentralState>,
}

struct CentralState {
    free_list: FreeList,
}

impl CentralPool {
    const fn new() -> Self {
        Self {
            state: Mutex::new(CentralState {
                free_list: FreeList::new(),
            }),
        }
    }

    fn refill(&self, class: SizeClass, local: &mut FreeList, target_count: u16) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        while local.len() < target_count {
            let slot_ptr = state.free_list.pop();
            if slot_ptr.is_null() {
                allocate_run(class, &mut state.free_list);
                continue;
            }

            unsafe {
                local.push(slot_ptr);
            }
        }
    }

    fn drain(&self, local: &mut FreeList, retain: u16) {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        while local.len() > retain {
            let slot_ptr = local.pop();
            if slot_ptr.is_null() {
                break;
            }

            unsafe {
                state.free_list.push(slot_ptr);
            }
        }
    }
}

fn allocate_run(class: SizeClass, free_list: &mut FreeList) {
    let slot_size = class.slot_size;
    let slot_count = class.batch_count();
    let total_size = match slot_size.checked_mul(slot_count) {
        Some(total_size) => total_size,
        None => std::alloc::handle_alloc_error(layout_for_run(slot_size, 1)),
    };
    let layout = layout_for_run(total_size, 16);
    let base_ptr = unsafe { system::alloc(layout) };

    for index in 0..slot_count {
        let slot_ptr = unsafe { base_ptr.add(index * slot_size) };
        unsafe {
            write_small_header(slot_ptr.add(SMALL_OFFSET), class.index);
            free_list.push(slot_ptr);
        }
    }
}

fn layout_for_run(size: usize, align: usize) -> Layout {
    match Layout::from_size_align(size, align) {
        Ok(layout) => layout,
        Err(_) => panic!("invalid central run layout: size={size} align={align}"),
    }
}
