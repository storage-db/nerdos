use riscv::register::time;

use super::super::interrupt::{self, ScauseIntCode};
use super::super::misc::sbi;

const NANOS_PER_TICK: u64 = crate::timer::NANOS_PER_SEC / crate::config::TIMER_FREQUENCY as u64;

pub fn current_ticks() -> u64 {
    time::read() as u64
}

pub fn nanos_to_ticks(nanos: u64) -> u64 {
    nanos / NANOS_PER_TICK
}

pub fn ticks_to_nanos(ticks: u64) -> u64 {
    ticks * NANOS_PER_TICK
}

pub fn set_oneshot_timer(deadline_ns: u64) {
    sbi::set_timer(nanos_to_ticks(deadline_ns));
}

pub fn init() {
    interrupt::register_handler(ScauseIntCode::Timer as _, crate::timer::handle_timer_irq);
    interrupt::set_enable(ScauseIntCode::Timer as _, true);
}
