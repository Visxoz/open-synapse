use serde::{Deserialize, Serialize};

const FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Closing the window leaves the app in the tray.
    pub close_to_tray: bool,
    /// Profiles linked to a program become active while it is in front.
    pub auto_switch: bool,
    /// Draw with Direct3D 12 rather than on the CPU, which needs a fraction of
    /// the memory; applies from the next start.
    pub gpu: bool,
    /// Show each model as Razer pictures it, downloaded once from Razer's site.
    pub pictures: bool,
    pub window: Option<WindowGeometry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            auto_switch: true,
            gpu: false,
            pictures: true,
            window: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        crate::storage::load(FILE)
    }

    pub fn save(&self) {
        crate::storage::save(FILE, self);
    }
}
