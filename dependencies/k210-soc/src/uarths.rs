use k210_pac as pac;

use crate::sysctl::{self, clock};
pub fn uarths_init(){
    unsafe {
        let ptr = pac::UARTHS::ptr();
        let freq = sysctl::clock_get_freq(clock::CPU);
        let div =freq / 115200 - 1;
        (*ptr).div.write(|w|w.div().bits(div as u16));
        (*ptr).txctrl.write(|w|{w.txen().set_bit()});
        (*ptr).rxctrl.write(|w|w.rxen().set_bit());
        (*ptr).txctrl.write(|w|w.txcnt().bits(0));
        (*ptr).rxctrl.write(|w|w.rxcnt().bits(0));
        (*ptr).ip.write(|w|w.txwm().set_bit());
        (*ptr).ip.write(|w|w.rxwm().set_bit());
        (*ptr).ie.write(|w|w.txwm().clear_bit());
        (*ptr).ie.write(|w|w.rxwm().set_bit());
    }
}

pub fn uarths_getchar() -> u8 {
    unsafe {
        let ptr = pac::UARTHS::ptr();
        let recv = (*ptr).rxdata.read();
        match recv.empty().bits() {
            true => {
                0
            },
            false => {
               recv.data().bits() & 0xff
            }
        }
    }
}

pub fn uarths_putchar(ch:u8) {
    unsafe {
        let ptr = pac::UARTHS::ptr();
        while (*ptr).txdata.read().full().bit() {continue;}
        (*ptr).txdata.write(|w|w.data().bits(ch))
    }
}