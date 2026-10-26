//! What is macOS's own: its folders, a launch agent to start at sign-in,
//! and NSWorkspace for the program in front.

use std::path::PathBuf;

use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};

use super::home;
use crate::platform::Fonts;

pub const FONTS: Option<Fonts> = Some(Fonts {
    display: "DIN Alternate",
    body: "",
    mono: "Menlo",
});

/// Every program listed is one the user opened.
pub const SYSTEM: &[&str] = &[];

/// `~/Library/Application Support`.
pub fn config_dir() -> Option<PathBuf> {
    Some(home()?.join("Library/Application Support"))
}

/// Starting at sign-in is a launch agent, which launchd reads at sign-in.
pub fn autostart_file() -> Option<PathBuf> {
    Some(home()?.join("Library/LaunchAgents/io.github.visxoz.open-synapse.plist"))
}

pub fn autostart_entry(exe: &str, arg: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.github.visxoz.open-synapse</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>{arg}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
        escape(exe)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// NSWorkspace, which answers on the main thread, where Slint runs.
pub struct Display;

impl Display {
    pub fn open() -> Option<Self> {
        Some(Self)
    }

    /// The process in front.
    pub fn front(&self) -> Option<u32> {
        let pid = NSWorkspace::sharedWorkspace()
            .frontmostApplication()?
            .processIdentifier();
        u32::try_from(pid).ok()
    }

    /// The program process `pid` runs.
    pub fn name(&self, pid: u32) -> Option<String> {
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(i32::try_from(pid).ok()?)?;
        program(&app)
    }

    /// The programs with a place in the Dock, in no order.
    pub fn programs(&self) -> Vec<String> {
        NSWorkspace::sharedWorkspace()
            .runningApplications()
            .iter()
            .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .filter_map(|app| program(&app))
            .collect()
    }
}

/// An application's lower-case executable name.
fn program(app: &NSRunningApplication) -> Option<String> {
    Some(app.executableURL()?.lastPathComponent()?.to_string().to_lowercase())
}
