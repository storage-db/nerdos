#![allow(dead_code)]

use core::fmt;

use super::PAGE_SIZE;
use crate::config::{PA_MAX_BITS, PHYS_VIRT_OFFSET, VA_MAX_BITS};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(usize);

pub const fn phys_to_virt(paddr: usize) -> usize {
    paddr + PHYS_VIRT_OFFSET
}

pub const fn virt_to_phys(vaddr: usize) -> usize {
    vaddr - PHYS_VIRT_OFFSET
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}

impl PhysAddr {
    pub const MAX: usize = (1 << PA_MAX_BITS) - 1;

    pub const fn new(pa: usize) -> Self {
        Self(pa & Self::MAX)
    }
    pub const fn as_usize(&self) -> usize {
        self.0
    }
    pub const fn into_kvaddr(self) -> VirtAddr {
        VirtAddr::new(phys_to_virt(self.0))
    }
    pub const fn align_down(&self) -> Self {
        Self(align_down(self.0, PAGE_SIZE))
    }
    pub const fn align_up(&self) -> Self {
        Self(align_up(self.0, PAGE_SIZE))
    }
    pub const fn page_offset(&self) -> usize {
        page_offset(self.0, PAGE_SIZE)
    }
    pub const fn is_aligned(&self) -> bool {
        is_aligned(self.0, PAGE_SIZE)
    }
}

impl VirtAddr {
    pub const fn new(va: usize) -> Self {
        if cfg!(any(
            target_arch = "x86_64",
            target_arch = "riscv32",
            target_arch = "riscv64"
        )) {
            // Top bits must be sign extended
            let top_mask = usize::MAX << (VA_MAX_BITS - 1);
            if (va & top_mask) != 0 && (va & top_mask) != top_mask {
                panic!("invalid VA!")
            }
        } else if cfg!(target_arch = "aarch64") {
            // Top 16 bits must be all 0s or all 1s.
            let top_bits = va >> VA_MAX_BITS;
            if top_bits != 0 && top_bits != 0xffff {
                panic!("invalid VA!")
            }
        }
        Self(va)
    }
    pub const fn new_extended(va: usize) -> Self {
        if cfg!(any(
            target_arch = "x86_64",
            target_arch = "riscv32",
            target_arch = "riscv64"
        )) {
            let top_mask = usize::MAX << (VA_MAX_BITS - 1);
            if (va & top_mask) != 0 {
                return Self(va | top_mask);
            }
        }
        Self(va)
    }

    pub const fn as_usize(&self) -> usize {
        self.0
    }
    pub const fn as_ptr(&self) -> *const u8 {
        self.as_usize() as _
    }
    pub const fn as_mut_ptr(&self) -> *mut u8 {
        self.as_usize() as _
    }
    pub const fn align_down(&self) -> Self {
        Self(align_down(self.0, PAGE_SIZE))
    }
    pub const fn align_up(&self) -> Self {
        Self(align_up(self.0, PAGE_SIZE))
    }
    pub const fn page_offset(&self) -> usize {
        page_offset(self.0, PAGE_SIZE)
    }
    pub const fn is_aligned(&self) -> bool {
        is_aligned(self.0, PAGE_SIZE)
    }
}

pub const fn align_down(addr: usize, page_size: usize) -> usize {
    addr & !(page_size - 1)
}

pub const fn align_up(addr: usize, page_size: usize) -> usize {
    (addr + page_size - 1) & !(page_size - 1)
}

pub const fn page_offset(addr: usize, page_size: usize) -> usize {
    addr & (page_size - 1)
}

pub const fn is_aligned(addr: usize, page_size: usize) -> bool {
    page_offset(addr, page_size) == 0
}
