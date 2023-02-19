use alloc::vec::Vec;

use super::{address::virt_to_phys, PhysAddr, PAGE_SIZE};
use crate::config::PHYS_MEMORY_END;
use crate::sync::SpinNoIrqLock;
use crate::utils::allocator::FreeListAllocator;

static FRAME_ALLOCATOR: SpinNoIrqLock<FreeListAllocator> =
    SpinNoIrqLock::new(FreeListAllocator::empty());

#[derive(Debug)]
pub struct PhysFrame {
    start_paddr: PhysAddr,
}

impl PhysFrame {
    pub fn alloc() -> Option<Self> {
        FRAME_ALLOCATOR.lock().alloc().map(|value| Self {
            start_paddr: PhysAddr::new(value * PAGE_SIZE),
        })
    }
    pub fn frame_alloc_more(num: usize) -> Option<Vec<PhysFrame>> {
        FRAME_ALLOCATOR.lock().alloc_more(num).map(|x| {
            x.iter()
                .map(|&value| Self {
                    start_paddr: PhysAddr::new(value * PAGE_SIZE),
                })
                .collect()
        })
    }
    pub fn alloc_zero() -> Option<Self> {
        let mut f = Self::alloc()?;
        f.zero();
        Some(f)
    }

    pub fn start_paddr(&self) -> PhysAddr {
        self.start_paddr
    }

    pub fn zero(&mut self) {
        unsafe { core::ptr::write_bytes(self.start_paddr.into_kvaddr().as_mut_ptr(), 0, PAGE_SIZE) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.start_paddr.into_kvaddr().as_ptr(), PAGE_SIZE) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.start_paddr.into_kvaddr().as_mut_ptr(), PAGE_SIZE)
        }
    }
}

impl Drop for PhysFrame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR
            .lock()
            .dealloc(self.start_paddr.as_usize() / PAGE_SIZE);
    }
}

pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    let start_paddr = PhysAddr::new(virt_to_phys(ekernel as usize)).align_up();
    let end_paddr = PhysAddr::new(PHYS_MEMORY_END).align_down();
    println!(
        "Initializing frame allocator at: [{:#x?}, {:#x?})",
        start_paddr, end_paddr
    );
    FRAME_ALLOCATOR
        .lock()
        .init(start_paddr.as_usize() / PAGE_SIZE..end_paddr.as_usize() / PAGE_SIZE);
}

#[allow(dead_code)]
pub fn frame_allocator_test() {
    let mut v: Vec<PhysFrame> = Vec::new();
    for _ in 0..5 {
        let frame = PhysFrame::alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for _ in 0..5 {
        let frame = PhysFrame::alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
