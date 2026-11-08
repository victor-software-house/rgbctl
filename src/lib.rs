//! rgbctl reads and changes the lighting of an MSI Mystic Light controller
//! (USB `0db0:0076`) through Linux hidraw.
//!
//! The controller keeps its whole lighting state in HID feature report
//! `0x50` ([`report`]). Every write reads that report, changes only the bytes
//! rgbctl owns, writes it back, and reads it again to verify ([`write`]).
//! A write is a preview unless it sets the save flag.
//!
//! The binary is the product; this library exists so its parts are tested
//! and documented one by one.

pub mod cli;
pub mod color;
pub mod config;
pub mod effect;
#[cfg(target_os = "linux")]
pub mod hidraw;
pub mod outcome;
pub mod report;
pub mod run;
pub mod write;
