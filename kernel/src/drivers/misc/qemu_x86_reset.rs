use x86_64::instructions::port::PortWriteOnly;

pub fn shutdown() -> ! {
    if cfg!(feature = "rvm") {
        loop {
            crate::arch::instructions::wait_for_ints();
        }
    } else {
        warn!("Shutting down...");
        unsafe { PortWriteOnly::new(0x604).write(0x2000u16) };
        unreachable!("It should shutdown!")
    }
}
