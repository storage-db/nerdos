use x86_64::instructions::port::Port;

static mut PIC1: Pic = Pic::new(0x20);
static mut PIC2: Pic = Pic::new(0xA0);

struct Pic {
    _cmd: Port<u8>,
    data: Port<u8>,
}

impl Pic {
    pub const fn new(port: u16) -> Self {
        Self {
            _cmd: Port::new(port),
            data: Port::new(port + 1),
        }
    }

    unsafe fn set_mask(&mut self, irq_mask: u8) {
        self.data.write(irq_mask)
    }
}

pub fn init() {
    // Don't use the 8259A interrupt controllers, use APIC instead.
    unsafe {
        PIC1.set_mask(0xff);
        PIC2.set_mask(0xff);
    }
}
