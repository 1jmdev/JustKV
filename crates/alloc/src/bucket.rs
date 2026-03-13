use std::alloc::Layout;
use std::mem;
use std::ops::{Index, IndexMut};
use std::ptr;

use crate::lock::SpinLock;
use crate::system;

const MAX_BUCKET_POWER: usize = usize::BITS as usize - 1;
static BUCKET_SLOTS: [BucketSlot; MAX_BUCKET_POWER + 1] =
    [const { BucketSlot::new() }; MAX_BUCKET_POWER + 1];

struct BucketSlot {
    head: SpinLock<usize>,
}

struct FreeBucketNode {
    next: usize,
}

impl BucketSlot {
    const fn new() -> Self {
        Self {
            head: SpinLock::new(0),
        }
    }

    fn pop(&self) -> *mut u32 {
        let mut head = self.head.lock();
        let node = *head as *mut FreeBucketNode;
        if node.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            *head = (*node).next;
        }
        node.cast::<u32>()
    }

    fn push(&self, ptr: *mut u32) {
        let node = ptr.cast::<FreeBucketNode>();
        let mut head = self.head.lock();

        unsafe {
            (*node).next = *head;
            *head = node as usize;
        }
    }
}

pub struct BucketArray {
    ptr: *mut u32,
    len: usize,
    pool_index: usize,
    pooled: bool,
}

unsafe impl Send for BucketArray {}
unsafe impl Sync for BucketArray {}

impl BucketArray {
    pub fn filled(len: usize, value: u32) -> Self {
        assert!(len != 0, "bucket arrays must be non-empty");

        let pooled = len.is_power_of_two();
        let pool_index = len.trailing_zeros() as usize;
        let ptr = if pooled && pool_index <= MAX_BUCKET_POWER {
            let pooled = BUCKET_SLOTS[pool_index].pop();
            if pooled.is_null() {
                alloc_bucket_block(len)
            } else {
                pooled
            }
        } else {
            alloc_bucket_block(len)
        };

        let mut buckets = Self {
            ptr,
            len,
            pool_index,
            pooled,
        };
        buckets.fill(value);
        buckets
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const u32 {
        self.ptr.cast_const()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut u32 {
        self.ptr
    }

    pub fn fill(&mut self, value: u32) {
        if is_byte_repeated(value) {
            unsafe {
                ptr::write_bytes(
                    self.ptr.cast::<u8>(),
                    value.to_ne_bytes()[0],
                    self.len * mem::size_of::<u32>(),
                );
            }
            return;
        }

        unsafe {
            self.ptr.write(value);
        }

        let mut initialized = 1usize;
        while initialized < self.len {
            let copy_len = (self.len - initialized).min(initialized);
            unsafe {
                ptr::copy_nonoverlapping(self.ptr, self.ptr.add(initialized), copy_len);
            }
            initialized += copy_len;
        }
    }
}

impl Drop for BucketArray {
    fn drop(&mut self) {
        if self.pooled && self.pool_index <= MAX_BUCKET_POWER {
            BUCKET_SLOTS[self.pool_index].push(self.ptr);
            return;
        }

        unsafe {
            system::dealloc(self.ptr.cast::<u8>(), bucket_layout(self.len));
        }
    }
}

impl Index<usize> for BucketArray {
    type Output = u32;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len, "bucket index out of range");
        unsafe { &*self.ptr.add(index) }
    }
}

impl IndexMut<usize> for BucketArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len, "bucket index out of range");
        unsafe { &mut *self.ptr.add(index) }
    }
}

fn is_byte_repeated(value: u32) -> bool {
    let bytes = value.to_ne_bytes();
    bytes[1] == bytes[0] && bytes[2] == bytes[0] && bytes[3] == bytes[0]
}

fn alloc_bucket_block(len: usize) -> *mut u32 {
    unsafe { system::alloc(bucket_layout(len)).cast::<u32>() }
}

fn bucket_layout(len: usize) -> Layout {
    match Layout::array::<u32>(len) {
        Ok(layout) => layout,
        Err(_) => panic!("bucket layout overflow for length {len}"),
    }
}

#[cfg(test)]
mod tests {
    use super::BucketArray;

    #[test]
    fn fills_with_requested_value() {
        let buckets = BucketArray::filled(64, u32::MAX);
        for index in 0..buckets.len() {
            assert_eq!(buckets[index], u32::MAX);
        }
    }

    #[test]
    fn reuses_bucket_storage_by_capacity() {
        let first_ptr = {
            let buckets = BucketArray::filled(64, u32::MAX);
            buckets.as_ptr()
        };

        let second_ptr = {
            let buckets = BucketArray::filled(64, u32::MAX);
            buckets.as_ptr()
        };

        assert_eq!(first_ptr, second_ptr);
    }
}
