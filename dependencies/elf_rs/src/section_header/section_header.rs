use core::ptr::read_unaligned;
use num_traits::PrimInt;

use super::{SectionHeader, SectionHeaderFlags, SectionType};

#[repr(C)]
#[derive(Debug)]
pub struct SectionHeaderGen<T: PrimInt> {
    sh_name: u32,
    sh_type: u32,
    sh_flags: T,
    sh_addr: T,
    sh_offset: T,
    sh_size: T,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: T,
    sh_entsize: T,
}

impl<T: PrimInt + Into<u64>> SectionHeader for SectionHeaderGen<T> {
    fn name_off(&self) -> u32 {
        unsafe { read_unaligned(&self.sh_name) }
    }

    fn sh_type(&self) -> SectionType {
        unsafe { read_unaligned(&self.sh_type).into() }
    }

    fn flags(&self) -> SectionHeaderFlags {
        let flags = unsafe { read_unaligned(&self.sh_flags).to_u64().unwrap() };
        SectionHeaderFlags::from_bits_truncate(flags)
    }

    fn addr(&self) -> u64 {
        unsafe { read_unaligned(&self.sh_addr).into() }
    }

    fn offset(&self) -> u64 {
        unsafe { read_unaligned(&self.sh_offset).into() }
    }

    fn size(&self) -> u64 {
        unsafe { read_unaligned(&self.sh_size).into() }
    }

    fn link(&self) -> u32 {
        unsafe { read_unaligned(&self.sh_link).into() }
    }

    fn info(&self) -> u32 {
        unsafe { read_unaligned(&self.sh_info).into() }
    }

    fn addralign(&self) -> u64 {
        unsafe { read_unaligned(&self.sh_addralign).into() }
    }

    fn entsize(&self) -> u64 {
        unsafe { read_unaligned(&self.sh_entsize).into() }
    }
}

pub type SectionHeader32 = SectionHeaderGen<u32>;
pub type SectionHeader64 = SectionHeaderGen<u64>;
