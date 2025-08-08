//! Container to store and set display properties

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

use display_interface::{DataFormat, DisplayError};

use crate::{command::Command, display::DisplayVariant, displayrotation::DisplayRotation};

/// Display properties struct
pub struct DisplayProperties<DV, DI> {
    _variant: DV,
    iface: DI,
    display_rotation: DisplayRotation,
    draw_area_start: (u8, u8),
    draw_area_end: (u8, u8),
}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl<DV, DI> DisplayProperties<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
    DV: DisplayVariant,
{
    /// Create new DisplayProperties instance
    pub fn new(
        variant: DV,
        iface: DI,
        display_rotation: DisplayRotation,
    ) -> DisplayProperties<DV, DI> {
        DisplayProperties {
            _variant: variant,
            iface,
            display_rotation,
            draw_area_start: (0, 0),
            draw_area_end: (0, 0),
        }
    }

    /// Initialise the display in column mode (i.e. a byte walks down a column of 8 pixels) with
    /// column 0 on the left and column _(display_width - 1)_ on the right.
    pub async fn init_column_mode(&mut self) -> Result<(), DisplayError> {
        let display_rotation = self.display_rotation;
        DV::init_column_mode(&mut self.iface).await?;
        self.set_rotation(display_rotation).await?;

        Ok(())
    }

    /// Set the position in the framebuffer of the display where any sent data should be
    /// drawn. 
    ///
    /// This method can be used for changing the affected area on the screen
    pub fn set_draw_area(
        &mut self,
        start: (u8, u8),
        end: (u8, u8),
    ) {
        self.draw_area_start = start;
        self.draw_area_end = end;
    }

    /// Send the data to the display for drawing at the current position in the framebuffer
    /// and advance the position accordingly. Cf. `set_draw_area` to modify the affected area by
    /// this method.
    pub async fn draw(&mut self, buffer: &[u8]) -> Result<(), DisplayError> {
        let width = self.draw_area_end.0 - self.draw_area_start.0;
        let base_page = self.draw_area_start.1 / 8;

        for (page_off, chunk) in buffer.chunks(width as usize).enumerate() {
            let page = base_page + page_off as u8;
            self.draw_page(page, self.draw_area_start.0, chunk).await?;
        }

        Ok(())
    }

    /// Draws a subset of a page to screen
    ///
    /// start_col specifies the column offset in screen space, not in page space
    /// so the user doesn't need to offset it themselves
    pub async fn draw_page(
        &mut self,
        page_addr: u8,
        start_col: u8,
        buf: &[u8],
    ) -> Result<(), DisplayError> {
        let start_col = start_col + DV::COLUMN_OFFSET;
        // set page/column addresses
        for cmd in [
            if DV::LARGE_PAGE_ADDRESS {
                Command::LargePageAddress(page_addr)
            } else {
                Command::PageAddress(page_addr)
            },
            Command::ColumnAddressLow(0xF & start_col),
            Command::ColumnAddressHigh(0xF & (start_col >> 4)),
        ] {
            cmd.send(&mut self.iface).await?;
        }

        self.iface.send_data(DataFormat::U8(buf)).await
    }

    // Get the configured display size
    //pub fn get_size(&self) -> DisplaySize {
    //    self.display_size
    //}

    /// Get display dimensions, taking into account the current rotation of the display
    pub fn get_dimensions(&self) -> (u8, u8) {
        let (w, h) = DV::dimensions();

        match self.display_rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => (w, h),
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => (h, w),
        }
    }

    /// Get the display rotation
    pub fn get_rotation(&self) -> DisplayRotation {
        self.display_rotation
    }

    /// Set the display rotation
    pub async fn set_rotation(
        &mut self,
        display_rotation: DisplayRotation,
    ) -> Result<(), DisplayError> {
        self.display_rotation = display_rotation;

        match display_rotation {
            DisplayRotation::Rotate0 => {
                Command::SegmentRemap(true).send(&mut self.iface).await?;
                Command::ReverseComDir(true).send(&mut self.iface).await
            }
            DisplayRotation::Rotate90 => {
                Command::SegmentRemap(false).send(&mut self.iface).await?;
                Command::ReverseComDir(true).send(&mut self.iface).await
            }
            DisplayRotation::Rotate180 => {
                Command::SegmentRemap(false).send(&mut self.iface).await?;
                Command::ReverseComDir(false).send(&mut self.iface).await
            }
            DisplayRotation::Rotate270 => {
                Command::SegmentRemap(true).send(&mut self.iface).await?;
                Command::ReverseComDir(false).send(&mut self.iface).await
            }
        }
    }

    /// Turn the display on or off. The display can be drawn to and retains all
    /// of its memory even while off.
    pub async fn display_on(&mut self, on: bool) -> Result<(), DisplayError> {
        Command::DisplayOn(on).send(&mut self.iface).await
    }

    /// Set the display contrast
    pub async fn set_contrast(&mut self, contrast: u8) -> Result<(), DisplayError> {
        Command::Contrast(contrast).send(&mut self.iface).await
    }
}
