use crate::{
    raw,
    peripherals::{
        syscon,
    },
    typestates::{
        init_state,
    }
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
}
