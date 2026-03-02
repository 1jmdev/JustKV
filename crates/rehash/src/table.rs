use super::constants::NIL;

pub(super) struct Table {
    pub(super) heads: Vec<u32>,
    pub(super) mask: usize,
}

impl Table {
    #[inline(always)]
    pub(super) fn with_buckets(count: usize) -> Self {
        let _trace = profiler::scope("rehash::table::with_buckets");
        let count = count.max(1).next_power_of_two();
        Self {
            heads: vec![NIL; count],
            mask: count - 1,
        }
    }

    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        let _trace = profiler::scope("rehash::table::len");
        self.heads.len()
    }
}
