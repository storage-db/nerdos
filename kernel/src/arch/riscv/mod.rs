#[macro_use]
mod macros;

mod context;
mod page_table;
mod percpu;
mod trap;

pub mod config;
pub mod instructions;

pub use self::context::{TaskContext, TrapFrame};
pub use self::page_table::{PageTable, PageTableEntry};
pub use self::percpu::ArchPerCpu;

use riscv::register::{sie, sscratch};

pub fn init() {}

pub fn init_percpu() {
    unsafe {
        sscratch::write(0);
        sie::clear_sext();
        sie::clear_ssoft();
        sie::clear_stimer();
    }
    trap::init();
}
