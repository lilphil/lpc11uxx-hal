use crate::{
    peripherals::syscon,
    raw,
    typestates::init_state,
};

crate::wrap_stateful_peripheral!(Iocon, IOCON);

impl<State> Iocon<State> {
    pub fn enabled(mut self, syscon: &mut syscon::Syscon) -> Iocon<init_state::Enabled> {
        syscon.enable_clock(&mut self.raw);

        Iocon {
            raw: self.raw,
            _state: init_state::Enabled(()),
        }
    }

    pub fn disabled(mut self, syscon: &mut syscon::Syscon) -> Iocon<init_state::Disabled> {
        syscon.disable_clock(&mut self.raw);

        Iocon {
            raw: self.raw,
            _state: init_state::Disabled,
        }
    }
}

impl Iocon<init_state::Enabled> {
    pub fn as_pac(&self) -> &raw::IOCON {
        &self.raw
    }

    /// Configure a pin via `(port, pin)`. See [`configure`].
    pub fn configure(&self, port: u8, pin: u8, config: PinConfig) {
        configure(&self.raw, port, pin, config)
    }

    /// Write a raw `modefunc` word (OpenSC `Chip_IOCON_PinMuxSet` style).
    pub fn pin_mux_set(&self, port: u8, pin: u8, modefunc: u32) {
        pin_mux_set(&self.raw, port, pin, modefunc)
    }
}

/// Pull resistor selection (IOCON `MODE`, bits 4:3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    /// Inactive — no pull-up / pull-down.
    Inactive = 0,
    PullDown = 1,
    PullUp = 2,
    Repeater = 3,
}

/// Raw IOCON pin settings: `FUNC` / `MODE` / `HYS` / `DIGIMODE`.
///
/// Packs the same fields as NXP `Chip_IOCON_PinMuxSet` / `Chip_IOCON_PinMux`.
/// Function numbers are pin-specific; see the LPC11Uxx user manual.
///
/// Common Steam Controller muxes (board code still chooses the `(port, pin)`):
///
/// | Pin | Func | Use |
/// |-----|------|-----|
/// | PIO0_21 | 1 | CT16B1_MAT0 (status LED PWM) |
/// | PIO1_17 | 2 | RXD (USART) |
/// | PIO1_18 | 2 | TXD (USART) |
/// | PIO0_6 | 1 | USB_CONNECT |
/// | PIO0_3 | 0 + pull-down | VBUS sense |
/// | PIO0_8/9, PIO1_29 | 1 | SSP0 MISO/MOSI/SCK |
/// | PIO0_11/12/13/14/22 | 2 or 1 | ADC channels |
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinConfig {
    pub func: u8,
    pub mode: Mode,
    pub hys: bool,
    /// Digital mode (bit 7). Meaningful on ADC-capable pins; ignored elsewhere.
    pub digimode: bool,
}

impl PinConfig {
    /// `FUNC = func`, inactive pulls, hysteresis off, digital mode off.
    pub const fn func(func: u8) -> Self {
        Self {
            func,
            mode: Mode::Inactive,
            hys: false,
            digimode: false,
        }
    }

    pub const fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub const fn hys(mut self, enable: bool) -> Self {
        self.hys = enable;
        self
    }

    pub const fn digimode(mut self, enable: bool) -> Self {
        self.digimode = enable;
        self
    }

    /// Pack into the 32-bit IOCON register value.
    pub const fn bits(self) -> u32 {
        let mut v = (self.func as u32) & 0x7;
        v |= (self.mode as u32) << 3;
        if self.hys {
            v |= 1 << 5;
        }
        if self.digimode {
            v |= 1 << 7;
        }
        v
    }
}

/// True if `(port, pin)` has an IOCON register on LPC11Uxx.
///
/// Valid: port 0 pins 0–23; port 1 pins 0–29 and 31 (PIO1_30 does not exist).
pub fn pin_is_valid(port: u8, pin: u8) -> bool {
    match port {
        0 => pin <= 23,
        1 => pin <= 29 || pin == 31,
        _ => false,
    }
}

/// Configure IOCON for `(port, pin)`.
///
/// # Panics
/// Panics if the pin is invalid (see [`pin_is_valid`]).
pub fn configure(iocon: &raw::IOCON, port: u8, pin: u8, config: PinConfig) {
    pin_mux_set(iocon, port, pin, config.bits());
}

/// Write a raw mode/function word to IOCON (OpenSC `Chip_IOCON_PinMuxSet`).
///
/// # Panics
/// Panics if the pin is invalid (see [`pin_is_valid`]).
pub fn pin_mux_set(iocon: &raw::IOCON, port: u8, pin: u8, modefunc: u32) {
    let index = match register_index(port, pin) {
        Some(i) => i,
        None => panic!("IOCON: invalid pin P{}[{}]", port, pin),
    };

    // PAC `IOCON` is a ZST token; MMIO is the `Deref` target (`IOCON::ptr()`).
    // Never cast `&IOCON` itself — that points at the token, not 0x4004_4000.
    let regs: &raw::iocon::RegisterBlock = iocon;
    let base = regs as *const raw::iocon::RegisterBlock as *mut u32;
    unsafe {
        core::ptr::write_volatile(base.add(index), modefunc);
    }
}

fn register_index(port: u8, pin: u8) -> Option<usize> {
    if !pin_is_valid(port, pin) {
        return None;
    }
    Some(match port {
        0 => pin as usize,
        1 => 24 + pin as usize,
        _ => return None,
    })
}
