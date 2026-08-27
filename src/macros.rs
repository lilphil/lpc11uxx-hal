#[macro_export]
macro_rules! reg {
    ($ty:ident, $target:ty, $peripheral:path, $field:ident) => {
        unsafe impl $crate::traits::reg_proxy::Reg for $ty {
            type Target = $target;

            fn get() -> *const Self::Target {
                unsafe { &(*<$peripheral>::ptr()).$field as *const $ty }
            }
        }
    };
}

#[macro_export]
macro_rules! reg_cluster {
    ($ty:ident, $target:ty, $peripheral:path, $field:ident) => {
        unsafe impl $crate::traits::reg_proxy::RegCluster for $ty {
            type Target = $target;

            fn get() -> *const [Self::Target] {
                unsafe { &(*<$peripheral>::ptr()).$field as *const [$ty] }
            }
        }
    };
}

#[macro_export]
macro_rules! wrap_always_on_peripheral {
    ($hal_name:ident, $pac_name:ident) => {
        use crate::raw;

        pub struct $hal_name {
            pub(crate) raw: raw::$pac_name,
        }

        impl core::convert::From<raw::$pac_name> for $hal_name {
            fn from(raw: raw::$pac_name) -> Self {
                $hal_name::new(raw)
            }
        }

        impl $hal_name {
            fn new(raw: raw::$pac_name) -> Self {
                $hal_name { raw }
            }

            pub unsafe fn steal() -> Self {
                Self::new(raw::Peripherals::steal().$pac_name)
            }

            pub fn release(self) -> raw::$pac_name {
                self.raw
            }
        }
    };
}

#[macro_export]
macro_rules! wrap_stateful_peripheral {
    ($hal_name:ident, $pac_name:ident) => {
        pub struct $hal_name<State = init_state::Unknown> {
            pub(crate) raw: raw::$pac_name,
            pub _state: State,
        }

        impl core::convert::From<raw::$pac_name> for $hal_name {
            fn from(raw: raw::$pac_name) -> Self {
                $hal_name::new(raw)
            }
        }

        impl $hal_name {
            fn new(raw: raw::$pac_name) -> Self {
                $hal_name {
                    raw,
                    _state: init_state::Unknown,
                }
            }

            pub unsafe fn steal() -> Self {
                Self::new(raw::Peripherals::steal().$pac_name)
            }
        }

        impl<State> $hal_name<State> {
            pub fn release(self) -> raw::$pac_name {
                self.raw
            }
        }
    };
}

#[macro_export]
macro_rules! stateful_peripheral_enable_disable {
    ($hal_name:ident) => {
        impl $hal_name {
            pub fn enabled(mut self, syscon: &mut syscon::Syscon) -> $hal_name<init_state::Enabled> {
                syscon.enable_clock(&mut self.raw);

                $hal_name {
                    raw: self.raw,
                    _state: init_state::Enabled(()),
                }
            }

            pub fn disabled(mut self, syscon: &mut syscon::Syscon) -> $hal_name<init_state::Disabled> {
                syscon.disable_clock(&mut self.raw);

                $hal_name {
                    raw: self.raw,
                    _state: init_state::Disabled,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! reg_read {
    ($peripheral:ident, $register:ident, $field:ident, $what:ident) => {
        unsafe { &(*hal::raw::$peripheral::ptr()) }.$register.read().$field().$what()
    };
    ($peripheral:ident, $register:ident, $field:ident) => {
        unsafe { &(*hal::raw::$peripheral::ptr()) }.$register.read().$field().bits()
    };
    ($peripheral:ident, $register:ident) => {
        unsafe { &(*hal::raw::$peripheral::ptr()) }.$register.read().bits()
    };
}
