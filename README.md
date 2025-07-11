# SH1106 driver

[![Crates.io](https://img.shields.io/crates/v/oled_async.svg)](https://crates.io/crates/oled_async)
[![Docs.rs](https://docs.rs/sh1106/badge.svg)](https://docs.rs/oled_async)

[![SH1107 SPI and I2C display modules showing the Rust logo](readme_banner.jpg?raw=true)](examples/image.rs)

I2C and SPI async driver for the SH11xx and SSD1xxx OLED displays written in 100% Rust. The library is designed to make it straigfotward to add new variants. Currently supported chips:

- SH1107
- SH1108
- SSD1309

## [Documentation](https://docs.rs/oled_async)

## [Examples]

This crate uses [`probe-run`](https://crates.io/crates/probe-run) to run the examples. Once set up,
it should be as simple as `cargo run --example image --features=embassy-stm32 --features=spi --release`. `--release` will be
required for some examples to reduce FLASH usage.

From [`examples/text.rs`](examples/text.rs):

```rust
#![no_std]
#![no_main]

mod bsp;

use embassy_executor::Spawner;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_async::delay::DelayNs;

use oled_async::{prelude::*, Builder};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let (di, mut reset, mut delay) = bsp::board::get_board();

    type Display = oled_async::displays::sh1107::Sh1107_128_128;
    //type Display = oled_async::displays::sh1108::Sh1108_64_160;
    //type Display = oled_async::displays::ssd1309::Ssd1309_128_64;

    let raw_disp = Builder::new(Display {})
        .with_rotation(crate::DisplayRotation::Rotate180)
        .connect(di);

    let mut display: GraphicsMode<_, _, { 128 * 128 / 8 }> = raw_disp.into();

    display.reset(&mut reset, &mut delay).unwrap();
    display.init().await.unwrap();
    display.clear();
    display.flush().await.unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    display.flush().await.unwrap();

    loop {
        delay.delay_ms(1000).await;
    }
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
