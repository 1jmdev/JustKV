use std::alloc::Layout;

use crate::header::SMALL_OFFSET;

pub const MAX_SMALL_ALIGN: usize = 16;
pub const MAX_SMALL_SIZE: usize = 2048;
pub const CLASS_COUNT: usize = 23;
pub const USABLE_SIZES: [usize; CLASS_COUNT] = [
    8, 16, 24, 32, 40, 48, 64, 80, 96, 128, 160, 192, 224, 256, 320, 384, 512, 640, 768, 1024,
    1280, 1536, 2048,
];
pub const BATCH_TARGET_BYTES: usize = 32 * 1024;
pub const LOCAL_REFILL_COUNT: u16 = 64;
pub const LOCAL_FLUSH_COUNT: u16 = 128;
const SIZE_CLASS_LOOKUP: [u8; MAX_SMALL_SIZE + 1] = build_size_class_lookup();

#[derive(Clone, Copy)]
pub struct SizeClass {
    pub index: usize,
    pub usable_size: usize,
    pub slot_size: usize,
}

impl SizeClass {
    #[inline(always)]
    pub const fn from_index(index: usize) -> Self {
        let usable_size = USABLE_SIZES[index];
        Self {
            index,
            usable_size,
            slot_size: SMALL_OFFSET + usable_size,
        }
    }

    #[inline(always)]
    pub const fn batch_count(self) -> usize {
        let count = BATCH_TARGET_BYTES / self.slot_size;
        if count < LOCAL_REFILL_COUNT as usize {
            LOCAL_REFILL_COUNT as usize
        } else {
            count
        }
    }
}

#[inline(always)]
pub fn class_for(layout: Layout) -> Option<SizeClass> {
    if layout.size() == 0 || layout.align() > MAX_SMALL_ALIGN {
        return None;
    }

    let minimum_usable = layout.size().max(std::mem::size_of::<usize>());
    if minimum_usable > MAX_SMALL_SIZE {
        return None;
    }

    Some(SizeClass::from_index(
        SIZE_CLASS_LOOKUP[minimum_usable] as usize,
    ))
}

const fn build_size_class_lookup() -> [u8; MAX_SMALL_SIZE + 1] {
    let mut lookup = [0u8; MAX_SMALL_SIZE + 1];
    let mut size = 0;
    let mut class_index = 0;

    while size <= MAX_SMALL_SIZE {
        while class_index + 1 < CLASS_COUNT && size > USABLE_SIZES[class_index] {
            class_index += 1;
        }
        lookup[size] = class_index as u8;
        size += 1;
    }

    lookup
}
