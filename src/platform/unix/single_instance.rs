//! Keeps one copy of the app running. The first holds a lock on a file,
//! which the system lets go of however it exits, and listens on a socket
//! beside it; a second launch connects to ask it to show itself, and exits.

use std::{
    fs::{self, File, TryLockError},
    io::Write,
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
};

/// Holds the lock until dropped.
pub struct Guard {
    lock: Option<File>,
    socket: PathBuf,
    listener: Option<UnixListener>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if self.lock.is_some() {
            let _ = fs::remove_file(&self.socket);
        }
    }
}

impl Guard {
    /// Calls `f` on a thread of its own each time another launch asks this
    /// copy to show itself.
    pub fn on_activate(&mut self, f: impl Fn() + Send + 'static) {
        let Some(listener) = self.listener.take() else { return };
        std::thread::Builder::new()
            .name("activate".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stream.is_ok() {
                        f();
                    }
                }
            })
            .expect("activation thread starts");
    }
}

/// Returns `None` if another copy is running, after asking it to show itself.
pub fn acquire() -> Option<Guard> {
    // The user's own runtime folder, else their data folder, as on macOS;
    // never one other users share.
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(crate::storage::data_dir, PathBuf::from);
    let _ = fs::create_dir_all(&dir);
    acquire_in(&dir, "open-synapse")
}

fn acquire_in(dir: &Path, name: &str) -> Option<Guard> {
    let socket = dir.join(format!("{name}.sock"));
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(format!("{name}.lock")))
        .map_err(TryLockError::Error)
        .and_then(|file| file.try_lock().map(|()| file));
    match lock {
        Ok(lock) => {
            // Left behind by a copy that did not exit cleanly.
            let _ = fs::remove_file(&socket);
            let listener = UnixListener::bind(&socket)
                .inspect_err(|e| log::warn!("cannot listen for other launches: {e}"))
                .ok();
            Some(Guard {
                lock: Some(lock),
                socket,
                listener,
            })
        }
        Err(TryLockError::WouldBlock) => {
            if let Ok(mut other) = UnixStream::connect(&socket) {
                let _ = other.write_all(b"\n");
            }
            None
        }
        // Without a lock, this copy runs regardless.
        Err(TryLockError::Error(e)) => {
            log::warn!("cannot tell whether another copy is running: {e}");
            Some(Guard {
                lock: None,
                socket,
                listener: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn a_second_launch_activates_the_first() {
        let (dir, name) = (
            std::env::temp_dir(),
            format!("open-synapse-test-{}", std::process::id()),
        );
        let mut first = acquire_in(&dir, &name).expect("the first copy runs");
        let (tx, rx) = mpsc::channel();
        first.on_activate(move || {
            let _ = tx.send(());
        });
        assert!(acquire_in(&dir, &name).is_none(), "a second copy stops");
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_ok(),
            "and the first is asked to show itself"
        );
        drop(first);
        assert!(
            acquire_in(&dir, &name).is_some(),
            "a new copy runs once the first lets go"
        );
        let _ = fs::remove_file(dir.join(format!("{name}.lock")));
    }
}
