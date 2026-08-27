//! GPIO pin interrupts (PINT).
//!
//! Maps a `(port, pin)` to one of eight pin-interrupt channels, configures
//! edge detect, and clears status. NVIC enable stays in the application.

use crate::raw::{GPIO_PIN_INT, SYSCON};

/// Pin-interrupt channel 0–7 (`PIN_INT0`–`PIN_INT7`).
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

    pub const fn mask(self) -> u32 {
        1 << (self as u8)
    }
}

/// Thin handle around `GPIO_PIN_INT` + `SYSCON.PINTSEL`.
pub struct Pint<'a> {
    pint: &'a GPIO_PIN_INT,
    syscon: &'a SYSCON,
}

impl<'a> Pint<'a> {
    /// Enable the PINT AHB clock.
    pub fn new(syscon: &'a SYSCON, pint: &'a GPIO_PIN_INT) -> Self {
        enable_clock(syscon);
        Self { pint, syscon }
    }

    /// `'static` handle via PAC ZST tokens.
    pub unsafe fn steal() -> Pint<'static> {
        Pint {
            pint: &*core::ptr::NonNull::<GPIO_PIN_INT>::dangling().as_ptr(),
            syscon: &*core::ptr::NonNull::<SYSCON>::dangling().as_ptr(),
        }
    }

    /// Map `channel` to GPIO `(port, pin)` via `PINTSEL`.
    ///
    /// LPC11Uxx encoding: `port * 24 + pin` (PIO0_0–23, PIO1_0–31).
    pub fn select(&self, channel: Channel, port: u8, pin: u8) {
        let sel = u16::from(port) * 24 + u16::from(pin);
        let intpin = if sel > 0xff { 0 } else { sel as u8 };
        self.syscon.pintsel[channel.index()].write(|w| unsafe { w.intpin().bits(intpin) });
    }

    /// Edge-sensitive mode for `channel` (`ISEL` bit = 0).
    pub fn set_edge_mode(&self, channel: Channel) {
        let mask = channel.mask();
        self.pint.isel.modify(|r, w| unsafe { w.bits(r.bits() & !mask) });
    }

    /// Enable rising-edge interrupt (`SIENR`).
    pub fn enable_rising(&self, channel: Channel) {
        self.pint.sienr.write(|w| unsafe { w.bits(channel.mask()) });
    }

    /// Disable rising-edge interrupt (`CIENR`).
    pub fn disable_rising(&self, channel: Channel) {
        self.pint.cienr.write(|w| unsafe { w.bits(channel.mask()) });
    }

    /// Enable falling-edge interrupt (`SIENF`).
    pub fn enable_falling(&self, channel: Channel) {
        self.pint.sienf.write(|w| unsafe { w.bits(channel.mask()) });
    }

    /// Disable falling-edge interrupt (`CIENF`).
    pub fn disable_falling(&self, channel: Channel) {
        self.pint.cienf.write(|w| unsafe { w.bits(channel.mask()) });
    }

    /// True if `channel` interrupt is pending (`IST`).
    pub fn pending(&self, channel: Channel) -> bool {
        self.pint.ist.read().bits() & channel.mask() != 0
    }

    /// Clear pending status for `channel` (write-1-to-clear `IST`).
    pub fn clear(&self, channel: Channel) {
        self.pint.ist.write(|w| unsafe { w.bits(channel.mask()) });
    }
}

/// Gate the GPIO pin-interrupt register interface clock (`SYSAHBCLKCTRL.PINT`).
pub fn enable_clock(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| w.pint().enabled());
}
