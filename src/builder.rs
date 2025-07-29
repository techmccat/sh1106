//! Interface factory
//!
//! This is the easiest way to create a driver instance. You can set various parameters of the
//! driver and give it an interface to use. The builder will return a
//! [`mode::RawMode`](../mode/raw/struct.RawMode.html) object which you should coerce to a richer
//! display mode, like [mode::Graphics](../mode/graphics/struct.GraphicsMode.html) for drawing
//! primitives and text.
//!
//! # Examples
//!
//! Connect 180 deg rotation to a 128x128 SH1107 based display:
//!
//! ```rust,no_run
//! use oled_async::{mode::GraphicsMode, Builder};
//! let spi = /* Create an SPI 'device' that implements embedded_hal::SpiDevice  using a HAL of your choice */
//! let di = /*  Use spi to create an interface that implements display_interface::AsyncWriteOnlyDataCommand using a bus that matches your hardware such as display_interface_spi::SPIInterface */
//!
//! let mut raw_display = Builder::new(oled_async::displays::sh1107::Sh1107_128_128 {})
//!         .with_rotation(crate::DisplayRotation::Rotate180)
//!         .connect(di);
//! ```
//!
//! The driver is intended to support multiple chipsets, at least in the SH11xx
//! and SSH13xx families. It is intended to be easy to add additional specific
//! display variants. This can be done by adding to the provided modules in
//! src/displays (please submit a PR) or creating a new display variant out of
//! tree in user crate.
//!
//! The above example will produce a [RawMode](../mode/raw/struct.RawMode.html) instance
//! by default. You need to coerce them into a mode by specifying a type on assignment. For
//! example, to use [`GraphicsMode` mode](../mode/graphics/struct.GraphicsMode.html):
//!
//! ```rust,no_run
//! use oled_async::{mode::GraphicsMode, Builder};
//! let mut display: GraphicsMode<_, _> = raw_display.into();
//! ```

#[cfg(not(feature = "blocking"))]
use display_interface::AsyncWriteOnlyDataCommand;
#[cfg(feature = "blocking")]
use display_interface::WriteOnlyDataCommand;

use hal::digital::OutputPin;

use crate::{
    displayrotation::DisplayRotation,
    mode::{displaymode::DisplayMode, raw::RawMode},
    properties::DisplayProperties,
};

/// Builder struct. Driver options and interface are set using its methods.
///
/// See the [module level documentation](crate::builder) for more details.
#[maybe_async_cfg::maybe(
    sync(
        feature = "blocking",
        keep_self,
        idents(AsyncWriteOnlyDataCommand(sync = "WriteOnlyDataCommand"),)
    ),
    async(not(feature = "blocking"), keep_self)
)]
#[derive(Clone, Copy)]
pub struct Builder<DV> {
    variant: DV,
    rotation: DisplayRotation,
}

impl<DV> Builder<DV> {
    /// Create new builder with a default size of 128 x 64 pixels and no rotation.
    pub fn new(variant: DV) -> Builder<DV> {
        Builder::<DV> {
            variant,
            rotation: DisplayRotation::Rotate0,
        }
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
impl<DV> Builder<DV> {
    /// Set the rotation of the display to one of four values. Defaults to no rotation.
    pub fn with_rotation(self, rotation: DisplayRotation) -> Self {
        Self { rotation, ..self }
    }

    /// Finish the builder and use the given interface to communicate with the display.
    pub fn connect<DI>(self, interface: DI) -> DisplayMode<RawMode<DV, DI>>
    where
        DI: AsyncWriteOnlyDataCommand,
        DV: crate::display::DisplayVariant,
    {
        let properties = DisplayProperties::new(self.variant, interface, self.rotation);
        DisplayMode::<RawMode<DV, DI>>::new(properties)
    }
}

/// Marker type for no reset pin.
#[derive(Clone, Copy)]
pub enum NoOutputPin {}

impl OutputPin for NoOutputPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl hal::digital::ErrorType for NoOutputPin {
    type Error = core::convert::Infallible;
}

#[cfg(test)]
mod tests {
    use super::NoOutputPin;
    use embedded_hal::digital::v2::OutputPin;

    enum SomeError {}

    struct SomeDriver<P: OutputPin<Error = SomeError>> {
        #[allow(dead_code)]
        p: P,
    }

    #[test]
    fn test_output_pin() {
        let p = NoOutputPin::new();
        let _d = SomeDriver { p };

        assert!(true);
    }
}
