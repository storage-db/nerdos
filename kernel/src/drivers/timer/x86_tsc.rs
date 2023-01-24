use raw_cpuid::CpuId;

use super::x86_hpet;
use crate::sync::LazyInit;
use crate::timer::MICROS_PER_SEC;
use crate::utils::ratio::Ratio;

static TSC_TO_NANOS_RATIO: LazyInit<Ratio> = LazyInit::new();
static NANOS_TO_TSC_RATIO: LazyInit<Ratio> = LazyInit::new();

pub fn current_ticks() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn ticks_to_nanos(ticks: u64) -> u64 {
    TSC_TO_NANOS_RATIO.mul(ticks)
}

#[allow(dead_code)]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    NANOS_TO_TSC_RATIO.mul(nanos)
}

pub(super) fn calibrate_tsc() {
    if let Some(freq) = CpuId::new()
        .get_processor_frequency_info()
        .map(|info| info.processor_base_frequency())
    {
        if freq > 0 {
            println!("Got TSC frequency by CPUID: {} MHz", freq);
            TSC_TO_NANOS_RATIO.init_by(Ratio::new(1_000, freq as u32));
            NANOS_TO_TSC_RATIO.init_by(TSC_TO_NANOS_RATIO.inverse());
            return;
        }
    }

    let mut best_freq_khz = u32::MAX;
    for _ in 0..5 {
        let tsc_start = current_ticks();
        let hpet_start = x86_hpet::current_ticks();
        x86_hpet::wait_millis(10);
        let tsc_end = current_ticks();
        let hpet_end = x86_hpet::current_ticks();

        let nanos = x86_hpet::ticks_to_nanos(hpet_end.wrapping_sub(hpet_start));
        let freq_khz = ((tsc_end - tsc_start) * MICROS_PER_SEC / nanos) as u32;

        if freq_khz < best_freq_khz {
            best_freq_khz = freq_khz;
        }
    }
    println!(
        "Calibrated TSC frequency: {}.{:03} MHz",
        best_freq_khz / 1_000,
        best_freq_khz % 1_000,
    );

    TSC_TO_NANOS_RATIO.init_by(Ratio::new(MICROS_PER_SEC as u32, best_freq_khz));
    NANOS_TO_TSC_RATIO.init_by(TSC_TO_NANOS_RATIO.inverse());
}
