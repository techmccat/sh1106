//! Raw mode for coercion into richer driver types
//!
//! A display driver instance without high level functionality used as a return type from the
//! builder. Used as a source to coerce the driver into richer modes like
//! [`GraphicsMode`](../graphics/index.html).

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

use crate::{display, mode::displaymode::DisplayModeTrait, properties::DisplayProperties};

/// Raw display mode
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
pub struct RawMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
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
impl<DV, DI> DisplayModeTrait<DV, DI> for RawMode<DV, DI>
where
    DI: AsyncWriteOnlyDataCommand,
{
    /// Create new RawMode instance
    fn new(properties: DisplayProperties<DV, DI>) -> Self {
        RawMode { properties }
    }

    /// Release all resources used by RawMode
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
impl<DV: display::DisplayVariant, DI: AsyncWriteOnlyDataCommand> RawMode<DV, DI> {
    /// Create a new raw display mode
    pub fn new(properties: DisplayProperties<DV, DI>) -> Self {
        RawMode { properties }
    }
}
