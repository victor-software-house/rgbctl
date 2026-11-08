//! Colours as the controller stores them: three bytes, red, green, blue.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// One palette colour.
///
/// Written as `#rrggbb`, `rrggbb`, or one of the names in [`NAMED`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Rgb {
    /// Red, 0 to 255.
    pub red: u8,
    /// Green, 0 to 255.
    pub green: u8,
    /// Blue, 0 to 255.
    pub blue: u8,
}

impl Rgb {
    /// All channels off.
    pub const BLACK: Self = Self::new(0, 0, 0);

    /// A colour from its three channels.
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// The three bytes in report order.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }

    /// A colour from three bytes in report order.
    #[must_use]
    pub const fn from_bytes([red, green, blue]: [u8; 3]) -> Self {
        Self::new(red, green, blue)
    }
}

/// The colour names the reference script accepts, with its values, in English.
///
/// Source: the `COLORS` table of `rgb-placa.py` at 166a622. Its yellow and
/// orange are tuned for the LEDs, not the CSS values. The script also accepted
/// `purpura` as a second name for purple; one name per colour is kept.
pub const NAMED: [(&str, Rgb); 9] = [
    ("red", Rgb::new(255, 0, 0)),
    ("green", Rgb::new(0, 255, 0)),
    ("blue", Rgb::new(0, 0, 255)),
    ("white", Rgb::new(255, 255, 255)),
    ("purple", Rgb::new(160, 0, 255)),
    ("cyan", Rgb::new(0, 255, 255)),
    ("yellow", Rgb::new(255, 180, 0)),
    ("orange", Rgb::new(255, 60, 0)),
    ("pink", Rgb::new(255, 0, 120)),
];

/// A colour that is neither a known name nor six hex digits.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown colour `{0}`: use #rrggbb or one of red, green, blue, white, purple, cyan, yellow, orange, pink"
)]
pub struct ParseRgbError(String);

impl FromStr for Rgb {
    type Err = ParseRgbError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let lowered = text.trim().to_ascii_lowercase();
        if let Some((_, rgb)) = NAMED.iter().find(|(name, _)| *name == lowered) {
            return Ok(*rgb);
        }
        let hex = lowered.strip_prefix('#').unwrap_or(&lowered);
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseRgbError(text.to_owned()));
        }
        let channel = |at: usize| {
            hex.get(at..at + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        };
        match (hex.len(), channel(0), channel(2), channel(4)) {
            (6, Some(red), Some(green), Some(blue)) => Ok(Self::new(red, green, blue)),
            _ => Err(ParseRgbError(text.to_owned())),
        }
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

impl From<Rgb> for String {
    fn from(rgb: Rgb) -> Self {
        rgb.to_string()
    }
}

impl TryFrom<String> for Rgb {
    type Error = ParseRgbError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_parses_to_its_table_value() {
        for (name, rgb) in NAMED {
            assert_eq!(name.parse::<Rgb>(), Ok(rgb));
            assert_eq!(name.to_ascii_uppercase().parse::<Rgb>(), Ok(rgb));
        }
    }

    #[test]
    fn hex_parses_with_and_without_hash() {
        assert_eq!("#ff3c00".parse::<Rgb>(), Ok(Rgb::new(255, 60, 0)));
        assert_eq!("FF3C00".parse::<Rgb>(), Ok(Rgb::new(255, 60, 0)));
    }

    #[test]
    fn malformed_colours_are_refused() {
        for text in [
            "", "#fff", "#ff3c0", "#ff3c000", "#gg0000", "purpura", "+f+f+f",
        ] {
            assert!(text.parse::<Rgb>().is_err(), "{text}");
        }
    }

    #[test]
    fn display_round_trips() {
        let rgb = Rgb::new(1, 171, 255);
        assert_eq!(rgb.to_string(), "#01abff");
        assert_eq!(rgb.to_string().parse::<Rgb>(), Ok(rgb));
    }
}
