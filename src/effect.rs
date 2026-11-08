//! The effects an operator asks for, and the lighting bytes each one writes.
//!
//! The byte values follow what the reference script sends for each command.

use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::report::{Brightness, Direction, Lighting, Mode, PALETTE_SLOTS, Source, Speed, Style};

/// One lighting effect with its settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    /// LEDs off.
    Off,
    /// One steady colour.
    Static {
        /// The colour.
        color: Rgb,
        /// 1 to 5.
        #[serde(default)]
        brightness: Brightness,
    },
    /// One colour fading in and out.
    Breathing {
        /// The colour.
        color: Rgb,
        /// 1 to 5.
        #[serde(default)]
        brightness: Brightness,
        /// How fast it fades.
        #[serde(default)]
        speed: Speed,
    },
    /// Colours travelling along the strip.
    Wave {
        /// One to four colours. None means the controller's generated rainbow,
        /// and the palette bytes keep what they held.
        #[serde(default)]
        colors: Vec<Rgb>,
        /// 1 to 5.
        #[serde(default)]
        brightness: Brightness,
        /// How fast it travels.
        #[serde(default)]
        speed: Speed,
        /// Which way it travels.
        #[serde(default)]
        direction: Direction,
    },
    /// The controller's own rainbow. The palette bytes keep what they held.
    Rainbow {
        /// 1 to 5.
        #[serde(default)]
        brightness: Brightness,
        /// How fast it cycles.
        #[serde(default)]
        speed: Speed,
        /// Which way it travels.
        #[serde(default)]
        direction: Direction,
    },
}

/// A wave given more colours than the palette holds.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a wave takes at most {PALETTE_SLOTS} colours, not {0}")]
pub struct TooManyColors(pub usize);

impl Effect {
    /// Refuse settings the controller cannot hold.
    ///
    /// # Errors
    ///
    /// [`TooManyColors`] for a wave of more than four colours.
    pub fn check(&self) -> Result<(), TooManyColors> {
        match self {
            Self::Wave { colors, .. } if colors.len() > PALETTE_SLOTS => {
                Err(TooManyColors(colors.len()))
            }
            Self::Off
            | Self::Static { .. }
            | Self::Breathing { .. }
            | Self::Wave { .. }
            | Self::Rainbow { .. } => Ok(()),
        }
    }

    /// The lighting bytes this effect writes. `current` is the palette the
    /// controller holds now, which a generated-colour effect leaves in place.
    #[must_use]
    pub fn lighting(&self, current: [Rgb; PALETTE_SLOTS]) -> Lighting {
        let style = |brightness, speed, direction, source| Style {
            animate: true,
            direction,
            source,
            brightness,
            speed,
        };
        match self {
            Self::Off => Lighting {
                mode: Mode::Off,
                palette: [Rgb::BLACK; PALETTE_SLOTS],
                sub_parameter: 0x00,
                style: style(
                    Brightness::MAX,
                    Speed::Medium,
                    Direction::Right,
                    Source::Palette,
                ),
            },
            Self::Static { color, brightness } => Lighting {
                mode: Mode::Static,
                palette: [*color; PALETTE_SLOTS],
                sub_parameter: 0x00,
                style: style(
                    *brightness,
                    Speed::Medium,
                    Direction::Right,
                    Source::Palette,
                ),
            },
            Self::Breathing {
                color,
                brightness,
                speed,
            } => Lighting {
                mode: Mode::Breathing,
                palette: [*color; PALETTE_SLOTS],
                sub_parameter: 0x00,
                style: style(*brightness, *speed, Direction::Right, Source::Palette),
            },
            Self::Wave {
                colors,
                brightness,
                speed,
                direction,
            } => {
                let (palette, source) = match fill_palette(colors) {
                    Some(palette) => (palette, Source::Palette),
                    None => (current, Source::Generated),
                };
                Lighting {
                    mode: Mode::Wave,
                    palette,
                    sub_parameter: 0x03,
                    style: style(*brightness, *speed, *direction, source),
                }
            }
            Self::Rainbow {
                brightness,
                speed,
                direction,
            } => Lighting {
                mode: Mode::Rainbow,
                palette: current,
                sub_parameter: 0x03,
                style: style(*brightness, *speed, *direction, Source::Generated),
            },
        }
    }
}

/// Repeat `colors` in order until all four slots hold one; `None` for no
/// colours.
fn fill_palette(colors: &[Rgb]) -> Option<[Rgb; PALETTE_SLOTS]> {
    let mut cycle = colors.iter().copied().cycle();
    let first = cycle.next()?;
    let mut palette = [first; PALETTE_SLOTS];
    for slot in palette.iter_mut().skip(1) {
        *slot = cycle.next().unwrap_or(first);
    }
    Some(palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb::new(255, 0, 0);
    const BLUE: Rgb = Rgb::new(0, 0, 255);
    const HELD: [Rgb; 4] = [
        Rgb::new(1, 2, 3),
        Rgb::new(4, 5, 6),
        Rgb::new(7, 8, 9),
        Rgb::new(10, 11, 12),
    ];

    fn bytes(effect: &Effect) -> (u8, u8, u8) {
        let lighting = effect.lighting(HELD);
        (
            lighting.mode.into(),
            lighting.sub_parameter,
            lighting.style.into(),
        )
    }

    #[test]
    fn off_writes_what_the_saved_off_dump_holds() {
        assert_eq!(bytes(&Effect::Off), (0x00, 0x00, 0xb5));
        assert_eq!(Effect::Off.lighting(HELD).palette, [Rgb::BLACK; 4]);
    }

    #[test]
    fn static_repeats_its_colour_in_every_slot() {
        let effect = Effect::Static {
            color: RED,
            brightness: Brightness::try_from(3).unwrap(),
        };
        assert_eq!(bytes(&effect), (0x02, 0x00, 0x80 | 0x20 | (3 << 2) | 1));
        assert_eq!(effect.lighting(HELD).palette, [RED; 4]);
    }

    #[test]
    fn breathing_carries_its_speed() {
        let effect = Effect::Breathing {
            color: BLUE,
            brightness: Brightness::MAX,
            speed: Speed::Fast,
        };
        assert_eq!(bytes(&effect), (0x04, 0x00, 0x80 | 0x20 | (5 << 2) | 2));
    }

    #[test]
    fn a_wave_of_colours_reads_the_palette() {
        let effect = Effect::Wave {
            colors: vec![RED, BLUE],
            brightness: Brightness::MAX,
            speed: Speed::Slow,
            direction: Direction::Left,
        };
        assert_eq!(bytes(&effect), (0x01, 0x03, 0x80 | 0x40 | 0x20 | (5 << 2)));
        assert_eq!(effect.lighting(HELD).palette, [RED, BLUE, RED, BLUE]);
    }

    #[test]
    fn a_wave_without_colours_generates_them_and_keeps_the_palette() {
        let effect = Effect::Wave {
            colors: vec![],
            brightness: Brightness::MAX,
            speed: Speed::Medium,
            direction: Direction::Right,
        };
        assert_eq!(bytes(&effect), (0x01, 0x03, 0x80 | (5 << 2) | 1));
        assert_eq!(effect.lighting(HELD).palette, HELD);
    }

    #[test]
    fn rainbow_generates_colours_and_keeps_the_palette() {
        let effect = Effect::Rainbow {
            brightness: Brightness::MAX,
            speed: Speed::Fast,
            direction: Direction::Left,
        };
        assert_eq!(bytes(&effect), (0x05, 0x03, 0x80 | 0x40 | (5 << 2) | 2));
        assert_eq!(effect.lighting(HELD).palette, HELD);
    }

    #[test]
    fn fill_palette_cycles_in_order() {
        assert_eq!(fill_palette(&[]), None);
        assert_eq!(fill_palette(&[RED]), Some([RED; 4]));
        assert_eq!(
            fill_palette(&[RED, BLUE, Rgb::BLACK]),
            Some([RED, BLUE, Rgb::BLACK, RED])
        );
        assert_eq!(fill_palette(&HELD), Some(HELD));
    }

    #[test]
    fn a_wave_of_five_colours_is_refused() {
        let effect = Effect::Wave {
            colors: vec![RED; 5],
            brightness: Brightness::MAX,
            speed: Speed::Medium,
            direction: Direction::Right,
        };
        assert_eq!(effect.check(), Err(TooManyColors(5)));
    }
}
