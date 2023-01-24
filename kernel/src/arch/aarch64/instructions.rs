use core::arch::asm;

use cortex_a::registers::{DAIF, TPIDR_EL1, TTBR0_EL1, TTBR1_EL1};
use tock_registers::interfaces::{Readable, Writeable};

#[inline]
pub fn enable_irqs() {
    unsafe { asm!("msr daifclr, #2") };
}

#[inline]
pub fn disable_irqs() {
    unsafe { asm!("msr daifset, #2") };
}

#[inline]
pub fn irqs_disabled() -> bool {
    DAIF.matches_all(DAIF::I::Masked)
}

#[inline]
pub fn thread_pointer() -> usize {
    TPIDR_EL1.get() as _
}

#[inline]
pub unsafe fn set_thread_pointer(tp: usize) {
    TPIDR_EL1.set(tp as _)
}

#[inline]
pub unsafe fn set_kernel_page_table_root(root_paddr: usize) {
    // kernel space page table use TTBR1 (0xffff_0000_0000_0000..0xffff_ffff_ffff_ffff)
    TTBR1_EL1.set(root_paddr as _);
    // clear user page table root when initialize kernel page table.
    TTBR0_EL1.set(0);
    flush_tlb_all();
}

pub unsafe fn set_user_page_table_root(root_paddr: usize) {
    // user space page table use TTBR0 (0x0..0xffff_ffff_ffff)
    let old_root = TTBR0_EL1.get();
    trace!("set page table root: {:#x} => {:#x}", old_root, root_paddr);
    if old_root != root_paddr as u64 {
        TTBR0_EL1.set(root_paddr as u64);
        flush_tlb_all();
    }
}

#[inline]
pub fn flush_tlb_all() {
    unsafe { asm!("tlbi vmalle1; dsb sy; isb") };
}

#[inline]
pub fn flush_icache_all() {
    unsafe { asm!("ic iallu; dsb sy; isb") };
}

#[inline]
#[allow(dead_code)]
pub fn wait_for_ints() {
    cortex_a::asm::wfi();
}
