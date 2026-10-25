//! Starting with Windows, through the current user's Run key.

use std::{io, ptr};

use windows_sys::Win32::{
    Foundation::ERROR_FILE_NOT_FOUND,
    System::Registry::{HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW},
};

use super::wide;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "Open Synapse";
/// Passed when started with Windows, to begin in the tray.
pub const TRAY_ARG: &str = "--tray";

pub fn enabled() -> bool {
    let (key, value) = (wide(RUN_KEY), wide(VALUE));
    let mut size = 0;
    // SAFETY: the strings are NUL-terminated; a null buffer only asks for the size.
    let err = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut size,
        )
    };
    err == 0
}

pub fn set(on: bool) -> io::Result<()> {
    let (key, value) = (wide(RUN_KEY), wide(VALUE));
    let err = if on {
        let exe = std::env::current_exe()?;
        let command = wide(&format!("\"{}\" {TRAY_ARG}", exe.display()));
        // SAFETY: the strings are NUL-terminated and `command` holds the byte count given.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        // SAFETY: the strings are NUL-terminated.
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr()) }
    };
    match err {
        0 => Ok(()),
        // Turning off what was never on.
        ERROR_FILE_NOT_FOUND if !on => Ok(()),
        e => Err(io::Error::from_raw_os_error(e as i32)),
    }
}
