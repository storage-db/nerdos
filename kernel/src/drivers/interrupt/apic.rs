//! Intel Local APIC and IO APIC.

#![allow(dead_code)]

use x2apic::ioapic::{IoApic, IrqFlags, IrqMode};
use x2apic::lapic::{xapic_base, LocalApic, LocalApicBuilder};

use self::vectors::*;
use crate::mm::PhysAddr;
use crate::sync::{LazyInit, PerCpuData, SpinNoIrqLock};
use crate::utils::irq_handler::{IrqHandler, IrqHandlerTable};

pub mod vectors {
    pub const PIT_GSI: usize = 2; // TODO: lookup ACPI tables
    pub const PIT_VECTOR: usize = 0x20;

    pub const APIC_TIMER_VECTOR: usize = 0xf0;
    pub const APIC_SPURIOUS_VECTOR: usize = 0xf1;
    pub const APIC_ERROR_VECTOR: usize = 0xf2;
}

const IRQ_COUNT: usize = 256;

const IO_APIC_BASE: PhysAddr = PhysAddr::new(0xFEC0_0000);

static LOCAL_APIC: LazyInit<PerCpuData<LocalApic>> = LazyInit::new();
static IO_APIC: LazyInit<SpinNoIrqLock<IoApic>> = LazyInit::new();
static HANDLERS: IrqHandlerTable<IRQ_COUNT> = IrqHandlerTable::new();

fn lapic_eoi() {
    unsafe { local_apic().end_of_interrupt() };
}

pub fn set_enable(gsi: usize, enable: bool) {
    unsafe {
        if enable {
            IO_APIC.lock().enable_irq(gsi as u8);
        } else {
            IO_APIC.lock().disable_irq(gsi as u8);
        }
    }
}

#[allow(dead_code)]
fn configure_irq(gsi: usize, vector: usize) {
    let mut io_apic = IO_APIC.lock();
    unsafe {
        let mut entry = io_apic.table_entry(gsi as u8);
        entry.set_dest((local_apic().id() >> 24) as u8); // TODO: distinguish x2apic/x2apic
        entry.set_vector(vector as u8);
        entry.set_mode(IrqMode::Fixed);
        entry.set_flags(IrqFlags::MASKED);
        io_apic.set_table_entry(gsi as u8, entry);
    }
}

pub fn handle_irq(vector: usize) {
    HANDLERS.handle(vector);
    lapic_eoi();
}

pub fn register_handler(vector: usize, handler: IrqHandler) {
    HANDLERS.register_handler(vector, handler);
}

pub fn init() {
    println!("Initializing Local APIC...");
    super::i8259_pic::init();

    let base_vaddr = PhysAddr::new(unsafe { xapic_base() } as usize).into_kvaddr();
    let mut lapic = LocalApicBuilder::new()
        .timer_vector(APIC_TIMER_VECTOR)
        .error_vector(APIC_ERROR_VECTOR)
        .spurious_vector(APIC_SPURIOUS_VECTOR)
        .set_xapic_base(base_vaddr.as_usize() as u64)
        .build()
        .unwrap();
    unsafe { lapic.enable() };
    LOCAL_APIC.init_by(PerCpuData::new(lapic));

    let io_apic = unsafe { IoApic::new(IO_APIC_BASE.into_kvaddr().as_usize() as u64) };
    IO_APIC.init_by(SpinNoIrqLock::new(io_apic));

    super::register_handler(APIC_TIMER_VECTOR, crate::timer::handle_timer_irq);
}

pub fn local_apic() -> &'static mut LocalApic {
    unsafe { LOCAL_APIC.as_mut() }
}
