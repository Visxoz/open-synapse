//! Keeps one copy of the app running. A second launch asks the first to
//! show itself, through a named event, and exits.

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0},
    System::Threading::{
        CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, INFINITE, OpenEventW, SetEvent, WaitForSingleObject,
    },
    UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow},
};

use super::wide;

const NAME: &str = "Local\\razer-native";

/// Holds the mutex until dropped.
pub struct Guard {
    mutex: HANDLE,
    activate: Option<HANDLE>,
}

// SAFETY: kernel handles may be used and closed from any thread.
unsafe impl Send for Guard {}

impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateMutexW and is closed once.
        unsafe { CloseHandle(self.mutex) };
    }
}

impl Guard {
    /// Calls `f` on a thread of its own each time another launch asks this
    /// copy to show itself.
    pub fn on_activate(&mut self, f: impl Fn() + Send + 'static) {
        let Some(event) = self.activate.take() else { return };
        let event = event as usize;
        std::thread::Builder::new()
            .name("activate".into())
            .spawn(move || {
                // The thread keeps the event, open until the process ends, so
                // it is never closed while waited on.
                // SAFETY: `event` is a handle from CreateEventW that nothing closes.
                while unsafe { WaitForSingleObject(event as HANDLE, INFINITE) } == WAIT_OBJECT_0 {
                    f();
                }
            })
            .expect("activation thread starts");
    }
}

/// Returns `None` if another copy is running, after asking it to show itself.
pub fn acquire() -> Option<Guard> {
    acquire_named(NAME)
}

fn acquire_named(name: &str) -> Option<Guard> {
    let (mutex_name, event_name) = (wide(&format!("{name}-instance")), wide(&format!("{name}-activate")));
    // SAFETY: the name is NUL-terminated; a null mutex is handled below.
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
    // SAFETY: reads this thread's last error, set by CreateMutexW.
    let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if mutex.is_null() || !exists {
        // SAFETY: the name is NUL-terminated; an auto-reset event, initially unset.
        let event = unsafe { CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr()) };
        return Some(Guard {
            mutex,
            activate: (!event.is_null()).then_some(event),
        });
    }
    // SAFETY: the mutex is ours to close; the other copy keeps its own.
    unsafe { CloseHandle(mutex) };
    // SAFETY: the name is NUL-terminated; the handle is checked and closed.
    unsafe {
        let event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
        if !event.is_null() {
            // Launched by the user, this copy may hand the foreground on.
            AllowSetForegroundWindow(ASFW_ANY);
            SetEvent(event);
            CloseHandle(event);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn a_second_launch_activates_the_first() {
        let name = format!("Local\\razer-native-test-{}", std::process::id());
        let mut first = acquire_named(&name).expect("the first copy runs");
        let (tx, rx) = mpsc::channel();
        first.on_activate(move || {
            let _ = tx.send(());
        });
        assert!(acquire_named(&name).is_none(), "a second copy stops");
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_ok(),
            "and the first is asked to show itself"
        );
        drop(first);
        assert!(acquire_named(&name).is_some(), "a new copy runs once the first lets go");
    }
}
