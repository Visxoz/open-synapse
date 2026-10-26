//! Razer mice on Windows, Linux and macOS: discovery, hot-plug
//! notifications and typed access over HID feature reports.

#[cfg(any(target_os = "linux", target_os = "macos", test))]
mod descriptor;
mod device;
mod state;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod sys;
#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod sys;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod sys;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
compile_error!("Open Synapse runs on Windows, Linux and macOS");

use razer_protocol::{Model, VENDOR_ID};

pub use device::{Device, Error, Result};
pub use state::State;
pub use sys::{ACCESS_HINT, Watcher};

/// A HID interface, or on Windows a collection within one, and the USB
/// product it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    path: sys::Path,
    pub pid: u16,
}

impl Interface {
    pub fn display_path(&self) -> String {
        sys::display(&self.path)
    }
}

/// HID interfaces belonging to known Razer mice; several per mouse, of
/// which `Device::open` accepts the control one.
pub fn candidates() -> Vec<Interface> {
    let mut found = sys::interfaces(VENDOR_ID);
    found.retain(|i| Model::find(i.pid).is_some());
    found
}
