//! HID feature report `0x50`: the controller's whole lighting state.
//!
//! The report is 290 bytes, its id included. rgbctl owns bytes 273 to 287 and
//! the save flag at 289; every other byte is opaque, and a write sends it back
//! exactly as the controller returned it. The layout comes from USB captures
//! of MSI Center, decoded by the reference script `rgb-placa.py`.

use std::fmt;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::color::Rgb;

/// The report id, which is also the first byte of every report.
pub const REPORT_ID: u8 = 0x50;
/// Length of the report in bytes, the id included.
pub const REPORT_LEN: usize = 290;

const MODE: usize = 273;
const PALETTE: usize = 274;
/// Colours in the palette.
pub const PALETTE_SLOTS: usize = 4;
const SUB_PARAMETER: usize = 286;
const STYLE: usize = 287;
const SAVE_FLAG: usize = 289;

/// The bytes rgbctl decodes and writes, besides the save flag.
pub const LIGHTING_BYTES: RangeInclusive<usize> = MODE..=STYLE;

/// A report as the controller returned it, or as rgbctl will send it.
#[derive(Clone, PartialEq, Eq)]
pub struct FeatureReport([u8; REPORT_LEN]);

/// A byte sequence that cannot be report `0x50`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReportError {
    /// The controller returned a different number of bytes.
    #[error("report is {0} bytes; report 0x50 is {REPORT_LEN}")]
    Length(usize),
    /// The first byte is not the report id.
    #[error("report starts with 0x{0:02x}; report 0x50 starts with 0x50")]
    Id(u8),
}

impl FeatureReport {
    /// A zeroed buffer carrying only the report id: what a read asks for.
    #[must_use]
    pub fn request() -> Self {
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = REPORT_ID;
        Self(bytes)
    }

    /// Check the length and the id of bytes read from the controller.
    ///
    /// # Errors
    ///
    /// [`ReportError::Length`] when `bytes` is not 290 long, and
    /// [`ReportError::Id`] when it does not start with `0x50`.
    #[expect(
        clippy::map_err_ignore,
        reason = "TryFromSliceError carries nothing beyond the length the error reports"
    )]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReportError> {
        let array: [u8; REPORT_LEN] = bytes
            .try_into()
            .map_err(|_| ReportError::Length(bytes.len()))?;
        match array[0] {
            REPORT_ID => Ok(Self(array)),
            other => Err(ReportError::Id(other)),
        }
    }

    /// The raw bytes, the id first.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; REPORT_LEN] {
        &self.0
    }

    /// The raw bytes, for an ioctl that fills them.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; REPORT_LEN] {
        &mut self.0
    }

    /// The four palette colours. Every byte value is a colour, so this cannot
    /// fail even when the rest of the lighting bytes do not decode.
    #[must_use]
    pub fn palette(&self) -> [Rgb; PALETTE_SLOTS] {
        std::array::from_fn(|slot| {
            let at = PALETTE + slot * 3;
            Rgb::from_bytes([self.0[at], self.0[at + 1], self.0[at + 2]])
        })
    }

    /// Decode bytes 273 to 287.
    ///
    /// # Errors
    ///
    /// [`DecodeError`] naming the byte and the value no known effect uses.
    pub fn lighting(&self) -> Result<Lighting, DecodeError> {
        Ok(Lighting {
            mode: Mode::try_from(self.0[MODE])?,
            palette: self.palette(),
            sub_parameter: self.0[SUB_PARAMETER],
            style: Style::try_from(self.0[STYLE])?,
        })
    }

    /// Encode `lighting` into bytes 273 to 287, leaving every other byte.
    pub fn set_lighting(&mut self, lighting: &Lighting) {
        self.0[MODE] = lighting.mode.into();
        for (slot, rgb) in lighting.palette.iter().enumerate() {
            let at = PALETTE + slot * 3;
            self.0[at..at + 3].copy_from_slice(&rgb.to_bytes());
        }
        self.0[SUB_PARAMETER] = lighting.sub_parameter;
        self.0[STYLE] = lighting.style.into();
    }

    /// Set the save flag: [`Persistence::Save`] commits the lighting to the
    /// controller's flash, [`Persistence::Preview`] lasts until power-off.
    pub fn set_persistence(&mut self, persistence: Persistence) {
        self.0[SAVE_FLAG] = match persistence {
            Persistence::Preview => 0x00,
            Persistence::Save => 0x01,
        };
    }

    /// Byte offsets where `self` and `other` differ, the save flag left out:
    /// the controller reports it as it likes, and a write sets it.
    #[must_use]
    pub fn differences(&self, other: &Self) -> Vec<usize> {
        (0..REPORT_LEN)
            .filter(|&at| at != SAVE_FLAG && self.0[at] != other.0[at])
            .collect()
    }

    /// Bytes 273 to 289 in hex, for a reader comparing against a capture.
    #[must_use]
    pub fn owned_hex(&self) -> String {
        self.0[MODE..=SAVE_FLAG]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl fmt::Debug for FeatureReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FeatureReport")
            .field(&self.owned_hex())
            .finish()
    }
}

/// Whether a write survives power-off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Persistence {
    /// Shown until power-off; the flash keeps the saved setting.
    Preview,
    /// Committed to the controller's flash.
    Save,
}

/// The decoded lighting bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Lighting {
    /// Byte 273.
    pub mode: Mode,
    /// Bytes 274 to 285. Single-colour effects repeat their colour in all four.
    pub palette: [Rgb; PALETTE_SLOTS],
    /// Byte 286: `0x03` on palette and rainbow effects, `0x00` on static and
    /// breathing. Kept raw: its meaning beyond that is unknown.
    pub sub_parameter: u8,
    /// Byte 287.
    pub style: Style,
}

/// A byte value no known effect uses.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// Byte 273.
    #[error("unknown mode 0x{0:02x} in byte 273")]
    Mode(u8),
    /// Bits 2 to 4 of byte 287.
    #[error("brightness {0} in byte 287 is outside 1 to 5")]
    Brightness(u8),
    /// Bits 0 and 1 of byte 287.
    #[error("speed {0} in byte 287 is outside 0 to 2")]
    Speed(u8),
}

/// Byte 273: the effect the controller runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// `0x00`: LEDs off.
    Off,
    /// `0x01`: colours travel along the strip.
    Wave,
    /// `0x02`: one steady colour.
    Static,
    /// `0x04`: one colour fading in and out.
    Breathing,
    /// `0x05`: the controller's own rainbow.
    Rainbow,
}

impl TryFrom<u8> for Mode {
    type Error = DecodeError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0x00 => Ok(Self::Off),
            0x01 => Ok(Self::Wave),
            0x02 => Ok(Self::Static),
            0x04 => Ok(Self::Breathing),
            0x05 => Ok(Self::Rainbow),
            other => Err(DecodeError::Mode(other)),
        }
    }
}

impl From<Mode> for u8 {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Off => 0x00,
            Mode::Wave => 0x01,
            Mode::Static => 0x02,
            Mode::Breathing => 0x04,
            Mode::Rainbow => 0x05,
        }
    }
}

/// Byte 287, bit by bit:
/// `128 * animate + 64 * direction + 32 * palette + 4 * brightness + speed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Style {
    /// Bit 7: the effect moves. Every effect the reference script sends sets
    /// it.
    pub animate: bool,
    /// Bit 6.
    pub direction: Direction,
    /// Bit 5.
    pub source: Source,
    /// Bits 2 to 4.
    pub brightness: Brightness,
    /// Bits 0 and 1.
    pub speed: Speed,
}

impl TryFrom<u8> for Style {
    type Error = DecodeError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Ok(Self {
            animate: byte & 0x80 != 0,
            direction: if byte & 0x40 == 0 {
                Direction::Right
            } else {
                Direction::Left
            },
            source: if byte & 0x20 == 0 {
                Source::Generated
            } else {
                Source::Palette
            },
            brightness: Brightness::try_from((byte >> 2) & 0x07)?,
            speed: Speed::try_from(byte & 0x03)?,
        })
    }
}

impl From<Style> for u8 {
    fn from(style: Style) -> Self {
        let flag = |on: bool, bit: u8| if on { bit } else { 0 };
        flag(style.animate, 0x80)
            | flag(style.direction == Direction::Left, 0x40)
            | flag(style.source == Source::Palette, 0x20)
            | (style.brightness.get() << 2)
            | u8::from(style.speed)
    }
}

/// Which way a moving effect travels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Right: bit 6 clear.
    #[default]
    Right,
    /// Left: bit 6 set.
    Left,
}

/// Where an effect takes its colours from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Bit 5 set: the palette bytes.
    Palette,
    /// Bit 5 clear: a rainbow the controller generates.
    Generated,
}

/// Brightness level, 1 (dimmest) to 5 (brightest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub struct Brightness(u8);

impl Brightness {
    /// The brightest level, the default.
    pub const MAX: Self = Self(5);

    /// The level, 1 to 5.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self::MAX
    }
}

impl TryFrom<u8> for Brightness {
    type Error = DecodeError;

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        match level {
            1..=5 => Ok(Self(level)),
            other => Err(DecodeError::Brightness(other)),
        }
    }
}

impl From<Brightness> for u8 {
    fn from(brightness: Brightness) -> Self {
        brightness.0
    }
}

impl std::str::FromStr for Brightness {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let level = text
            .parse::<u8>()
            .map_err(|error| format!("brightness `{text}`: {error}"))?;
        Self::try_from(level).map_err(|error| error.to_string())
    }
}

impl fmt::Display for Brightness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// How fast a moving effect runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Speed {
    /// Slowest: 0 in bits 0 and 1.
    Slow,
    /// The default: 1.
    #[default]
    Medium,
    /// Fastest: 2.
    Fast,
}

impl TryFrom<u8> for Speed {
    type Error = DecodeError;

    fn try_from(bits: u8) -> Result<Self, Self::Error> {
        match bits {
            0 => Ok(Self::Slow),
            1 => Ok(Self::Medium),
            2 => Ok(Self::Fast),
            other => Err(DecodeError::Speed(other)),
        }
    }
}

impl From<Speed> for u8 {
    fn from(speed: Speed) -> Self {
        match speed {
            Speed::Slow => 0,
            Speed::Medium => 1,
            Speed::Fast => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hex dump in the read-back probe's format: `offset: byte byte …`.
    fn parse_dump(dump: &str) -> FeatureReport {
        let bytes: Vec<u8> = dump
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(offset, _)| offset.trim().parse::<usize>().is_ok())
            .flat_map(|(_, bytes)| bytes.split_whitespace())
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect();
        FeatureReport::from_bytes(&bytes).unwrap()
    }

    fn saved_off() -> FeatureReport {
        parse_dump(include_str!("../tests/fixtures/report-0x50-off.txt"))
    }

    #[test]
    fn the_saved_off_dump_decodes_as_off_at_full_brightness() {
        let lighting = saved_off().lighting().unwrap();
        assert_eq!(
            lighting,
            Lighting {
                mode: Mode::Off,
                palette: [Rgb::BLACK; 4],
                sub_parameter: 0,
                style: Style {
                    animate: true,
                    direction: Direction::Right,
                    source: Source::Palette,
                    brightness: Brightness::MAX,
                    speed: Speed::Medium,
                },
            }
        );
    }

    #[test]
    fn encoding_the_decoded_lighting_reproduces_the_dump() {
        let report = saved_off();
        let mut written = report.clone();
        written.set_lighting(&report.lighting().unwrap());
        assert_eq!(written, report);
    }

    #[test]
    fn set_lighting_touches_only_bytes_273_to_287() {
        let before = saved_off();
        let mut after = before.clone();
        after.set_lighting(&Lighting {
            mode: Mode::Wave,
            palette: [Rgb::new(255, 255, 255); 4],
            sub_parameter: 0x03,
            style: Style {
                animate: true,
                direction: Direction::Left,
                source: Source::Generated,
                brightness: Brightness::try_from(2).unwrap(),
                speed: Speed::Fast,
            },
        });
        let changed = after.differences(&before);
        assert_ne!(changed, Vec::<usize>::new());
        assert!(
            changed.iter().all(|at| LIGHTING_BYTES.contains(at)),
            "{changed:?}"
        );
    }

    #[test]
    fn persistence_sets_only_byte_289() {
        let before = saved_off();
        let mut saved = before.clone();
        saved.set_persistence(Persistence::Save);
        assert_eq!(saved.as_bytes()[289], 0x01);
        assert_eq!(saved.differences(&before), Vec::<usize>::new());
        saved.set_persistence(Persistence::Preview);
        assert_eq!(saved, before);
    }

    #[test]
    fn the_style_formula_matches_the_reference() {
        for byte in 0..=u8::MAX {
            let brightness = (byte >> 2) & 0x07;
            let speed = byte & 0x03;
            match Style::try_from(byte) {
                Ok(style) => {
                    assert!((1..=5).contains(&brightness) && speed <= 2, "{byte:#04x}");
                    assert_eq!(u8::from(style), byte);
                    let formula = 128 * u8::from(style.animate)
                        + 64 * u8::from(style.direction == Direction::Left)
                        + 32 * u8::from(style.source == Source::Palette)
                        + 4 * style.brightness.get()
                        + u8::from(style.speed);
                    assert_eq!(formula, byte);
                }
                Err(DecodeError::Brightness(level)) => assert_eq!(level, brightness),
                Err(DecodeError::Speed(bits)) => assert_eq!(bits, speed),
                Err(DecodeError::Mode(_)) => panic!("style never reports a mode"),
            }
        }
    }

    #[test]
    fn every_mode_byte_round_trips_and_others_are_refused() {
        for byte in 0..=u8::MAX {
            match Mode::try_from(byte) {
                Ok(mode) => assert_eq!(u8::from(mode), byte),
                Err(error) => {
                    assert!(matches!(byte, 0x03 | 0x06..=0xff), "{byte:#04x}");
                    assert_eq!(error, DecodeError::Mode(byte));
                }
            }
        }
    }

    #[test]
    fn brightness_accepts_one_to_five() {
        assert_eq!(Brightness::try_from(0), Err(DecodeError::Brightness(0)));
        for level in 1..=5 {
            assert_eq!(Brightness::try_from(level).map(Brightness::get), Ok(level));
        }
        assert_eq!(Brightness::try_from(6), Err(DecodeError::Brightness(6)));
    }

    #[test]
    fn a_report_of_the_wrong_shape_is_refused() {
        assert_eq!(
            FeatureReport::from_bytes(&[0x50; 64]),
            Err(ReportError::Length(64))
        );
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = 0x01;
        assert_eq!(
            FeatureReport::from_bytes(&bytes),
            Err(ReportError::Id(0x01))
        );
    }

    #[test]
    fn owned_hex_spans_bytes_273_to_289() {
        assert_eq!(
            saved_off().owned_hex(),
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 b5 00 00"
        );
    }
}
