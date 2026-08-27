//! CT32 match-interrupt timer (µs tick).
//!
//! Enables a 32-bit counter/timer with prescale so TC increments once per
//! microsecond. Match channels schedule IRQs; NVIC enable stays in the app.
//! Pin mux / PWM rework of CT16 is out of scope here.

use crate::raw::{CT32B0, SYSCON};

/// Match channel 0–3 (`MR0`–`MR3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MatchChannel {
    Mr0 = 0,
    Mr1 = 1,
    Mr2 = 2,
    Mr3 = 3,
}

impl MatchChannel {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn ir_mask(self) -> u32 {
        1 << (self as u8)
    }
}

/// Free-running CT32B0 with 1 µs ticks and per-channel match IRQs.
pub struct MatchTimer<'a> {
    timer: &'a CT32B0,
}

impl<'a> MatchTimer<'a> {
    /// Enable clock, set 1 µs prescale from `system_hz`, start free-running.
    pub fn new(syscon: &SYSCON, timer: &'a CT32B0, system_hz: u32) -> Self {
        enable_clock(syscon);

        let handle = MatchTimer { timer };
        // PR = clocks_per_tick - 1.
        let pr = system_hz.max(1_000_000) / 1_000_000 - 1;
        unsafe {
            handle.timer.pr.write(|w| w.pcval().bits(pr));
        }
        // Clear pending match flags.
        handle.timer.ir.write(|w| {
            w.mr0int()
                .set_bit()
                .mr1int()
                .set_bit()
                .mr2int()
                .set_bit()
                .mr3int()
                .set_bit()
        });
        // No reset/stop on match by default; IRQs armed per channel.
        handle.timer.mcr.write(|w| unsafe { w.bits(0) });
        handle.timer.tcr.write(|w| w.crst().reset());
        handle
            .timer
            .tcr
            .write(|w| w.crst().do_nothing().cen().the_timer_counter_an());
        handle
    }

    /// Wrap an already-configured CT32B0 (e.g. for ISR / `'static` steal).
    pub fn from_pac(timer: &'a CT32B0) -> Self {
        Self { timer }
    }

    /// `'static` handle via PAC ZST token (MMIO through `CT32B0::ptr()`).
    pub unsafe fn steal() -> MatchTimer<'static> {
        MatchTimer {
            timer: &*core::ptr::NonNull::<CT32B0>::dangling().as_ptr(),
        }
    }

    /// Current timer count (microseconds since start, wrapping).
    pub fn count(&self) -> u32 {
        self.timer.tc.read().tc().bits()
    }

    /// Program match register `channel` to `value`.
    pub fn set_match(&self, channel: MatchChannel, value: u32) {
        self.timer.mr[channel.index()].write(|w| unsafe { w.bits(value) });
    }

    /// Enable interrupt when TC matches `channel` (`MCR.MRnI`).
    pub fn enable_match_interrupt(&self, channel: MatchChannel) {
        match channel {
            MatchChannel::Mr0 => self.timer.mcr.modify(|_, w| w.mr0i().enabled()),
            MatchChannel::Mr1 => self.timer.mcr.modify(|_, w| w.mr1i().enabled()),
            MatchChannel::Mr2 => self.timer.mcr.modify(|_, w| w.mr2i().enabled()),
            MatchChannel::Mr3 => self.timer.mcr.modify(|_, w| w.mr3i().enabled()),
        }
    }

    /// Disable match interrupt for `channel`.
    pub fn disable_match_interrupt(&self, channel: MatchChannel) {
        match channel {
            MatchChannel::Mr0 => self.timer.mcr.modify(|_, w| w.mr0i().disabled()),
            MatchChannel::Mr1 => self.timer.mcr.modify(|_, w| w.mr1i().disabled()),
            MatchChannel::Mr2 => self.timer.mcr.modify(|_, w| w.mr2i().disabled()),
            MatchChannel::Mr3 => self.timer.mcr.modify(|_, w| w.mr3i().disabled()),
        }
    }

    /// True if match interrupt flag for `channel` is set.
    pub fn match_pending(&self, channel: MatchChannel) -> bool {
        self.timer.ir.read().bits() & channel.ir_mask() != 0
    }

    /// Clear match interrupt flag for `channel` (write-1-to-clear).
    pub fn clear_match(&self, channel: MatchChannel) {
        match channel {
            MatchChannel::Mr0 => self.timer.ir.write(|w| w.mr0int().set_bit()),
            MatchChannel::Mr1 => self.timer.ir.write(|w| w.mr1int().set_bit()),
            MatchChannel::Mr2 => self.timer.ir.write(|w| w.mr2int().set_bit()),
            MatchChannel::Mr3 => self.timer.ir.write(|w| w.mr3int().set_bit()),
        }
    }
}

/// Gate the CT32B0 AHB clock on.
pub fn enable_clock(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| w.ct32b0().enabled());
}
