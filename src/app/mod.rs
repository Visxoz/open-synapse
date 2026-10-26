//! Ties the window and tray to the device worker, the profiles and the
//! settings. Everything here runs on the UI thread.

mod bind;
mod devices;
mod lighting;
mod panel;
mod pictures;
mod profiles;
mod updates;
mod view;

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use razer_protocol::EffectParams;
use slint::{ComponentHandle, Timer, TimerMode, winit_030::WinitWindowAccessor};

use crate::{
    AppWindow, Theme, Tray, TrayPanel,
    platform::{self, programs::ForegroundHook},
    profiles::Profiles,
    settings::Settings,
    update::Update,
    worker::{Event, Info, Worker},
};

const SAVE_AFTER: Duration = Duration::from_millis(800);
const NOTICE_FOR: Duration = Duration::from_secs(6);
const POINTER_EVERY: Duration = Duration::from_secs(2);

thread_local! {
    static CURRENT: RefCell<Weak<App>> = const { RefCell::new(Weak::new()) };
}

/// Shows the window and brings it to the front, as when launched again.
pub fn activate() {
    with_app(|app| app.show());
}

/// Runs `f` with the app, if it is still alive.
fn with_app(f: impl FnOnce(&Rc<App>)) {
    if let Some(app) = CURRENT.with_borrow(Weak::upgrade) {
        f(&app);
    }
}

struct State {
    settings: Settings,
    profiles: Profiles,
    devices: Vec<Info>,
    selected: usize,
    zone: usize,
    /// The selected zone's colours, speed and direction, kept while
    /// switching effects.
    params: EffectParams,
    /// The profile picked by hand, to return to when a linked program
    /// leaves the front.
    chosen: usize,
    /// When the user last changed a setting, so values polled meanwhile do
    /// not undo it.
    touched: Option<Instant>,
    pictures: pictures::Pictures,
    /// A newer release found on GitHub, ready to install.
    update: Option<Update>,
}

pub struct App {
    ui: slint::Weak<AppWindow>,
    tray: Tray,
    panel: TrayPanel,
    /// When the panel last closed, so the click that closed it does not reopen it.
    panel_hidden: Cell<Option<Instant>>,
    /// The panel has been drawn since it was last shown.
    panel_drawn: Cell<bool>,
    panel_fade: panel::Fade,
    worker: Worker,
    state: RefCell<State>,
    save_timer: Timer,
    notice_timer: Timer,
    update_timer: Timer,
    /// Rereads Windows' pointer settings while the window is open, as other
    /// programs change them too.
    pointer_timer: Timer,
    foreground: RefCell<Option<ForegroundHook>>,
}

impl App {
    pub fn new(ui: &AppWindow, tray: Tray, panel: TrayPanel, settings: Settings) -> Rc<Self> {
        let profiles = Profiles::load();
        let worker = Worker::spawn(|event| {
            let _ = slint::invoke_from_event_loop(move || with_app(|app| app.on_event(event)));
        });
        let app = Rc::new(Self {
            ui: ui.as_weak(),
            tray,
            panel,
            panel_hidden: Cell::new(None),
            panel_drawn: Cell::new(false),
            // The renderer chosen at start; `SLINT_BACKEND` may pick another.
            panel_fade: panel::Fade::new(!settings.gpu && std::env::var_os("SLINT_BACKEND").is_none()),
            worker,
            state: RefCell::new(State {
                chosen: profiles.active,
                settings,
                profiles,
                devices: Vec::new(),
                selected: 0,
                zone: 0,
                params: EffectParams::default(),
                touched: None,
                pictures: Default::default(),
                update: None,
            }),
            save_timer: Timer::default(),
            notice_timer: Timer::default(),
            update_timer: Timer::default(),
            pointer_timer: Timer::default(),
            foreground: RefCell::new(None),
        });
        CURRENT.set(Rc::downgrade(&app));
        set_fonts(ui, &app.panel);
        app.bind(ui);
        app.watch_panel();
        app.watch_foreground();
        app.schedule_update_checks();
        app.refresh();
        app
    }

    fn ui(&self) -> AppWindow {
        self.ui.upgrade().expect("window outlives the app")
    }

    fn on_event(&self, event: Event) {
        match event {
            Event::Connected(info) => self.connected(info),
            Event::Disconnected(id) => self.disconnected(id),
            Event::Changed(info) => self.changed(info),
            Event::Failed(message) => self.notify(&message),
            Event::Scanned => {
                self.ui().global::<crate::Backend>().set_scanned(true);
                self.panel.set_scanned(true);
            }
        }
    }

    pub fn settings(&self) -> Settings {
        self.state.borrow().settings.clone()
    }

    pub fn set_settings(&self, edit: impl FnOnce(&mut Settings)) {
        let mut st = self.state.borrow_mut();
        edit(&mut st.settings);
        st.settings.save();
    }

    /// Saves the profiles shortly after the last change, so dragging a
    /// slider costs one write.
    fn save_soon(&self) {
        self.save_timer.start(TimerMode::SingleShot, SAVE_AFTER, || {
            with_app(|app| app.state.borrow().profiles.save());
        });
    }

    fn notify(&self, message: &str) {
        self.ui().global::<crate::Backend>().set_notice(message.into());
        self.notice_timer.start(TimerMode::SingleShot, NOTICE_FOR, || {
            with_app(|app| app.ui().global::<crate::Backend>().set_notice("".into()));
        });
    }

    pub fn show(&self) {
        let ui = self.ui();
        if ui.show().is_ok() {
            ui.window().set_minimized(false);
            self.refresh_pointer();
            self.sync_shown();
            present(20);
        }
    }

    pub fn hide(&self) {
        let _ = self.ui().hide();
        self.sync_shown();
    }

    pub fn quit(&self) {
        self.save_timer.stop();
        // Settings are saved as they change; profiles wait for the timer.
        self.state.borrow().profiles.save();
        let _ = slint::quit_event_loop();
    }
}

/// The native window appears a moment after `show`, so this retries until
/// it can style it and bring it forward.
fn present(attempts: u32) {
    with_app(|app| {
        let ui = app.ui();
        let window = ui.window();
        if window.has_winit_window() {
            platform::window::style_title_bar(window, crate::BACKGROUND);
            platform::window::bring_to_front(window);
        } else if attempts > 0 {
            Timer::single_shot(Duration::from_millis(50), move || present(attempts - 1));
        } else {
            log::debug!("no native window to bring forward");
        }
    });
}

/// Sets the UI in this platform's typefaces, where Windows' are missing.
fn set_fonts(ui: &AppWindow, panel: &TrayPanel) {
    let Some(fonts) = platform::FONTS else { return };
    for theme in [ui.global::<Theme>(), panel.global::<Theme>()] {
        theme.set_display(fonts.display.into());
        theme.set_body(fonts.body.into());
        theme.set_mono(fonts.mono.into());
    }
}
