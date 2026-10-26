//! What differs between Windows, Linux and macOS, behind the same names:
//! starting at sign-in, keeping one copy running, HTTPS, hashing, the
//! program in front, the window's native touches and the system's pointer
//! settings.

use std::path::Path;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

/// Passed when started at sign-in, to begin in the tray.
pub const TRAY_ARG: &str = "--tray";

/// The typefaces the UI is set in, where `ui/theme.slint`'s, which ship
/// with Windows, are missing.
pub struct Fonts {
    /// DIN-style: numbers, labels and headings.
    pub display: &'static str,
    pub body: &'static str,
    pub mono: &'static str,
}

/// The system's pointer settings, which apply to every mouse: Windows'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    /// 1 to 20; Windows starts at 10.
    pub speed: u32,
    /// "Enhance pointer precision", Windows' acceleration.
    pub precision: bool,
    pub scroll_lines: u32,
    pub double_click_ms: u32,
}

/// A path's lower-case file name, as profiles link programs by.
fn file_name_lower(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_string_lossy().to_lowercase())
}

/// This program's own lower-case file name.
pub fn own_exe() -> Option<String> {
    file_name_lower(&std::env::current_exe().ok()?)
}

/// The programs in `list`, sorted and once each, without this one or any of
/// `system`'s, which are parts of the desktop rather than programs to link.
fn linkable(mut list: Vec<String>, system: &[&str]) -> Vec<String> {
    let own = own_exe();
    list.retain(|e| Some(e) != own.as_ref() && !system.contains(&e.as_str()));
    list.sort_unstable();
    list.dedup();
    list
}
