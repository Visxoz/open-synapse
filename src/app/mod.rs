//! Ties the window and tray to the device worker, the profiles and the
//! settings. Everything here runs on the UI thread.

mod bind;
mod devices;
mod lighting;
mod pictures;
mod profiles;
mod updates;
mod view;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use razer_protocol::EffectParams;
use slint::{ComponentHandle, Timer, TimerMode};

use crate::{
    AppWindow, Tray,
    profiles::Profiles,
    settings::Settings,
    update::Update,
    win::programs::ForegroundHook,
    worker::{Event, Info, Request, Worker},
};

const SAVE_AFTER: Duration = Duration::from_millis(800);
const NOTICE_FOR: Duration = Duration::from_secs(6);

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
    worker: Worker,
    state: RefCell<State>,
    save_timer: Timer,
    notice_timer: Timer,
    update_timer: Timer,
    foreground: RefCell<Option<ForegroundHook>>,
}

impl App {
    pub fn new(ui: &AppWindow, tray: Tray, settings: Settings) -> Rc<Self> {
        let profiles = Profiles::load();
        let worker = Worker::spawn(|event| {
            let _ = slint::invoke_from_event_loop(move || with_app(|app| app.on_event(event)));
        });
        let app = Rc::new(Self {
            ui: ui.as_weak(),
            tray,
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
            foreground: RefCell::new(None),
        });
        CURRENT.set(Rc::downgrade(&app));
        app.bind(ui);
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
            Event::Scanned => self.ui().global::<crate::Backend>().set_scanned(true),
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
            self.worker.send(Request::Shown(true));
            style_title_bar(20);
        }
    }

    pub fn hide(&self) {
        let _ = self.ui().hide();
        self.worker.send(Request::Shown(false));
    }

    pub fn quit(&self) {
        self.save_timer.stop();
        let st = self.state.borrow();
        st.profiles.save();
        st.settings.save();
        let _ = slint::quit_event_loop();
    }
}

/// The native window appears a moment after `show`, so this retries until
/// it can style it and bring it forward.
fn style_title_bar(attempts: u32) {
    with_app(|app| match crate::win::window::hwnd(app.ui().window()) {
        Some(hwnd) => {
            crate::win::window::style_title_bar(hwnd, crate::BACKGROUND);
            crate::win::window::bring_to_front(hwnd);
        }
        None if attempts > 0 => {
            Timer::single_shot(Duration::from_millis(50), move || style_title_bar(attempts - 1));
        }
        None => log::debug!("no window handle to style"),
    });
}
