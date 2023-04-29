use crate::elf::ElfFile;
use core::fmt::{Debug, Formatter};
use core::ops::Deref;

mod section_header;

pub use section_header::{SectionHeader32, SectionHeader64};

const SHT_LOOS: u32 = 0x60000000;
const SHT_HIOS: u32 = 0x6fffffff;
const SHT_LOPROC: u32 = 0x70000000;
const SHT_HIPROC: u32 = 0x7fffffff;
const SHT_LOUSER: u32 = 0x80000000;
const SHT_HIUSER: u32 = 0xffffffff;
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SectionType {
    SHT_NULL,          // 0x0,
    SHT_PROGBITS,      // 0x1,
    SHT_SYMTAB,        // 0x2,
    SHT_STRTAB,        // 0x3,
    SHT_RELA,          // 0x4,
    SHT_HASH,          // 0x5,
    SHT_DYNAMIC,       // 0x6,
    SHT_NOTE,          // 0x7,
    SHT_NOBITS,        // 0x8,
    SHT_REL,           // 0x9,
    SHT_SHLIB,         // 0x0A,
    SHT_DYNSYM,        // 0x0B,
    SHT_INIT_ARRAY,    // 0x0E,
    SHT_FINI_ARRAY,    // 0x0F,
    SHT_PREINIT_ARRAY, // 0x10,
    SHT_GROUP,         // 0x11,
    SHT_SYMTAB_SHNDX,  // 0x12,
    SHT_NUM,           // 0x13,
    OsSpecific(u32),
    ProcessorSpecific(u32),
    ApplicationSpecific(u32),
    Unknown(u32),
}

impl From<u32> for SectionType {
    fn from(n: u32) -> Self {
        match n {
            0x0 => SectionType::SHT_NULL,
            0x1 => SectionType::SHT_PROGBITS,
            0x2 => SectionType::SHT_SYMTAB,
            0x3 => SectionType::SHT_STRTAB,
            0x4 => SectionType::SHT_RELA,
            0x5 => SectionType::SHT_HASH,
            0x6 => SectionType::SHT_DYNAMIC,
            0x7 => SectionType::SHT_NOTE,
            0x8 => SectionType::SHT_NOBITS,
            0x9 => SectionType::SHT_REL,
            0x0A => SectionType::SHT_SHLIB,
            0x0B => SectionType::SHT_DYNSYM,
            0x0E => SectionType::SHT_INIT_ARRAY,
            0x0F => SectionType::SHT_FINI_ARRAY,
            0x10 => SectionType::SHT_PREINIT_ARRAY,
            0x11 => SectionType::SHT_GROUP,
            0x12 => SectionType::SHT_SYMTAB_SHNDX,
            0x13 => SectionType::SHT_NUM,
            x @ SHT_LOOS..=SHT_HIOS => SectionType::OsSpecific(x),
            x @ SHT_LOPROC..=SHT_HIPROC => SectionType::ProcessorSpecific(x),
            x @ SHT_LOUSER..=SHT_HIUSER => SectionType::ApplicationSpecific(x),
            n => SectionType::Unknown(n),
        }
    }
}

bitflags! {
    pub struct SectionHeaderFlags: u64 {
        const SHF_WRITE             = 0x1;
        const SHF_ALLOC             = 0x2;
        const SHF_EXECINSTR         = 0x4;
        const SHF_MERGE             = 0x10;
        const SHF_STRINGS           = 0x20;
        const SHF_INFO_LINK         = 0x40;
        const SHF_LINK_ORDER        = 0x80;
        const SHF_OS_NONCONFORMING  = 0x100;
        const SHF_GROUP             = 0x200;
        const SHF_TLS	            = 0x400;
        const SHF_MASKOS            = 0x0ff00000;
        const SHF_MASKPROC          = 0xf0000000;
        const SHF_ORDERED           = 0x40000000;
        const SHF_EXCLUDE           = 0x80000000;
    }
}

pub trait SectionHeader {
    fn name_off(&self) -> u32;

    fn sh_type(&self) -> SectionType;

    fn flags(&self) -> SectionHeaderFlags;

    fn addr(&self) -> u64;

    fn offset(&self) -> u64;

    fn size(&self) -> u64;

    fn link(&self) -> u32;

    fn info(&self) -> u32;

    fn addralign(&self) -> u64;

    fn entsize(&self) -> u64;
}

pub struct SectionHeaderWrapper<'a> {
    elf_file: &'a dyn ElfFile,
    inner: &'a dyn SectionHeader,
}

impl<'a> Deref for SectionHeaderWrapper<'a> {
    type Target = dyn SectionHeader + 'a;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a> SectionHeaderWrapper<'a> {
    pub fn new(elf_file: &'a dyn ElfFile, inner: &'a dyn SectionHeader) -> Self {
        Self { elf_file, inner }
    }

    pub fn content(&self) -> &'a [u8] {
        let offset = self.inner.offset() as usize;
        let size = self.inner.size() as usize;
        &self.elf_file.content()[offset..offset + size]
    }

    pub fn section_name(&self) -> &[u8] {
        let name_off = self.inner.name_off() as usize;
        let shstr_section = self
            .elf_file
            .shstr_section()
            .expect("shstr section not found!");
        let shstr_content = shstr_section.content();
        let name_len = shstr_content[name_off..]
            .iter()
            .position(|&x| x == b'\0')
            .unwrap();
        &shstr_content[name_off..name_off + name_len]
    }
}

pub struct SectionHeaderIter<'a> {
    elf_file: &'a dyn ElfFile,
    index: usize,
}

impl<'a> SectionHeaderIter<'a> {
    pub fn new(elf_file: &'a dyn ElfFile) -> Self {
        Self { elf_file, index: 0 }
    }
}

impl<'a> Iterator for SectionHeaderIter<'a> {
    type Item = SectionHeaderWrapper<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.elf_file.section_header_nth(self.index).map(|e| {
            self.index += 1;
            e
        })
    }
}

impl<'a> Debug for SectionHeaderWrapper<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), core::fmt::Error> {
        f.debug_struct("Section Header")
            .field("name", &core::str::from_utf8(self.section_name()).unwrap())
            .field("type", &self.sh_type())
            .field("flags", &self.flags())
            .field("addr", &self.addr())
            .field("offset", &self.offset())
            .field("size", &self.size())
            .field("link", &self.link())
            .field("info", &self.info())
            .field("address alignment", &self.addralign())
            .field("entry size", &self.entsize())
            .finish()
    }
}
