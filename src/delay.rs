//! Delays based on the current CPU clock frequency.
//!
//! [`Delay`] uses SysTick. [`spin_ms`] / [`spin_us`] use a calibrated busy loop,
//! which can be more reliable when SysTick was left in an unknown state by a
//! prior boot stage.

use cortex_m::peripheral::syst::SystClkSource;
use embedded_hal::delay::DelayNs;
use crate::raw::SYST;

pub struct Delay {
    syst: SYST,
    clock_hz: u32,
}

impl Delay {
    pub fn new(mut syst: SYST, clock_hz: u32) -> Self {
        syst.disable_counter();
        syst.clear_current();
        syst.set_clock_source(SystClkSource::Core);
        Delay {
            syst,
            clock_hz: clock_hz.max(1),
        }
    }

    pub fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }

    pub fn delay_us(&mut self, us: u32) {
        let clock_mhz = clock_mhz(self.clock_hz);
        let max_us = (1 << 24) / clock_mhz / 1_000;
        let mut remaining = us;

        while remaining > max_us {
            self.delay_us_inner(max_us, clock_mhz);
            remaining -= max_us;
        }
        if remaining > 0 {
            self.delay_us_inner(remaining, clock_mhz);
        }
    }

    fn delay_us_inner(&mut self, us: u32, clock_mhz: u32) {
        let reload = (u32::from(us).saturating_mul(clock_mhz)).clamp(1, (1 << 24) - 1);

        self.syst.set_reload(reload);
        self.syst.clear_current();
        self.syst.enable_counter();

        while !self.syst.has_wrapped() {
            cortex_m::asm::nop();
        }

        self.syst.disable_counter();
    }
}

impl DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        self.delay_us((ns + 999) / 1_000);
    }
}

/// Busy-loop delay calibrated with [`crate::clock::system_hz`].
pub fn spin_ms(clock_hz: u32, ms: u32) {
    spin_cycles(clock_hz.saturating_mul(ms) / 1_000);
}

/// Busy-loop delay in microseconds.
pub fn spin_us(clock_hz: u32, us: u32) {
    spin_cycles(clock_hz.saturating_mul(us) / 1_000_000);
}

fn spin_cycles(mut cycles: u32) {
    while cycles > 0 {
        cortex_m::asm::nop();
        cycles -= 1;
    }
}

fn clock_mhz(clock_hz: u32) -> u32 {
    ((clock_hz + 500_000) / 1_000_000).max(1)
}
