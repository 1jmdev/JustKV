use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

const INLINE_BYTES_CAPACITY: usize = 22;
const INLINE_VALUE_CAPACITY: usize = 7;

#[derive(Clone, Debug)]
pub enum CompactBytes<const INLINE_CAPACITY: usize> {
    Inline {
        len: u8,
        data: [u8; INLINE_CAPACITY],
    },
    Heap(Box<[u8]>),
}

impl<const INLINE_CAPACITY: usize> CompactBytes<INLINE_CAPACITY> {
    pub fn from_vec(value: Vec<u8>) -> Self {
        if value.len() <= INLINE_CAPACITY {
            let mut data = [0; INLINE_CAPACITY];
            data[..value.len()].copy_from_slice(&value);
            Self::Inline {
                len: value.len() as u8,
                data,
            }
        } else {
            Self::Heap(value.into_boxed_slice())
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Inline { len, data } => &data[..*len as usize],
            Self::Heap(value) => value,
        }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Inline { len, data } => data[..len as usize].to_vec(),
            Self::Heap(value) => value.into_vec(),
        }
    }
}

impl<const INLINE_CAPACITY: usize> PartialEq for CompactBytes<INLINE_CAPACITY> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const INLINE_CAPACITY: usize> Eq for CompactBytes<INLINE_CAPACITY> {}

impl<const INLINE_CAPACITY: usize> Hash for CompactBytes<INLINE_CAPACITY> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<const INLINE_CAPACITY: usize> Borrow<[u8]> for CompactBytes<INLINE_CAPACITY> {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

pub type CompactKey = CompactBytes<INLINE_BYTES_CAPACITY>;
pub type CompactValue = CompactBytes<INLINE_VALUE_CAPACITY>;

#[derive(Clone, Debug)]
pub struct Entry {
    pub value: CompactValue,
}

impl Entry {
    pub fn new(value: Vec<u8>) -> Self {
        Self {
            value: CompactValue::from_vec(value),
        }
    }
}
