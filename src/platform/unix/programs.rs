//! Which programs are running, and which one is in front, asked of the
//! window system now and then, as neither X11 nor NSWorkspace calls back
//! without a thread or an event loop of their own.

use std::{cell::Cell, time::Duration};

use slint::{Timer, TimerMode};

use super::desktop::{Display, SYSTEM};

/// How often the window in front is asked for.
const POLL: Duration = Duration::from_secs(1);

/// Programs with a window, sorted, without this one.
pub fn running() -> Vec<String> {
    crate::platform::linkable(Display::open().map(|d| d.programs()).unwrap_or_default(), SYSTEM)
}

/// Calls back on this thread with each program that comes to the front,
/// until dropped, or with an empty name for a window the system cannot
/// name, as a Wayland one is. Needs the thread's event loop, which Slint
/// runs.
pub struct ForegroundHook {
    _poll: Timer,
}

impl ForegroundHook {
    pub fn new(callback: impl Fn(String) + 'static) -> Option<Self> {
        let display = Display::open()?;
        let last = Cell::new(None);
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, POLL, move || {
            let front = display.front();
            if front.is_some() && front == last.get() {
                return;
            }
            // Only a named window is remembered. An unnamed one, as a Wayland
            // window or one still mapping is, is asked about again, and the
            // app hears of it each time, as only it can tell its own.
            let name = front.and_then(|f| display.name(f));
            last.set(front.filter(|_| name.is_some()));
            callback(name.unwrap_or_default());
        });
        Some(Self { _poll: timer })
    }
}
