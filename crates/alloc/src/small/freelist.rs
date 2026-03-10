#[repr(C)]
struct FreeNode {
    next: usize,
}

#[derive(Clone, Copy)]
pub struct FreeList {
    head: usize,
    len: u16,
}

impl FreeList {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { head: 0, len: 0 }
    }

    #[inline(always)]
    pub const fn from_raw(head: usize, len: u16) -> Self {
        Self { head, len }
    }

    #[inline(always)]
    pub const fn head(&self) -> usize {
        self.head
    }

    #[inline(always)]
    pub const fn len(&self) -> u16 {
        self.len
    }

    #[inline(always)]
    pub fn pop(&mut self) -> *mut u8 {
        if self.head == 0 {
            return std::ptr::null_mut();
        }

        let node = self.head as *mut FreeNode;
        unsafe {
            self.head = (*node).next;
        }
        self.len -= 1;
        node.cast::<u8>()
    }

    #[inline(always)]
    pub unsafe fn push(&mut self, slot_ptr: *mut u8) {
        let node = slot_ptr.cast::<FreeNode>();
        unsafe {
            (*node).next = self.head;
        }
        self.head = node as usize;
        self.len = self.len.saturating_add(1);
    }
}
