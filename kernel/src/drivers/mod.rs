pub mod interrupt;
pub mod misc;
pub mod timer;
pub mod uart;

pub fn init_early() {
    uart::init_early();
}

pub fn init() {
    println!("Initializing drivers...");
    interrupt::init();
    uart::init();
    timer::init();
}
