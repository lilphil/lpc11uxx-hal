use crate::{
    raw,
    typestates::init_state,
    peripherals::syscon,
};

crate::wrap_stateful_peripheral!(Ct16b1, CT16B1);

impl Ct16b1 {
    pub fn enabled(mut self, syscon: &mut syscon::Syscon) -> Ct16b1<init_state::Enabled> {
        syscon.enable_clock(&mut self.raw);

        Ct16b1 {
            raw: self.raw,
            _state: init_state::Enabled(()),
        }
    }

    pub fn disabled(mut self, syscon: &mut syscon::Syscon) -> Ct16b1<init_state::Disabled> {
        syscon.disable_clock(&mut self.raw);

        Ct16b1 {
            raw: self.raw,
            _state: init_state::Disabled,
        }
    }
}

impl Ct16b1<init_state::Enabled> {
    pub fn as_pac(&self) -> &raw::CT16B1 {
        &self.raw
    }
}
