//! The window manager draws the frame and decides where windows go, and
//! the tray opens a menu (macOS) or may sit anywhere (Linux), so the tray
//! opens the window itself rather than a panel beside it.

use slint::winit_030::WinitWindowAccessor;

pub const TRAY_PANEL: bool = false;

/// The window manager draws the title bar.
pub fn style_title_bar(_: &slint::Window, _: slint::Color) {}

/// Asks for the window to come in front of others, which Wayland grants
/// only as it sees fit.
pub fn bring_to_front(window: &slint::Window) {
    window.with_winit_window(|w| w.focus_window());
}

// The tray panel's, which never shows here.

pub fn set_alpha(_: &slint::Window, _: f32) {}

pub fn popup_origin(_: f32, _: f32) -> Option<(i32, i32)> {
    None
}
