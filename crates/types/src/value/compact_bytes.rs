use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::slice;

const INLINE_BYTES_CAPACITY: usize = 15;
const INLINE_VALUE_CAPACITY: usize = 15;
const HEAP_TAG: u8 = u8::MAX;

pub struct CompactBytes<const INLINE_CAPACITY: usize> {
    tag: u8,
    data: [u8; INLINE_CAPACITY],
}

impl<const INLINE_CAPACITY: usize> CompactBytes<INLINE_CAPACITY> {
    pub fn from_slice(value: &[u8]) -> Self {
        let _trace = profiler::scope("crates::types::src::value::from_slice");
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..value.len()].copy_from_slice(value);
            Self {
                tag: value.len() as u8,
                data,
            }
        } else {
            Self::from_boxed_slice(value.to_vec().into_boxed_slice())
        }
    }

    pub fn from_vec(value: Vec<u8>) -> Self {
        let _trace = profiler::scope("crates::types::src::value::from_vec");
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..value.len()].copy_from_slice(&value);
            Self {
                tag: value.len() as u8,
                data,
            }
        } else {
            Self::from_boxed_slice(value.into_boxed_slice())
        }
    }

    fn from_boxed_slice(value: Box<[u8]>) -> Self {
        let len = u32::try_from(value.len()).expect("CompactBytes supports at most u32::MAX bytes");
        let ptr = Box::into_raw(value) as *mut u8 as usize;
        let ptr_bytes = ptr.to_ne_bytes();
        let len_bytes = len.to_ne_bytes();
        let mut data = [0; INLINE_CAPACITY];
        data[..std::mem::size_of::<usize>()].copy_from_slice(&ptr_bytes);
        data[std::mem::size_of::<usize>()..std::mem::size_of::<usize>() + 4]
            .copy_from_slice(&len_bytes);
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
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(
            &self.data[std::mem::size_of::<usize>()..std::mem::size_of::<usize>() + 4],
        );
        u32::from_ne_bytes(len_bytes) as usize
    }

    #[inline(always)]
    fn heap_ptr(&self) -> *mut u8 {
        let mut ptr_bytes = [0u8; std::mem::size_of::<usize>()];
        ptr_bytes.copy_from_slice(&self.data[..std::mem::size_of::<usize>()]);
        usize::from_ne_bytes(ptr_bytes) as *mut u8
    }

    #[inline(always)]
    pub fn slice(&self) -> &[u8] {
        if self.is_heap() {
            unsafe { slice::from_raw_parts(self.heap_ptr(), self.heap_len()) }
        } else {
            &self.data[..self.tag as usize]
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        let _trace = profiler::scope("crates::types::src::value::as_slice");
        self.slice()
    }

    pub fn len(&self) -> usize {
        let _trace = profiler::scope("crates::types::src::value::len");
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        let _trace = profiler::scope("crates::types::src::value::is_empty");
        self.as_slice().is_empty()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let _trace = profiler::scope("crates::types::src::value::to_vec");
        self.as_slice().to_vec()
    }

    pub fn into_vec(self) -> Vec<u8> {
        let _trace = profiler::scope("crates::types::src::value::into_vec");
        if self.is_heap() {
            let ptr = self.heap_ptr();
            let len = self.heap_len();
            std::mem::forget(self);
            unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)).into_vec() }
        } else {
            self.data[..self.tag as usize].to_vec()
        }
    }

    pub fn make_ascii_uppercase(&mut self) {
        let _trace = profiler::scope("crates::types::src::value::make_ascii_uppercase");
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
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice())
    }
}

impl<const INLINE_CAPACITY: usize> Drop for CompactBytes<INLINE_CAPACITY> {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CompactBytes")
            .field(&self.as_slice())
            .finish()
    }
}

impl<const INLINE_CAPACITY: usize> PartialEq for CompactBytes<INLINE_CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        let _trace = profiler::scope("crates::types::src::value::eq");
        self.as_slice() == other.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Eq for CompactBytes<INLINE_CAPACITY> {}

impl<const INLINE_CAPACITY: usize> Ord for CompactBytes<INLINE_CAPACITY> {
    fn cmp(&self, other: &Self) -> Ordering {
        let _trace = profiler::scope("crates::types::src::value::cmp");
        self.as_slice().cmp(other.as_slice())
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl<const INLINE_CAPACITY: usize> PartialOrd for CompactBytes<INLINE_CAPACITY> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _trace = profiler::scope("crates::types::src::value::partial_cmp");
        Some(Ord::cmp(self, other))
    }
}

impl<const INLINE_CAPACITY: usize> Hash for CompactBytes<INLINE_CAPACITY> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let _trace = profiler::scope("crates::types::src::value::hash");
        self.as_slice().hash(state);
    }
}

impl<const INLINE_CAPACITY: usize> Borrow<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn borrow(&self) -> &[u8] {
        let _trace = profiler::scope("crates::types::src::value::borrow");
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> AsRef<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn as_ref(&self) -> &[u8] {
        let _trace = profiler::scope("crates::types::src::value::as_ref");
        self.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Deref for CompactBytes<INLINE_CAPACITY> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let _trace = profiler::scope("crates::types::src::value::deref");
        self.as_slice()
    }
}

pub type CompactKey = CompactBytes<INLINE_BYTES_CAPACITY>;
pub type CompactValue = CompactBytes<INLINE_VALUE_CAPACITY>;
pub type CompactArg = CompactBytes<INLINE_BYTES_CAPACITY>;
