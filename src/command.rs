use display_interface::{AsyncWriteOnlyDataCommand, DataFormat, DisplayError};

/// oled_async Commands

/// Commands
#[derive(Debug)]
#[allow(dead_code)]
pub enum Command {
    /// Set the addressing mode.
    /// `false` is page addressing mode.
    /// `true` is vertical addressing mode.
    AddressMode(bool),
    /// Set contrast. Higher number is higher contrast. Default = 0x7F
    Contrast(u8),
    /// Turn entire display on. If set, all pixels will
    /// be set to on, if not, the value in memory will be used.
    AllOn(bool),
    /// Invert display.
    Invert(bool),
    /// Set display resolution.
    DisplayResolution(u8),
    /// Turn display on or off.
    DisplayOn(bool),
    /// Set column address lower 4 bits
    ColumnAddressLow(u8),
    /// Set column address higher 4 bits
    ColumnAddressHigh(u8),
    /// Set page address
    PageAddress(u8),
    /// Set page address(large variant for sh1108)
    LargePageAddress(u8),
    /// Set display start line from 0-63
    StartLine(u8),
    /// Reverse columns from 127-0
    SegmentRemap(bool),
    /// Set multipex ratio from 15-63 (MUX-1)
    Multiplex(u8),
    /// Set the scan direction of the output.
    /// `false` scans from COM0 to COM[n-1].
    /// `true` scans from COM[n-1] to COM0.
    /// Also known as SetCommonScanDir
    ReverseComDir(bool),
    /// Set vertical shift
    DisplayOffset(u8),
    /// Setup com hardware configuration
    /// First value indicates sequential (false) or alternative (true)
    /// pin configuration.
    ComPinConfig(bool),
    /// Set up display clock.
    /// First value is oscillator frequency, increasing with higher value
    /// Second value is divide ratio - 1
    DisplayClockDiv(u8, u8),
    /// Set up phase 1 and 2 of precharge period. each value is from 0-63
    PreChargePeriod(u8, u8),
    /// Set Vcomh Deselect level
    VcomhDeselect(VcomhLevel),
    /// NOOP
    Noop,
    /// Enable charge pump
    ChargePump(bool),
}

impl Command {
    /// Send command to oled_async
    pub async fn send<DI>(self, iface: &mut DI) -> Result<(), DisplayError>
    where
        DI: AsyncWriteOnlyDataCommand,
    {
        // Transform command into a fixed size array of 2 u8 (seems to be the maximum) and the real length for sending
        let (data, len) = match self {
            Command::AddressMode(mode) => ([0x20 | (mode as u8), 0], 1),
            Command::Contrast(val) => ([0x81, val], 2),
            Command::AllOn(on) => ([0xA4 | (on as u8), 0], 1),
            Command::Invert(inv) => ([0xA6 | (inv as u8), 0], 1),
            Command::DisplayResolution(resolution) => ([0xA9, resolution], 2),
            Command::DisplayOn(on) => ([0xAE | (on as u8), 0], 1),
            Command::ColumnAddressLow(addr) => ([0xF & addr, 0], 1),
            Command::ColumnAddressHigh(addr) => ([0x10 | (0xF & addr), 0], 1),
            Command::PageAddress(page) => ([0xB0 | (page), 0], 1),
            Command::LargePageAddress(page) => ([0xB0, page], 2),
            Command::StartLine(line) => ([0x40 | (0x3F & line), 0], 1),
            Command::SegmentRemap(remap) => ([0xA0 | (remap as u8), 0], 1),
            Command::Multiplex(ratio) => ([0xA8, ratio], 2),
            Command::ReverseComDir(rev) => ([0xC0 | ((rev as u8) << 3), 0], 1),
            Command::DisplayOffset(offset) => ([0xD3, offset], 2),
            Command::ComPinConfig(alt) => ([0xDA, 0x02 | ((alt as u8) << 4)], 2),
            Command::DisplayClockDiv(fosc, div) => ([0xD5, ((0xF & fosc) << 4) | (0xF & div)], 2),
            Command::PreChargePeriod(phase1, phase2) => ([0xD9, ((0xF & phase2) << 4) | (0xF & phase1)], 2),
            Command::VcomhDeselect(level) => ([0xDB, (level as u8) << 4], 2),
            Command::Noop => ([0xE3, 0], 1),
            Command::ChargePump(en) => ([0xAD, 0x8A | (en as u8)], 2),
        };
        // Send command over the interface
        iface.send_commands(DataFormat::U8(&data[..len])).await
    }
}

/// Frame interval
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum NFrames {
    /// 2 Frames
    F2 = 0b111,
    /// 3 Frames
    F3 = 0b100,
    /// 4 Frames
    F4 = 0b101,
    /// 5 Frames
    F5 = 0b000,
    /// 25 Frames
    F25 = 0b110,
    /// 64 Frames
    F64 = 0b001,
    /// 128 Frames
    F128 = 0b010,
    /// 256 Frames
    F256 = 0b011,
}

/// Vcomh Deselect level
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum VcomhLevel {
    /// 0.65 * Vcc
    V065 = 0b001,
    /// 0.77 * Vcc
    V077 = 0b010,
    /// 0.83 * Vcc
    V083 = 0b011,
    /// Auto
    Auto = 0b100,
}
