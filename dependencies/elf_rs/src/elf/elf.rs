use super::ElfFile;
use crate::elf_header::{ElfHeader, ElfHeader32, ElfHeader64, ElfHeaderWrapper};
use crate::program_header::{
    ProgramHeader, ProgramHeader32, ProgramHeader64, ProgramHeaderIter, ProgramHeaderWrapper,
};
use crate::section_header::{
    SectionHeader, SectionHeader32, SectionHeader64, SectionHeaderIter, SectionHeaderWrapper,
};
use core::fmt::{Debug, Error, Formatter};
use core::marker::PhantomData;
use core::mem::size_of;
use core::slice::from_raw_parts;

pub trait ElfType {
    type ElfHeader: ElfHeader;
    type ProgramHeader: ProgramHeader;
    type SectionHeader: SectionHeader;
}

#[derive(Debug)]
pub enum ElfType64 {}

impl ElfType for ElfType64 {
    type ElfHeader = ElfHeader64;
    type ProgramHeader = ProgramHeader64;
    type SectionHeader = SectionHeader64;
}

#[derive(Debug)]
pub enum ElfType32 {}

impl ElfType for ElfType32 {
    type ElfHeader = ElfHeader32;
    type ProgramHeader = ProgramHeader32;
    type SectionHeader = SectionHeader32;
}

pub struct ElfGen<'a, ET>(&'a [u8], PhantomData<ET>);

impl<'a, ET: ElfType> ElfGen<'a, ET> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self(buf, PhantomData)
    }

    pub fn from_bytes(buf: &'a [u8]) -> Result<Self, crate::Error> {
        if buf.len() < size_of::<ET::ElfHeader>() {
            return Err(crate::Error::BufferTooShort);
        }
        let elf = Self(buf, PhantomData);
        if buf.len() < elf.elf_header().elf_header_size() as usize {
            Err(crate::Error::BufferTooShort)
        } else {
            Ok(elf)
        }
    }

    fn content(&self) -> &[u8] {
        self.0
    }

    pub fn elf_header_raw(&self) -> &ET::ElfHeader {
        unsafe { &*(self.content().as_ptr() as *const ET::ElfHeader) }
    }

    fn program_header_raw(&'a self) -> &'a [ET::ProgramHeader] {
        let ph_off = self.elf_header().program_header_offset() as usize;
        let ph_num = self.elf_header().program_header_entry_num() as usize;
        unsafe {
            let ph_ptr = self.content().as_ptr().add(ph_off);
            from_raw_parts(ph_ptr as *const ET::ProgramHeader, ph_num)
        }
    }

    pub fn program_header_iter(&self) -> ProgramHeaderIter {
        ProgramHeaderIter::new(self)
    }

    pub fn program_header_nth(&self, index: usize) -> Option<ProgramHeaderWrapper> {
        self.program_header_raw()
            .get(index)
            .map(|ph| ProgramHeaderWrapper::new(self, ph))
    }

    fn section_header_raw(&self) -> &[ET::SectionHeader] {
        let sh_off = self.elf_header().section_header_offset() as usize;
        let sh_num = self.elf_header().section_header_entry_num() as usize;
        unsafe {
            let sh_ptr = self.content().as_ptr().add(sh_off);
            from_raw_parts(sh_ptr as *const ET::SectionHeader, sh_num)
        }
    }

    pub fn section_header_iter(&self) -> SectionHeaderIter {
        SectionHeaderIter::new(self)
    }

    pub fn section_header_nth(&self, index: usize) -> Option<SectionHeaderWrapper> {
        self.section_header_raw()
            .get(index)
            .map(|sh| SectionHeaderWrapper::new(self, sh))
    }
}

impl<'a, ET: ElfType> ElfFile for ElfGen<'a, ET> {
    fn content(&self) -> &[u8] {
        self.content()
    }

    fn elf_header(&self) -> crate::elf_header::ElfHeaderWrapper {
        ElfHeaderWrapper::new(self, self.elf_header_raw())
    }

    fn program_header_nth(&self, index: usize) -> Option<ProgramHeaderWrapper> {
        self.program_header_nth(index)
    }

    fn program_header_iter(&self) -> ProgramHeaderIter {
        self.program_header_iter()
    }

    fn section_header_iter(&self) -> SectionHeaderIter {
        self.section_header_iter()
    }

    fn section_header_nth(&self, index: usize) -> Option<SectionHeaderWrapper> {
        self.section_header_nth(index)
    }
}

pub type Elf32<'a> = ElfGen<'a, ElfType32>;
pub type Elf64<'a> = ElfGen<'a, ElfType64>;

impl<'a, ET: ElfType> Debug for ElfGen<'a, ET> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Elf File")
            .field("Memory Location", &self.content().as_ptr())
            .finish()
    }
}
