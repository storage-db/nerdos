#![allow(unused)]
use k210_hal::gpio::Gpio;
use k210_pac as pac;

use crate::gpio::GpioPinEdge;

use super::gpio;
use super::utils::{get_bit, set_bit};

pub fn set_direction(pin: u8, direction: gpio::direction) {
    unsafe {
        let ptr = pac::GPIOHS::ptr();
        (*ptr)
            .output_en
            .modify(|r, w| w.bits(set_bit(r.bits(), pin, direction == gpio::direction::OUTPUT)));
        (*ptr)
            .input_en
            .modify(|r, w| w.bits(set_bit(r.bits(), pin, direction == gpio::direction::INPUT)));
    }
}

pub fn set_pin(pin: u8, value: bool) {
    unsafe {
        let ptr = pac::GPIOHS::ptr();
        (*ptr)
            .output_val
            .modify(|r, w| w.bits(set_bit(r.bits(), pin, value)));
    }
}

pub fn get_pin(pin: u8) -> bool {
    unsafe {
        let ptr = pac::GPIOHS::ptr();
        get_bit((*ptr).input_val.read().bits(), pin)
    }
}

pub fn set_pin_edge(pin: u8, edge: GpioPinEdge) {
    unsafe {
        let ptr = pac::GPIOHS::ptr();
        (*ptr)
        .low_ie
        .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        (*ptr)
        .low_ip
        .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        (*ptr)
        .low_ie
        .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // (*ptr)
        //     .rise_ie
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // (*ptr)
        //     .fall_ie
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // (*ptr)
        //     .low_ie
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // (*ptr)
        //     .high_ie
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        
        
        // (*ptr)
        //     .rise_ip
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // (*ptr)
        //     .fall_ip
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // (*ptr)
        //     .low_ip
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // (*ptr)
        //     .high_ip
        //     .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));

        // (*ptr)
        //         .low_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        
        // if edge.contains(GpioPinEdge::GPIO_PE_FALLING) {
        //     (*ptr)
        //         .fall_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // } else {
        //     (*ptr)
        //         .fall_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // }

        // if edge.contains(GpioPinEdge::GPIO_PE_RISING) {
        //     (*ptr)
        //         .rise_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // } else {
        //     (*ptr)
        //         .rise_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // }

        // if edge.contains(GpioPinEdge::GPIO_PE_LOW) {
        //     (*ptr)
        //         .low_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // } else {
        //     (*ptr)
        //         .low_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // }

        // if edge.contains(GpioPinEdge::GPIO_PE_HIGH) {
        //     (*ptr)
        //         .high_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, true)));
        // } else {
        //     (*ptr)
        //         .high_ie
        //         .modify(|r, w| w.bits(set_bit(r.bits(), pin, false)));
        // }
    }
}
