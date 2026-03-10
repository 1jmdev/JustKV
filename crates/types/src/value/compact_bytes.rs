use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::ops::Deref;
use std::ptr;
use std::slice;

const INLINE_BYTES_CAPACITY: usize = 15;
const INLINE_VALUE_CAPACITY: usize = 15;
const HEAP_TAG: u8 = u8::MAX;

pub struct CompactBytes<const INLINE_CAPACITY: usize> {
    tag: u8,
    data: [u8; INLINE_CAPACITY],
}

impl<const INLINE_CAPACITY: usize> CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    pub fn from_slice(value: &[u8]) -> Self {
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            unsafe {
                ptr::copy_nonoverlapping(value.as_ptr(), data.as_mut_ptr(), value.len());
            }
            Self {
                tag: value.len() as u8,
                data,
            }
        } else {
            Self::from_boxed_slice(value.to_vec().into_boxed_slice())
        }
    }

    #[inline(always)]
    pub fn from_vec(value: Vec<u8>) -> Self {
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0u8; INLINE_CAPACITY];
            unsafe {
                ptr::copy_nonoverlapping(value.as_ptr(), data.as_mut_ptr(), value.len());
            }
            Self {
                tag: value.len() as u8,
                data,
            }
        } else {
            Self::from_boxed_slice(value.into_boxed_slice())
        }
    }

    #[inline(always)]
    fn from_boxed_slice(value: Box<[u8]>) -> Self {
        let len = u32::try_from(value.len()).expect("CompactBytes supports at most u32::MAX bytes");
        let ptr = Box::into_raw(value) as *mut u8;

        let mut data = [0u8; INLINE_CAPACITY];
        unsafe {
            (data.as_mut_ptr() as *mut usize).write_unaligned(ptr as usize);
            (data.as_mut_ptr().add(size_of::<usize>()) as *mut u32).write_unaligned(len);
        }

        Self {
            tag: HEAP_TAG,
            data,
        }
    }

    #[inline(always)]
    fn is_heap(&self) -> bool {
        self.tag == HEAP_TAG
    }

    #[inline(always)]
    fn heap_len(&self) -> usize {
        unsafe {
            (self.data.as_ptr().add(size_of::<usize>()) as *const u32).read_unaligned() as usize
        }
    }

    #[inline(always)]
    fn heap_ptr(&self) -> *mut u8 {
        unsafe { (self.data.as_ptr() as *const usize).read_unaligned() as *mut u8 }
    }

    #[inline(always)]
    pub fn slice(&self) -> &[u8] {
        if self.is_heap() {
            unsafe { slice::from_raw_parts(self.heap_ptr(), self.heap_len()) }
        } else {
            unsafe { self.data.get_unchecked(..self.tag as usize) }
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.slice()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        if self.is_heap() {
            self.heap_len()
        } else {
            self.tag as usize
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    #[inline(always)]
    pub fn into_vec(self) -> Vec<u8> {
        if self.is_heap() {
            let ptr = self.heap_ptr();
            let len = self.heap_len();
            std::mem::forget(self);
            unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)).into_vec() }
        } else {
            self.data[..self.tag as usize].to_vec()
        }
    }

    #[inline(always)]
    pub fn make_ascii_uppercase(&mut self) {
        if self.is_heap() {
            unsafe {
                slice::from_raw_parts_mut(self.heap_ptr(), self.heap_len()).make_ascii_uppercase();
            }
        } else {
            self.data[..self.tag as usize].make_ascii_uppercase();
        }
    }
}

impl<const INLINE_CAPACITY: usize> Clone for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice())
    }
}

impl<const INLINE_CAPACITY: usize> Drop for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn drop(&mut self) {
        if self.is_heap() {
            unsafe {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    self.heap_ptr(),
                    self.heap_len(),
                )));
            }
        }
    }
}

impl<const INLINE_CAPACITY: usize> fmt::Debug for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CompactBytes")
            .field(&self.as_slice())
            .finish()
    }
}

impl<const INLINE_CAPACITY: usize> PartialEq for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Eq for CompactBytes<INLINE_CAPACITY> {}

impl<const INLINE_CAPACITY: usize> Ord for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl<const INLINE_CAPACITY: usize> PartialOrd for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl<const INLINE_CAPACITY: usize> Hash for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<const INLINE_CAPACITY: usize> Borrow<[u8]> for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> AsRef<[u8]> for CompactBytes<INLINE_CAPACITY> {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Deref for CompactBytes<INLINE_CAPACITY> {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

pub type CompactKey = CompactBytes<INLINE_BYTES_CAPACITY>;
pub type CompactValue = CompactBytes<INLINE_VALUE_CAPACITY>;
pub type CompactArg = CompactBytes<INLINE_BYTES_CAPACITY>;
