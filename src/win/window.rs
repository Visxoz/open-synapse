use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
    UI::WindowsAndMessaging::SetForegroundWindow,
};

pub fn hwnd(window: &slint::Window) -> Option<HWND> {
    let handle = window.window_handle();
    match handle.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as HWND),
        _ => None,
    }
}

/// A dark title bar in the window's own background colour, so it reads as
/// part of the app.
pub fn style_title_bar(hwnd: HWND, background: slint::Color) {
    let dark: i32 = 1;
    let colour: u32 =
        u32::from(background.red()) | u32::from(background.green()) << 8 | u32::from(background.blue()) << 16;
    // SAFETY: `hwnd` is a live window and each value outlives its call.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&dark as *const i32).cast(),
            4,
        );
        DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR as u32, (&colour as *const u32).cast(), 4);
    }
}

/// Brings the window in front of others, which Windows allows once the
/// launch that asked for it has passed on its right to.
pub fn bring_to_front(hwnd: HWND) {
    // SAFETY: `hwnd` is a live window.
    unsafe { SetForegroundWindow(hwnd) };
}
