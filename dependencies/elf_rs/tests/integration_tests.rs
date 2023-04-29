const TEST_ELF_FILE: &str = "tests/data/ls";

#[test]
fn test_unaligned_buffer() {
    extern crate elf_rs;

    use std::fs::File;
    use std::io::Read;

    use elf_rs::{Elf, ElfFile};

    for offset in 0..64 {
        let mut elf_file = File::open(TEST_ELF_FILE).expect("failed to open file");
        let mut elf_buf = Vec::<u8>::new();

        for _ in 0..offset {
            elf_buf.push(0)
        }

        elf_file
            .read_to_end(&mut elf_buf)
            .expect("failed to read file");

        let elf = Elf::from_bytes(&mut elf_buf[offset..]).expect("fail to load elf file");

        println!("elf {:?}", elf);
        if let Elf::Elf64(e) = elf {
            println!("elf header {:?}", e.elf_header());
        }
    }
}
