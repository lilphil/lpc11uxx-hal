use crate::{
    raw,
    typestates::init_state,
    peripherals::syscon,
};

crate::wrap_stateful_peripheral!(Gpio, GPIO_PORT);

impl Gpio {
    /// Consumes disabled Gpio, returns an enabled one.
    pub fn enabled(mut self, syscon: &mut syscon::Syscon) -> Gpio<init_state::Enabled> {
        syscon.enable_clock(&mut self.raw);

        Gpio {
            raw: self.raw,
            _state: init_state::Enabled(()),
        }
    }

    /// Consumes enabled Gpio, returns a disabled one.
    pub fn disabled(mut self, syscon: &mut syscon::Syscon) -> Gpio<init_state::Disabled> {
        syscon.disable_clock(&mut self.raw);

        Gpio {
            raw: self.raw,
            _state: init_state::Disabled,
        }
    }
}

impl Gpio<init_state::Enabled> {
    pub fn as_pac(&self) -> &raw::GPIO_PORT {
        &self.raw
    }

    /// Configure a pin as a push-pull output.
    pub fn make_output(&self, port: usize, pin: u8) {
        assert!(port <= 1);
        assert!(pin < 32);
        let mask = 1u32 << pin;
        self.raw.dir[port].modify(|r, w| unsafe { w.bits(r.bits() | mask) });
    }

    /// Configure a pin as a high-impedance input (clear DIR bit).
    pub fn make_input(&self, port: usize, pin: u8) {
        assert!(port <= 1);
        assert!(pin < 32);
        let mask = 1u32 << pin;
        self.raw
            .dir[port]
            .modify(|r, w| unsafe { w.bits(r.bits() & !mask) });
    }

    pub fn is_high(&self, port: usize, pin: u8) -> bool {
        assert!(port <= 1);
        assert!(pin < 32);
        let mask = 1u32 << pin;
        self.raw.pin[port].read().bits() & mask != 0
    }

    pub fn is_low(&self, port: usize, pin: u8) -> bool {
        !self.is_high(port, pin)
    }

    pub fn set_high(&self, port: usize, pin: u8) {
        let mask = 1u32 << pin;
        self.raw.set[port].write(|w| unsafe { w.bits(mask) });
    }

    pub fn set_low(&self, port: usize, pin: u8) {
        let mask = 1u32 << pin;
        self.raw.clr[port].write(|w| unsafe { w.bits(mask) });
    }

    pub fn toggle(&self, port: usize, pin: u8) {
        if self.is_high(port, pin) {
            self.set_low(port, pin);
        } else {
            self.set_high(port, pin);
        }
    }
}

/// Free functions for apps that hold a PAC `&GPIO_PORT` (same DIR/PIN rules).
pub fn make_input(gpio: &raw::GPIO_PORT, port: usize, pin: u8) {
    assert!(port <= 1);
    assert!(pin < 32);
    let mask = 1u32 << pin;
    gpio.dir[port].modify(|r, w| unsafe { w.bits(r.bits() & !mask) });
}

pub fn make_output(gpio: &raw::GPIO_PORT, port: usize, pin: u8) {
    assert!(port <= 1);
    assert!(pin < 32);
    let mask = 1u32 << pin;
    gpio.dir[port].modify(|r, w| unsafe { w.bits(r.bits() | mask) });
}

pub fn is_high(gpio: &raw::GPIO_PORT, port: usize, pin: u8) -> bool {
    assert!(port <= 1);
    assert!(pin < 32);
    let mask = 1u32 << pin;
    gpio.pin[port].read().bits() & mask != 0
}

pub fn is_low(gpio: &raw::GPIO_PORT, port: usize, pin: u8) -> bool {
    !is_high(gpio, port, pin)
}

pub fn set_high(gpio: &raw::GPIO_PORT, port: usize, pin: u8) {
    assert!(port <= 1);
    assert!(pin < 32);
    let mask = 1u32 << pin;
    gpio.set[port].write(|w| unsafe { w.bits(mask) });
}

pub fn set_low(gpio: &raw::GPIO_PORT, port: usize, pin: u8) {
    assert!(port <= 1);
    assert!(pin < 32);
    let mask = 1u32 << pin;
    gpio.clr[port].write(|w| unsafe { w.bits(mask) });
}
