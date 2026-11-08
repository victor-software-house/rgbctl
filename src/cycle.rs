//! The write cycle's two pure steps: the report to send, and the check of what
//! the controller reads back. The I/O between them lives in [`crate::run`].

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

/// The controller reads back other bytes than were written.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "the controller read back different bytes at offsets {offsets:?}: wrote {written}, read {read} \
     (bytes 273 to 289)"
)]
pub struct Mismatch {
    /// Offsets that differ, the save flag left out.
    pub offsets: Vec<usize>,
    /// Bytes 273 to 289 as written.
    pub written: String,
    /// Bytes 273 to 289 as read back.
    pub read: String,
}

/// Compare the report read back after a write with the one written.
///
/// # Errors
///
/// [`Mismatch`] naming every offset that differs, the save flag left out.
pub fn verify(written: &FeatureReport, read: &FeatureReport) -> Result<(), Mismatch> {
    let offsets = written.differences(read);
    if offsets.is_empty() {
        Ok(())
    } else {
        Err(Mismatch {
            offsets,
            written: written.owned_hex(),
            read: read.owned_hex(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::report::{Brightness, LIGHTING_BYTES, REPORT_ID, REPORT_LEN};

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
        assert_eq!(mismatch.offsets, [273, 274, 277, 280, 283]);
    }
}
