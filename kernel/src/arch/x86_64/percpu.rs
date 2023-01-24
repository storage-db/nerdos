use memoffset::offset_of;
use x86_64::structures::tss::TaskStateSegment;

use super::gdt::{GdtStruct, TSS_SELECTOR};
use crate::mm::VirtAddr;
use crate::percpu::PERCPU_ARCH_OFFSET;

#[allow(dead_code)]
pub(super) const PERCPU_USER_RSP_OFFSET: usize =
    PERCPU_ARCH_OFFSET + offset_of!(ArchPerCpu, saved_user_rsp);

#[allow(dead_code)]
pub(super) const PERCPU_KERNEL_RSP_OFFSET: usize = PERCPU_ARCH_OFFSET
    + offset_of!(ArchPerCpu, tss)
    + offset_of!(TaskStateSegment, privilege_stack_table);

pub struct ArchPerCpu {
    saved_user_rsp: u64,
    tss: TaskStateSegment,
    gdt: GdtStruct,
}

impl ArchPerCpu {
    pub fn new() -> Self {
        Self {
            saved_user_rsp: 0,
            tss: TaskStateSegment::new(),
            gdt: GdtStruct::alloc(),
        }
    }

    pub fn init(&'static mut self, cpu_id: usize) {
        println!("Loading GDT for CPU {}...", cpu_id);
        self.gdt.init(&self.tss);
        self.gdt.load();
        self.gdt.load_tss(TSS_SELECTOR);
    }

    pub fn kernel_stack_top(&self) -> VirtAddr {
        VirtAddr::new(self.tss.privilege_stack_table[0].as_u64() as usize)
    }

    pub fn set_kernel_stack_top(&mut self, kstack_top: VirtAddr) {
        trace!("set percpu kernel stack: {:#x?}", kstack_top);
        self.tss.privilege_stack_table[0] = x86_64::VirtAddr::new(kstack_top.as_usize() as u64);
    }
}
