use std::mem;
use std::ptr;

pub const SMALL_OFFSET: usize = 16;
pub const FALLBACK_CLASS: u16 = u16::MAX;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AllocationHeader {
    pub class_index: u16,
    pub reserved: u16,
    pub offset: u32,
}

#[inline(always)]
pub unsafe fn write_small_header(user_ptr: *mut u8, class_index: usize) {
    unsafe {
        ptr::write(
            header_ptr(user_ptr),
            AllocationHeader {
                class_index: class_index as u16,
                reserved: 0,
                offset: SMALL_OFFSET as u32,
            },
        );
    }
}

#[inline(always)]
pub unsafe fn read_header(user_ptr: *mut u8) -> AllocationHeader {
    unsafe { ptr::read(header_ptr(user_ptr)) }
}

#[inline(always)]
pub unsafe fn slot_ptr_from_user(user_ptr: *mut u8) -> *mut u8 {
    unsafe { user_ptr.sub(SMALL_OFFSET) }
}

#[inline(always)]
pub const fn header_size() -> usize {
    mem::size_of::<AllocationHeader>()
}

#[inline(always)]
unsafe fn header_ptr(user_ptr: *mut u8) -> *mut AllocationHeader {
    unsafe { user_ptr.sub(header_size()).cast::<AllocationHeader>() }
}
