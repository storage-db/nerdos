cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_hpet;
        mod x86_tsc;
        mod x86_common;
        use x86_common as imp;
    } else if #[cfg(target_arch = "aarch64")] {
        mod arm_generic_timer;
        use arm_generic_timer as imp;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        mod riscv;
        use self::riscv as imp;
    }
}

pub(super) use self::imp::init;
pub use self::imp::{current_ticks, nanos_to_ticks, set_oneshot_timer, ticks_to_nanos};
