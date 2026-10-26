//! Windows: HID collections through cfgmgr32 and hid.dll.

mod hid;
mod watch;

pub use hid::{Hid, Path, display, interfaces};
pub use watch::Watcher;

/// Collections open without access rights, so refusal is unexpected.
pub const ACCESS_HINT: &str = "Close other programs that may hold it, such as Razer Synapse.";
