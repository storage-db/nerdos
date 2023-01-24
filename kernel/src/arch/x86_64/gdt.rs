#![allow(dead_code)]

use alloc::boxed::Box;
use core::arch::asm;
use core::fmt::{Debug, Formatter, Result};

use x86_64::addr::VirtAddr;
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags};
use x86_64::structures::{tss::TaskStateSegment, DescriptorTablePointer};
use x86_64::{registers::segmentation::SegmentSelector, PrivilegeLevel};

pub const KCODE32_SELECTOR: SegmentSelector = SegmentSelector::new(1, PrivilegeLevel::Ring0);
pub const KCODE64_SELECTOR: SegmentSelector = SegmentSelector::new(2, PrivilegeLevel::Ring0);
pub const KDATA_SELECTOR: SegmentSelector = SegmentSelector::new(3, PrivilegeLevel::Ring0);
pub const UCODE32_SELECTOR: SegmentSelector = SegmentSelector::new(4, PrivilegeLevel::Ring3);
pub const UDATA_SELECTOR: SegmentSelector = SegmentSelector::new(5, PrivilegeLevel::Ring3);
pub const UCODE64_SELECTOR: SegmentSelector = SegmentSelector::new(6, PrivilegeLevel::Ring3);
pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(7, PrivilegeLevel::Ring0);

pub(super) struct GdtStruct {
    table: &'static mut [u64],
}

impl GdtStruct {
    pub fn alloc() -> Self {
        Self {
            table: Box::leak(Box::new([0u64; 16])),
        }
    }

    pub fn init(&mut self, tss: &'static TaskStateSegment) {
        // first 3 entries are the same as in multiboot.S
        self.table[1] = DescriptorFlags::KERNEL_CODE32.bits(); // 0x00cf9b000000ffff
        self.table[2] = DescriptorFlags::KERNEL_CODE64.bits(); // 0x00af9b000000ffff
        self.table[3] = DescriptorFlags::KERNEL_DATA.bits(); // 0x00cf93000000ffff
        self.table[4] = DescriptorFlags::USER_CODE32.bits(); // 0x00cffb000000ffff
        self.table[5] = DescriptorFlags::USER_DATA.bits(); // 0x00cff3000000ffff
        self.table[6] = DescriptorFlags::USER_CODE64.bits(); // 0x00affb000000ffff
        if let Descriptor::SystemSegment(low, high) = Descriptor::tss_segment(tss) {
            self.table[7] = low;
            self.table[8] = high;
        }
    }

    fn pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            base: VirtAddr::new(self.table.as_ptr() as u64),
            limit: (core::mem::size_of_val(self.table) - 1) as u16,
        }
    }

    pub fn load(&'static self) {
        unsafe {
            asm!("lgdt [{}]", in(reg) &self.pointer(), options(readonly, nostack, preserves_flags))
        }
    }

    pub fn load_tss(&'static self, selector: SegmentSelector) {
        unsafe { load_tss(selector) };
    }
}

impl Debug for GdtStruct {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("GdtStruct")
            .field("pointer", &self.pointer())
            .field("table", &self.table)
            .finish()
    }
}
