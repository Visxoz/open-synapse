//! macOS: HID interfaces through I/O Kit.

mod hid;
mod watch;

pub use hid::{Hid, Path, display, interfaces};
pub use watch::Watcher;

/// macOS asks before letting a program open devices that type.
pub const ACCESS_HINT: &str = "Allow Open Synapse under System Settings ▸ Privacy & Security ▸ Input Monitoring.";
