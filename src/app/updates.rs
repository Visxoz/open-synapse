//! Checking GitHub for a newer release and installing it, off the UI thread.

use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, Timer, TimerMode};

use super::{App, with_app};
use crate::{Backend, update};

/// Left until the app has settled after starting.
const FIRST_CHECK: Duration = Duration::from_secs(10);
const CHECK_EVERY: Duration = Duration::from_secs(12 * 60 * 60);

/// Runs `work` on a thread of its own, then `done` with its result on the
/// UI thread.
fn off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, done: fn(&Rc<App>, T)) {
    let spawned = std::thread::Builder::new().name("update".into()).spawn(move || {
        let result = work();
        let _ = slint::invoke_from_event_loop(move || with_app(|app| done(app, result)));
    });
    if let Err(e) = spawned {
        log::warn!("could not start the update thread: {e}");
    }
}

impl App {
    pub(super) fn schedule_update_checks(&self) {
        Timer::single_shot(FIRST_CHECK, || with_app(|app| app.check_for_update()));
        self.update_timer.start(TimerMode::Repeated, CHECK_EVERY, || {
            with_app(|app| app.check_for_update());
        });
    }

    /// Looks for a newer release, when checks are on and none is waiting.
    pub(super) fn check_for_update(&self) {
        let st = self.state.borrow();
        if !st.settings.check_updates || st.update.is_some() {
            return;
        }
        drop(st);
        off_thread(update::check, |app, result| match result {
            Ok(Some(found)) => {
                app.notify(&format!(
                    "Open Synapse {} is available. Update from Settings.",
                    found.version
                ));
                app.state.borrow_mut().update = Some(found);
                app.refresh_settings();
            }
            Ok(None) => {}
            Err(e) => log::debug!("{e}"),
        });
    }

    /// Downloads and checks the update, then quits so it can take over.
    pub(super) fn install_update(&self) {
        let Some(found) = self.state.borrow().update.clone() else {
            return;
        };
        let ui = self.ui();
        let b = ui.global::<Backend>();
        if b.get_update_busy() {
            return;
        }
        b.set_update_busy(true);
        b.set_update_status("Downloading…".into());
        off_thread(
            move || {
                let mut shown = 0;
                let progress = |fraction: f32| {
                    let percent = (fraction * 100.) as u8;
                    if percent != std::mem::replace(&mut shown, percent) {
                        let _ = slint::invoke_from_event_loop(move || {
                            with_app(|app| {
                                let status = format!("Downloading… {percent}%");
                                app.ui().global::<Backend>().set_update_status(status.into());
                            });
                        });
                    }
                };
                update::download(&found, progress).and_then(|build| update::stage(&found, &build))
            },
            |app, result| match result {
                Ok(()) => app.quit(),
                Err(e) => {
                    let ui = app.ui();
                    let b = ui.global::<Backend>();
                    b.set_update_busy(false);
                    b.set_update_status(format!("The update failed: {e}").into());
                }
            },
        );
    }
}
