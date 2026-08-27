//! LPC11Uxx ADC (10-bit, burst / software start).
//!
//! Pin mux stays in the board / app (IOCON FUNC + analog mode). This module
//! powers the block, sets the sample clock, enables channels, and reads results.
//!
//! Max conversion clock is 4.5 MHz; default sample rate is 400 kHz at 10 bits.

use crate::raw::{ADC, SYSCON};

/// Maximum recommended sample rate (Hz).
pub const MAX_SAMPLE_RATE_HZ: u32 = 400_000;

/// ADC channel 0–7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Channel {
    Ch0 = 0,
    Ch1 = 1,
    Ch2 = 2,
    Ch3 = 3,
    Ch4 = 4,
    Ch5 = 5,
    Ch6 = 6,
    Ch7 = 7,
}

impl Channel {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn mask(self) -> u8 {
        1 << (self as u8)
    }
}

/// ADC setup (rate, resolution, burst flag for divider math).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Desired samples per second (per channel in burst).
    pub sample_rate_hz: u32,
    /// `CLKS` field: 0 = 11 clocks / 10 bits … 7 = 4 clocks / 3 bits.
    pub clks: u8,
    /// When true, divider uses burst-mode conversion-clock count.
    pub burst: bool,
}

impl Config {
    /// 400 kHz, 10-bit, burst divider (Steam Controller recipe).
    pub const fn steam_controller() -> Self {
        Self {
            sample_rate_hz: MAX_SAMPLE_RATE_HZ,
            clks: 0,
            burst: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::steam_controller()
    }
}

/// ADC peripheral handle.
pub struct Adc<'a> {
    adc: &'a ADC,
    syscon: &'a SYSCON,
}

impl<'a> Adc<'a> {
    /// Power up ADC, enable AHB clock, apply [`Config`], leave burst off.
    pub fn new(syscon: &'a SYSCON, adc: &'a ADC, config: Config) -> Self {
        power_up(syscon);
        enable_clock(syscon);

        let handle = Adc { adc, syscon };
        handle.apply_config(config);
        // START must stay 000 for burst; clear channel interrupts.
        handle.adc.inten.write(|w| unsafe { w.bits(0) });
        handle
    }

    fn apply_config(&self, config: Config) {
        let div = clk_div(
            system_hz(self.syscon),
            config.burst,
            config.sample_rate_hz,
            clocks_per_conversion(config.clks),
        );
        // PAC omits CR.PDN (bit 21); set it with the other CR fields.
        const CR_PDN: u32 = 1 << 21;
        let cr = CR_PDN
            | (u32::from(div) << 8)
            | (u32::from(config.clks & 7) << 17);
        self.adc.cr.write(|w| unsafe { w.bits(cr) });
    }

    /// Bitmask of currently selected channels (`CR.SEL`).
    pub fn channel_mask(&self) -> u8 {
        self.adc.cr.read().sel().bits()
    }

    pub fn enable_channel(&self, channel: Channel) {
        let mask = self.channel_mask() | channel.mask();
        self.adc.cr.modify(|_, w| unsafe { w.sel().bits(mask) });
    }

    pub fn disable_channel(&self, channel: Channel) {
        // Clear START before clearing the SEL bit.
        self.adc
            .cr
            .modify(|_, w| w.start().no_start_this_value());
        let mask = self.channel_mask() & !channel.mask();
        self.adc.cr.modify(|_, w| unsafe { w.sel().bits(mask) });
    }

    /// Enable or disable hardware burst scan (`CR.BURST`). START is forced to 0.
    pub fn set_burst(&self, enable: bool) {
        self.adc
            .cr
            .modify(|_, w| w.start().no_start_this_value());
        if enable {
            self.adc.cr.modify(|_, w| w.burst().hardware_scan());
        } else {
            self.adc
                .cr
                .modify(|_, w| w.burst().software_controlled());
        }
    }

    /// Read a channel if `DONE` is set; clears DONE by reading `DR`.
    pub fn read(&self, channel: Channel) -> Option<u16> {
        let dr = self.adc.dr[channel.index()].read();
        if dr.done().bit_is_set() {
            Some(dr.v_vref().bits())
        } else {
            None
        }
    }

    /// Busy-wait until `channel` has `DONE`, then return the 10-bit result.
    pub fn read_blocking(&self, channel: Channel) -> u16 {
        loop {
            if let Some(v) = self.read(channel) {
                return v;
            }
        }
    }

    /// Burst-sample enabled channels `samples` times and return per-channel averages.
    ///
    /// Waits until every selected channel reports DONE in `STAT`, then reads `DR`.
    pub fn sample_averaged(&self, samples: u8) -> [u16; 8] {
        let mut acc = [0u32; 8];
        let mask = self.channel_mask();
        if mask == 0 || samples == 0 {
            return [0u16; 8];
        }

        // Discard stale DONE by reading all selected DRs.
        for i in 0..8u8 {
            if mask & (1 << i) != 0 {
                let _ = self.adc.dr[i as usize].read().bits();
            }
        }

        self.set_burst(true);
        for _ in 0..samples {
            while self.adc.stat.read().done().bits() & mask != mask {}
            for i in 0..8u8 {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let dr = self.adc.dr[i as usize].read();
                acc[i as usize] += u32::from(dr.v_vref().bits());
            }
        }
        self.set_burst(false);

        let n = u32::from(samples);
        let mut out = [0u16; 8];
        for i in 0..8 {
            out[i] = (acc[i] / n) as u16;
        }
        out
    }

    pub fn enable_clock(&self) {
        enable_clock(self.syscon);
    }

    pub fn disable_clock(&self) {
        disable_clock(self.syscon);
    }
}

/// Power the ADC analog block (`PDRUNCFG.ADC_PD` = powered).
pub fn power_up(syscon: &SYSCON) {
    syscon.pdruncfg.modify(|_, w| w.adc_pd().powered());
}

/// Gate the ADC AHB clock on.
pub fn enable_clock(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| w.adc().enabled());
}

/// Gate the ADC AHB clock off.
pub fn disable_clock(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| w.adc().disabled());
}

fn clocks_per_conversion(clks_field: u8) -> u8 {
    // CLKS 0 → 11 clocks (10-bit) … CLKS 7 → 4 clocks (3-bit).
    11u8.saturating_sub(clks_field.min(7))
}

/// Round `((sys_hz * 2 + full) / (full * 2)) - 1`, clamped to u8.
fn clk_div(sys_hz: u32, burst: bool, adc_rate: u32, conv_clks: u8) -> u8 {
    let full = if burst {
        adc_rate.saturating_mul(u32::from(conv_clks))
    } else {
        adc_rate.saturating_mul(11)
    };
    if full == 0 {
        return 0;
    }
    let div = ((sys_hz * 2 + full) / (full * 2)).saturating_sub(1);
    if div > 0xff {
        0xff
    } else {
        div as u8
    }
}

fn system_hz(syscon: &SYSCON) -> u32 {
    crate::clock::system_hz(syscon)
}
