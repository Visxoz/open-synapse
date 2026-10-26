//! Linux: HID interfaces through hidraw, found in sysfs.

mod hid;
mod watch;

pub use hid::{Hid, Path, display, interfaces};
pub use watch::Watcher;

/// hidraw nodes belong to root until a udev rule hands them to the user.
pub const ACCESS_HINT: &str = "Install the udev rule that comes with Open Synapse, then plug the mouse in again.";
