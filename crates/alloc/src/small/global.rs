#![allow(unsafe_op_in_unsafe_fn)]

use std::alloc::{GlobalAlloc, Layout};
use std::cmp;
use std::ptr;

use crate::fallback;
use crate::header::{FALLBACK_CLASS, SMALL_OFFSET, read_header, slot_ptr_from_user};
use crate::small::class::{SizeClass, class_for};
use crate::small::local;

pub struct BetterKvAllocator;

unsafe impl GlobalAlloc for BetterKvAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return dangling(layout.align());
        }

        match class_for(layout) {
            Some(class) => alloc_small(class),
            None => fallback::alloc(layout),
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return dangling(layout.align());
        }

        match class_for(layout) {
            Some(class) => {
                let user_ptr = alloc_small(class);
                ptr::write_bytes(user_ptr, 0, layout.size());
                user_ptr
            }
            None => fallback::alloc_zeroed(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() == 0 {
            return;
        }

        let header = read_header(ptr);
        if header.class_index == FALLBACK_CLASS {
            fallback::dealloc(ptr, layout);
            return;
        }

        let class = SizeClass::from_index(header.class_index as usize);
        local::dealloc(class, slot_ptr_from_user(ptr));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.size() == 0 {
            let new_layout = match Layout::from_size_align(new_size, layout.align()) {
                Ok(new_layout) => new_layout,
                Err(_) => return ptr::null_mut(),
            };
            return self.alloc(new_layout);
        }

        if new_size == 0 {
            self.dealloc(ptr, layout);
            return dangling(layout.align());
        }

        let new_layout = match Layout::from_size_align(new_size, layout.align()) {
            Ok(new_layout) => new_layout,
            Err(_) => return ptr::null_mut(),
        };
        let header = read_header(ptr);
        if header.class_index != FALLBACK_CLASS {
            let class = SizeClass::from_index(header.class_index as usize);
            if layout.align() <= SMALL_OFFSET && new_size <= class.usable_size {
                return ptr;
            }
        }

        let new_ptr = self.alloc(new_layout);
        if new_ptr.is_null() {
            return ptr::null_mut();
        }

        ptr::copy_nonoverlapping(ptr, new_ptr, cmp::min(layout.size(), new_size));
        self.dealloc(ptr, layout);
        new_ptr
    }
}

#[inline(always)]
unsafe fn alloc_small(class: SizeClass) -> *mut u8 {
    let slot_ptr = local::alloc(class);
    slot_ptr.add(SMALL_OFFSET)
}

#[inline(always)]
fn dangling(align: usize) -> *mut u8 {
    align as *mut u8
}

#[cfg(test)]
mod tests {
    use std::alloc::{GlobalAlloc, Layout};

    use super::BetterKvAllocator;

    #[test]
    fn reuses_small_blocks_after_free() {
        let allocator = BetterKvAllocator;
        let layout = Layout::from_size_align(32, 8).unwrap_or_else(|_| panic!("invalid layout"));

        let first = unsafe { allocator.alloc(layout) };
        unsafe { allocator.dealloc(first, layout) };
        let second = unsafe { allocator.alloc(layout) };

        assert_eq!(first, second);

        unsafe { allocator.dealloc(second, layout) };
    }

    #[test]
    fn keeps_small_realloc_in_place() {
        let allocator = BetterKvAllocator;
        let layout = Layout::from_size_align(20, 8).unwrap_or_else(|_| panic!("invalid layout"));
        let ptr = unsafe { allocator.alloc(layout) };
        unsafe {
            ptr.write_bytes(0xAB, layout.size());
        }

        let resized = unsafe { allocator.realloc(ptr, layout, 24) };
        assert_eq!(ptr, resized);

        for index in 0..20 {
            let byte = unsafe { *resized.add(index) };
            assert_eq!(byte, 0xAB);
        }

        let resized_layout =
            Layout::from_size_align(24, 8).unwrap_or_else(|_| panic!("invalid layout"));
        unsafe { allocator.dealloc(resized, resized_layout) };
    }

    #[test]
    fn handles_large_allocations() {
        let allocator = BetterKvAllocator;
        let layout = Layout::from_size_align(4096, 64).unwrap_or_else(|_| panic!("invalid layout"));
        let ptr = unsafe { allocator.alloc_zeroed(layout) };

        for index in 0..layout.size() {
            let byte = unsafe { *ptr.add(index) };
            assert_eq!(byte, 0);
        }

        unsafe { allocator.dealloc(ptr, layout) };
    }
}
