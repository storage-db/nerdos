use x2apic::lapic::{TimerDivide, TimerMode};

use super::{x86_hpet, x86_tsc};
use crate::drivers::interrupt::local_apic;
use crate::sync::LazyInit;
use crate::timer::{current_time_nanos, NANOS_PER_SEC};
use crate::utils::ratio::Ratio;

pub use x86_tsc::{current_ticks, nanos_to_ticks, ticks_to_nanos}; // use TSC as the clock source.

static NANOS_TO_LAPIC_TICKS_RATIO: LazyInit<Ratio> = LazyInit::new();

pub fn set_oneshot_timer(deadline_ns: u64) {
    let now_ns = current_time_nanos();
    unsafe {
        if now_ns < deadline_ns {
            let apic_ticks = NANOS_TO_LAPIC_TICKS_RATIO.mul(deadline_ns - now_ns);
            assert!(apic_ticks <= u32::MAX as u64);
            local_apic().set_timer_initial(apic_ticks.max(1) as u32);
        } else {
            local_apic().set_timer_initial(1);
        }
    }
}

fn calibrate_lapic_timer() {
    let lapic = local_apic();
    unsafe {
        lapic.set_timer_mode(TimerMode::OneShot);
        lapic.set_timer_divide(TimerDivide::Div256); // divide 1
    }

    let mut best_freq_hz = 0;
    for _ in 0..5 {
        unsafe { lapic.set_timer_initial(u32::MAX) };
        let hpet_start = x86_hpet::current_ticks();
        x86_hpet::wait_millis(10);
        let ticks = u32::MAX - unsafe { lapic.timer_current() };
        let hpet_end = x86_hpet::current_ticks();

        let nanos = x86_hpet::ticks_to_nanos(hpet_end.wrapping_sub(hpet_start));
        let ticks_per_sec = (ticks as u64 * NANOS_PER_SEC / nanos) as u32;
        // pick the max frequency to avoid early alarm when call `set_onshot_timer()`.
        if ticks_per_sec > best_freq_hz {
            best_freq_hz = ticks_per_sec;
        }
    }
    println!(
        "Calibrated LAPIC frequency: {}.{:03} MHz",
        best_freq_hz / 1_000_000,
        best_freq_hz % 1_000_000 / 1_000,
    );

    NANOS_TO_LAPIC_TICKS_RATIO.init_by(Ratio::new(best_freq_hz, NANOS_PER_SEC as u32));
    unsafe { lapic.enable_timer() } // enable APIC timer IRQ
}

pub fn init() {
    super::x86_hpet::init();
    x86_tsc::calibrate_tsc();
    calibrate_lapic_timer();
}
