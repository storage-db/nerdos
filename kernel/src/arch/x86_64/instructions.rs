use core::arch::asm;

use x86::controlregs::{cr3, cr3_write};
use x86_64::registers::{model_specific::GsBase, rflags, rflags::RFlags};
use x86_64::VirtAddr;

#[inline]
pub fn enable_irqs() {
    unsafe { asm!("sti") };
}

#[inline]
pub fn disable_irqs() {
    unsafe { asm!("cli") };
}

#[inline]
pub fn irqs_disabled() -> bool {
    !rflags::read().contains(RFlags::INTERRUPT_FLAG)
}

#[inline]
pub fn thread_pointer() -> usize {
    // read PerCpu::self_vaddr
    let ret;
    unsafe { core::arch::asm!("mov {0}, gs:0", out(reg) ret, options(pure, readonly, nostack)) };
    ret
}

#[inline]
pub unsafe fn set_thread_pointer(tp: usize) {
    GsBase::write(VirtAddr::new(tp as u64));
}

pub unsafe fn set_kernel_page_table_root(root_paddr: usize) {
    // x86 does not has separate page tables for kernel and user.
    set_user_page_table_root(root_paddr);
}

pub unsafe fn set_user_page_table_root(root_paddr: usize) {
    let old_root = cr3();
    trace!("set page table root: {:#x} => {:#x}", old_root, root_paddr);
    if old_root != root_paddr as u64 {
        cr3_write(root_paddr as u64);
    }
}

#[inline]
pub fn flush_tlb_all() {
    unsafe { cr3_write(cr3()) }
}

pub fn flush_icache_all() {}

#[inline]
pub fn wait_for_ints() {
    if !irqs_disabled() {
        x86_64::instructions::hlt();
    }
}
