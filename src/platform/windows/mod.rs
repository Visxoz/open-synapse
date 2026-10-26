//! Windows integration.

pub mod autostart;
pub mod hash;
pub mod https;
pub mod pointer;
pub mod programs;
pub mod single_instance;
pub mod window;

use std::path::PathBuf;

/// `theme.slint`'s own.
pub const FONTS: Option<super::Fonts> = None;

/// The folder for programs' settings: `%APPDATA%`.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
