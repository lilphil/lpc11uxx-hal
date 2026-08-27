//! SSP0 SPI master (blocking).
//!
//! Pin mux and chip-select GPIOs stay in the board / app. This module clocks
//! SSP0, sets frame format / bit rate, and transfers bytes.

use core::convert::Infallible;

use embedded_hal::spi::{ErrorType, Mode, Phase, Polarity, SpiBus};

use crate::raw::{SSP0, SYSCON};

/// Steam Controller Cirque trackpad SPI: Mode 1, 8-bit, 6 MHz @ 48 MHz PCLK.
pub const STEAM_CONTROLLER_BITRATE_HZ: u32 = 6_000_000;

/// SSP0 configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// SPI mode (CPOL/CPHA).
    pub mode: Mode,
    /// Target SCK frequency in Hz.
    pub bitrate_hz: u32,
    /// `SYSCON.SSP0CLKDIV` (1 = SSP clock equals main clock).
    pub sspclkdiv: u8,
}

impl Config {
    /// Mode 1 @ 6 MHz — OpenSteamController Cirque recipe.
    pub const fn steam_controller() -> Self {
        Self {
            mode: Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnSecondTransition,
            },
            bitrate_hz: STEAM_CONTROLLER_BITRATE_HZ,
            sspclkdiv: 1,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::steam_controller()
    }
}

/// Blocking SSP0 master handle.
pub struct Ssp<'a> {
    ssp: &'a SSP0,
}

impl<'a> Ssp<'a> {
    /// Enable AHB + SSP clocks, reset the block, apply [`Config`], enable master.
    pub fn new(syscon: &SYSCON, ssp: &'a SSP0, config: Config) -> Self {
        enable_clock(syscon);
        syscon
            .ssp0clkdiv
            .write(|w| unsafe { w.div().bits(config.sspclkdiv.max(1)) });
        // Pulse SSP0 reset (active-low).
        syscon
            .presetctrl
            .modify(|_, w| w.ssp0_rst_n().resets_the_ssp0_peri());
        syscon
            .presetctrl
            .modify(|_, w| w.ssp0_rst_n().ssp0_reset_de_assert());

        let handle = Ssp { ssp };
        handle.apply_config(syscon, config);
        handle.enable();
        handle
    }

    /// Wrap an already-configured SSP0 (e.g. ISR / `'static` steal).
    pub fn from_pac(ssp: &'a SSP0) -> Self {
        Self { ssp }
    }

    /// `'static` handle via PAC ZST token (MMIO through `SSP0::ptr()`).
    pub unsafe fn steal() -> Ssp<'static> {
        Ssp {
            ssp: &*core::ptr::NonNull::<SSP0>::dangling().as_ptr(),
        }
    }

    fn apply_config(&self, syscon: &SYSCON, config: Config) {
        // Disable while changing format / rate.
        self.ssp.cr1.modify(|_, w| w.sse().disabled());

        let cpol_high = config.mode.polarity == Polarity::IdleHigh;
        let cpha_second = config.mode.phase == Phase::CaptureOnSecondTransition;
        let (scr, cpsr) = clock_dividers(ssp_pclk_hz(syscon), config.bitrate_hz);

        self.ssp.cr0.write(|w| {
            w.dss()
                ._8_bit_transfer()
                .frf()
                .spi();
            if cpol_high {
                w.cpol().high();
            } else {
                w.cpol().low();
            }
            if cpha_second {
                w.cpha().secondclock();
            } else {
                w.cpha().firstclock();
            }
            unsafe { w.scr().bits(scr) }
        });
        self.ssp.cpsr.write(|w| unsafe { w.cpsdvsr().bits(cpsr) });
        self.ssp.cr1.write(|w| w.ms().master().sse().disabled());
    }

    /// Enable the SSP controller (`CR1.SSE`).
    pub fn enable(&self) {
        self.ssp.cr1.modify(|_, w| w.sse().enabled());
    }

    /// Disable the SSP controller.
    pub fn disable(&self) {
        self.ssp.cr1.modify(|_, w| w.sse().disabled());
    }

    /// Discard any leftover RX FIFO words.
    pub fn flush_rx(&self) {
        while self.ssp.sr.read().rne().bit_is_set() {
            let _ = self.ssp.dr.read().bits();
        }
    }

    /// Full-duplex transfer: write `tx`, fill `rx` (same length).
    pub fn transfer(&self, tx: &[u8], rx: &mut [u8]) {
        assert_eq!(tx.len(), rx.len());
        self.flush_rx();
        for (t, r) in tx.iter().zip(rx.iter_mut()) {
            while self.ssp.sr.read().tnf().bit_is_clear() {}
            self.ssp.dr.write(|w| unsafe { w.data().bits(u16::from(*t)) });
            while self.ssp.sr.read().rne().bit_is_clear() {}
            *r = self.ssp.dr.read().data().bits() as u8;
        }
    }

    /// Full-duplex in place (MOSI from `buf`, MISO overwrites `buf`).
    pub fn transfer_in_place(&self, buf: &mut [u8]) {
        self.flush_rx();
        for b in buf.iter_mut() {
            while self.ssp.sr.read().tnf().bit_is_clear() {}
            self.ssp.dr.write(|w| unsafe { w.data().bits(u16::from(*b)) });
            while self.ssp.sr.read().rne().bit_is_clear() {}
            *b = self.ssp.dr.read().data().bits() as u8;
        }
    }

    /// Write `words`, discarding MISO.
    pub fn write(&self, words: &[u8]) {
        self.flush_rx();
        for &b in words {
            while self.ssp.sr.read().tnf().bit_is_clear() {}
            self.ssp.dr.write(|w| unsafe { w.data().bits(u16::from(b)) });
            while self.ssp.sr.read().rne().bit_is_clear() {}
            let _ = self.ssp.dr.read().bits();
        }
    }

    /// Clock out `0xFF` and fill `words` from MISO.
    pub fn read(&self, words: &mut [u8]) {
        self.flush_rx();
        for b in words.iter_mut() {
            while self.ssp.sr.read().tnf().bit_is_clear() {}
            self.ssp
                .dr
                .write(|w| unsafe { w.data().bits(0x00FF) });
            while self.ssp.sr.read().rne().bit_is_clear() {}
            *b = self.ssp.dr.read().data().bits() as u8;
        }
    }
}

impl ErrorType for Ssp<'_> {
    type Error = Infallible;
}

impl SpiBus<u8> for Ssp<'_> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        Ssp::read(self, words);
        Ok(())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        Ssp::write(self, words);
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        Ssp::transfer(self, write, read);
        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        Ssp::transfer_in_place(self, words);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        while self.ssp.sr.read().bsy().bit_is_set() {}
        Ok(())
    }
}

/// Gate the SSP0 AHB clock on.
pub fn enable_clock(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| w.ssp0().enabled());
}

fn ssp_pclk_hz(syscon: &SYSCON) -> u32 {
    let div = u32::from(syscon.ssp0clkdiv.read().div().bits()).max(1);
    crate::clock::system_hz(syscon) / div
}

/// Pick SCR (0–255) and even CPSDVSR (2–254) for ≈ `bitrate_hz`.
fn clock_dividers(pclk_hz: u32, bitrate_hz: u32) -> (u8, u8) {
    let target = bitrate_hz.max(1);
    let mut best_scr = 0u8;
    let mut best_cpsr = 2u8;
    let mut best_err = u32::MAX;

    let mut cpsr = 2u16;
    while cpsr <= 254 {
        // bitrate = pclk / (cpsr * (scr+1)) → scr+1 = pclk / (cpsr * bitrate)
        let denom = u32::from(cpsr).saturating_mul(target);
        if denom == 0 {
            cpsr += 2;
            continue;
        }
        let scr_p1 = ((pclk_hz + denom / 2) / denom).clamp(1, 256);
        let scr = (scr_p1 - 1) as u8;
        let actual = pclk_hz / (u32::from(cpsr) * scr_p1);
        let err = actual.abs_diff(target);
        if err < best_err {
            best_err = err;
            best_scr = scr;
            best_cpsr = cpsr as u8;
            if err == 0 {
                break;
            }
        }
        cpsr += 2;
    }
    (best_scr, best_cpsr)
}
