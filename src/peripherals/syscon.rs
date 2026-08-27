//! API for system configuration (SYSCON) — always on.
//!
//! This module mostly provides clock-enable infrastructure used by other HAL
//! peripherals. Only a subset of SYSCON functionality is implemented.

crate::wrap_always_on_peripheral!(Syscon, SYSCON);

impl Syscon {
    pub fn device_id(&self) -> u32 {
        self.raw.device_id.read().deviceid().bits()
    }

    /// Enables the clock for a peripheral or other hardware component.
    pub fn enable_clock<P: ClockControl>(&mut self, peripheral: &mut P) {
        peripheral.enable_clock(self);
    }

    /// Disable peripheral clock.
    pub fn disable_clock<P: ClockControl>(&mut self, peripheral: &mut P) {
        peripheral.disable_clock(self);
    }

    /// Check if peripheral clock is enabled.
    pub fn is_clock_enabled<P: ClockControl>(&self, peripheral: &P) -> bool {
        peripheral.is_clock_enabled(self)
    }
}

/// Internal trait for controlling peripheral clocks.
///
/// This trait is an internal implementation detail and should neither be
/// implemented nor used outside of this crate.
pub trait ClockControl {
    fn enable_clock(&self, s: &mut Syscon);
    fn disable_clock(&self, s: &mut Syscon);
    fn is_clock_enabled(&self, s: &Syscon) -> bool;
}

macro_rules! impl_clock_control {
    ($clock_control:ty, $clock:ident) => {
        impl ClockControl for $clock_control {
            fn enable_clock(&self, s: &mut Syscon) {
                s.raw.sysahbclkctrl.modify(|_, w| w.$clock().enabled());
                while s.raw.sysahbclkctrl.read().$clock().is_disabled() {}
            }

            fn disable_clock(&self, s: &mut Syscon) {
                s.raw.sysahbclkctrl.modify(|_, w| w.$clock().disabled());
            }

            fn is_clock_enabled(&self, s: &Syscon) -> bool {
                s.raw.sysahbclkctrl.read().$clock().is_enabled()
            }
        }
    };
}

impl_clock_control!(raw::IOCON, iocon);
impl_clock_control!(raw::GPIO_PORT, gpio);
impl_clock_control!(raw::CT32B0, ct32b0);
impl_clock_control!(raw::CT32B1, ct32b1);
impl_clock_control!(raw::CT16B0, ct16b0);
impl_clock_control!(raw::CT16B1, ct16b1);
