//! Razer mice on Windows: discovery, hot-plug notifications and typed
//! access over HID feature reports.

mod device;
mod hid;
mod state;
mod watch;

use razer_protocol::{Model, VENDOR_ID};

pub use device::{Device, Error, Result};
pub use hid::Interface;
pub use state::State;
pub use watch::Watcher;

/// HID collections belonging to known Razer mice; several per mouse, of
/// which `Device::open` accepts the control one.
pub fn candidates() -> Vec<Interface> {
    let mut found = hid::interfaces(VENDOR_ID);
    found.retain(|i| Model::find(i.pid).is_some());
    found
}
