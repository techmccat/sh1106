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

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

use display_interface::DisplayError;
use hal::{delay::DelayNs, digital::OutputPin};

use crate::{
    display, displayrotation::DisplayRotation, mode::displaymode::DisplayModeTrait,
    properties::DisplayProperties,
};

const DEFAULT_BUFFER_SIZE: usize = 160 * 160 / 8;

/// Graphics mode handler
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
pub struct GraphicsMode<DV, DI, const BS: usize = DEFAULT_BUFFER_SIZE>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    properties: DisplayProperties<DV, DI>,
    buffer: [u8; BS],
    top_left: (u8, u8),
    bot_right: (u8, u8),
}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
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
            top_left: (DV::WIDTH, DV::HEIGHT),
            bot_right: (0, 0),
        }
    }

    /// Release all resources used by GraphicsMode
    fn release(self) -> DisplayProperties<DV, DI> {
        self.properties
    }
}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DV, DI, const BS: usize> GraphicsMode<DV, DI, BS>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    /// Clear the display buffer. You need to call `display.flush()` for any effect on the screen
    pub fn clear(&mut self) {
        self.buffer = [0; BS];
        self.top_left = (0, 0);
        self.bot_right = (DV::WIDTH - 1, DV::HEIGHT - 1);
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
        // nothing drawn since last flush
        if self.top_left.0 > self.bot_right.0 || self.top_left.1 > self.bot_right.1 {
            return Ok(());
        }

        let base_col = self.top_left.0;
        let end_col = self.bot_right.0;

        let base_page = self.top_left.1 / 8;
        let end_page = self.bot_right.1.div_ceil(8);

        // for each page in the modified area
        for (page_num, buf) in self
            .buffer
            .chunks_exact(DV::WIDTH as usize)
            .enumerate()
            .skip(base_page as usize)
            .take((end_page - base_page) as usize)
        {
            // crop to columns in the modified area
            let buf = &buf[(base_col as usize)..=(end_col as usize)];
            // column offsetting done in draw_page
            self.properties
                .draw_page(page_num as u8, base_col, buf)
                .await?;
        }

        self.top_left = (DV::WIDTH - 1, DV::HEIGHT);
        self.bot_right = (0, 0);

        Ok(())
    }

    /// Turn a pixel on or off. A non-zero `value` is treated as on, `0` as off. If the X and Y
    /// coordinates are out of the bounds of the display, this method call is a noop.
    pub fn set_pixel(&mut self, x: u32, y: u32, value: u8) {
        let (display_width, _) = DV::dimensions();
        let display_rotation = self.properties.get_rotation();

        let (x, y) = match display_rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => (x, y),
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => (y, x),
        };
        self.top_left.0 = self.top_left.0.min(x as u8);
        self.top_left.1 = self.top_left.1.min(y as u8);

        self.bot_right.0 = self.bot_right.0.max(x as u8);
        self.bot_right.1 = self.bot_right.1.max(y as u8);

        let idx = (y as usize / 8) * display_width as usize + x as usize;

        if idx >= self.buffer.len() {
            return;
        }
        let bit_index = y % 8;
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

    #[cfg(feature = "graphics")]
    /// Needs y to be a multiple of 8, excess height is ignored
    fn fill_solid_aligned(&mut self, x: u32, y: u32, width: u32, height: u32, fill: u8) {
        // fill whole 8px tall chunks
        for block in (y / 8)..((height + y) / 8) {
            self.buffer[(x + block * DV::WIDTH as u32) as usize..][..width as usize].fill(fill);
        }
    }
    #[cfg(feature = "graphics")]
    fn apply_mask_to_page(&mut self, mask: u8, color: bool, page: u8, x: u8, width: u8) {
        let col_offset = x as usize + page as usize * DV::WIDTH as usize;
        let iter = self.buffer[col_offset..(col_offset + width as usize)].iter_mut();
        if color {
            iter.for_each(|b| *b |= mask);
        } else {
            iter.for_each(|b| *b &= !mask);
        };
    }
}

#[cfg(feature = "graphics")]
use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{Dimensions, OriginDimensions, Size},
    pixelcolor::BinaryColor,
    prelude::Point,
    primitives::Rectangle,
    Pixel,
};

#[cfg(feature = "graphics")]
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
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

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let Rectangle {
            top_left: Point { x, y },
            size: Size { width, height },
        } = area.intersection(&self.bounding_box());
        // swap coordinates if rotated
        let (x, mut y, width, mut height) = match self.properties.get_rotation() {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => (x, y, width, height),
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => (y, x, height, width),
        };
        self.top_left.0 = self.top_left.0.min(x as u8);
        self.top_left.1 = self.top_left.1.min(y as u8);

        self.bot_right.0 = self.bot_right.0.max(x as u8 + width as u8);
        self.bot_right.1 = self.bot_right.1.max(y as u8 + height as u8);

        // unaligned top
        if y % 8 != 0 {
            let mask_height = core::cmp::min(height, 8 - (y as u32 % 8));
            let mask = ((1u8 << mask_height) - 1) << (y % 8);
            self.apply_mask_to_page(mask, color.is_on(), (y / 8) as u8, x as u8, width as u8);

            height -= mask_height;
            y += mask_height as i32;
        }
        // potentially many full pages
        if height != 0 {
            let fill = if color.is_on() { 0xff } else { 0 };
            self.fill_solid_aligned(x as u32, y as u32, width, height, fill);
        }
        if (height % 8) != 0 {
            let mask = (1u8 << (height % 8)) - 1;
            let page = (y as u8 + height as u8) / 8;
            self.apply_mask_to_page(mask, color.is_on(), page, x as u8, width as u8);
        }

        let fill = if color.is_on() { 0xff } else { 0 };
        if y % 8 == 0 && height % 8 == 0 {
            self.fill_solid_aligned(x as u32, y as u32, width, height, fill);
        } else if y / 8 - (y + height as i32) / 8 > 1 {
            // perform a fast draw in solid fills that include a 8 row tall block
            // slower fallback draw, top
            let top_height = 8 - y % 8;
            self.fill_solid(
                &Rectangle::new(Point::new(x, y), Size::new(width, top_height as u32)),
                color,
            )?;
            // slower fallback draw, bottom
            let bottom_y = y + height as i32 - (y + height as i32) % 8;
            let bottom_height = (y as u32 + height) % 8;
            self.fill_solid(
                &Rectangle::new(Point::new(x, bottom_y), Size::new(width, bottom_height)),
                color,
            )?;
            // fast draw for the aligned block in the middle
            let mid_block = (y / 8 + 1) as u32;
            let mid_count = mid_block - (y as u32 + height) / 8 * 8;
            self.fill_solid_aligned(x as u32, mid_block * 8, width, mid_count * 8, fill);
        } else {
            // no happy path :'(
            self.fill_contiguous(area, core::iter::repeat(color))?;
        }
        Ok(())
    }
}

#[cfg(feature = "graphics")]
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
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
