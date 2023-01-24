use bit_field::BitField;
use bitflags::bitflags;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

use crate::mm::{PhysAddr, VirtAddr};
use crate::sync::LazyInit;
use crate::timer::NANOS_PER_SEC;
use crate::utils::ratio::Ratio;

const HPET_BASE: PhysAddr = PhysAddr::new(0xFED0_0000);

static HPET: LazyInit<Hpet> = LazyInit::new();

bitflags! {
    struct TimerConfCaps: u64 {
        /// 0 - this timer generates edge-triggered interrupts. 1 - this timer
        /// generates level-triggered interrupts.
        const TN_INT_TYPE_CNF = 1 << 1;
        /// Setting this bit to 1 enables triggering of interrupts.
        const TN_INT_ENB_CNF =  1 << 2;
        /// If Tn_PER_INT_CAP is 1, then writing 1 to this field enables periodic
        /// timer.
        const TN_TYPE_CNF =     1 << 3;
        /// If this read-only bit is set to 1, this timer supports periodic mode.
        const TN_PER_INT_CAP =  1 << 4;
        /// If this read-only bit is set to 1, the size of the timer is 64-bit.
        const TN_SIZE_CAP =     1 << 5;
        /// This field is used to allow software to directly set periodic timer's
        /// accumulator.
        const TN_VAL_SET_CNF =  1 << 6;
        /// For 64-bit timer, if this field is set, the timer will be forced to
        /// work in 32-bit mode.
        const TN_32MODE_CNF =   1 << 8;
    }
}

register_structs! {
    HpetRegs {
        /// General Capabilities and ID Register.
        (0x000 => general_caps: ReadOnly<u64>),
        (0x008 => _reserved_0),
        /// General Configuration Register.
        (0x010 => general_config: ReadWrite<u64>),
        (0x018 => _reserved_1),
        /// General Interrupt Status Register.
        (0x020 => general_int_status: ReadWrite<u64>),
        (0x028 => _reserved_2),
        /// Main Counter Value Register.
        (0x0f0 => main_counter_value: ReadWrite<u64>),
        (0x100 => @END),
    }
}

register_structs! {
    HpetTimerRegs {
        /// Timer N Configuration and Capability Register.
        (0x0 => conf_caps: ReadWrite<u64>),
        /// Timer N Comparator Value Register.
        (0x8 => comparator_value: ReadWrite<u64>),
        /// Timer N FSB Interrupt Route Register.
        (0x10 => fsb_int_route: ReadWrite<u64>),
        (0x20 => @END),
    }
}

struct Hpet {
    base_vaddr: VirtAddr,
    num_timers: u8,
    period_fs: u64,
    nanos_to_ticks_ratio: Ratio,
    ticks_to_nanos_ratio: Ratio,
    ticks_per_ms: u64,
    is_64bit: bool,
}

impl Hpet {
    const fn new(base_vaddr: VirtAddr) -> Self {
        Self {
            base_vaddr,
            num_timers: 0,
            period_fs: 0,
            ticks_to_nanos_ratio: Ratio::zero(),
            nanos_to_ticks_ratio: Ratio::zero(),
            ticks_per_ms: 0,
            is_64bit: false,
        }
    }

    const fn regs(&self) -> &HpetRegs {
        unsafe { &*(self.base_vaddr.as_ptr() as *const _) }
    }

    const fn timer_regs(&self, n: u8) -> &HpetTimerRegs {
        assert!(n < self.num_timers);
        unsafe { &*((self.base_vaddr.as_usize() + 0x100 + n as usize * 0x20) as *const _) }
    }

    fn init(&mut self) {
        println!("Initializing HPET...");
        let cap = self.regs().general_caps.get();
        let num_timers = cap.get_bits(8..=12) as u8 + 1;
        let period_fs = cap.get_bits(32..);
        let is_64bit = cap.get_bit(13);
        let has_legacy_replacement = cap.get_bit(15);
        let freq_hz = 1_000_000_000_000_000 / period_fs;
        info!("HPET capabilities: {:#x}", cap);
        assert!(has_legacy_replacement);
        println!(
            "HPET: {}.{:06} MHz, {}-bit, {} timers",
            freq_hz / 1_000_000,
            freq_hz % 1_000_000,
            if is_64bit { 64 } else { 32 },
            num_timers,
        );

        self.num_timers = num_timers;
        self.period_fs = period_fs;
        self.nanos_to_ticks_ratio = Ratio::new(freq_hz as u32, NANOS_PER_SEC as u32);
        self.ticks_to_nanos_ratio = self.nanos_to_ticks_ratio.inverse();
        self.ticks_per_ms = freq_hz / 1000;
        self.is_64bit = is_64bit;

        self.set_enable(false);
        for i in 0..num_timers {
            // disable all timers
            let conf_caps =
                unsafe { TimerConfCaps::from_bits_unchecked(self.timer_regs(i).conf_caps.get()) };
            self.timer_regs(i)
                .conf_caps
                .set((conf_caps - TimerConfCaps::TN_INT_ENB_CNF).bits());
        }
        self.set_enable(true);
    }

    fn set_enable(&mut self, enable: bool) {
        const LEG_RT_CNF: u64 = 1 << 1; // Legacy replacement mapping will disable PIT IRQs
        const ENABLE_CNF: u64 = 1 << 0;
        let config = &self.regs().general_config;
        if enable {
            config.set(LEG_RT_CNF | ENABLE_CNF);
        } else {
            config.set(0);
        }
    }

    #[allow(dead_code)]
    fn set_periodic_timer(&mut self, n: u8, period_nanos: u64) {
        let timer_regs = self.timer_regs(n);
        let mut conf_caps =
            unsafe { TimerConfCaps::from_bits_unchecked(timer_regs.conf_caps.get()) };
        assert!(conf_caps.contains(TimerConfCaps::TN_PER_INT_CAP));

        let ticks = self.nanos_to_ticks_ratio.mul(period_nanos);
        conf_caps |= TimerConfCaps::TN_INT_ENB_CNF
            | TimerConfCaps::TN_TYPE_CNF
            | TimerConfCaps::TN_VAL_SET_CNF;
        timer_regs.conf_caps.set(conf_caps.bits());
        timer_regs
            .comparator_value
            .set(self.regs().main_counter_value.get() + ticks);
        timer_regs.comparator_value.set(ticks);
    }

    fn wait_millis(&self, millis: u64) {
        let main_counter_value = &self.regs().main_counter_value;
        let ticks = millis * self.ticks_per_ms;
        let init = main_counter_value.get();
        while main_counter_value.get().wrapping_sub(init) < ticks {}
    }
}

#[allow(dead_code)]
pub fn current_ticks() -> u64 {
    // TODO: deal with overflow for 32-bit HPET.
    HPET.regs().main_counter_value.get()
}

#[allow(dead_code)]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    HPET.ticks_to_nanos_ratio.mul(ticks)
}

#[allow(dead_code)]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    HPET.nanos_to_ticks_ratio.mul(nanos)
}

pub(super) fn wait_millis(millis: u64) {
    HPET.wait_millis(millis);
}

pub(super) fn init() {
    let mut hpet = Hpet::new(HPET_BASE.into_kvaddr());
    hpet.init();
    HPET.init_by(hpet);
}
