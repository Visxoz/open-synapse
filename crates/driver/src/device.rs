use std::{
    cell::Cell,
    fmt, io, thread,
    time::{Duration, Instant},
};

use razer_protocol::{Batch, Model, Query, REPORT_LEN, Report, Setting, Status, commands};

use crate::{Interface, sys::Hid};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// The device did not answer in time; a wireless mouse may be asleep.
    NoResponse,
    Unsupported,
    Refused(Status),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::NoResponse => f.write_str("no response"),
            Self::Unsupported => f.write_str("not supported by this device"),
            Self::Refused(s) => write!(f, "refused ({s:?})"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Transaction IDs Razer mice use; most accept any, some only one.
const TIDS: [u8; 3] = [0x1F, 0x3F, 0xFF];
const POLL_EVERY: Duration = Duration::from_millis(2);

/// An open Razer mouse, or the wireless receiver standing in for it.
pub struct Device {
    hid: Hid,
    pub model: &'static Model,
    pub path: String,
    pub firmware: (u8, u8),
    pub serial: String,
    tid: Cell<u8>,
    /// `tid` has been answered, so silence means the device is busy or
    /// asleep rather than ignoring it.
    confirmed: Cell<bool>,
}

impl Device {
    /// Opens `iface` if it is the control interface of a known mouse, the
    /// one whose feature report is a whole report after its ID. A receiver
    /// whose mouse is asleep opens too, but has yet to identify it.
    pub fn open(iface: &Interface) -> io::Result<Option<Self>> {
        let Some(model) = Model::find(iface.pid) else {
            return Ok(None);
        };
        let Some(hid) = Hid::open(&iface.path, REPORT_LEN + 1)? else {
            return Ok(None);
        };
        let mut device = Self {
            hid,
            model,
            path: iface.display_path(),
            firmware: (0, 0),
            serial: String::new(),
            tid: Cell::new(model.tid),
            confirmed: Cell::new(false),
        };
        device.identify();
        Ok(Some(device))
    }

    pub fn identified(&self) -> bool {
        self.firmware != (0, 0)
    }

    /// Reads the firmware version and serial number, once the device answers.
    pub fn identify(&mut self) {
        match self.query(commands::firmware()) {
            Ok(firmware) => {
                self.firmware = firmware;
                self.serial = self.query(commands::serial()).unwrap_or_default();
            }
            Err(e) => log::debug!("{} did not answer: {e}", self.model.name),
        }
    }

    pub fn query<T>(&self, q: Query<T>) -> Result<T> {
        self.exchange(&q.request).map(|r| (q.parse)(&r))
    }

    pub fn send(&self, batch: Batch) -> Result<()> {
        if batch.is_empty() {
            return Err(Error::Unsupported);
        }
        batch.iter().try_for_each(|r| self.exchange(r).map(drop))
    }

    pub fn apply(&self, setting: &Setting) -> Result<()> {
        self.send(setting.batch(self.model))
    }

    /// Tries the transaction ID the model expects for this command, else the
    /// one that worked last, then the others.
    fn exchange(&self, request: &Report) -> Result<Report> {
        let expected = self.model.tid_for(request.class, request.id);
        let first = expected.unwrap_or(self.tid.get());
        let mut last = Error::NoResponse;
        for tid in std::iter::once(first).chain(TIDS.into_iter().filter(|&t| t != first)) {
            match self.transact(&request.with_tid(tid)) {
                Ok(r) => {
                    // A command with its own ID says nothing of the others'.
                    if expected.is_none() {
                        self.tid.set(tid);
                    }
                    self.confirmed.set(true);
                    return Ok(r);
                }
                // A receiver reports a timeout while its mouse is asleep.
                Err(e @ (Error::NoResponse | Error::Refused(Status::Timeout))) if self.confirmed.get() => {
                    return Err(e);
                }
                Err(e @ (Error::NoResponse | Error::Refused(_))) => last = e,
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }

    /// Sends one request and polls for its answer, which receivers take
    /// tens of milliseconds to relay.
    fn transact(&self, request: &Report) -> Result<Report> {
        let mut buf = [0u8; REPORT_LEN + 1];
        buf[1..].copy_from_slice(&request.encode());
        self.hid.set_feature(&buf)?;

        let wait = Duration::from_micros(self.model.wait.micros());
        let deadline = Instant::now() + (wait * 3).max(Duration::from_millis(30));
        thread::sleep((wait / 3).max(Duration::from_micros(500)));
        let mut busy = None;
        loop {
            buf.fill(0);
            self.hid.get_feature(&mut buf)?;
            let response = Report::decode(buf[1..].try_into().expect("report length"));
            if response.answers(request) {
                match response.status {
                    Status::Ok => return Ok(response),
                    Status::Busy => busy = Some(response),
                    Status::New => {}
                    Status::Unsupported => return Err(Error::Unsupported),
                    status => return Err(Error::Refused(status)),
                }
            }
            if Instant::now() >= deadline {
                // Some commands keep reporting busy yet take effect.
                return busy.ok_or(Error::NoResponse);
            }
            thread::sleep(POLL_EVERY);
        }
    }
}
