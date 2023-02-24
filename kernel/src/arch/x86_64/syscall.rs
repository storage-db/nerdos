use core::arch::global_asm;

use x86_64::addr::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, KernelGsBase, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;

use super::context::TrapFrame;
use super::gdt::{KCODE64_SELECTOR, KDATA_SELECTOR, UCODE64_SELECTOR, UDATA_SELECTOR};
use super::percpu::{PERCPU_KERNEL_RSP_OFFSET, PERCPU_USER_RSP_OFFSET};
use crate::syscall::syscall;

global_asm!(
    include_str!("syscall.S"),
    saved_user_rsp_offset = const PERCPU_USER_RSP_OFFSET,
    saved_kernel_rsp_offset = const PERCPU_KERNEL_RSP_OFFSET,
);

#[no_mangle]
fn x86_syscall_handler(tf: &mut TrapFrame) {
    tf.rax = syscall(tf, tf.rax as _, tf.rdi as _, tf.rsi as _, tf.rdx as _) as u64;
}

pub fn init_percpu() {
    extern "C" {
        fn syscall_entry();
    }
    unsafe {
        LStar::write(VirtAddr::new(syscall_entry as usize as _));
        Star::write(
            UCODE64_SELECTOR,
            UDATA_SELECTOR,
            KCODE64_SELECTOR,
            KDATA_SELECTOR,
        )
        .unwrap();
        SFMask::write(
            RFlags::TRAP_FLAG
                | RFlags::INTERRUPT_FLAG
                | RFlags::DIRECTION_FLAG
                | RFlags::IOPL_LOW
                | RFlags::IOPL_HIGH
                | RFlags::NESTED_TASK
                | RFlags::ALIGNMENT_CHECK,
        ); // TF | IF | DF | IOPL | AC | NT (0x47700)
        Efer::update(|efer| *efer |= EferFlags::SYSTEM_CALL_EXTENSIONS);
        KernelGsBase::write(VirtAddr::new(0));
    }
}
