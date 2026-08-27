//! USART0 serial driver (blocking + `embedded-hal-nb`).
//!
//! Pin mux (e.g. PIO1_17 RXD / PIO1_18 TXD) stays in the board crate via IOCON
//! helpers. NVIC priority and interrupt enabling are left to the application;
//! use [`Serial::enable_rx_interrupts`] / [`Serial::disable_rx_interrupts`] for
//! the peripheral-side IER bits.

use core::fmt;

use embedded_hal_nb::serial::{Error as SerialError, ErrorKind, ErrorType, Read, Write};
use nb;

use crate::raw::{usart, SYSCON, USART};

/// Steam Controller radio UART recipe at 48 MHz UART clock (`uartclkdiv = 1`):
/// DLL = 3, FDR mul = 11, divadd = 1 → ≈ 916667 baud (nRF link).
pub const STEAM_CONTROLLER_DLL: u16 = 3;
pub const STEAM_CONTROLLER_MULVAL: u8 = 11;
pub const STEAM_CONTROLLER_DIVADDVAL: u8 = 1;

/// USART configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Divisor latch (DLL + 256×DLM), written with DLAB = 1.
    pub dll: u16,
    /// FDR `MULVAL` (1–15).
    pub mulval: u8,
    /// FDR `DIVADDVAL` (0–14, must be < mulval).
    pub divaddval: u8,
    /// `SYSCON.UARTCLKDIV` divider (1 = UART clock equals main clock).
    pub uartclkdiv: u8,
}

impl Config {
    /// Baud recipe used by `steam_controller_custom_firmware` for the nRF link.
    pub const fn steam_controller() -> Self {
        Self {
            dll: STEAM_CONTROLLER_DLL,
            mulval: STEAM_CONTROLLER_MULVAL,
            divaddval: STEAM_CONTROLLER_DIVADDVAL,
            uartclkdiv: 1,
        }
    }

    /// Raw DLL + FDR values (8N1, `uartclkdiv = 1`).
    pub const fn raw(dll: u16, mulval: u8, divaddval: u8) -> Self {
        Self {
            dll,
            mulval,
            divaddval,
            uartclkdiv: 1,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::steam_controller()
    }
}

/// Serial error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Framing,
    Noise,
    Overrun,
    Parity,
}

impl SerialError for Error {
    fn kind(&self) -> ErrorKind {
        match self {
            Error::Framing => ErrorKind::FrameFormat,
            Error::Noise => ErrorKind::Noise,
            Error::Overrun => ErrorKind::Overrun,
            Error::Parity => ErrorKind::Parity,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Framing => write!(f, "framing error"),
            Error::Noise => write!(f, "noise error"),
            Error::Overrun => write!(f, "overrun"),
            Error::Parity => write!(f, "parity error"),
        }
    }
}

/// USART0 handle (owned borrow of the PAC peripheral).
pub struct Serial<'a> {
    usart: &'a USART,
}

/// Transmit half after [`Serial::split`].
pub struct Tx<'a> {
    usart: &'a USART,
}

/// Receive half after [`Serial::split`].
pub struct Rx<'a> {
    usart: &'a USART,
}

impl<'a> Serial<'a> {
    /// Enable the USART clock, configure FIFOs / 8N1 / baud, leave IRQs off.
    pub fn new(syscon: &SYSCON, usart: &'a USART, config: Config) -> Self {
        enable_clock(syscon, config.uartclkdiv);
        configure_fifos_8n1(usart);
        set_baud(usart, config);
        Serial { usart }
    }

    /// Split into independent Tx/Rx halves (both share the same USART registers).
    pub fn split(self) -> (Tx<'a>, Rx<'a>) {
        (Tx { usart: self.usart }, Rx { usart: self.usart })
    }

    pub fn free(self) -> &'a USART {
        self.usart
    }

    /// Enable RBR and RLS interrupts in IER. Does not touch NVIC.
    pub fn enable_rx_interrupts(&self) {
        enable_rx_interrupts(self.usart);
    }

    /// Disable RBR and RLS interrupts in IER.
    pub fn disable_rx_interrupts(&self) {
        disable_rx_interrupts(self.usart);
    }

    /// Enable THRE interrupt in IER (for IRQ-driven TX). Does not touch NVIC.
    pub fn enable_tx_interrupt(&self) {
        ier(self.usart).modify(|_, w| w.threinten().enable_the_thre_inte());
    }

    /// Disable THRE interrupt in IER.
    pub fn disable_tx_interrupt(&self) {
        ier(self.usart).modify(|_, w| w.threinten().disable_the_thre_int());
    }

    /// Non-blocking read (same as [`Read::read`]).
    pub fn read(&mut self) -> nb::Result<u8, Error> {
        read_byte(self.usart)
    }

    /// Non-blocking write (same as [`Write::write`]).
    pub fn write(&mut self, byte: u8) -> nb::Result<(), Error> {
        write_byte(self.usart, byte)
    }

    /// Block until a byte is received.
    pub fn read_blocking(&mut self) -> Result<u8, Error> {
        nb::block!(self.read())
    }

    /// Block until the byte is accepted into the TX FIFO.
    pub fn write_blocking(&mut self, byte: u8) -> Result<(), Error> {
        nb::block!(self.write(byte))
    }

    /// Block until TX FIFO and shift register are empty.
    pub fn flush_blocking(&mut self) -> Result<(), Error> {
        nb::block!(flush(self.usart))
    }
}

impl<'a> Tx<'a> {
    pub fn write_blocking(&mut self, byte: u8) -> Result<(), Error> {
        nb::block!(write_byte(self.usart, byte))
    }

    pub fn flush_blocking(&mut self) -> Result<(), Error> {
        nb::block!(flush(self.usart))
    }

    pub fn enable_tx_interrupt(&self) {
        ier(self.usart).modify(|_, w| w.threinten().enable_the_thre_inte());
    }

    pub fn disable_tx_interrupt(&self) {
        ier(self.usart).modify(|_, w| w.threinten().disable_the_thre_int());
    }
}

impl<'a> Rx<'a> {
    pub fn read_blocking(&mut self) -> Result<u8, Error> {
        nb::block!(read_byte(self.usart))
    }

    pub fn enable_rx_interrupts(&self) {
        enable_rx_interrupts(self.usart);
    }

    pub fn disable_rx_interrupts(&self) {
        disable_rx_interrupts(self.usart);
    }
}

impl ErrorType for Serial<'_> {
    type Error = Error;
}

impl Read<u8> for Serial<'_> {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        read_byte(self.usart)
    }
}

impl Write<u8> for Serial<'_> {
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        write_byte(self.usart, word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        flush(self.usart)
    }
}

impl ErrorType for Tx<'_> {
    type Error = Error;
}

impl Write<u8> for Tx<'_> {
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        write_byte(self.usart, word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        flush(self.usart)
    }
}

impl ErrorType for Rx<'_> {
    type Error = Error;
}

impl Read<u8> for Rx<'_> {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        read_byte(self.usart)
    }
}

fn enable_clock(syscon: &SYSCON, uartclkdiv: u8) {
    syscon
        .sysahbclkctrl
        .modify(|_, w| w.usart().enabled());
    syscon
        .uartclkdiv
        .write(|w| unsafe { w.div().bits(uartclkdiv) });
}

fn configure_fifos_8n1(usart: &USART) {
    fcr(usart).write(|w| {
        w.fifoen()
            .enabled()
            .rxfifores()
            .clear()
            .txfifores()
            .clear()
    });

    usart.lcr.write(|w| {
        w.wls()
            ._8_bit_character_leng()
            .sbs()
            ._1_stop_bit()
            .pe()
            .disabled()
    });

    // Match Chip_UART_Init: FDR mul=1 before baud setup.
    usart.fdr.write(|w| unsafe { w.mulval().bits(1).divaddval().bits(0) });

    fcr(usart).write(|w| w.fifoen().enabled().rxtl().level2());
}

fn set_baud(usart: &USART, config: Config) {
    usart.lcr.modify(|_, w| w.dlab().enable_access_to_div());

    let dll_lo = (config.dll & 0xff) as u8;
    let dll_hi = ((config.dll >> 8) & 0xff) as u8;

    dll(usart).write(|w| unsafe { w.dllsb().bits(dll_lo) });
    usart.dlm.write(|w| unsafe { w.dlmsb().bits(dll_hi) });
    usart.fdr.write(|w| unsafe {
        w.mulval()
            .bits(config.mulval)
            .divaddval()
            .bits(config.divaddval)
    });

    usart.lcr.modify(|_, w| w.dlab().disable_access_to_di());
}

fn enable_rx_interrupts(usart: &USART) {
    ier(usart).modify(|_, w| {
        w.rbrinten()
            .enable_the_rda_inter()
            .rlsinten()
            .enable_the_rls_inter()
    });
}

fn disable_rx_interrupts(usart: &USART) {
    ier(usart).modify(|_, w| {
        w.rbrinten()
            .disable_the_rda_inte()
            .rlsinten()
            .disable_the_rls_inte()
    });
}

fn read_byte(usart: &USART) -> nb::Result<u8, Error> {
    let lsr = usart.lsr.read();

    if lsr.oe().is_active() {
        let _ = usart.rbr.read();
        return Err(nb::Error::Other(Error::Overrun));
    }
    if lsr.pe().is_active() {
        let _ = usart.rbr.read();
        return Err(nb::Error::Other(Error::Parity));
    }
    if lsr.fe().is_active() {
        let _ = usart.rbr.read();
        return Err(nb::Error::Other(Error::Framing));
    }
    if lsr.bi().is_active() {
        let _ = usart.rbr.read();
        return Err(nb::Error::Other(Error::Framing));
    }

    if lsr.rdr().is_valid() {
        Ok(usart.rbr.read().rbr().bits())
    } else {
        Err(nb::Error::WouldBlock)
    }
}

fn write_byte(usart: &USART, byte: u8) -> nb::Result<(), Error> {
    if usart.lsr.read().thre().is_empty() {
        thr(usart).write(|w| unsafe { w.thr().bits(byte) });
        Ok(())
    } else {
        Err(nb::Error::WouldBlock)
    }
}

fn flush(usart: &USART) -> nb::Result<(), Error> {
    if usart.lsr.read().temt().is_empty() {
        Ok(())
    } else {
        Err(nb::Error::WouldBlock)
    }
}

// --- Aliased register accessors (crates.io lpc11uxx 0.3 omits these methods) ---

fn dll(usart: &USART) -> &usart::DLL {
    // DLL overlays RBR at offset 0 when DLAB = 1.
    unsafe { &*(&usart.rbr as *const usart::RBR as *const usart::DLL) }
}

fn thr(usart: &USART) -> &usart::THR {
    // THR overlays RBR at offset 0 when DLAB = 0.
    unsafe { &*(&usart.rbr as *const usart::RBR as *const usart::THR) }
}

fn ier(usart: &USART) -> &usart::IER {
    // IER overlays DLM at offset 4 when DLAB = 0.
    unsafe { &*(&usart.dlm as *const usart::DLM as *const usart::IER) }
}

fn fcr(usart: &USART) -> &usart::FCR {
    // FCR is write-only at the same address as IIR (offset 8).
    unsafe { &*(&usart.iir as *const usart::IIR as *const usart::FCR) }
}
