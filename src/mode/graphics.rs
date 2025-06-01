//! Buffered display module for use with the [embedded-graphics] crate
//!
//! ```rust,no_run
//!
//! use embedded_graphics::{
//!     mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
//!     pixelcolor::BinaryColor,
//!     prelude::*,
//!     text::{Baseline, Text},
//! };
//! async fn run_display(display_interface: SomeInstanceOfDisplayInterface) {
//!     let mut disp: GraphicsMode<_, _> = Builder::new(Display {})
//!         .with_rotation(crate::DisplayRotation::Rotate180)
//!         .connect(display_interface)
//!         .into();
//!
//!     disp.reset(&mut reset, &mut delay).unwrap();
//!     disp.init().await.unwrap();
//!     disp.clear();
//!     disp.flush().await.unwrap();
//!
//!     let text_style = MonoTextStyleBuilder::new()
//!         .font(&FONT_6X10)
//!         .text_color(BinaryColor::On)
//!         .build();
//!     Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
//!         .draw(&mut disp)
//!         .unwrap();
//!
//!     disp.flush().await.unwrap();
//! }
//! ```

use display_interface::{AsyncWriteOnlyDataCommand, DisplayError};
use hal::{delay::DelayNs, digital::OutputPin};

use crate::{
    display, displayrotation::DisplayRotation, mode::displaymode::DisplayModeTrait,
    properties::DisplayProperties,
};

const DEFAULT_BUFFER_SIZE: usize = 160 * 160 / 8;

/// Graphics mode handler
pub struct GraphicsMode<DV, DI, const BS: usize = DEFAULT_BUFFER_SIZE>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    properties: DisplayProperties<DV, DI>,
    buffer: [u8; BS],
}

impl<DV, DI, const BS: usize> DisplayModeTrait<DV, DI> for GraphicsMode<DV, DI, BS>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    /// Create new GraphicsMode instance
    fn new(properties: DisplayProperties<DV, DI>) -> Self {
        GraphicsMode {
            properties,
            buffer: [0u8; BS],
        }
    }

    /// Release all resources used by GraphicsMode
    fn release(self) -> DisplayProperties<DV, DI> {
        self.properties
    }
}

impl<DV, DI, const BS: usize> GraphicsMode<DV, DI, BS>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    /// Clear the display buffer. You need to call `display.flush()` for any effect on the screen
    pub fn clear(&mut self) {
        self.buffer = [0; BS];
    }

    /// Reset display
    pub fn reset<RST, DELAY, PinE>(&mut self, rst: &mut RST, delay: &mut DELAY) -> Result<(), PinE>
    where
        RST: OutputPin<Error = PinE>,
        DELAY: DelayNs,
    {
        rst.set_high()?;
        delay.delay_ms(1);
        rst.set_low()?;
        delay.delay_ms(10);
        rst.set_high()
    }

    /// Write out data to display
    pub async fn flush(&mut self) -> Result<(), DisplayError> {
        // Ensure the display buffer is at the origin of the display before we send the full frame
        // to prevent accidental offsets
        let (display_width, display_height) = DV::dimensions();
        let column_offset = DV::COLUMN_OFFSET;
        self.properties
            .set_draw_area(
                (column_offset, 0),
                (display_width + column_offset, display_height),
            )
            .await?;

        let length = (display_width as usize) * (display_height as usize) / 8;

        self.properties.draw(&self.buffer[..length]).await
    }

    /// Turn a pixel on or off. A non-zero `value` is treated as on, `0` as off. If the X and Y
    /// coordinates are out of the bounds of the display, this method call is a noop.
    pub fn set_pixel(&mut self, x: u32, y: u32, value: u8) {
        let (display_width, _) = DV::dimensions();
        let display_rotation = self.properties.get_rotation();

        let idx = match display_rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => {
                if x >= display_width as u32 {
                    return;
                }
                ((y as usize) / 8 * display_width as usize) + (x as usize)
            }

            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => {
                if y >= display_width as u32 {
                    return;
                }
                ((x as usize) / 8 * display_width as usize) + (y as usize)
            }
        };

        if idx >= self.buffer.len() {
            return;
        }

        let bit_index = match display_rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => y % 8,
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => x % 8,
        };
        let bit = 1 << bit_index;

        if value == 0 {
            self.buffer[idx] &= !bit;
        } else {
            self.buffer[idx] |= bit;
        }
    }

    /// Display is set up in column mode, i.e. a byte walks down a column of 8 pixels from
    /// column 0 on the left, to column _n_ on the right
    pub async fn init(&mut self) -> Result<(), DisplayError> {
        self.properties.init_column_mode().await
    }

    /// Get display dimensions, taking into account the current rotation of the display
    pub fn get_dimensions(&self) -> (u8, u8) {
        self.properties.get_dimensions()
    }

    /// Get the display rotation
    pub fn get_rotation(&self) -> DisplayRotation {
        self.properties.get_rotation()
    }

    /// Set the display rotation
    pub async fn set_rotation(&mut self, rot: DisplayRotation) -> Result<(), DisplayError> {
        self.properties.set_rotation(rot).await
    }

    /// Turn the display on or off. The display can be drawn to and retains all
    /// of its memory even while off.
    pub async fn display_on(&mut self, on: bool) -> Result<(), DisplayError> {
        self.properties.display_on(on).await
    }

    /// Set the display contrast
    pub async fn set_contrast(&mut self, contrast: u8) -> Result<(), DisplayError> {
        self.properties.set_contrast(contrast).await
    }
}

#[cfg(feature = "graphics")]
use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};

#[cfg(feature = "graphics")]
impl<DV, DI, const BS: usize> DrawTarget for GraphicsMode<DV, DI, BS>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    type Color = BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let bb = self.bounding_box();

        pixels
            .into_iter()
            .filter(|Pixel(pos, _color)| bb.contains(*pos))
            .for_each(|Pixel(pos, color)| {
                self.set_pixel(pos.x as u32, pos.y as u32, color.is_on().into())
            });

        Ok(())
    }
}

#[cfg(feature = "graphics")]
impl<DV, DI, const BS: usize> OriginDimensions for GraphicsMode<DV, DI, BS>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    fn size(&self) -> Size {
        let (w, h) = self.get_dimensions();

        Size::new(w.into(), h.into())
    }
}
