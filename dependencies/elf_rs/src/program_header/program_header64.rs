use crate::program_header::{ProgramHeader, ProgramHeaderFlags, ProgramType};
use core::ptr::read_unaligned;

#[derive(Debug)]
#[repr(C)]
pub struct ProgramHeader64 {
    p_type: u32,
    p_flags: ProgramHeaderFlags,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

impl ProgramHeader for ProgramHeader64 {
    fn ph_type(&self) -> ProgramType {
        unsafe { read_unaligned(&self.p_type).into() }
    }

    fn flags(&self) -> ProgramHeaderFlags {
        unsafe { read_unaligned(&self.p_flags) }
    }

    fn offset(&self) -> u64 {
        unsafe { read_unaligned(&self.p_offset) }
    }

    fn vaddr(&self) -> u64 {
        unsafe { read_unaligned(&self.p_vaddr) }
    }

    fn paddr(&self) -> u64 {
        unsafe { read_unaligned(&self.p_paddr) }
    }

    fn filesz(&self) -> u64 {
        unsafe { read_unaligned(&self.p_filesz) }
    }

    fn memsz(&self) -> u64 {
        unsafe { read_unaligned(&self.p_memsz) }
    }

    fn align(&self) -> u64 {
        unsafe { read_unaligned(&self.p_align) }
    }
}
