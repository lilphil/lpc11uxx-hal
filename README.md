# lpc11uxx-hal

Hardware Abstraction Layer (HAL) for [NXP LPC11Uxx](https://www.nxp.com/products/processors-and-microcontrollers/arm-microcontrollers/general-purpose-mcus/lpc1100-cortex-m0-plus-m0:LPC1100) series Cortex-M0 microcontrollers, written in Rust.

This crate sits above the [`lpc11uxx`](https://crates.io/crates/lpc11uxx) PAC and implements useful pieces of [`embedded-hal`](https://crates.io/crates/embedded-hal).

## Status

Implemented and usable:

- System clock setup (12 MHz crystal → 48 MHz PLL) and clock helpers
- SysTick and busy-loop delays
- GPIO (direction, set/clear/toggle)
- IOCON wrappers and raw `(port, pin)` FUNC/MODE/HYS/DIGIMODE helpers
- SYSCON wrappers
- USART0 serial (`embedded-hal-nb`, Steam Controller baud preset)
- USB clock / USBRAM enable helpers for use with `lpc11uxx-usbd`
- CT16B1 PWM (match channels MAT0–MAT3)

Board-specific pin mux, memory layouts, and examples live in application crates.

## Usage

```toml
[dependencies]
lpc11uxx-hal = { git = "https://github.com/lilphil/lpc11uxx-hal" }
```

Target: `thumbv6m-none-eabi`

```bash
rustup target add thumbv6m-none-eabi
cargo check
```

## Acknowledgements

Structure and patterns are adapted from [lpc55-hal](https://github.com/nickray/lpc55-hal) (nickray / lpc55). That project in turn incorporates ideas and code from [lpc8xx-hal](https://github.com/lpc-rs/lpc8xx-hal) and the [stm32-rs](https://github.com/stm32-rs) HALs.

Thanks to [steam_controller_custom_firmware](https://github.com/h1k421/steam_controller_custom_firmware), [OpenSteamController](https://github.com/greggersaurus/OpenSteamController), and [lpc-rs/lpc11uxx-hal](https://github.com/lpc-rs/lpc11uxx-hal) (including [roblabla’s poc branch](https://github.com/roblabla/lpc11uxx-hal)) for LPC11Uxx reference code and documentation this crate builds on.

Register access uses the [`lpc11uxx`](https://crates.io/crates/lpc11uxx) PAC.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
