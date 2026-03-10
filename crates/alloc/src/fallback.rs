use std::alloc::Layout;
use std::cmp;
use std::mem;
use std::ptr;

use crate::header::{AllocationHeader, FALLBACK_CLASS, header_size, read_header};
use crate::system;

#[inline(always)]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    let align = cmp::max(layout.align(), mem::align_of::<AllocationHeader>());
    let total = allocation_size(layout);
    let backing_layout = unsafe { Layout::from_size_align_unchecked(total, align) };
    let base_ptr = unsafe { system::alloc(backing_layout) };
    let user_ptr = align_ptr(unsafe { base_ptr.add(header_size()) }, layout.align());
    let offset = user_ptr as usize - base_ptr as usize;

    unsafe {
        ptr::write(
            user_ptr.sub(header_size()).cast::<AllocationHeader>(),
            AllocationHeader {
                class_index: FALLBACK_CLASS,
                reserved: 0,
                offset: offset as u32,
            },
        );
    }

    user_ptr
}

#[inline(always)]
pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    let ptr = unsafe { alloc(layout) };
    unsafe { ptr::write_bytes(ptr, 0, layout.size()) };
    ptr
}

#[inline(always)]
pub unsafe fn dealloc(user_ptr: *mut u8, layout: Layout) {
    let header = unsafe { read_header(user_ptr) };
    let base_ptr = unsafe { user_ptr.sub(header.offset as usize) };
    let align = cmp::max(layout.align(), mem::align_of::<AllocationHeader>());
    let total = allocation_size(layout);
    let backing_layout = unsafe { Layout::from_size_align_unchecked(total, align) };
    unsafe { system::dealloc(base_ptr, backing_layout) };
}

#[inline(always)]
fn allocation_size(layout: Layout) -> usize {
    match layout
        .size()
        .checked_add(layout.align())
        .and_then(|value| value.checked_add(header_size()))
    {
        Some(total) => total,
        None => std::alloc::handle_alloc_error(layout),
    }
}

#[inline(always)]
fn align_ptr(ptr: *mut u8, align: usize) -> *mut u8 {
    let aligned = (ptr as usize + (align - 1)) & !(align - 1);
    aligned as *mut u8
}
