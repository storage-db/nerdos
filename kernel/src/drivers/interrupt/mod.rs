cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod apic;
        mod i8259_pic;
        use apic as imp;
        pub use apic::local_apic;
        pub use apic::vectors::*;
    } else if #[cfg(target_arch = "aarch64")] {
        mod gicv2;
        use gicv2 as imp;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        mod riscv_intc;
        use riscv_intc as imp;
        pub use riscv_intc::ScauseIntCode;
    }
}

pub use self::imp::handle_irq;

#[allow(unused_imports)]
pub(super) use self::imp::{init, register_handler, set_enable};
