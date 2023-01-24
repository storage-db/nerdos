use core::fmt;

use x86_64::structures::paging::page_table::PageTableFlags as PTF;

use crate::mm::paging::{GenericPTE, PageTableImpl, PageTableLevels4};
use crate::mm::{MemFlags, PhysAddr};

impl From<PTF> for MemFlags {
    fn from(f: PTF) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::READ;
        if f.contains(PTF::WRITABLE) {
            ret |= Self::WRITE;
        }
        if !f.contains(PTF::NO_EXECUTE) {
            ret |= Self::EXECUTE;
        }
        if f.contains(PTF::USER_ACCESSIBLE) {
            ret |= Self::USER;
        }
        if f.contains(PTF::NO_CACHE) {
            ret |= Self::DEVICE;
        }
        ret
    }
}

impl From<MemFlags> for PTF {
    fn from(f: MemFlags) -> Self {
        if f.is_empty() {
            return Self::empty();
        }
        let mut ret = Self::PRESENT;
        if f.contains(MemFlags::WRITE) {
            ret |= Self::WRITABLE;
        }
        if !f.contains(MemFlags::EXECUTE) {
            ret |= Self::NO_EXECUTE;
        }
        if f.contains(MemFlags::USER) {
            ret |= Self::USER_ACCESSIBLE;
        }
        if f.contains(MemFlags::DEVICE) {
            ret |= Self::NO_CACHE | Self::WRITE_THROUGH;
        }
        ret
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const PHYS_ADDR_MASK: usize = 0x000f_ffff_ffff_f000; // 12..52
}

impl GenericPTE for PageTableEntry {
    fn new_page(paddr: PhysAddr, flags: MemFlags, is_block: bool) -> Self {
        let mut flags = PTF::from(flags);
        if is_block {
            flags |= PTF::HUGE_PAGE;
        }
        Self(flags.bits() | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u64)
    }
    fn new_table(paddr: PhysAddr) -> Self {
        let flags = PTF::PRESENT | PTF::WRITABLE | PTF::USER_ACCESSIBLE;
        Self(flags.bits() | (paddr.as_usize() & Self::PHYS_ADDR_MASK) as u64)
    }
    fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.0 as usize & Self::PHYS_ADDR_MASK)
    }
    fn flags(&self) -> MemFlags {
        PTF::from_bits_truncate(self.0).into()
    }
    fn is_unused(&self) -> bool {
        self.0 == 0
    }
    fn is_present(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::PRESENT)
    }
    fn is_block(&self) -> bool {
        PTF::from_bits_truncate(self.0).contains(PTF::HUGE_PAGE)
    }
    fn clear(&mut self) {
        self.0 = 0
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("raw", &self.0)
            .field("paddr", &self.paddr())
            .field("flags", &self.flags())
            .finish()
    }
}

pub type PageTable = PageTableImpl<PageTableLevels4, PageTableEntry>;
