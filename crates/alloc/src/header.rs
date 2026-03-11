use std::mem;
use std::ptr;

pub const FALLBACK_CLASS: u16 = u16::MAX;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AllocationHeader {
    pub class_index: u16,
    pub reserved: u16,
    pub offset: u32,
}

#[inline(always)]
pub unsafe fn read_header(user_ptr: *mut u8) -> AllocationHeader {
    unsafe { ptr::read(header_ptr(user_ptr)) }
}

#[inline(always)]
pub const fn header_size() -> usize {
    mem::size_of::<AllocationHeader>()
}

#[inline(always)]
unsafe fn header_ptr(user_ptr: *mut u8) -> *mut AllocationHeader {
    unsafe { user_ptr.sub(header_size()).cast::<AllocationHeader>() }
}
