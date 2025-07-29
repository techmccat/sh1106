//! Display variant

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
use display_interface::DisplayError;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

/// Trait to represent a speciffic display
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
pub trait DisplayVariant {
    /// Width of display
    const WIDTH: u8;
    /// Height of display
    const HEIGHT: u8;
    /// Coumn offset
    const COLUMN_OFFSET: u8 = 0;
    /// Large Page Address
    const LARGE_PAGE_ADDRESS: bool = false;

    /// Get integral dimensions from DisplaySize
    fn dimensions() -> (u8, u8) {
        (Self::WIDTH, Self::HEIGHT)
    }

    /// Initialise the display for column mode
    #[allow(async_fn_in_trait)]
    async fn init_column_mode<DI>(iface: &mut DI) -> Result<(), DisplayError>
    where
        DI: AsyncWriteOnlyDataCommand;
}
