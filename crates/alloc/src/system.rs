use std::alloc::{GlobalAlloc, Layout, System, handle_alloc_error};

#[inline(always)]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    let ptr = unsafe { System.alloc(layout) };
    if ptr.is_null() {
        handle_alloc_error(layout);
    }
    ptr
}

#[inline(always)]
pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    unsafe { System.dealloc(ptr, layout) };
}
