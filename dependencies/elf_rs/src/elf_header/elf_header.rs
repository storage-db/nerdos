use core::ptr::read_unaligned;
use num_traits::PrimInt;

use super::{ElfAbi, ElfClass, ElfEndian, ElfHeader, ElfMachine, ElfType};

#[repr(C)]
#[derive(Debug)]
pub struct ElfHeaderGen<T: PrimInt> {
    magic: [u8; 4],
    class: u8,
    endianness: u8,
    header_version: u8,
    abi: u8,
    abi_version: u8,
    unused: [u8; 7],
    elftype: u16,
    machine: u16,
    elf_version: u32,
    entry: T,
    phoff: T,
    shoff: T,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

impl<T: PrimInt + Into<u64>> ElfHeader for ElfHeaderGen<T> {
    fn class(&self) -> ElfClass {
        self.class.into()
    }

    fn endianness(&self) -> ElfEndian {
        self.endianness.into()
    }

    fn header_version(&self) -> u8 {
        self.header_version
    }

    fn abi(&self) -> ElfAbi {
        self.abi.into()
    }

    fn abi_version(&self) -> u8 {
        self.abi_version
    }

    fn elftype(&self) -> ElfType {
        unsafe { read_unaligned(&self.elftype).into() }
    }

    fn machine(&self) -> ElfMachine {
        unsafe { read_unaligned(&self.machine).into() }
    }

    fn elf_version(&self) -> u32 {
        unsafe { read_unaligned(&self.elf_version) }
    }

    fn entry_point(&self) -> u64 {
        unsafe { read_unaligned(&self.entry).into() }
    }

    fn program_header_offset(&self) -> u64 {
        unsafe { read_unaligned(&self.phoff).into() }
    }

    fn section_header_offset(&self) -> u64 {
        unsafe { read_unaligned(&self.shoff).into() }
    }

    fn flags(&self) -> u32 {
        unsafe { read_unaligned(&self.flags) }
    }

    fn elf_header_size(&self) -> u16 {
        unsafe { read_unaligned(&self.ehsize) }
    }

    fn program_header_entry_size(&self) -> u16 {
        unsafe { read_unaligned(&self.phentsize) }
    }

    fn program_header_entry_num(&self) -> u16 {
        unsafe { read_unaligned(&self.phnum) }
    }

    fn section_header_entry_size(&self) -> u16 {
        unsafe { read_unaligned(&self.shentsize) }
    }

    fn section_header_entry_num(&self) -> u16 {
        unsafe { read_unaligned(&self.shnum) }
    }

    fn shstr_index(&self) -> u16 {
        unsafe { read_unaligned(&self.shstrndx) }
    }
}

pub type ElfHeader32 = ElfHeaderGen<u32>;
pub type ElfHeader64 = ElfHeaderGen<u64>;
