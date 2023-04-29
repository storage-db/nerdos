extern crate elf_rs;

use std::env;
use std::fs::File;
use std::io::Read;

use elf_rs::*;

fn read_elf(filename: &String) -> Result<(), ()> {
    let mut elf_file = File::open(filename).map_err(|e| {
        println!("failed to open file {}: {}", filename, e);
        ()
    })?;
    let mut elf_buf = Vec::<u8>::new();

    elf_file.read_to_end(&mut elf_buf).map_err(|e| {
        println!("failed to read file {}: {}", filename, e);
        ()
    })?;

    let elf = Elf::from_bytes(&mut elf_buf).map_err(|e| {
        println!("failed to extract elf file {}: {:?}", filename, e);
        ()
    })?;

    println!("{:?} header: {:?}", elf, elf.elf_header());

    for p in elf.program_header_iter() {
        println!("{:x?}", p);
    }

    for s in elf.section_header_iter() {
        println!("{:x?}", s);
    }

    if let Some(s) = elf.lookup_section(b".text") {
        println!(".test section {:?}", s);
    }

    Ok(())
}

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Need specify file path!");
        return Err(());
    }

    let filename = &args[1];
    read_elf(&filename)?;

    Ok(())
}
