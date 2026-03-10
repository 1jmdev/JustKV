use betterkv_alloc::BucketArray;

use super::constants::{INITIAL_BUCKETS, NIL};

pub(super) struct Table {
    pub(super) heads: BucketArray,
    pub(super) mask: usize,
}

impl Table {
    pub(super) fn with_buckets(count: usize) -> Self {
        let _trace = profiler::scope("rehash::table::with_buckets");
        let count = count.max(INITIAL_BUCKETS).next_power_of_two();
        Self {
            heads: BucketArray::filled(count, NIL),
            mask: count - 1,
        }
    }

    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.mask + 1
    }

    #[inline(always)]
    pub(super) fn bucket(&self, hash: u32) -> usize {
        (hash as usize) & self.mask
    }
}
