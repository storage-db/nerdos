use crate::elf::ElfFile;
use core::fmt::{Debug, Formatter};
use core::ops::Deref;

mod program_header32;
mod program_header64;

pub use program_header32::ProgramHeader32;
pub use program_header64::ProgramHeader64;

const LOOS: u32 = 0x60000000;
const HIOS: u32 = 0x6FFFFFFF;
const LOPROC: u32 = 0x70000000;
const HIPROC: u32 = 0x7FFFFFFF;

bitflags! {
    /// The flags of an ELF program header. Always 32 bit long, also
    /// for 64-bit ELFs.
    ///
    /// Also called "Segment Permissions" in ELF specification or "p_flags".
    pub struct ProgramHeaderFlags: u32 {
        const EXECUTE = 1;
        const WRITE = 2;
        const READ = 4;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProgramType {
    NULL,                   // 0x00000000,
    LOAD,                   // 0x00000001,
    DYNAMIC,                // 0x00000002,
    INTERP,                 // 0x00000003,
    NOTE,                   // 0x00000004,
    SHLIB,                  // 0x00000005,
    PHDR,                   // 0x00000006,
    OsSpecific(u32),        // 0x60000000 - 0x6FFFFFFF,
    ProcessorSpecific(u32), // 0x70000000 - 0x7FFFFFFF,

    Unknown(u32),
}

impl From<u32> for ProgramType {
    fn from(n: u32) -> Self {
        match n {
            0x00000000 => ProgramType::NULL,
            0x00000001 => ProgramType::LOAD,
            0x00000002 => ProgramType::DYNAMIC,
            0x00000003 => ProgramType::INTERP,
            0x00000004 => ProgramType::NOTE,
            0x00000005 => ProgramType::SHLIB,
            0x00000006 => ProgramType::PHDR,
            x @ LOOS..=HIOS => ProgramType::OsSpecific(x),
            x @ LOPROC..=HIPROC => ProgramType::ProcessorSpecific(x),
            x => ProgramType::Unknown(x),
        }
    }
}

pub trait ProgramHeader {
    fn ph_type(&self) -> ProgramType;

    fn flags(&self) -> ProgramHeaderFlags;

    fn offset(&self) -> u64;

    fn vaddr(&self) -> u64;

    fn paddr(&self) -> u64;

    fn filesz(&self) -> u64;

    fn memsz(&self) -> u64;

    fn align(&self) -> u64;
}

pub struct ProgramHeaderWrapper<'a> {
    elf_file: &'a dyn ElfFile,
    inner: &'a dyn ProgramHeader,
}

impl<'a> ProgramHeaderWrapper<'a> {
    pub fn new(elf_file: &'a dyn ElfFile, inner: &'a dyn ProgramHeader) -> Self {
        Self { elf_file, inner }
    }

    pub fn content(&self) -> &'a [u8] {
        let offset = self.inner.offset() as usize;
        let size = self.inner.filesz() as usize;
        &self.elf_file.content()[offset..offset + size]
    }
}

impl<'a> Deref for ProgramHeaderWrapper<'a> {
    type Target = dyn ProgramHeader + 'a;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a> Debug for ProgramHeaderWrapper<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), core::fmt::Error> {
        f.debug_struct("Program Header")
            .field("type", &self.ph_type())
            .field("flags", &self.flags())
            .field("offset", &self.offset())
            .field("vaddr", &self.vaddr())
            .field("paddr", &self.paddr())
            .field("filesize", &self.filesz())
            .field("memsize", &self.memsz())
            .field("alignment", &self.align())
            .finish()
    }
}

pub struct ProgramHeaderIter<'a> {
    elf_file: &'a dyn ElfFile,
    index: usize,
}

impl<'a> ProgramHeaderIter<'a> {
    pub fn new(elf_file: &'a dyn ElfFile) -> Self {
        Self { elf_file, index: 0 }
    }
}

impl<'a> Iterator for ProgramHeaderIter<'a> {
    type Item = ProgramHeaderWrapper<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.elf_file.program_header_nth(self.index).map(|e| {
            self.index += 1;
            e
        })
    }
}
