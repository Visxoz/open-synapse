//! Pointer speed and the like are the desktop's own settings here, each
//! desktop keeping them its own way, so the app leaves them alone and hides
//! their section.

use crate::platform::Pointer;

pub const AVAILABLE: bool = false;

pub fn read() -> Option<Pointer> {
    None
}

pub fn set_speed(_: u32) {}

pub fn set_precision(_: bool) {}

pub fn set_scroll_lines(_: u32) {}

pub fn set_double_click(_: u32) {}
