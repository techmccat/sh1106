//! Display mode with lower level control over what's buffered
//!
//! Displays suported by this crate use page addressing, which means that every byte in the
//! framebuffer represents an aligned vertical group of 8 pixels, and the next byte represents the
//! next group to the right, until it wraps around to a new page

use crate::{display, properties::DisplayProperties};
#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::DisplayError;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;
#[cfg(feature = "graphics")]
use embedded_graphics_core::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle};

/// An arbitrary number of aligned 8 pixel tall columns, each represented by a byte
///
/// LSB is at the top of the column
/// Width must fit inside a u8, it's a usize for now
/// because generic const exprs aren't done yet
#[derive(Debug, Clone)]
pub struct Page<const W: usize>(pub [u8; W]);

/// Operations to perform on a buffered pixel
#[derive(Debug, Clone, Copy)]
pub enum PixelOperation {
    /// Set the pixel to 1
    Set,
    /// Set the pixel to 0
    Clear,
    /// Toggle the pixel
    Toggle,
}

#[cfg(feature = "graphics")]
impl From<BinaryColor> for PixelOperation {
    fn from(value: BinaryColor) -> Self {
        if value.is_on() {
            Self::Set
        } else {
            Self::Clear
        }
    }
}

impl<const W: usize> Page<W> {
    /// Creates a new page with the provided pixel pattern
    pub fn new(pattern: u8) -> Self {
        Self([pattern; W])
    }
    /// Applies the provided operation to a single pixel
    pub fn modify_pixel(&mut self, x: u8, y: u8, op: PixelOperation) {
        if x < W as u8 {
            let mask = 1 << (y % 8);
            let cell = &mut self.0[x as usize];
            match op {
                PixelOperation::Set => *cell |= mask,
                PixelOperation::Clear => *cell &= !mask,
                PixelOperation::Toggle => *cell ^= mask,
            }
        }
    }

    /// Applies a mask with a user-defined operation to a subsection of the page
    pub fn apply_mask(&mut self, mask: u8, start: u8, end: u8, op: PixelOperation) {
        if end < start {
            return;
        }
        let start = W.min(start as usize);
        let end = W.max(end as usize);
        for cell in self.0[start..end].iter_mut() {
            match op {
                PixelOperation::Set => *cell |= mask,
                PixelOperation::Clear => *cell &= !mask,
                PixelOperation::Toggle => *cell ^= mask,
            }
        }
    }
}

/// A collection of pages with an offset from the origin
#[derive(Debug, Clone)]
pub struct Tile<const W: usize, const P: usize> {
    /// Pages backing the tile
    pub pages: [Page<W>; P],
    /// Tile's initial column
    pub col_offset: u8,
    /// Tile's initial page's address
    pub page_offset: u8,
}

impl<const W: usize, const P: usize> Tile<W, P> {
    /// sets pixel relative to this tile's base position
    pub fn modify_pixel(&mut self, x: u8, y: u8, op: PixelOperation) {
        if x < W as u8 {
            self.pages
                .get_mut(y as usize / 8)
                .map(|page| page.modify_pixel(x, y % 8, op));
        }
    }
}

#[cfg(feature = "graphics")]
impl<const W: usize> Dimensions for Page<W> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle {
            top_left: Point::new(0, 0),
            size: Size::new(W as u32, 8),
        }
    }
}
#[cfg(feature = "graphics")]
impl<const W: usize> DrawTarget for Page<W> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.0.fill(if color.is_on() { 0xff } else { 0 });
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels
            .into_iter()
            .for_each(|p| self.modify_pixel(p.0.x as u8, p.0.y as u8, p.1.into()));
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let Rectangle {
            top_left: Point { x, y },
            size: Size { width, height },
        } = area.intersection(&self.bounding_box());
        // height bits set to 1, then shifted left by height
        let mask_height = core::cmp::min(height, 8 - (y as u32 % 8));
        let mask = (1u8 << mask_height).wrapping_sub(1) << (y % 8);
        self.apply_mask(mask, x as u8, x as u8 + width as u8, color.into());

        Ok(())
    }
}

#[cfg(feature = "graphics")]
impl<const W: usize, const P: usize> Dimensions for Tile<W, P> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle {
            top_left: Point::new(self.col_offset as i32, self.page_offset as i32 * 8),
            size: Size::new(W as u32, P as u32 * 8),
        }
    }
}
#[cfg(feature = "graphics")]
impl<const W: usize, const P: usize> DrawTarget for Tile<W, P> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let fill = if color.is_on() { 0xff } else { 0 };
        for page in self.pages.iter_mut() {
            page.0.fill(fill);
        }
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels.into_iter().for_each(|p| {
            self.modify_pixel(
                p.0.x as u8 - self.col_offset,
                p.0.y as u8 - self.page_offset * 8,
                p.1.into(),
            )
        });
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let Rectangle {
            top_left: Point { x, mut y },
            size: Size { width, mut height },
        } = area.intersection(&self.bounding_box());

        let op = color.into();
        // unaligned top
        if y % 8 != 0 {
            let mask_height = core::cmp::min(height, 8 - (y as u32 % 8));
            let mask = ((1u8 << mask_height).wrapping_sub(1)) << (y % 8);
            self.pages[y as usize / 8].apply_mask(mask, x as u8, x as u8 + width as u8, op);

            height -= mask_height;
            y += mask_height as i32;
        }

        // aligned center rows
        let fill = if color.is_on() { 0xff } else { 0 };
        // self.fill_solid_aligned(x as u32, y as u32, width, height, fill);
        for page in &mut self.pages[(y as usize / 8)..][..(height as usize / 8)] {
            page.0[x as usize..][..width as usize].fill(fill);
        }

        // bottom unaligned rows
        if (height % 8) != 0 {
            let mask = (1u8 << (height % 8)).wrapping_sub(1);
            let page = (y as u8 + height as u8) / 8;
            self.pages[page as usize].apply_mask(mask, x as u8, x as u8 + width as u8, op);
        }

        Ok(())
    }
}

/// Tiled mode handler
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
pub struct TiledMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    properties: DisplayProperties<DV, DI>,
}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DV, DI> super::displaymode::DisplayModeTrait<DV, DI> for TiledMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    /// Create new GraphicsMode instance
    fn new(properties: DisplayProperties<DV, DI>) -> Self {
        TiledMode { properties }
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
impl<DV, DI> TiledMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: display::DisplayVariant,
{
    /// Draws a page to the screen, at the provided address and column offset
    pub async fn draw_page<const W: usize>(
        &mut self,
        addr: u8,
        col: u8,
        page: &Page<W>,
    ) -> Result<(), DisplayError> {
        self.properties.draw_page(addr, col, &page.0).await
    }

    /// Draws a tile to screen
    pub async fn draw_tile<const W: usize, const P: usize>(
        &mut self,
        tile: &Tile<W, P>,
    ) -> Result<(), DisplayError> {
        for (addr, page) in tile.pages.iter().enumerate() {
            self.draw_page(addr as u8 + tile.page_offset, tile.col_offset, page)
                .await?;
        }
        Ok(())
    }

    /// Clears screen with a provided pattern
    ///
    /// The const parameter is there because generic parameters aren't allowed in const exprs yet,
    /// otherwise it would be inferred from the display width
    pub async fn clear<const W: usize>(&mut self, pattern: u8) -> Result<(), DisplayError> {
        let page = Page::<W>::new(pattern);
        for addr in 0..DV::HEIGHT.div_ceil(8) {
            self.draw_page(addr, 0, &page).await?;
        }
        Ok(())
    }
}
