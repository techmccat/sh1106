//! SSD1309 display variants and specifics

use crate::display::DisplayVariant;
#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::DisplayError;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

use crate::command::{Command, VcomhLevel};

/// Generic 128x128 with SSD1309 controller
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
#[derive(Debug, Clone, Copy)]
pub struct Ssd1309_128_64 {}

#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
impl DisplayVariant for Ssd1309_128_64 {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;

    async fn init_column_mode<DI>(
        iface: &mut DI,
        //display_rotation: DisplayRotation,
    ) -> Result<(), DisplayError>
    where
        DI: AsyncWriteOnlyDataCommand,
    {
        init_column_mode_common(iface, Self::dimensions()).await?;
        Command::DisplayOffset(0).send(iface).await?;
        Command::ComPinConfig(true).send(iface).await?;

        Ok(())
    }
}

/// Initialise the display in column mode (i.e. a byte walks down a column of 8 pixels) with
/// column 0 on the left and column _(display_width - 1)_ on the right.
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
pub async fn init_column_mode_common<DI>(
    iface: &mut DI,
    dimensions: (u8, u8),
) -> Result<(), DisplayError>
where
    DI: AsyncWriteOnlyDataCommand,
{
    //iface.init().await?;
    // TODO: Break up into nice bits so display modes can pick whathever they need
    let (_, display_height) = dimensions;

    Command::DisplayOn(false).send(iface).await?;
    Command::DisplayClockDiv(0x8, 0x0).send(iface).await?;
    Command::Multiplex(display_height - 1).send(iface).await?;

    Command::StartLine(0).send(iface).await?;
    // TODO: Ability to turn charge pump on/off
    // Display must be off when performing this command
    Command::ChargePump(true).send(iface).await?;

    Command::Contrast(0x80).send(iface).await?;
    Command::PreChargePeriod(0x1, 0xF).send(iface).await?;
    Command::VcomhDeselect(VcomhLevel::Auto).send(iface).await?;
    Command::AllOn(false).send(iface).await?;
    Command::Invert(false).send(iface).await?;
    Command::DisplayOn(true).send(iface).await?;

    Ok(())
}
