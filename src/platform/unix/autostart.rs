//! Starting at sign-in, through a file the desktop reads then: an XDG
//! autostart entry on Linux, a launch agent on macOS.

use std::{fs, io};

use super::desktop::{autostart_entry, autostart_file};
use crate::platform::TRAY_ARG;

pub fn enabled() -> bool {
    autostart_file().is_some_and(|f| f.exists())
}

pub fn set(on: bool) -> io::Result<()> {
    let file = autostart_file().ok_or_else(|| io::Error::other("no home folder"))?;
    if !on {
        return match fs::remove_file(&file) {
            // Turning off what was never on.
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        };
    }
    let exe = std::env::current_exe()?;
    let exe = exe
        .to_str()
        .ok_or_else(|| io::Error::other("the app's path is not UTF-8"))?;
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file, autostart_entry(exe, TRAY_ARG))
}
