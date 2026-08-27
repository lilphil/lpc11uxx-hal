//! PWM output via CT16Bx match registers (MAT0–MAT3).
//!
//! Configures one match channel as PWM and uses MR3 as the period counter.

use crate::raw::{CT16B1, SYSCON};

/// PWM channel selection (CT16B match output 0–3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PwmChannel {
    Mat0 = 0,
    Mat1 = 1,
    Mat2 = 2,
    Mat3 = 3,
}

/// PWM timer setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PwmConfig {
    pub channel: PwmChannel,
    /// MR3 reset value — defines the PWM period (exclusive upper bound for duty).
    pub period: u16,
}

impl Default for PwmConfig {
    fn default() -> Self {
        Self {
            channel: PwmChannel::Mat0,
            period: 0xFFF,
        }
    }
}

/// Single-channel PWM on a 16-bit CTIMER block.
pub struct Pwm<'a> {
    timer: &'a CT16B1,
    channel: PwmChannel,
    period: u16,
}

impl<'a> Pwm<'a> {
    /// Enable the CT16B1 clock, configure PWM on `config.channel`, and start the timer.
    pub fn new(syscon: &SYSCON, timer: &'a CT16B1, config: PwmConfig) -> Self {
        syscon.sysahbclkctrl.modify(|_, w| w.ct16b1().enabled());

        let pwm = Pwm {
            timer,
            channel: config.channel,
            period: config.period,
        };
        pwm.apply_config(config);
        pwm
    }

    fn apply_config(&self, config: PwmConfig) {
        unsafe {
            self.timer.pr.write(|w| w.pcval().bits(0));
        }

        self.enable_pwm_channel(config.channel);

        unsafe {
            self.timer.mr[3].write(|w| w.bits(u32::from(config.period)));
            self.timer.mr[channel_index(config.channel)]
                .write(|w| w.bits(u32::from(config.period)));
        }

        self.timer
            .mcr
            .modify(|_, w| w.mr3r().enabled());

        reset_timer(self.timer);

        self.timer
            .tcr
            .modify(|_, w| w.cen().the_timer_counter_an());
    }

    fn enable_pwm_channel(&self, channel: PwmChannel) {
        self.timer.pwmc.write(|w| match channel {
            PwmChannel::Mat0 => w.pwmen0().enabled(),
            PwmChannel::Mat1 => w.pwmen1().enabled(),
            PwmChannel::Mat2 => w.pwmen2().enabled(),
            PwmChannel::Mat3 => w.pwmen3().enabled(),
        });
    }

    pub fn period(&self) -> u16 {
        self.period
    }

    pub fn max_duty(&self) -> u16 {
        self.period
    }

    pub fn set_duty(&self, duty: u16) {
        let duty = duty.min(self.period);
        unsafe {
            self.timer.mr[channel_index(self.channel)]
                .write(|w| w.bits(u32::from(duty)));
        }
    }

    pub fn duty(&self) -> u16 {
        self.timer.mr[channel_index(self.channel)]
            .read()
            .bits() as u16
    }
}

fn channel_index(channel: PwmChannel) -> usize {
    channel as usize
}

fn reset_timer(timer: &CT16B1) {
    let backup_tcr = timer.tcr.read().bits();

    timer.tcr.write(|w| w.cen().the_counters_are_dis());

    unsafe {
        timer.tc.write(|w| w.tc().bits(1));
    }

    timer.tcr.write(|w| w.crst().reset());

    while timer.tc.read().bits() != 0 {}

    unsafe {
        timer.tcr.write(|w| w.bits(backup_tcr));
    }
}
