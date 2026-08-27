//! Frequency, baud, and duration helper types.

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Hertz(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Kilohertz(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Megahertz(pub u32);

impl From<Kilohertz> for Hertz {
    fn from(khz: Kilohertz) -> Self {
        Hertz(1_000 * khz.0)
    }
}

impl From<Megahertz> for Kilohertz {
    fn from(mhz: Megahertz) -> Self {
        Kilohertz(1_000 * mhz.0)
    }
}

impl From<Megahertz> for Hertz {
    fn from(mhz: Megahertz) -> Self {
        Hertz(1_000_000 * mhz.0)
    }
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Baud(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Kilobaud(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Megabaud(pub u32);

impl From<Kilobaud> for Baud {
    fn from(kbd: Kilobaud) -> Self {
        Baud(1_000 * kbd.0)
    }
}

impl From<Megabaud> for Kilobaud {
    fn from(mbd: Megabaud) -> Self {
        Kilobaud(1_000 * mbd.0)
    }
}

impl From<Megabaud> for Baud {
    fn from(mbd: Megabaud) -> Self {
        Baud(1_000_000 * mbd.0)
    }
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Seconds(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct MiliSeconds(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct MicroSeconds(pub u32);
#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct NanoSeconds(pub u32);

impl From<Seconds> for MiliSeconds {
    fn from(s: Seconds) -> Self {
        MiliSeconds(1_000 * s.0)
    }
}

impl From<Seconds> for MicroSeconds {
    fn from(s: Seconds) -> Self {
        MicroSeconds(1_000_000 * s.0)
    }
}

impl From<MiliSeconds> for MicroSeconds {
    fn from(ms: MiliSeconds) -> Self {
        MicroSeconds(1_000 * ms.0)
    }
}

impl From<MicroSeconds> for NanoSeconds {
    fn from(us: MicroSeconds) -> Self {
        NanoSeconds(1_000 * us.0)
    }
}

pub trait U32Ext {
    fn hz(self) -> Hertz;
    fn khz(self) -> Kilohertz;
    fn mhz(self) -> Megahertz;
    fn bps(self) -> Baud;
    fn kbps(self) -> Kilobaud;
    fn mbps(self) -> Megabaud;
    fn s(self) -> Seconds;
    fn ms(self) -> MiliSeconds;
    fn us(self) -> MicroSeconds;
    fn ns(self) -> NanoSeconds;
}

impl U32Ext for u32 {
    fn hz(self) -> Hertz {
        Hertz(self)
    }

    fn khz(self) -> Kilohertz {
        Kilohertz(self)
    }

    fn mhz(self) -> Megahertz {
        Megahertz(self)
    }

    fn bps(self) -> Baud {
        Baud(self)
    }

    fn kbps(self) -> Kilobaud {
        Kilobaud(self)
    }

    fn mbps(self) -> Megabaud {
        Megabaud(self)
    }

    fn s(self) -> Seconds {
        Seconds(self)
    }

    fn ms(self) -> MiliSeconds {
        MiliSeconds(self)
    }

    fn us(self) -> MicroSeconds {
        MicroSeconds(self)
    }

    fn ns(self) -> NanoSeconds {
        NanoSeconds(self)
    }
}
