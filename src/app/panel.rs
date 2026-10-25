//! The panel that opens from the tray icon: a small always-on-top window
//! in the corner by the tray, closed by a click elsewhere, Escape or the
//! icon again.

use std::time::{Duration, Instant};

use slint::{
    ComponentHandle, PhysicalPosition, Timer,
    winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent},
};

use super::{App, with_app};
use crate::{win::window, worker::Request};

/// A click on the tray icon takes the focus first, which closes the panel;
/// that click must not open it again.
const REOPEN_GRACE: Duration = Duration::from_millis(400);
/// Where the panel waits to be drawn, off every screen.
const PARKED: PhysicalPosition = PhysicalPosition::new(-32000, -32000);
/// How often, and how many times, placing the panel is retried.
const PLACE_EVERY: Duration = Duration::from_millis(20);
const PLACE_TRIES: u32 = 25;

impl App {
    /// Closes the panel whenever another window takes the focus, and notes
    /// each drawing of it.
    pub(super) fn watch_panel(&self) {
        self.panel.window().on_winit_window_event(|_, event| {
            match event {
                WindowEvent::Focused(false) => {
                    // After the backend has finished with the event.
                    Timer::single_shot(Duration::ZERO, || with_app(|app| app.hide_panel()));
                }
                WindowEvent::RedrawRequested => with_app(|app| app.panel_drawn.set(true)),
                _ => {}
            }
            EventResult::Propagate
        });
    }

    /// Opens the panel, or closes it if it is open.
    pub(super) fn toggle_panel(&self) {
        if self.panel.window().is_visible() {
            self.hide_panel();
        } else if !self.panel_hidden.get().is_some_and(|at| at.elapsed() < REOPEN_GRACE) {
            self.show_panel();
        }
    }

    /// The panel's rows exist only once it has been drawn, so its height is
    /// not known until then: it is shown off-screen first, and moved into
    /// place once drawn at its size.
    fn show_panel(&self) {
        let window = self.panel.window();
        self.panel_drawn.set(false);
        window.set_position(PARKED);
        if window.show().is_ok() {
            self.sync_shown();
            self.place_panel(PLACE_TRIES);
        }
    }

    pub(super) fn hide_panel(&self) {
        let window = self.panel.window();
        if window.is_visible() {
            let _ = window.hide();
            self.panel_hidden.set(Some(Instant::now()));
            self.sync_shown();
        }
    }

    /// Tells the worker whether anything is on screen, which sets how often
    /// it polls the mice.
    pub(super) fn sync_shown(&self) {
        let shown = self.ui().window().is_visible() || self.panel.window().is_visible();
        self.worker.send(Request::Shown(shown));
    }

    /// Moves the panel into place and gives it the focus, which the click on
    /// the tray icon allows, once its native window exists and it has been
    /// drawn at its size; both come a moment after `show`, so this retries.
    fn place_panel(&self, tries: u32) {
        let window = self.panel.window();
        if !window.is_visible() {
            return;
        }
        let (width, height) = (self.panel.get_panel_width(), self.panel.get_panel_height());
        let settled = self.panel_drawn.get() && window.size().height == (height * window.scale_factor()).round() as u32;
        match window::hwnd(window) {
            Some(hwnd) if settled || tries == 0 => {
                if let Some((x, y)) = window::popup_origin(width, height) {
                    window.set_position(PhysicalPosition::new(x, y));
                }
                window::bring_to_front(hwnd);
            }
            None if tries == 0 => log::debug!("no window handle for the panel"),
            _ => Timer::single_shot(PLACE_EVERY, move || with_app(|app| app.place_panel(tries - 1))),
        }
    }
}
