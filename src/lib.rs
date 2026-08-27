#![no_std]

//! Hardware Abstraction Layer for NXP LPC11Uxx (Cortex-M0) microcontrollers.
//!
//! Provides clock setup, GPIO, IOCON, SYSCON, USART0, USB clock enable, CT16B1 PWM, and SysTick delays.

pub extern crate lpc11uxx as raw;

pub mod clock;
pub mod delay;
pub mod macros;
pub mod prelude;
pub mod pwm;
pub mod serial;
pub mod time;
pub mod traits;
pub mod typestates;
pub mod usb;

pub use typestates::init_state::Enabled;

pub mod peripherals;
pub use peripherals::{ct16b1::Ct16b1, gpio::Gpio, iocon::Iocon, syscon::Syscon};
pub use peripherals::iocon::{configure as iocon_configure, Mode as IoconMode, PinConfig};
pub use pwm::{Pwm, PwmChannel, PwmConfig};
pub use serial::{Config as SerialConfig, Error as SerialError, Rx as SerialRx, Serial, Tx as SerialTx};

pub fn new() -> Peripherals {
    take().unwrap()
}

pub fn take() -> Option<Peripherals> {
    Some(Peripherals::from((
        raw::Peripherals::take()?,
        raw::CorePeripherals::take()?,
    )))
}

pub fn from(raw: (raw::Peripherals, raw::CorePeripherals)) -> Peripherals {
    Peripherals::from(raw)
}

/// Entry point to the HAL API.
#[allow(non_snake_case)]
pub struct Peripherals {
    pub gpio: Gpio,
    pub iocon: Iocon,
    pub syscon: Syscon,

    pub MPU: raw::MPU,
    pub NVIC: raw::NVIC,
    pub SCB: raw::SCB,
    pub SYST: raw::SYST,
}

impl From<(raw::Peripherals, raw::CorePeripherals)> for Peripherals {
    fn from(raw: (raw::Peripherals, raw::CorePeripherals)) -> Self {
        let cp = raw.1;
        let p = raw.0;
        Peripherals {
            gpio: Gpio::from(p.GPIO_PORT),
            iocon: Iocon::from(p.IOCON),
            syscon: Syscon::from(p.SYSCON),
            MPU: cp.MPU,
            NVIC: cp.NVIC,
            SCB: cp.SCB,
            SYST: cp.SYST,
        }
    }
}

impl Peripherals {
    pub fn take() -> Option<Self> {
        Some(Self::from((
            raw::Peripherals::take()?,
            raw::CorePeripherals::take()?,
        )))
    }

    pub unsafe fn steal() -> Self {
        Self::from((raw::Peripherals::steal(), raw::CorePeripherals::steal()))
    }
}
