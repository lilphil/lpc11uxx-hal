//! Peripheral enable/disable typestates.
//!
//! The default state is `Unknown` because firmware may start after a bootloader
//! rather than from a cold reset.

pub mod init_state {
    pub trait InitState {}

    /// Peripheral state is not known.
    pub struct Unknown;
    impl InitState for Unknown {}

    /// Hardware component is enabled and usable.
    pub struct Enabled<T = ()>(pub T);
    impl InitState for Enabled {}

    /// Hardware component is disabled.
    pub struct Disabled;
    impl InitState for Disabled {}
}

pub mod main_clock {
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum MainClock {
        Irc12Mhz,
        SysOsc,
        Pll,
        Wdt,
    }
}

pub mod reg_proxy;
