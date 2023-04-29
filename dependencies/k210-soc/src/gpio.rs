#![allow(non_camel_case_types)]

use bitflags::bitflags;
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum direction {
    INPUT,
    OUTPUT,
}

bitflags! {
    pub struct GpioPinEdge: u8 {
        const GPIO_PE_NONE = 1 << 0;
        const GPIO_PE_FALLING = 1 << 1;
        const GPIO_PE_RISING = 1 << 2;
        const GPIO_PE_BOTH = 1 << 3;
        const GPIO_PE_LOW = 1 << 4;
        const GPIO_PE_HIGH = 1 << 5;
    }
}
