use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::{
            Dwm::{DWMWA_CAPTION_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
            Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        },
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::{
                FindWindowW, GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, GetWindowRect, IsWindowVisible, LWA_ALPHA,
                SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, WS_EX_LAYERED,
            },
        },
    },
    core::w,
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

/// Makes the window `alpha` opaque, 0 to 1, as a layered window. Only for
/// windows drawn through GDI, as the software renderer does: Direct3D's
/// flip-model swap chains cannot present to a layered window.
pub fn set_alpha(hwnd: HWND, alpha: f32) {
    let layered = WS_EX_LAYERED as isize;
    // SAFETY: `hwnd` is a live window.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if style & layered == 0 {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | layered);
        }
        SetLayeredWindowAttributes(hwnd, 0, (alpha * 255.).round() as u8, LWA_ALPHA);
    }
}

const EMPTY: RECT = RECT {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};

/// The edge of the screen the taskbar sits on.
#[derive(Clone, Copy)]
enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

impl Edge {
    /// The taskbar is on the edge where the work area falls short of the
    /// screen; none when it is hidden.
    fn of_taskbar(screen: &RECT, work: &RECT) -> Option<Self> {
        if work.bottom < screen.bottom {
            Some(Self::Bottom)
        } else if work.top > screen.top {
            Some(Self::Top)
        } else if work.left > screen.left {
            Some(Self::Left)
        } else if work.right < screen.right {
            Some(Self::Right)
        } else {
            None
        }
    }
}

/// Where the taskbar's hidden-icons flyout is while it is open: it stays
/// open after a click on an icon in it.
fn overflow_flyout() -> Option<RECT> {
    // Windows 11's class, then Windows 10's.
    [
        w!("TopLevelWindowForOverflowXamlIsland"),
        w!("NotifyIconOverflowWindow"),
    ]
    .into_iter()
    .find_map(|class| {
        let mut rect = EMPTY;
        // SAFETY: `class` is a NUL-terminated literal and `rect` outlives
        // the call; a null handle, when the window is missing, fails both
        // checks.
        let open = unsafe {
            let hwnd = FindWindowW(class, std::ptr::null());
            IsWindowVisible(hwnd) != 0 && GetWindowRect(hwnd, &mut rect) != 0
        };
        (open && rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
    })
}

/// Where a pop-up of `width` by `height` logical pixels goes, in physical
/// pixels: in the corner at the tray's end of the taskbar on the pointer's
/// screen, as Windows' own flyouts do, past the hidden-icons flyout while
/// that is open, or by the pointer when no taskbar is showing.
pub fn popup_origin(width: f32, height: f32) -> Option<(i32, i32)> {
    let mut cursor = POINT { x: 0, y: 0 };
    // SAFETY: `cursor` outlives the call.
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return None;
    }
    // SAFETY: any point has a nearest monitor.
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: EMPTY,
        rcWork: EMPTY,
        dwFlags: 0,
    };
    // SAFETY: `info` carries its size and outlives the call.
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    let (mut dpi, mut dpi_y) = (96u32, 96u32);
    // SAFETY: both out-pointers outlive the call.
    unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) };
    let scale = dpi.max(1) as f32 / 96.0;
    let px = |logical: f32| (logical * scale).round() as i32;
    let (w, h, margin) = (px(width), px(height), px(12.0));

    // The free area is the work area less the flyout, which hugs the taskbar.
    let edge = Edge::of_taskbar(&info.rcMonitor, &info.rcWork);
    let mut free = info.rcWork;
    if let Some(edge) = edge
        && let Some(f) = overflow_flyout()
    {
        match edge {
            Edge::Bottom => free.bottom = free.bottom.min(f.top),
            Edge::Top => free.top = free.top.max(f.bottom),
            Edge::Left => free.left = free.left.max(f.right),
            Edge::Right => free.right = free.right.min(f.left),
        }
    }
    let (left, right) = (free.left + margin, free.right - w - margin);
    let (top, bottom) = (free.top + margin, free.bottom - h - margin);
    let (x, y) = match edge {
        Some(Edge::Bottom | Edge::Right) => (right, bottom),
        Some(Edge::Top) => (right, top),
        Some(Edge::Left) => (left, bottom),
        None => (cursor.x - w / 2, cursor.y - h / 2),
    };
    Some((x.clamp(left, right.max(left)), y.clamp(top, bottom.max(top))))
}
