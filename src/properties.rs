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
    draw_column: u8,
    draw_row: u8,
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
            draw_column: 0,
            draw_row: 0,
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
    /// drawn. This method can be used for changing the affected area on the screen as well
    /// as (re-)setting the start point of the next `draw` call.
    pub async fn set_draw_area(
        &mut self,
        start: (u8, u8),
        end: (u8, u8),
    ) -> Result<(), DisplayError> {
        self.draw_area_start = start;
        self.draw_area_end = end;
        self.draw_column = start.0;
        self.draw_row = start.1;

        self.send_draw_address().await
    }

    /// Send the data to the display for drawing at the current position in the framebuffer
    /// and advance the position accordingly. Cf. `set_draw_area` to modify the affected area by
    /// this method.
    pub async fn draw(&mut self, mut buffer: &[u8]) -> Result<(), DisplayError> {
        while !buffer.is_empty() {
            let count = self.draw_area_end.0 - self.draw_column;
            self.iface
                .send_data(DataFormat::U8(&buffer[..count as usize]))
                .await?;
            self.draw_column += count;

            if self.draw_column >= self.draw_area_end.0 {
                self.draw_column = self.draw_area_start.0;

                self.draw_row += 1;
                if self.draw_row >= self.draw_area_end.1 {
                    self.draw_row = self.draw_area_start.1;
                }

                self.send_draw_address().await?;
            }

            buffer = &buffer[count as usize..];
        }

        Ok(())
    }

    async fn send_draw_address(&mut self) -> Result<(), DisplayError> {
        if DV::LARGE_PAGE_ADDRESS {
            Command::LargePageAddress(self.draw_row.into())
                .send(&mut self.iface)
                .await?;
        } else {
            Command::PageAddress(self.draw_row.into())
                .send(&mut self.iface)
                .await?;
        }
        Command::ColumnAddressLow(0xF & self.draw_column)
            .send(&mut self.iface)
            .await?;
        Command::ColumnAddressHigh(0xF & (self.draw_column >> 4))
            .send(&mut self.iface)
            .await
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
