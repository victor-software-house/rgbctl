//! The write cycle's two pure steps: the report to send, and the check of what
//! the controller reads back. The I/O between them lives in [`crate::run`].

use std::fmt;

use crate::effect::Effect;
use crate::report::{FeatureReport, Persistence};

/// The report to send: `before` with the effect's lighting bytes and the save
/// flag set, and every other byte as the controller returned it.
#[must_use]
pub fn plan(before: &FeatureReport, effect: &Effect, persistence: Persistence) -> FeatureReport {
    let mut next = before.clone();
    next.set_lighting(&effect.lighting(before.palette()));
    next.set_persistence(persistence);
    next
}

/// One byte the controller read back differently from what was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Offset in the report, the report id at 0.
    pub offset: usize,
    /// The byte written.
    pub written: u8,
    /// The byte read back.
    pub read: u8,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "offset {}: wrote {:#04x}, read {:#04x}",
            self.offset, self.written, self.read
        )
    }
}

/// The controller reads back other bytes than were written.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the controller read back different bytes: {}", list(differences))]
pub struct Mismatch {
    /// Every byte that differs, the save flag left out, in offset order.
    pub differences: Vec<Difference>,
}

fn list(differences: &[Difference]) -> String {
    differences
        .iter()
        .map(Difference::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Compare the report read back after a write with the one written.
///
/// # Errors
///
/// [`Mismatch`] naming every byte that differs, the save flag left out, with
/// the value written and the value read.
pub fn verify(written: &FeatureReport, read: &FeatureReport) -> Result<(), Mismatch> {
    let differences: Vec<Difference> = written
        .differences(read)
        .into_iter()
        .map(|offset| Difference {
            offset,
            written: written.as_bytes()[offset],
            read: read.as_bytes()[offset],
        })
        .collect();
    if differences.is_empty() {
        Ok(())
    } else {
        Err(Mismatch { differences })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::report::{Brightness, Direction, LIGHTING_BYTES, REPORT_ID, REPORT_LEN, Speed};

    fn saved_off() -> FeatureReport {
        let mut bytes = [0; REPORT_LEN];
        bytes[0] = REPORT_ID;
        bytes[1] = 0x01;
        bytes[14] = 0x03;
        bytes[15] = 0x94;
        bytes[287] = 0xb5;
        FeatureReport::from_bytes(&bytes).unwrap()
    }

    fn red() -> Effect {
        Effect::Static {
            color: Rgb::new(255, 0, 0),
            brightness: Brightness::MAX,
        }
    }

    #[test]
    fn plan_changes_only_the_lighting_bytes_and_the_save_flag() {
        let before = saved_off();
        let next = plan(&before, &red(), Persistence::Preview);
        let changed = next.differences(&before);
        assert_ne!(changed, Vec::<usize>::new());
        assert!(
            changed.iter().all(|at| LIGHTING_BYTES.contains(at)),
            "{changed:?}"
        );
        assert_eq!(next.as_bytes()[289], 0x00);
        assert_eq!(
            plan(&before, &red(), Persistence::Save).as_bytes()[289],
            0x01
        );
    }

    #[test]
    fn plan_writes_the_effect_bytes() {
        let next = plan(&saved_off(), &red(), Persistence::Preview);
        assert_eq!(
            next.owned_hex(),
            "02 ff 00 00 ff 00 00 ff 00 00 ff 00 00 00 b5 00 00"
        );
    }

    /// Bytes 273 to 289 that an MS-7E70 read back after each volatile write
    /// of the 2026-11-08 hardware test, run in this order. Every read-back
    /// verified. Rainbow's look is recorded in the README as unexplained.
    #[test]
    fn plan_writes_what_the_board_read_back() {
        let rgb = |name: &str| name.parse::<Rgb>().unwrap();
        let wave = Effect::Wave {
            colors: vec![rgb("red"), rgb("green"), rgb("blue")],
            brightness: Brightness::MAX,
            speed: Speed::Medium,
            direction: Direction::Right,
        };
        let cases = [
            (red(), "02 ff 00 00 ff 00 00 ff 00 00 ff 00 00 00 b5 00 00"),
            (
                Effect::Breathing {
                    color: rgb("blue"),
                    brightness: Brightness::MAX,
                    speed: Speed::Medium,
                },
                "04 00 00 ff 00 00 ff 00 00 ff 00 00 ff 00 b5 00 00",
            ),
            (
                wave.clone(),
                "01 ff 00 00 00 ff 00 00 00 ff ff 00 00 03 b5 00 00",
            ),
            (
                Effect::Off,
                "00 00 00 00 00 00 00 00 00 00 00 00 00 00 b5 00 00",
            ),
        ];
        for (effect, read_back) in cases {
            let next = plan(&saved_off(), &effect, Persistence::Preview);
            assert_eq!(next.owned_hex(), read_back, "{effect:?}");
        }
        // Rainbow keeps the palette it finds: the board held the wave's.
        let after_wave = plan(&saved_off(), &wave, Persistence::Preview);
        let rainbow = Effect::Rainbow {
            brightness: Brightness::MAX,
            speed: Speed::Medium,
            direction: Direction::Right,
        };
        assert_eq!(
            plan(&after_wave, &rainbow, Persistence::Preview).owned_hex(),
            "05 ff 00 00 00 ff 00 00 00 ff ff 00 00 03 95 00 00"
        );
    }

    #[test]
    fn verify_accepts_an_identical_read_back_whatever_its_save_flag() {
        let written = plan(&saved_off(), &red(), Persistence::Save);
        let read = plan(&saved_off(), &red(), Persistence::Preview);
        assert_eq!(verify(&written, &read), Ok(()));
    }

    #[test]
    fn verify_names_every_offset_that_differs() {
        let written = plan(&saved_off(), &red(), Persistence::Preview);
        let mismatch = verify(&written, &saved_off()).unwrap_err();
        let offsets: Vec<usize> = mismatch.differences.iter().map(|d| d.offset).collect();
        assert_eq!(offsets, [273, 274, 277, 280, 283]);
    }

    #[test]
    fn a_mismatch_outside_the_lighting_bytes_shows_both_values() {
        let written = saved_off();
        let mut bytes = *written.as_bytes();
        bytes[12] = 0x01;
        let read = FeatureReport::from_bytes(&bytes).unwrap();
        let mismatch = verify(&written, &read).unwrap_err();
        assert_eq!(
            mismatch.to_string(),
            "the controller read back different bytes: offset 12: wrote 0x00, read 0x01"
        );
    }
}
