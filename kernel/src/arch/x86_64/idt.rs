use alloc::boxed::Box;
use core::arch::asm;

use x86_64::addr::VirtAddr;
use x86_64::structures::idt::{Entry, HandlerFunc, InterruptDescriptorTable};
use x86_64::structures::DescriptorTablePointer;

use crate::sync::LazyInit;

const NUM_INT: usize = 256;

pub(super) static IDT: LazyInit<IdtStruct> = LazyInit::new();

pub(super) struct IdtStruct {
    table: &'static mut InterruptDescriptorTable,
}

impl IdtStruct {
    fn alloc() -> Self {
        Self {
            table: Box::leak(Box::new(InterruptDescriptorTable::new())),
        }
    }

    fn init(&mut self) {
        extern "C" {
            #[link_name = "trap_handler_table"]
            static ENTRIES: [extern "C" fn(); NUM_INT];
        }
        let entries = unsafe {
            core::slice::from_raw_parts_mut(self.table as *mut _ as *mut Entry<HandlerFunc>, 256)
        };
        for i in 0..NUM_INT {
            let opt = entries[i].set_handler_fn(unsafe { core::mem::transmute(ENTRIES[i]) });
            if i == 0x80 {
                // syscall via `int 0x80`
                opt.set_privilege_level(x86_64::PrivilegeLevel::Ring3);
            }
        }
    }

    fn pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            base: VirtAddr::new(self.table as *const _ as u64),
            limit: (core::mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
        }
    }

    pub fn load(&'static self) {
        unsafe {
            asm!("lidt [{}]", in(reg) &self.pointer(), options(readonly, nostack, preserves_flags))
        }
    }
}

pub fn init() {
    println!("Initializing IDT...");
    let mut idt = IdtStruct::alloc();
    idt.init();
    IDT.init_by(idt);
}
