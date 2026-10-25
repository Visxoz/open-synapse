//! Windows' own pointer settings, which apply to every mouse.

use std::ffi::c_void;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SPI_GETMOUSE, SPI_GETMOUSESPEED, SPI_GETWHEELSCROLLLINES, SPI_SETDOUBLECLICKTIME, SPI_SETMOUSE, SPI_SETMOUSESPEED,
    SPI_SETWHEELSCROLLLINES, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SystemParametersInfoW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    /// 1 to 20; Windows starts at 10.
    pub speed: u32,
    /// "Enhance pointer precision", Windows' acceleration.
    pub precision: bool,
    pub scroll_lines: u32,
    pub double_click_ms: u32,
}

/// Saves the change and tells running programs about it.
const PERSIST: u32 = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
/// Windows' own acceleration thresholds, restored with precision.
const ACCELERATION_ON: [i32; 3] = [6, 10, 1];
const ACCELERATION_OFF: [i32; 3] = [0, 0, 0];

pub const SPEED_RANGE: (u32, u32) = (1, 20);
pub const DOUBLE_CLICK_RANGE: (u32, u32) = (200, 900);

fn get<T: Default>(action: u32) -> T {
    let mut value = T::default();
    // SAFETY: each query writes exactly one `T` through the pointer.
    unsafe { SystemParametersInfoW(action, 0, (&mut value as *mut T).cast(), 0) };
    value
}

fn set(action: u32, param: u32, data: *mut c_void) {
    // SAFETY: `data` is null, a value passed as a pointer, or points to what
    // the action reads, alive for the call.
    unsafe { SystemParametersInfoW(action, param, data, PERSIST) };
}

pub fn read() -> Pointer {
    let acceleration: [i32; 3] = get(SPI_GETMOUSE);
    Pointer {
        speed: get::<u32>(SPI_GETMOUSESPEED),
        precision: acceleration[2] != 0,
        scroll_lines: get::<u32>(SPI_GETWHEELSCROLLLINES),
        // SAFETY: no arguments.
        double_click_ms: unsafe { GetDoubleClickTime() },
    }
}

pub fn set_speed(speed: u32) {
    // The speed goes in the pointer argument itself.
    set(
        SPI_SETMOUSESPEED,
        0,
        speed.clamp(SPEED_RANGE.0, SPEED_RANGE.1) as usize as *mut c_void,
    );
}

pub fn set_precision(on: bool) {
    let mut acceleration = if on { ACCELERATION_ON } else { ACCELERATION_OFF };
    set(SPI_SETMOUSE, 0, acceleration.as_mut_ptr().cast());
}

pub fn set_scroll_lines(lines: u32) {
    set(SPI_SETWHEELSCROLLLINES, lines.clamp(1, 100), std::ptr::null_mut());
}

pub fn set_double_click(ms: u32) {
    set(
        SPI_SETDOUBLECLICKTIME,
        ms.clamp(DOUBLE_CLICK_RANGE.0, DOUBLE_CLICK_RANGE.1),
        std::ptr::null_mut(),
    );
}
