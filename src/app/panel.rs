//! The panel that opens from the tray icon: a small always-on-top window
//! in the corner by the tray, closed by a click elsewhere, Escape or the
//! icon again.

use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use slint::{
    ComponentHandle, PhysicalPosition, Timer, TimerMode,
    winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent},
};

use super::{App, POINTER_EVERY, with_app};
use crate::{
    platform::{pointer, window},
    worker::Request,
};

/// A click on the tray icon takes the focus first, which closes the panel;
/// that click must not open it again.
const REOPEN_GRACE: Duration = Duration::from_millis(400);
/// Where the panel waits to be drawn, off every screen.
const PARKED: PhysicalPosition = PhysicalPosition::new(-32000, -32000);
/// How often, and how many times, placing the panel is retried.
const PLACE_EVERY: Duration = Duration::from_millis(20);
const PLACE_TRIES: u32 = 25;
/// How long the panel takes to fade in or out, and the step it fades by.
const FADE: Duration = Duration::from_millis(120);
const FADE_STEP: Duration = Duration::from_millis(15);

/// The panel's fade: how opaque its window is, where that is heading and
/// the timer stepping it there. The content slides and fades in Slint
/// meanwhile; the window itself fades only when `layered` allows.
pub(super) struct Fade {
    timer: Timer,
    alpha: Cell<f32>,
    target: Cell<f32>,
    /// Layered-window alpha suits the software renderer; the GPU renderer
    /// cannot present to a layered window, so there only the content fades.
    layered: bool,
}

impl Fade {
    pub(super) fn new(layered: bool) -> Self {
        Self {
            timer: Timer::default(),
            alpha: Cell::new(0.),
            target: Cell::new(0.),
            layered,
        }
    }

    fn closing(&self) -> bool {
        self.target.get() == 0.
    }
}

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

    /// Opens the panel, or closes it if it is open; shows the window instead
    /// where the platform has no place for a panel by the tray.
    pub(super) fn toggle_panel(&self) {
        if !window::TRAY_PANEL {
            self.show();
        } else if self.panel.window().is_visible() {
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
        self.panel_fade.target.set(1.);
        window.set_position(PARKED);
        if window.show().is_ok() {
            self.sync_shown();
            self.place_panel(PLACE_TRIES);
        }
    }

    /// Fades the panel out, then hides it.
    pub(super) fn hide_panel(&self) {
        let fade = &self.panel_fade;
        if !self.panel.window().is_visible() || fade.closing() {
            return;
        }
        fade.target.set(0.);
        self.panel.set_shown(false);
        if fade.alpha.get() == 0. {
            // Not yet placed, so nothing is showing.
            self.panel_hidden();
        } else {
            self.run_fade();
        }
    }

    /// Tells the worker whether anything is on screen, which sets how often
    /// it polls the mice.
    pub(super) fn sync_shown(&self) {
        let window = self.ui().window().is_visible();
        self.worker
            .send(Request::Shown(window || self.panel.window().is_visible()));
        // The pointer settings are polled while shown, where the system has them.
        if !window || !pointer::AVAILABLE {
            self.pointer_timer.stop();
        } else if !self.pointer_timer.running() {
            self.pointer_timer.start(TimerMode::Repeated, POINTER_EVERY, || {
                with_app(|app| app.refresh_pointer());
            });
        }
    }

    /// Moves the panel into place and gives it the focus, which the click on
    /// the tray icon allows, once its native window exists and it has been
    /// drawn at its size; both come a moment after `show`, so this retries.
    /// It arrives fully transparent and fades in from there.
    fn place_panel(&self, tries: u32) {
        let window = self.panel.window();
        if !window.is_visible() || self.panel_fade.closing() {
            return;
        }
        let (width, height) = (self.panel.get_panel_width(), self.panel.get_panel_height());
        let settled = self.panel_drawn.get() && window.size().height == (height * window.scale_factor()).round() as u32;
        if window.has_winit_window() && (settled || tries == 0) {
            if self.panel_fade.layered {
                window::set_alpha(window, 0.);
            }
            if let Some((x, y)) = window::popup_origin(width, height) {
                window.set_position(PhysicalPosition::new(x, y));
            }
            window::bring_to_front(window);
            self.panel.set_shown(true);
            self.run_fade();
        } else if tries == 0 {
            log::debug!("no native window for the panel");
        } else {
            Timer::single_shot(PLACE_EVERY, move || with_app(|app| app.place_panel(tries - 1)));
        }
    }

    fn run_fade(&self) {
        self.panel_fade
            .timer
            .start(TimerMode::Repeated, FADE_STEP, || with_app(|app| app.step_fade()));
    }

    /// One step of the fade towards its target; hides the window once it
    /// has faded out.
    fn step_fade(&self) {
        let fade = &self.panel_fade;
        let (alpha, target) = (fade.alpha.get(), fade.target.get());
        let step = FADE_STEP.as_secs_f32() / FADE.as_secs_f32();
        let alpha = if target > alpha {
            (alpha + step).min(target)
        } else {
            (alpha - step).max(target)
        };
        fade.alpha.set(alpha);
        if fade.layered {
            window::set_alpha(self.panel.window(), alpha);
        }
        if alpha == target {
            fade.timer.stop();
            if fade.closing() {
                self.panel_hidden();
            }
        }
    }

    fn panel_hidden(&self) {
        let _ = self.panel.window().hide();
        self.panel_fade.alpha.set(0.);
        self.panel_hidden.set(Some(Instant::now()));
        self.sync_shown();
    }
}
