//! Print:
//!
//!  Hello world!
//!
//!     <<=== (R) ===>>  // Moving RUST logo
//!
//!  Hello rust!
//!
//! the text with a 6x8 pixel font.
//!
//! This example is tested with an STM32G431 board connected to a SH1107 based display via SPI or I2C.
//!
//! It should be easy to modify:
//!  - Display type: Choose below
//!  - Chip/board, if STM32:
//!       - Modify chip in Cargo.toml
//!       - Choose different periperals and pins in bsp.rs
//!
//! Run with: `cargo run --example image --features=embassy-stm32 --features=spi --release`.
//! or
//! Run with: `cargo run --example image --features=embassy-stm32 --features=i2c --release`.
//!

#![no_std]
#![no_main]

mod bsp;

use embassy_executor::Spawner;
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

use oled_async::{prelude::*, Builder};
use {defmt_rtt as _, panic_probe as _};

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
async fn run() {
    let (di, mut reset, mut delay) = bsp::board::get_board();

    //type Display = oled_async::displays::sh1107::Sh1107_128_128;
    type Display = oled_async::displays::sh1108::Sh1108_128_160;
    //type Display = oled_async::displays::ssd1309::Ssd1309_128_64;

    let raw_disp = Builder::new(Display {})
        .with_rotation(crate::DisplayRotation::Rotate180)
        .connect(di);

    let mut disp: GraphicsMode<_, _, { 128 * 160 / 8 }> = raw_disp.into();

    disp.reset(&mut reset, &mut delay).unwrap();
    disp.init().await.unwrap();
    disp.clear();
    disp.flush().await.unwrap();

    let im: ImageRawLE<BinaryColor> = ImageRawLE::new(include_bytes!("./rust.raw"), 64);

    let (x_diff, y_diff) = {
        let dwidth = disp.get_dimensions().0 as i32;
        let dheight = disp.get_dimensions().1 as i32;
        let iwidth = im.size().width as i32;
        let iheight = im.size().height as i32;
        (dwidth - iwidth, dheight - iheight)
    };

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut dir = 1;
    let mut x = 0;
    loop {
        disp.clear();

        Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();

        Text::with_baseline(
            "Hello Rust!",
            Point::new(0, disp.get_dimensions().1 as i32),
            text_style,
            Baseline::Bottom,
        )
        .draw(&mut disp)
        .unwrap();

        Image::new(&im, Point::new(x, y_diff / 2))
            .draw(&mut disp)
            .unwrap();
        x += dir;
        if dir > 0 && x >= x_diff {
            dir = -1;
        } else if dir < 0 && x <= 0 {
            dir = 1;
        }
        disp.flush().await.unwrap();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    #[cfg(not(feature = "blocking"))]
    run().await;
    #[cfg(feature = "blocking")]
    run();
}
