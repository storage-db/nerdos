pub use crate::arch::config::*;
pub use crate::platform::config::*;

// Memory size

pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_BASE + PHYS_MEMORY_SIZE;

pub const BOOT_KERNEL_STACK_SIZE: usize = 4096 * 4; // 16K
pub const USER_STACK_SIZE: usize = 4096 * 4; // 16K
pub const USER_STACK_BASE: usize = USER_ASPACE_BASE + USER_ASPACE_SIZE - USER_STACK_SIZE;
pub const KERNEL_STACK_SIZE: usize = 4096 * 4; // 16K
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4M

// SMP

pub const MAX_CPUS: usize = 1;

// Scheduler

pub const TICKS_PER_SEC: u64 = 100;
