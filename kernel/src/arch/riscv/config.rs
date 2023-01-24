pub const USER_ASPACE_BASE: usize = 0;
pub const USER_ASPACE_SIZE: usize = 0x003f_ffff_f000;
pub const KERNEL_ASPACE_BASE: usize = 0xffff_ffc0_0000_0000;
pub const KERNEL_ASPACE_SIZE: usize = 0x0000_003f_ffff_f000;

pub const PHYS_VIRT_OFFSET: usize = 0xffff_ffc0_0000_0000;

pub const PA_MAX_BITS: usize = 56;
pub const VA_MAX_BITS: usize = 39; // Sv39
