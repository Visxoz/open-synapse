//! JSON files in the data folder: `%APPDATA%\open-synapse` on Windows,
//! `~/.config/open-synapse` on Linux and
//! `~/Library/Application Support/open-synapse` on macOS.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

pub fn data_dir() -> PathBuf {
    crate::platform::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("open-synapse")
}

pub fn load<T: DeserializeOwned + Default>(name: &str) -> T {
    let path = data_dir().join(name);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            log::warn!("ignoring unreadable {}: {e}", path.display());
            T::default()
        }),
        Err(_) => T::default(),
    }
}

/// Writes through a temporary file, so a crash midway leaves the old one whole.
pub fn save(name: &str, value: &impl Serialize) {
    let dir = data_dir();
    let result = std::fs::create_dir_all(&dir).and_then(|()| {
        let json = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        write_atomic(&dir.join(name), &json)
    });
    if let Err(e) = result {
        log::warn!("could not save {name}: {e}");
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let partial = path.with_extension("tmp");
    std::fs::write(&partial, bytes)?;
    std::fs::rename(partial, path)
}
