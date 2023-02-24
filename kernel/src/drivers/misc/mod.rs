cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod qemu_x86_reset;
        use qemu_x86_reset as imp;
    } else if #[cfg(target_arch = "aarch64")] {
        mod psci;
        use psci as imp;
    } else if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        pub mod sbi;
        use sbi as imp;
    }
}

pub use self::imp::shutdown;
