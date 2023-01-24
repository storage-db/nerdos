use alloc::vec::Vec;
use core::ops::Range;

pub struct FreeListAllocator {
    range: Range<usize>,
    next_available: usize,
    free_list: Vec<usize>,
}

impl FreeListAllocator {
    pub const fn empty() -> Self {
        Self {
            range: 0..0,
            next_available: 0,
            free_list: Vec::new(),
        }
    }

    pub fn init(&mut self, range: Range<usize>) {
        self.next_available = range.start;
        self.range = range;
    }

    pub fn available_space(&self) -> usize {
        self.free_list.len() + self.range.end - self.next_available
    }

    pub fn alloc(&mut self) -> Option<usize> {
        if let Some(value) = self.free_list.pop() {
            Some(value)
        } else if self.next_available >= self.range.end {
            None
        } else {
            self.next_available += 1;
            Some(self.next_available - 1)
        }
    }

    pub fn dealloc(&mut self, value: usize) {
        // validity check
        assert!(value >= self.range.start);
        assert!(value < self.next_available);
        assert!(!self.free_list.contains(&value));
        // recycle
        self.free_list.push(value);
    }
}
