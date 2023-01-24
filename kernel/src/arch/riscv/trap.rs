use riscv::register::scause::{self, Exception as E, Trap};
use riscv::register::{mtvec::TrapMode, stval, stvec};

use super::TrapFrame;
use crate::{syscall::syscall, task};

include_asm_marcos!();

core::arch::global_asm!(
    include_str!("trap.S"),
    trapframe_size = const core::mem::size_of::<TrapFrame>(),
);

pub fn init() {
    extern "C" {
        fn trap_vector_base();
    }
    unsafe { stvec::write(trap_vector_base as usize, TrapMode::Direct) };
}

#[no_mangle]
fn riscv_trap_handler(tf: &mut TrapFrame, from_user: bool) {
    let scause = scause::read();
    trace!("trap {:?} @ {:#x}: {:#x?}", scause.cause(), tf.sepc, tf);
    match scause.cause() {
        Trap::Exception(E::UserEnvCall) => {
            tf.sepc += 4;
            tf.regs.a0 = syscall(tf, tf.regs.a7, tf.regs.a0, tf.regs.a1, tf.regs.a2) as _;
        }
        Trap::Exception(E::LoadPageFault)
        | Trap::Exception(E::StorePageFault)
        | Trap::Exception(E::InstructionPageFault) => {
            if from_user {
                warn!(
                    "Page Fault @ {:#x}, stval={:#x}, scause={}, kernel killed it.",
                    tf.sepc,
                    stval::read(),
                    scause.code(),
                );
                task::current().exit(-1);
            } else {
                panic!(
                    "Kernel Page Fault @ {:#x}, stval={:#x}, scause={}",
                    tf.sepc,
                    stval::read(),
                    scause.code(),
                );
            }
        }
        Trap::Interrupt(_) => task::handle_irq(scause.bits()),
        _ => {
            panic!(
                "Unsupported trap {:?} @ {:#x}:\n{:#x?}",
                scause.cause(),
                tf.sepc,
                tf
            );
        }
    }
}
