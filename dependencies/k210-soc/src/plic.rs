#![allow(non_camel_case_types)]

use super::utils::set_bit;
use k210_pac as pac;
use pac::Interrupt;


pub fn plic_enable(source: Interrupt) {
    unsafe {
        let idx = source as usize;
        let ptr = pac::PLIC::ptr();
        (*ptr).target_enables[0].enable[idx / 32]
            .modify(|r, w| w.bits(set_bit(r.bits(), idx as u8 % 32, true)));
    }
}

pub fn set_priority(pin: Interrupt, value: u32) {
    unsafe {
        let ptr = pac::PLIC::ptr();
        (*ptr).priority[pin as usize].write(|w| w.bits(value))
    }
}

pub fn set_thershold(value: u32, hartid: usize) {
    unsafe {
        let ptr = pac::PLIC::ptr();
        (*ptr).targets[hartid].threshold.write(|w| w.bits(value));
    }
}

pub fn current_irq(hartid: usize) -> usize {
    unsafe {
        let ptr = pac::PLIC::ptr();
        (*ptr).targets[hartid].claim.read().bits() as usize
    }
}

pub fn clear_irq(irq: usize, hartid: usize) {
    unsafe {
        let ptr = pac::PLIC::ptr();
        (*ptr).targets[hartid].claim.write(|w|w.bits(irq as u32));
    }
}