pub mod block;
pub mod bus;
pub mod interrupt;
pub mod misc;
pub mod net;
pub mod timer;
pub mod uart;

pub use bus::*;
pub use net::*;

pub fn init_early() {
    uart::init_early();
}

pub fn init() {
    println!("Initializing drivers...");
    interrupt::init();
    uart::init();
    timer::init();
}
