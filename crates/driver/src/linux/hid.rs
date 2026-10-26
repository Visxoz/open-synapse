//! Listing hidraw nodes from sysfs and exchanging feature reports with
//! them by `ioctl`.

use std::{
    fs::{self, File, OpenOptions},
    io,
    os::fd::AsRawFd,
    path::PathBuf,
};

use razer_protocol::REPORT_LEN;

use crate::{Interface, descriptor};

const CLASS: &str = "/sys/class/hidraw";

/// A feature report with its ID in front, the only size exchanged.
type Feature = [u8; REPORT_LEN + 1];
const SET_FEATURE: libc::Ioctl = libc::_IOWR::<Feature>(b'H' as u32, 0x06);
const GET_FEATURE: libc::Ioctl = libc::_IOWR::<Feature>(b'H' as u32, 0x07);

/// A hidraw node, such as `/dev/hidraw3`, and the HID device behind it,
/// such as `0003:1532:00E6.0005`. A node's number is reused as soon as it is
/// free, so a mouse plugged straight back in may get the same one; the
/// device's is not reused before a reboot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    node: PathBuf,
    device: String,
}

pub fn display(path: &Path) -> String {
    format!("{} ({})", path.node.display(), path.device)
}

/// Every hidraw node whose vendor is `vid`.
pub fn interfaces(vid: u16) -> Vec<Interface> {
    let nodes = match fs::read_dir(CLASS) {
        Ok(nodes) => nodes,
        Err(e) => {
            log::debug!("cannot list {CLASS}: {e}");
            return Vec::new();
        }
    };
    nodes
        .flatten()
        .filter_map(|node| {
            let device = fs::read_link(node.path().join("device")).ok()?;
            let device = device.file_name()?.to_str()?;
            let (v, pid) = ids(device)?;
            (v == vid).then(|| Interface {
                path: Path {
                    node: PathBuf::from("/dev").join(node.file_name()),
                    device: device.to_owned(),
                },
                pid,
            })
        })
        .collect()
}

/// The vendor and product in a HID device's name, which the kernel writes
/// as bus, vendor, product and a sequence number: `0003:1532:00E6.0005`.
pub(super) fn ids(device: &str) -> Option<(u16, u16)> {
    let mut parts = device.split([':', '.']).skip(1);
    let mut hex = || u16::from_str_radix(parts.next()?, 16).ok();
    Some((hex()?, hex()?))
}

/// An open hidraw node.
pub struct Hid {
    file: File,
}

impl Hid {
    /// Opens the node if its longest feature report is `feature_len` bytes
    /// with the report ID, read from its descriptor beforehand, which sysfs
    /// shows to anyone.
    pub fn open(path: &Path, feature_len: usize) -> io::Result<Option<Self>> {
        let name = path.node.file_name().ok_or(io::ErrorKind::InvalidInput)?;
        let descriptor = fs::read(PathBuf::from(CLASS).join(name).join("device/report_descriptor"))?;
        if descriptor::feature_len(&descriptor) != feature_len {
            return Ok(None);
        }
        let file = OpenOptions::new().read(true).write(true).open(&path.node)?;
        Ok(Some(Self { file }))
    }

    pub fn set_feature(&self, report: &Feature) -> io::Result<()> {
        // SAFETY: `report` is the size the request encodes, and a set only
        // reads it.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), SET_FEATURE, report.as_ptr()) };
        if n < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }

    /// Reads the feature report whose ID is in `report[0]`.
    pub fn get_feature(&self, report: &mut Feature) -> io::Result<()> {
        // SAFETY: `report` is the size the request encodes.
        let n = unsafe { libc::ioctl(self.file.as_raw_fd(), GET_FEATURE, report.as_mut_ptr()) };
        if n < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ids_from_a_device_name() {
        assert_eq!(ids("0003:1532:00E6.0005"), Some((0x1532, 0x00E6)));
        assert_eq!(ids("0005:1532:00AB.000C"), Some((0x1532, 0x00AB)));
        assert_eq!(ids("hidraw3"), None);
    }

    #[test]
    fn encodes_the_feature_requests() {
        // HIDIOCSFEATURE(91) and HIDIOCGFEATURE(91) as <linux/hidraw.h> builds them on x86-64 and ARM64.
        if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
            assert_eq!(SET_FEATURE as u32, 0xC05B_4806);
            assert_eq!(GET_FEATURE as u32, 0xC05B_4807);
        }
    }
}
