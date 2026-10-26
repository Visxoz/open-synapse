//! Linux and macOS, which share most of what differs from Windows; what
//! they do not is in `linux.rs` and `macos.rs`.

pub mod autostart;
pub mod hash;
pub mod https;
pub mod pointer;
pub mod programs;
pub mod single_instance;
pub mod window;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod desktop;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod desktop;

use std::path::PathBuf;

pub use desktop::{FONTS, config_dir};

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
}
