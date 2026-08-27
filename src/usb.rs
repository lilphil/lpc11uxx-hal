//! USB device clock enable for use with an external stack (e.g. `lpc11uxx-usbd`).
//!
//! Call [`enable`] after the system clock is running (12 MHz crystal → 48 MHz
//! PLL or equivalent). This turns on the USB PLL, transceiver pad, and the
//! AHB clocks for the USB peripheral and USBRAM — enough for
//! `lpc11uxx_usbd::bus::UsbBus::new(...)` without duplicating SYSCON setup.
//!
//! **Out of scope here (keep in the board / app crate):**
//! - PIO0_3 VBUS sense pull and PIO0_6 `USB_CONNECT` SoftConnect policy
//! - ROM USBD stack (OpenSteamController bootloader path)
//! - HID/CDC descriptors and endpoint layout
//!
//! Example (board crate):
//!
//! ```ignore
//! use lpc11uxx_hal::{
//!     peripherals::iocon::{self, Mode, PinConfig},
//!     usb,
//! };
//!
//! usb::enable(&syscon);
//! iocon::configure(&iocon, 0, 3, PinConfig::func(0).mode(Mode::PullDown));
//! iocon::configure(&iocon, 0, 6, PinConfig::func(1)); // USB_CONNECT
//! let usb_bus = lpc11uxx_usbd::bus::UsbBus::new(usb_periph);
//! ```

use crate::raw::SYSCON;

/// Enable USB PLL, pad power, and AHB clocks for USB + USBRAM.
///
/// Does not configure SoftConnect / VBUS pins or construct a `UsbBus`.
#[inline]
pub fn enable(syscon: &SYSCON) {
    crate::clock::enable_usb(syscon);
}

/// Alias for [`enable`] — clocks and AHB are ready for `lpc11uxx-usbd`.
#[inline]
pub fn ready(syscon: &SYSCON) {
    enable(syscon);
}
