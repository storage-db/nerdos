elf_rs 
===
[![Build Status](https://app.travis-ci.com/vincenthouyi/elf_rs.svg?branch=master)](https://app.travis-ci.com/vincenthouyi/elf_rs)
[![Crates.io](https://img.shields.io/crates/v/elf_rs)](https://crates.io/crates/elf_rs)

This is a no_std library for ELF file handling.
It supports ELF32 and ELF64 format.

Usage
===
To read an elf file, supply `elf_rs::Elf` with a `&[u8]` memory:
```rust
extern crate elf_rs;

use std::io::Read;
use std::fs::File;
use std::env;

use elf_rs::*;

fn read_elf(filename: &String) {
    let mut elf_file = File::open(filename).expect("open file failed");
    let mut elf_buf = Vec::<u8>::new();
    elf_file.read_to_end(&mut elf_buf).expect("read file failed");

    let elf = Elf::from_bytes(&elf_buf).expect("load elf file failed");

    println!("{:?} header: {:?}", elf, elf.elf_header());

    for p in elf.program_header_iter() {
        println!("{:x?}", p);
    }

    for s in elf.section_header_iter() {
        println!("{:x?}", s);
    }

    let s = elf.lookup_section(b".text");
    println!("s {:?}", s);
}
```
Under example directory there is a demo `readelf` to read an ELF file.
```
$ cargo run --example readelf <path_to_elf_file>
```


License
===
it is distributed under the terms of the MIT license.

Please see LICENSE file for details.
