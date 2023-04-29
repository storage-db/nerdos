use crate::elf_header::ElfHeaderWrapper;
use crate::program_header::{ProgramHeaderIter, ProgramHeaderWrapper};
use crate::section_header::{SectionHeaderIter, SectionHeaderWrapper};

mod elf;
pub use elf::{Elf32, Elf64};

pub trait ElfFile {
    fn content(&self) -> &[u8];

    fn elf_header(&self) -> ElfHeaderWrapper;

    fn program_header_nth(&self, index: usize) -> Option<ProgramHeaderWrapper>;

    fn program_header_iter(&self) -> ProgramHeaderIter;

    fn section_header_nth(&self, index: usize) -> Option<SectionHeaderWrapper>;

    fn section_header_iter(&self) -> SectionHeaderIter;

    fn shstr_section(&self) -> Option<SectionHeaderWrapper> {
        let shstr_index = self.elf_header().shstr_index() as usize;
        self.section_header_nth(shstr_index)
    }

    fn lookup_section(&self, name: &[u8]) -> Option<SectionHeaderWrapper> {
        self.section_header_iter()
            .find(|s| s.section_name() == name)
    }

    fn entry_point(&self) -> u64 {
        self.elf_header().entry_point()
    }
}
