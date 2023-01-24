use core::arch::global_asm;

use x86::{controlregs::cr2, irq::*};

use super::context::TrapFrame;
use crate::{syscall::syscall, task};

global_asm!(include_str!("trap.S"));

const SYSCALL_VECTOR: u8 = 0x80;

const IRQ_VECTOR_START: u8 = 0x20;
const IRQ_VECTOR_END: u8 = 0xff;

#[no_mangle]
fn x86_trap_handler(tf: &mut TrapFrame) {
    trace!("trap {} @ {:#x}: {:#x?}", tf.vector, tf.rip, tf);
    match tf.vector as u8 {
        PAGE_FAULT_VECTOR => {
            if tf.is_user() {
                warn!(
                    "Page Fault @ {:#x}, fault_vaddr={:#x}, error_code={:#x}, kernel killed it.",
                    tf.rip,
                    unsafe { cr2() },
                    tf.error_code,
                );
                task::current().exit(-1);
            } else {
                panic!(
                    "Kernel Page Fault @ {:#x}, fault_vaddr={:#x}, error_code={:#x}",
                    tf.rip,
                    unsafe { cr2() },
                    tf.error_code,
                );
            }
        }
        GENERAL_PROTECTION_FAULT_VECTOR => {
            warn!(
                "General Protection Exception @ {:#x}, error_code = {:#x}, kernel killed it.",
                tf.rip, tf.error_code,
            );
            task::current().exit(-1);
        }
        SYSCALL_VECTOR => {
            tf.rax = syscall(tf, tf.rax as _, tf.rdi as _, tf.rsi as _, tf.rdx as _) as u64
        }
        IRQ_VECTOR_START..=IRQ_VECTOR_END => {
            task::handle_irq(tf.vector as usize);
        }
        _ => {
            panic!(
                "Unsupported exception {} (error_code = {:#x}) @ {:#x}:\n{:#x?}",
                tf.vector, tf.error_code, tf.rip, tf
            );
        }
    }
}
