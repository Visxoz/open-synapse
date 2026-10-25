//! Which programs are running, and which one is in front.

use std::{cell::RefCell, path::Path};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM},
    System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    },
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            EVENT_SYSTEM_FOREGROUND, EnumWindows, GW_OWNER, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId,
            IsWindowVisible, WINEVENT_OUTOFCONTEXT,
        },
    },
};

fn lower_file_name(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_string_lossy().to_lowercase())
}

/// This program's own lower-case file name.
pub fn own_exe() -> Option<String> {
    lower_file_name(&std::env::current_exe().ok()?)
}

/// The lower-case file name of the program that owns `hwnd`.
fn exe_of(hwnd: HWND) -> Option<String> {
    let mut pid = 0;
    // SAFETY: `pid` is a valid out pointer; a stale `hwnd` just yields 0.
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == 0 {
        return None;
    }
    // SAFETY: plain arguments; the handle is checked and closed below.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: `buf` holds `len` u16s and `process` is open until closed here.
    let ok = unsafe {
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        ok
    };
    if ok == 0 {
        return None;
    }
    lower_file_name(Path::new(&String::from_utf16_lossy(&buf[..len as usize])))
}

/// Parts of Windows that own visible windows but are not programs anyone links.
const SYSTEM: &[&str] = &[
    "applicationframehost.exe",
    "explorer.exe",
    "searchhost.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "systemsettings.exe",
    "textinputhost.exe",
];

/// Programs with a visible top-level window, sorted, without this one.
pub fn running() -> Vec<String> {
    unsafe extern "system" fn visit(hwnd: HWND, list: LPARAM) -> i32 {
        // SAFETY: `list` is the `Vec` passed to EnumWindows below, which
        // only calls back during that call.
        let list = unsafe { &mut *(list as *mut Vec<String>) };
        // SAFETY: `hwnd` comes from EnumWindows.
        let shown = unsafe {
            IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() && GetWindowTextLengthW(hwnd) > 0
        };
        if shown && let Some(exe) = exe_of(hwnd) {
            list.push(exe);
        }
        1
    }
    let mut list: Vec<String> = Vec::new();
    // SAFETY: `visit` only uses `list` while EnumWindows runs.
    unsafe { EnumWindows(Some(visit), &mut list as *mut Vec<String> as LPARAM) };
    let own = own_exe();
    list.retain(|e| Some(e) != own.as_ref() && !SYSTEM.contains(&e.as_str()));
    list.sort_unstable();
    list.dedup();
    list
}

type OnForeground = Box<dyn Fn(String)>;

thread_local! {
    static ON_FOREGROUND: RefCell<Option<OnForeground>> = const { RefCell::new(None) };
}

/// Calls back on this thread with the program that comes to the front,
/// until dropped. Needs the thread's message loop, which Slint runs.
pub struct ForegroundHook(HWINEVENTHOOK);

impl ForegroundHook {
    pub fn new(callback: impl Fn(String) + 'static) -> Option<Self> {
        unsafe extern "system" fn changed(_: HWINEVENTHOOK, _: u32, hwnd: HWND, _: i32, _: i32, _: u32, _: u32) {
            if let Some(exe) = exe_of(hwnd) {
                ON_FOREGROUND.with_borrow(|f| f.as_ref().map(|f| f(exe)));
            }
        }
        // SAFETY: `changed` is a valid callback for the life of the hook,
        // and out-of-context hooks run on this thread.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                std::ptr::null_mut(),
                Some(changed),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if hook.is_null() {
            return None;
        }
        // Events reach `changed` through this thread's message loop, so none
        // can arrive before the callback is in place.
        ON_FOREGROUND.set(Some(Box::new(callback)));
        Some(Self(hook))
    }
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        // SAFETY: the hook came from SetWinEventHook and is removed once.
        unsafe { UnhookWinEvent(self.0) };
        ON_FOREGROUND.set(None);
    }
}
