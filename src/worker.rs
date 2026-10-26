//! Owns the open devices on a thread of its own, since every exchange
//! blocks for milliseconds, and wireless ones for tens of them.

use std::{
    io,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

use razer_driver::{Device, Interface, State, Watcher};
use razer_protocol::{Model, Setting};

/// Plugging in a device announces each of its collections separately.
const SETTLE: Duration = Duration::from_millis(400);
const POLL_SHOWN: Duration = Duration::from_secs(2);
const POLL_HIDDEN: Duration = Duration::from_secs(60);

pub type DeviceId = u32;

pub enum Request {
    Rescan,
    Apply(DeviceId, Vec<Setting>),
    /// The window is shown, so live values are worth polling often.
    Shown(bool),
}

#[derive(Clone)]
pub struct Info {
    pub id: DeviceId,
    pub model: &'static Model,
    pub firmware: (u8, u8),
    pub serial: String,
    pub path: String,
    pub state: State,
}

pub enum Event {
    Connected(Info),
    Disconnected(DeviceId),
    Changed(Info),
    Failed(String),
    /// The first look for devices has finished.
    Scanned,
}

#[derive(Clone)]
pub struct Worker {
    tx: Sender<Request>,
}

impl Worker {
    pub fn spawn(sink: impl Fn(Event) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let notify = tx.clone();
        thread::Builder::new()
            .name("devices".into())
            .spawn(move || {
                let watcher = Watcher::new(move || drop(notify.send(Request::Rescan)))
                    .inspect_err(|e| log::warn!("no hot-plug notifications: {e}"))
                    .ok();
                Run {
                    sink: Box::new(sink),
                    devices: Vec::new(),
                    next_id: 1,
                    shown: true,
                    reread: false,
                }
                .run(&rx);
                drop(watcher);
            })
            .expect("device thread starts");
        Self { tx }
    }

    pub fn send(&self, request: Request) {
        let _ = self.tx.send(request);
    }

    pub fn apply(&self, id: DeviceId, settings: Vec<Setting>) {
        if !settings.is_empty() {
            self.send(Request::Apply(id, settings));
        }
    }
}

struct Open {
    id: DeviceId,
    device: Device,
    state: State,
    /// Everything has been read once; until then the mouse was asleep.
    complete: bool,
}

impl Open {
    fn info(&self) -> Info {
        Info {
            id: self.id,
            model: self.device.model,
            firmware: self.device.firmware,
            serial: self.device.serial.clone(),
            path: self.device.path.clone(),
            state: self.state.clone(),
        }
    }
}

struct Run {
    sink: Box<dyn Fn(Event) + Send>,
    devices: Vec<Open>,
    next_id: DeviceId,
    shown: bool,
    /// The window or panel has just opened, so the next poll reads every
    /// setting, as other programs may have changed any of them.
    reread: bool,
}

impl Run {
    fn run(mut self, rx: &Receiver<Request>) {
        #[cfg(debug_assertions)]
        self.demo();
        self.rescan();
        (self.sink)(Event::Scanned);
        let mut next_poll = Instant::now() + POLL_SHOWN;
        loop {
            let first = match rx.recv_timeout(next_poll.saturating_duration_since(Instant::now())) {
                Ok(request) => request,
                Err(RecvTimeoutError::Timeout) => {
                    self.poll();
                    next_poll = Instant::now() + self.poll_every();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            };
            let mut requests: Vec<Request> = std::iter::once(first).chain(rx.try_iter()).collect();
            if requests.iter().any(|r| matches!(r, Request::Rescan)) {
                thread::sleep(SETTLE);
                requests.extend(rx.try_iter());
            }
            self.handle(requests);
            // Showing the window may have shortened the wait, or ended it.
            let wait = if self.reread { Duration::ZERO } else { self.poll_every() };
            next_poll = next_poll.min(Instant::now() + wait);
        }
    }

    fn poll_every(&self) -> Duration {
        if self.shown { POLL_SHOWN } else { POLL_HIDDEN }
    }

    fn handle(&mut self, requests: Vec<Request>) {
        let mut rescan = false;
        // Only the newest value per device and slot is worth sending.
        let mut pending: Vec<(DeviceId, Setting)> = Vec::new();
        for request in requests {
            match request {
                Request::Rescan => rescan = true,
                Request::Shown(shown) => {
                    self.reread |= shown && !self.shown;
                    self.shown = shown;
                }
                Request::Apply(id, settings) => {
                    for setting in settings {
                        pending.retain(|(i, s)| *i != id || s.slot() != setting.slot());
                        pending.push((id, setting));
                    }
                }
            }
        }
        if rescan {
            self.rescan();
        }
        for (id, setting) in pending {
            let Some(open) = self.devices.iter().find(|o| o.id == id) else {
                continue;
            };
            if let Err(e) = open.device.apply(&setting) {
                log::warn!("{}: {setting:?} failed: {e}", open.device.model.name);
                (self.sink)(Event::Failed(format!(
                    "{} could not apply that: {e}",
                    open.device.model.name
                )));
            }
        }
    }

    fn rescan(&mut self) {
        let candidates = razer_driver::candidates();
        let paths: Vec<String> = candidates.iter().map(Interface::display_path).collect();
        let (kept, gone): (Vec<_>, Vec<_>) = std::mem::take(&mut self.devices)
            .into_iter()
            .partition(|o| paths.contains(&o.device.path));
        self.devices = kept;
        for open in gone {
            log::info!("{} disconnected", open.device.model.name);
            (self.sink)(Event::Disconnected(open.id));
        }
        // Mice whose control interface the system would not let this app open.
        let mut denied: Vec<&'static Model> = Vec::new();
        for (iface, path) in candidates.iter().zip(&paths) {
            if self.devices.iter().any(|o| o.device.path == *path) {
                continue;
            }
            let device = match Device::open(iface) {
                Ok(Some(device)) => device,
                Ok(None) => continue,
                Err(e) => {
                    log::debug!("cannot open {path}: {e}");
                    if e.kind() == io::ErrorKind::PermissionDenied
                        && let Some(model) = Model::find(iface.pid)
                        && !denied.iter().any(|m| m.pid == model.pid)
                    {
                        denied.push(model);
                    }
                    continue;
                }
            };
            // A mouse with two control collections answers on both; one that
            // is asleep cannot tell them apart, so the first stands for both.
            let duplicate = self.devices.iter().any(|o| {
                o.device.model.pid == device.model.pid && (device.serial.is_empty() || o.device.serial == device.serial)
            });
            if duplicate {
                continue;
            }
            // A mouse that has not answered yet is read by `poll` once it does.
            let state = if device.identified() {
                device.read_state()
            } else {
                State::default()
            };
            let open = Open {
                id: self.next_id,
                complete: state.awake,
                device,
                state,
            };
            self.next_id += 1;
            log::info!(
                "{} connected: firmware {:?}, {:?}",
                open.device.model.name,
                open.device.firmware,
                open.state
            );
            (self.sink)(Event::Connected(open.info()));
            self.devices.push(open);
        }
        for model in denied {
            if !self.devices.iter().any(|o| o.device.model.pid == model.pid) {
                (self.sink)(Event::Failed(format!(
                    "Open Synapse is not allowed to open the {}. {}",
                    model.name,
                    razer_driver::ACCESS_HINT
                )));
            }
        }
    }

    fn poll(&mut self) {
        let reread = std::mem::take(&mut self.reread);
        for open in &mut self.devices {
            let before = (open.device.firmware, open.state.clone());
            if !open.device.identified() {
                open.device.identify();
                if !open.device.identified() {
                    continue;
                }
            }
            if open.complete && reread {
                let fresh = open.device.read_state();
                // A mouse that has fallen asleep keeps what it last reported.
                if fresh.awake {
                    open.state = fresh;
                } else {
                    open.state.awake = false;
                }
            } else if open.complete {
                open.device.read_live(&mut open.state);
            } else {
                open.state = open.device.read_state();
                open.complete = open.state.awake;
            }
            if (open.device.firmware, &open.state) != (before.0, &before.1) {
                (self.sink)(Event::Changed(open.info()));
            }
        }
    }
}

#[cfg(debug_assertions)]
impl Run {
    /// `OPEN_SYNAPSE_DEMO=00AA` shows that model as though connected, to try the
    /// pages of mice not at hand. Its settings go nowhere.
    fn demo(&mut self) {
        use razer_protocol::{Dpi, DpiStages, Features, Power};
        let Some(model) = std::env::var("OPEN_SYNAPSE_DEMO")
            .ok()
            .and_then(|pid| u16::from_str_radix(&pid, 16).ok())
            .and_then(Model::find)
        else {
            return;
        };
        let has = |p| model.power.contains(p);
        let state = State {
            dpi: Some(Dpi::square(1600)),
            stages: model.features.contains(Features::DPI_STAGES).then(|| DpiStages {
                active: 1,
                stages: [800, 1600, 3200].map(Dpi::square).to_vec(),
            }),
            awake: true,
            poll: model.poll.rates().last().copied(),
            battery: has(Power::BATTERY).then_some(72),
            charging: has(Power::CHARGING).then_some(false),
            idle: has(Power::IDLE).then_some(300),
            low_battery: has(Power::LOW_BATTERY).then_some(15),
            brightness: model
                .lighting
                .zones
                .iter()
                .filter(|z| z.brightness)
                .map(|z| (z.led, 200))
                .collect(),
            ..State::default()
        };
        (self.sink)(Event::Connected(Info {
            id: 0,
            model,
            firmware: (1, 0),
            serial: "DEMO".into(),
            path: "demo".into(),
            state,
        }));
    }
}
