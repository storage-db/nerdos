use core::arch::asm;

use cortex_a::registers::SPSR_EL1;

use crate::arch::instructions;
use crate::mm::{PhysAddr, VirtAddr};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapFrame {
    /// General-purpose registers (R0..R30).
    pub r: [u64; 31],
    /// User Stack Pointer (SP_EL0).
    pub usp: u64,
    /// Exception Link Register (ELR_EL1).
    pub elr: u64,
    /// Saved Process Status Register (SPSR_EL1).
    pub spsr: u64,
}

impl TrapFrame {
    pub fn new_user(entry: VirtAddr, ustack_top: VirtAddr, arg0: usize) -> Self {
        let mut regs = [0; 31];
        regs[0] = arg0 as _;
        Self {
            usp: ustack_top.as_usize() as _,
            elr: entry.as_usize() as _,
            spsr: (SPSR_EL1::M::EL0t
                + SPSR_EL1::D::Masked
                + SPSR_EL1::A::Masked
                + SPSR_EL1::I::Unmasked
                + SPSR_EL1::F::Masked)
                .value, // enable IRQ, mask others
            r: regs,
        }
    }

    pub const fn new_clone(&self, ustack_top: VirtAddr) -> Self {
        let mut tf = *self;
        tf.usp = ustack_top.as_usize() as _;
        tf.r[0] = 0; // for child thread, clone returns 0
        tf
    }

    pub const fn new_fork(&self) -> Self {
        let mut tf = *self;
        tf.r[0] = 0; // for child process, fork returns 0
        tf
    }

    pub unsafe fn exec(&self, kstack_top: VirtAddr) -> ! {
        info!(
            "user task start: entry={:#x}, ustack={:#x}, kstack={:#x}",
            self.elr,
            self.usp,
            kstack_top.as_usize(),
        );
        instructions::disable_irqs();
        asm!("
            mov     sp, x1
            ldp     x30, x9, [x0, 30 * 8]
            ldp     x10, x11, [x0, 32 * 8]
            msr     sp_el0, x9
            msr     elr_el1, x10
            msr     spsr_el1, x11

            ldp     x28, x29, [x0, 28 * 8]
            ldp     x26, x27, [x0, 26 * 8]
            ldp     x24, x25, [x0, 24 * 8]
            ldp     x22, x23, [x0, 22 * 8]
            ldp     x20, x21, [x0, 20 * 8]
            ldp     x18, x19, [x0, 18 * 8]
            ldp     x16, x17, [x0, 16 * 8]
            ldp     x14, x15, [x0, 14 * 8]
            ldp     x12, x13, [x0, 12 * 8]
            ldp     x10, x11, [x0, 10 * 8]
            ldp     x8, x9, [x0, 8 * 8]
            ldp     x6, x7, [x0, 6 * 8]
            ldp     x4, x5, [x0, 4 * 8]
            ldp     x2, x3, [x0, 2 * 8]
            ldp     x0, x1, [x0]

            eret",
            in("x0") self,
            in("x1") kstack_top.as_usize(),
            options(noreturn),
        )
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TaskContext {
    pub sp: u64,
    pub tpidr_el0: u64,
    pub r19: u64,
    pub r20: u64,
    pub r21: u64,
    pub r22: u64,
    pub r23: u64,
    pub r24: u64,
    pub r25: u64,
    pub r26: u64,
    pub r27: u64,
    pub r28: u64,
    pub r29: u64,
    pub lr: u64, // r30
    pub ttbr0_el1: u64,
}

impl TaskContext {
    pub const fn default() -> Self {
        unsafe { core::mem::MaybeUninit::zeroed().assume_init() }
    }

    pub fn init(
        &mut self,
        entry: usize,
        kstack_top: VirtAddr,
        page_table_root: PhysAddr,
        is_kernel: bool,
    ) {
        self.sp = kstack_top.as_usize() as u64;
        self.lr = entry as u64;
        self.ttbr0_el1 = if is_kernel {
            0
        } else {
            page_table_root.as_usize() as u64
        };
    }

    pub fn switch_to(&mut self, next_ctx: &Self) {
        unsafe {
            instructions::set_user_page_table_root(next_ctx.ttbr0_el1 as usize);
            context_switch(self, next_ctx)
        }
    }
}

#[naked]
unsafe extern "C" fn context_switch(_current_task: &mut TaskContext, _next_task: &TaskContext) {
    asm!(
        "
        // save old context (callee-saved registers)
        stp     x29, x30, [x0, 12 * 8]
        stp     x27, x28, [x0, 10 * 8]
        stp     x25, x26, [x0, 8 * 8]
        stp     x23, x24, [x0, 6 * 8]
        stp     x21, x22, [x0, 4 * 8]
        stp     x19, x20, [x0, 2 * 8]
        mov     x19, sp
        mrs     x20, tpidr_el0
        stp     x19, x20, [x0]

        // restore new context
        ldp     x19, x20, [x1]
        mov     sp, x19
        msr     tpidr_el0, x20
        ldp     x19, x20, [x1, 2 * 8]
        ldp     x21, x22, [x1, 4 * 8]
        ldp     x23, x24, [x1, 6 * 8]
        ldp     x25, x26, [x1, 8 * 8]
        ldp     x27, x28, [x1, 10 * 8]
        ldp     x29, x30, [x1, 12 * 8]

        ret",
        options(noreturn),
    )
}
