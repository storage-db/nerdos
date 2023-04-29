use crate::program_header::{ProgramHeader, ProgramHeaderFlags, ProgramType};
use core::ptr::read_unaligned;

#[derive(Debug)]
#[repr(C)]
pub struct ProgramHeader32 {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: ProgramHeaderFlags,
    p_align: u32,
}

impl ProgramHeader for ProgramHeader32 {
    fn ph_type(&self) -> ProgramType {
        unsafe { read_unaligned(&self.p_type).into() }
    }

    fn flags(&self) -> ProgramHeaderFlags {
        unsafe { read_unaligned(&self.p_flags) }
    }

    fn offset(&self) -> u64 {
        unsafe { read_unaligned(&self.p_offset) as u64 }
    }

    fn vaddr(&self) -> u64 {
        unsafe { read_unaligned(&self.p_vaddr) as u64 }
    }

    fn paddr(&self) -> u64 {
        unsafe { read_unaligned(&self.p_paddr) as u64 }
    }

    fn filesz(&self) -> u64 {
        unsafe { read_unaligned(&self.p_filesz) as u64 }
    }

    fn memsz(&self) -> u64 {
        unsafe { read_unaligned(&self.p_memsz) as u64 }
    }

    fn align(&self) -> u64 {
        unsafe { read_unaligned(&self.p_align) as u64 }
    }
}
