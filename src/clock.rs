//! System clock setup for LPC11Uxx parts with an external crystal oscillator.
//!
//! Configures a 12 MHz crystal and system PLL for a 48 MHz CPU clock. Boards
//! with different crystals or speed targets should configure SYSCON themselves
//! or extend this module.

use crate::raw::{Peripherals, SYSCON};

/// External crystal frequency assumed by [`init`].
pub const DEFAULT_CRYSTAL_HZ: u32 = 12_000_000;
const PLL_M: u8 = 3;
const PLL_P: u8 = 1;

/// Frozen clock configuration returned after [`init`].
#[derive(Clone, Copy, Debug)]
pub struct Clocks {
    pub system_hz: u32,
}

/// Configure clocks and enable IOCON + GPIO peripheral clocks.
pub fn init(peripherals: &Peripherals) -> Clocks {
    configure_sys_pll(&peripherals.SYSCON);
    configure_flash_timing(&peripherals.SYSCON, &peripherals.FLASHCTRL);
    select_main_clock_pll(&peripherals.SYSCON);
    enable_app_clocks(&peripherals.SYSCON);

    let system_hz = system_clock_hz(&peripherals.SYSCON);
    Clocks { system_hz }
}

fn configure_sys_pll(syscon: &SYSCON) {
    syscon
        .pdruncfg
        .modify(|_, w| w.sysosc_pd().powered());

    // Main oscillator needs a short settle time before PLL use.
    for _ in 0..0x1600 {
        cortex_m::asm::nop();
    }

    syscon
        .syspllclksel
        .write(|w| w.sel().crystal_oscillator());
    syscon.syspllclkuen.write(|w| w.ena().no_change());
    syscon
        .syspllclkuen
        .write(|w| w.ena().update_clock_source());

    syscon
        .pdruncfg
        .modify(|_, w| w.syspll_pd().powered_down());

    syscon.syspllctrl.write(|w| unsafe {
        w.msel().bits(PLL_M).psel().bits(PLL_P)
    });

    syscon
        .pdruncfg
        .modify(|_, w| w.syspll_pd().powered());

    while syscon.syspllstat.read().lock().is_pll_not_locked() {}
}

fn configure_flash_timing(syscon: &SYSCON, flash: &crate::raw::FLASHCTRL) {
    syscon
        .sysahbclkdiv
        .write(|w| unsafe { w.div().bits(1) });

    flash
        .flashcfg
        .modify(|_, w| w.flashtim()._3_system_clocks_flas());
}

fn select_main_clock_pll(syscon: &SYSCON) {
    syscon.mainclksel.write(|w| w.sel().pll_output());
    syscon.mainclkuen.write(|w| w.ena().no_change());
    syscon
        .mainclkuen
        .write(|w| w.ena().update_clock_source());
}

/// Enable IOCON, GPIO, and RAM1 clocks for typical app startup.
///
/// Assumes the boot ROM or a prior boot stage already configured the CPU PLL.
pub fn enable_app_clocks(syscon: &SYSCON) {
    syscon.sysahbclkctrl.modify(|_, w| {
        w.iocon().enabled().gpio().enabled().ram1().enabled()
    });
}

/// Enable the USB PLL, transceiver, and peripheral clocks.
///
/// Does not touch the CPU/system PLL.
pub fn enable_usb(syscon: &SYSCON) {
    syscon
        .pdruncfg
        .modify(|_, w| w.sysosc_pd().powered());

    syscon
        .usbpllclksel
        .write(|w| w.sel().system_oscillator());
    syscon.usbpllclkuen.write(|w| w.ena().no_change());
    syscon
        .usbpllclkuen
        .write(|w| w.ena().update_clock_source());

    syscon.usbpllctrl.write(|w| unsafe {
        w.msel().bits(PLL_M).psel().bits(PLL_P)
    });

    syscon.pdruncfg.modify(|_, w| {
        w.usbpll_pd()
            .powered()
            .usbpad_pd()
            .usb_transceiver_poweered()
    });

    while syscon.usbpllstat.read().lock().is_pll_not_locked() {}

    syscon.usbclksel.write(|w| w.sel().usb_pll_out());
    syscon.usbclkuen.write(|w| w.ena().no_change());
    syscon
        .usbclkuen
        .write(|w| w.ena().update_clock_source());
    syscon
        .usbclkdiv
        .write(|w| unsafe { w.div().bits(1) });

    syscon.sysahbclkctrl.modify(|_, w| {
        w.usb().enabled().usbram().enabled()
    });
}

/// Stop the watchdog if a prior boot stage left it running.
pub fn stop_watchdog(peripherals: &Peripherals) {
    stop_watchdog_from_wwdt(&peripherals.WWDT);
}

/// Stop the watchdog via the WWDT register block.
pub fn stop_watchdog_from_wwdt(wwdt: &crate::raw::WWDT) {
    wwdt.mod_
        .modify(|_, w| w.wden().stopped());
}

/// Read the current system clock frequency from SYSCON without reconfiguring clocks.
///
/// Useful when a prior boot stage has already set up the PLL.
pub fn read(syscon: &SYSCON) -> Clocks {
    Clocks {
        system_hz: system_clock_hz(syscon),
    }
}

/// Alias for [`read`] — returns the CPU frequency in Hz.
pub fn system_hz(syscon: &SYSCON) -> u32 {
    system_clock_hz(syscon)
}

fn system_clock_hz(syscon: &SYSCON) -> u32 {
    use crate::raw::syscon::mainclksel::SELR;

    let main_hz = match syscon.mainclksel.read().sel() {
        SELR::IRC_OSCILLATOR | SELR::PLL_INPUT => DEFAULT_CRYSTAL_HZ,
        SELR::PLL_OUTPUT => {
            let m = syscon.syspllctrl.read().msel().bits();
            u32::from(m + 1) * DEFAULT_CRYSTAL_HZ
        }
        _ => DEFAULT_CRYSTAL_HZ,
    };

    let div = syscon.sysahbclkdiv.read().div().bits();
    if div == 0 {
        main_hz
    } else {
        main_hz / u32::from(div)
    }
}
