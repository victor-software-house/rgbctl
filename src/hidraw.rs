//! The controller's hidraw node: finding it, and reading and writing report
//! `0x50` through the kernel's feature-report ioctls.
//!
//! Plain ioctls keep the binary free of C libraries (hidapi, libusb,
//! libudev), so a static musl build runs on any Linux.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use rustix::ioctl::{Ioctl, IoctlOutput, Opcode, opcode};

use crate::report::{FeatureReport, REPORT_LEN, ReportError};

/// USB vendor id of the controller.
pub const VENDOR_ID: u16 = 0x0db0;
/// USB product id of the controller.
pub const PRODUCT_ID: u16 = 0x0076;

/// The `HID_ID` line the kernel writes in a hidraw device's `uevent`: bus
/// `0003` (USB), then the vendor and product ids.
fn hid_id_line() -> String {
    format!("HID_ID=0003:{VENDOR_ID:08X}:{PRODUCT_ID:08X}")
}

/// Why the controller could not be used.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// No hidraw node belongs to `0db0:0076`.
    #[error(
        "no MSI Mystic Light controller (USB {VENDOR_ID:04x}:{PRODUCT_ID:04x}) under {0}; \
         check that the board has one and that the hid driver bound it"
    )]
    NotFound(PathBuf),
    /// The node exists but did not open read-write.
    #[error(
        "cannot open {path} read-write: {source}; run as root, or install the udev rule \
         from the README"
    )]
    Open {
        /// The node.
        path: PathBuf,
        /// The failure.
        source: io::Error,
    },
    /// Listing `/sys/class/hidraw` failed.
    #[error("cannot list {path}: {source}")]
    List {
        /// The directory.
        path: PathBuf,
        /// The failure.
        source: io::Error,
    },
    /// An ioctl failed.
    #[error("{operation} report 0x50 on {path}: {source}")]
    Ioctl {
        /// `reading` or `writing`.
        operation: &'static str,
        /// The node.
        path: PathBuf,
        /// The failure.
        source: io::Error,
    },
    /// The controller returned bytes that are not report `0x50`.
    #[error("{path} returned {source}")]
    Report {
        /// The node.
        path: PathBuf,
        /// What is wrong with the bytes.
        source: ReportError,
    },
}

/// The hidraw nodes of `0db0:0076`, ordered by node number.
///
/// `sysfs` is normally `/sys`. Each `class/hidraw/hidrawN/device/uevent` that
/// holds the controller's `HID_ID` line yields `/dev/hidrawN`.
///
/// # Errors
///
/// [`DeviceError::List`] when `class/hidraw` cannot be read.
pub fn find(sysfs: &Path) -> Result<Vec<PathBuf>, DeviceError> {
    let class = sysfs.join("class/hidraw");
    let entries = fs::read_dir(&class).map_err(|source| DeviceError::List {
        path: class.clone(),
        source,
    })?;
    let wanted = hid_id_line();
    let mut nodes: Vec<(u32, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let number = name.strip_prefix("hidraw")?.parse::<u32>().ok()?;
            let uevent = fs::read_to_string(entry.path().join("device/uevent")).ok()?;
            uevent
                .lines()
                .any(|line| line.eq_ignore_ascii_case(&wanted))
                .then(|| (number, Path::new("/dev").join(&name)))
        })
        .collect();
    nodes.sort_unstable();
    Ok(nodes.into_iter().map(|(_, path)| path).collect())
}

/// An open controller node.
#[derive(Debug)]
pub struct Controller {
    path: PathBuf,
    file: File,
}

impl Controller {
    /// Open the first node of `0db0:0076` that answers a read of report
    /// `0x50`. A controller may expose more than one hidraw interface.
    ///
    /// # Errors
    ///
    /// [`DeviceError::NotFound`] when no node belongs to the controller; the
    /// last node's error when none answers.
    pub fn open(sysfs: &Path) -> Result<(Self, FeatureReport), DeviceError> {
        let mut last = DeviceError::NotFound(sysfs.join("class/hidraw"));
        for path in find(sysfs)? {
            match Self::open_node(path).and_then(|controller| {
                let report = controller.read()?;
                Ok((controller, report))
            }) {
                Ok(opened) => return Ok(opened),
                Err(error) => last = error,
            }
        }
        Err(last)
    }

    fn open_node(path: PathBuf) -> Result<Self, DeviceError> {
        match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => Ok(Self { path, file }),
            Err(source) => Err(DeviceError::Open { path, source }),
        }
    }

    /// The node, such as `/dev/hidraw0`.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read report `0x50` as the controller holds it now.
    ///
    /// # Errors
    ///
    /// [`DeviceError::Ioctl`] when the kernel refuses, and
    /// [`DeviceError::Report`] when the reply is short or not report `0x50`.
    pub fn read(&self) -> Result<FeatureReport, DeviceError> {
        let mut report = FeatureReport::request();
        let length = feature(&self.file, GET_FEATURE, report.as_bytes_mut())
            .map_err(|source| self.ioctl_error("reading", source))?;
        let bytes = report.as_bytes();
        FeatureReport::from_bytes(bytes.get(..length).unwrap_or(bytes)).map_err(|source| {
            DeviceError::Report {
                path: self.path.clone(),
                source,
            }
        })
    }

    /// Send `report` as it is.
    ///
    /// # Errors
    ///
    /// [`DeviceError::Ioctl`] when the kernel refuses.
    pub fn write(&self, report: &FeatureReport) -> Result<(), DeviceError> {
        let mut bytes = *report.as_bytes();
        feature(&self.file, SET_FEATURE, &mut bytes)
            .map(drop)
            .map_err(|source| self.ioctl_error("writing", source))
    }

    fn ioctl_error(&self, operation: &'static str, source: io::Error) -> DeviceError {
        DeviceError::Ioctl {
            operation,
            path: self.path.clone(),
            source,
        }
    }
}

/// `HIDIOCGFEATURE(290)`: `_IOWR('H', 0x07, len)`, from `linux/hidraw.h`.
const GET_FEATURE: Opcode = opcode::read_write::<[u8; REPORT_LEN]>(b'H', 0x07);
/// `HIDIOCSFEATURE(290)`: `_IOWR('H', 0x06, len)`, from `linux/hidraw.h`.
const SET_FEATURE: Opcode = opcode::read_write::<[u8; REPORT_LEN]>(b'H', 0x06);

/// A feature-report ioctl over a 290-byte buffer. rustix's `Updater` discards
/// the ioctl's return value, which for these two is the byte count.
struct Feature<'a> {
    opcode: Opcode,
    buffer: &'a mut [u8; REPORT_LEN],
}

// SAFETY: both opcodes encode the buffer's exact size, `REPORT_LEN`, so the
// kernel reads and writes at most `REPORT_LEN` bytes through the pointer,
// which stays valid and exclusively borrowed for the call.
#[expect(
    unsafe_code,
    reason = "implementing rustix's Ioctl is the only way to keep the byte count"
)]
unsafe impl Ioctl for Feature<'_> {
    type Output = usize;

    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        self.opcode
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.buffer.as_mut_ptr().cast()
    }

    #[expect(unsafe_code, reason = "the trait declares this method unsafe")]
    unsafe fn output_from_ptr(
        output: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<usize> {
        usize::try_from(output).ok().ok_or(rustix::io::Errno::RANGE)
    }
}

/// Run one feature-report ioctl and return the byte count.
#[expect(
    unsafe_code,
    reason = "no safe API issues hidraw feature-report ioctls; this is the one call site"
)]
fn feature(file: &File, opcode: Opcode, buffer: &mut [u8; REPORT_LEN]) -> io::Result<usize> {
    // SAFETY: `Feature` passes a pointer to a live, exclusively borrowed
    // buffer of exactly the size each opcode encodes.
    unsafe { rustix::ioctl::ioctl(file, Feature { opcode, buffer }) }.map_err(io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opcodes_match_the_kernel_macros() {
        assert_eq!(u64::from(GET_FEATURE), 0xc122_4807);
        assert_eq!(u64::from(SET_FEATURE), 0xc122_4806);
    }

    #[test]
    fn find_returns_the_controller_nodes_in_numeric_order() {
        let sysfs = tempfile::tempdir().unwrap();
        let node = |name: &str, uevent: &str| {
            let dir = sysfs.path().join("class/hidraw").join(name).join("device");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("uevent"), uevent).unwrap();
        };
        let ours = "DRIVER=hid-generic\nHID_ID=0003:00000DB0:00000076\nHID_NAME=MSI MYSTIC LIGHT\n";
        node("hidraw10", ours);
        node("hidraw2", ours);
        node("hidraw0", "HID_ID=0003:0000046D:0000C52B\n");
        node("hidraw3", "HID_ID=0003:00000DB0:00000077\n");
        assert_eq!(
            find(sysfs.path()).unwrap(),
            [
                PathBuf::from("/dev/hidraw2"),
                PathBuf::from("/dev/hidraw10")
            ]
        );
    }

    #[test]
    fn no_controller_is_reported_by_open() {
        let sysfs = tempfile::tempdir().unwrap();
        fs::create_dir_all(sysfs.path().join("class/hidraw")).unwrap();
        assert!(matches!(
            Controller::open(sysfs.path()),
            Err(DeviceError::NotFound(_))
        ));
    }
}
