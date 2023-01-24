use core::arch::asm;

use riscv::register::satp;
use riscv::register::sstatus;

#[inline]
pub fn enable_irqs() {
    unsafe { sstatus::set_sie() }
}

#[inline]
pub fn disable_irqs() {
    unsafe { sstatus::clear_sie() }
}

#[inline]
pub fn irqs_disabled() -> bool {
    !sstatus::read().sie()
}

#[inline]
pub fn thread_pointer() -> usize {
    let mut ret;
    unsafe { asm!("mv {}, tp", out(reg) ret) };
    ret
}

#[inline]
pub unsafe fn set_thread_pointer(tp: usize) {
    asm!("mv tp, {}", in(reg) tp)
}

pub unsafe fn set_kernel_page_table_root(root_paddr: usize) {
    // riscv does not has separate page tables for kernel and user.
    set_user_page_table_root(root_paddr)
}

pub unsafe fn set_user_page_table_root(root_paddr: usize) {
    let old_root = satp::read().ppn() << 12;
    trace!("set page table root: {:#x} => {:#x}", old_root, root_paddr);
    if old_root != root_paddr {
        satp::set(satp::Mode::Sv39, 0, root_paddr >> 12);
        flush_tlb_all();
    }
}

#[inline]
pub fn flush_tlb_all() {
    unsafe { riscv::asm::sfence_vma_all() }
}

#[inline]
pub fn flush_icache_all() {
    unsafe { asm!("fence.i") }
}

#[inline]
#[allow(dead_code)]
pub fn wait_for_ints() {
    unsafe { riscv::asm::wfi() }
}
