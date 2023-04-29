//! A simple no_std ELF file reader for ELF32 and ELF64.
//!
//! ## Minimal Example
//! ```ignore
//! use elf_rs::Elf;
//!
//! /// Minimal example. Works in `no_std`-contexts and the parsing
//! /// itself needs zero allocations.
//! fn main() {
//!     let elf_bytes = include_bytes!("path/to/file.elf");
//!     let elf = elf_rs::Elf::from_bytes(elf_bytes).unwrap();
//!     let elf64 = match elf {
//!         Elf::Elf64(elf) => elf,
//!         _ => panic!("got Elf32, expected Elf64"),
//!     };
//!     let pr_hdrs = elf64.program_header_iter().collect::<Vec<_>>();
//!     dbg!(pr_hdrs);
//! }
//! ```

#![no_std]
#![allow(non_camel_case_types)]

#[macro_use]
extern crate bitflags;
extern crate num_traits;

use core::mem::size_of;
mod elf;
mod elf_header;
mod program_header;
mod section_header;

pub use elf::{Elf32, Elf64, ElfFile};
pub use elf_header::{
    ElfAbi, ElfClass, ElfEndian, ElfHeader, ElfHeader32, ElfHeader64, ElfMachine, ElfType,
};
pub use program_header::{
    ProgramHeader, ProgramHeader32, ProgramHeader64, ProgramHeaderFlags, ProgramHeaderIter,
    ProgramHeaderWrapper, ProgramType,
};
pub use section_header::{
    SectionHeader, SectionHeader32, SectionHeader64, SectionHeaderFlags, SectionHeaderIter,
    SectionHeaderWrapper, SectionType,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Error {
    BufferTooShort,
    InvalidMagic,
    InvalidClass,
}

#[derive(Debug)]
pub enum Elf<'a> {
    Elf32(Elf32<'a>),
    Elf64(Elf64<'a>),
}

impl<'a> Elf<'a> {
    pub fn from_bytes(elf_buf: &'a [u8]) -> Result<Self, Error> {
        if elf_buf.len() < size_of::<ElfHeader32>() {
            return Err(Error::BufferTooShort);
        }

        if !elf_buf.starts_with(&elf_header::ELF_MAGIC) {
            return Err(Error::InvalidMagic);
        }

        let tmp_elf = Elf32::new(elf_buf);
        match tmp_elf.elf_header().class() {
            ElfClass::Elf64 => Elf64::from_bytes(elf_buf).map(|e| Elf::Elf64(e)),
            ElfClass::Elf32 => Elf32::from_bytes(elf_buf).map(|e| Elf::Elf32(e)),
            ElfClass::Unknown(_) => Err(Error::InvalidClass),
        }
    }
}

impl<'a> ElfFile for Elf<'a> {
    fn content(&self) -> &[u8] {
        match self {
            Elf::Elf32(e) => e.content(),
            Elf::Elf64(e) => e.content(),
        }
    }

    fn elf_header(&self) -> crate::elf_header::ElfHeaderWrapper {
        match self {
            Elf::Elf32(e) => e.elf_header(),
            Elf::Elf64(e) => e.elf_header(),
        }
    }

    fn program_header_nth(&self, index: usize) -> Option<ProgramHeaderWrapper> {
        match self {
            Elf::Elf32(e) => e.program_header_nth(index),
            Elf::Elf64(e) => e.program_header_nth(index),
        }
    }

    fn program_header_iter(&self) -> ProgramHeaderIter {
        match self {
            Elf::Elf32(e) => e.program_header_iter(),
            Elf::Elf64(e) => e.program_header_iter(),
        }
    }

    fn section_header_nth(&self, index: usize) -> Option<SectionHeaderWrapper> {
        match self {
            Elf::Elf32(e) => e.section_header_nth(index),
            Elf::Elf64(e) => e.section_header_nth(index),
        }
    }

    fn section_header_iter(&self) -> SectionHeaderIter {
        match self {
            Elf::Elf32(e) => e.section_header_iter(),
            Elf::Elf64(e) => e.section_header_iter(),
        }
    }
}
