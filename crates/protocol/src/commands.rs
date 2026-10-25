use core::ops::Deref;

use crate::model::{DpiRange, Features, Model, Poll};
use crate::report::{NOSTORE, Report, VARSTORE};
use crate::types::{Dpi, DpiStages, MAX_STAGES, ScrollSetting};

/// A request and how to read the device's answer to it.
#[derive(Clone, Copy)]
pub struct Query<T> {
    pub request: Report,
    pub parse: fn(&Report) -> T,
}

const fn query<T>(request: Report, parse: fn(&Report) -> T) -> Query<T> {
    Query { request, parse }
}

/// Up to three reports sent in order; empty when unsupported.
#[derive(Debug, Clone, Copy)]
pub struct Batch {
    len: usize,
    reports: [Report; 3],
}

impl Batch {
    pub const EMPTY: Self = Self {
        len: 0,
        reports: [Report::BLANK; 3],
    };

    pub fn of(reports: &[Report]) -> Self {
        let mut batch = Self::EMPTY;
        batch.reports[..reports.len()].copy_from_slice(reports);
        batch.len = reports.len();
        batch
    }
}

impl From<Report> for Batch {
    fn from(r: Report) -> Self {
        Self::of(&[r])
    }
}

impl Deref for Batch {
    type Target = [Report];
    fn deref(&self) -> &[Report] {
        &self.reports[..self.len]
    }
}

pub fn firmware() -> Query<(u8, u8)> {
    query(Report::new(0x00, 0x81, 0x02, &[]), |r| (r.args[0], r.args[1]))
}

pub fn serial() -> Query<String> {
    query(Report::new(0x00, 0x82, 0x16, &[]), |r| {
        r.args[..22]
            .iter()
            .take_while(|&&b| b != 0)
            .filter(|b| b.is_ascii_graphic())
            .map(|&b| b as char)
            .collect()
    })
}

/// The most a DPI byte can say, 252.
const BYTE_DPI_MAX: u16 = 6400;

/// Older mice take DPI as a byte counting 25 DPI from 100, set in steps of
/// 100 as razercfg's Naga and DeathAdder 2013 drivers do. OpenRazer scales
/// against 6750 instead, which strays from it away from 1800 DPI.
fn dpi_to_byte(v: u16) -> u8 {
    let hundreds = (v.clamp(DpiRange::MIN, BYTE_DPI_MAX) + 50) / 100;
    ((hundreds - 1) * 4) as u8
}

fn byte_to_dpi(b: u8) -> u16 {
    (u16::from(b) + 2) / 4 * 100 + 100
}

pub fn get_dpi(m: &Model) -> Option<Query<Dpi>> {
    match m.dpi {
        DpiRange::None => None,
        DpiRange::Word { .. } => {
            let store = if m.features.contains(Features::DPI_READ_VARSTORE) {
                VARSTORE
            } else {
                NOSTORE
            };
            Some(query(Report::new(0x04, 0x85, 0x07, &[store]), |r| Dpi {
                x: r.be16(1),
                y: r.be16(3),
            }))
        }
        DpiRange::Byte { .. } => Some(query(Report::new(0x04, 0x81, 0x03, &[]), |r| Dpi {
            x: byte_to_dpi(r.args[0]),
            y: byte_to_dpi(r.args[1]),
        })),
    }
}

pub fn set_dpi(m: &Model, dpi: Dpi) -> Batch {
    let clamp = |v: u16| v.clamp(DpiRange::MIN, m.dpi.max().max(DpiRange::MIN));
    let (x, y) = (clamp(dpi.x), clamp(dpi.y));
    match m.dpi {
        DpiRange::None => Batch::EMPTY,
        DpiRange::Word { .. } => {
            let [xh, xl] = x.to_be_bytes();
            let [yh, yl] = y.to_be_bytes();
            Report::new(0x04, 0x05, 0x07, &[VARSTORE, xh, xl, yh, yl]).into()
        }
        DpiRange::Byte { .. } => Report::new(0x04, 0x01, 0x03, &[dpi_to_byte(x), dpi_to_byte(y)]).into(),
    }
}

const STAGE_LEN: usize = 7;

pub fn get_stages() -> Query<DpiStages> {
    query(Report::new(0x04, 0x86, 0x26, &[VARSTORE]), |r| {
        let count = usize::from(r.args[2]).min(MAX_STAGES);
        let stages = (0..count)
            .map(|i| {
                let at = 3 + i * STAGE_LEN;
                Dpi {
                    x: r.be16(at + 1),
                    y: r.be16(at + 3),
                }
            })
            .collect();
        DpiStages {
            active: usize::from(r.args[1].saturating_sub(1)).min(count.saturating_sub(1)),
            stages,
        }
    })
}

pub fn set_stages(m: &Model, stages: &DpiStages) -> Batch {
    let count = stages.stages.len().min(MAX_STAGES);
    if count == 0 || !m.features.contains(Features::DPI_STAGES) {
        return Batch::EMPTY;
    }
    let mut args = [0u8; 3 + MAX_STAGES * STAGE_LEN];
    args[0] = VARSTORE;
    args[1] = stages.active.min(count - 1) as u8 + 1;
    args[2] = count as u8;
    let max = m.dpi.max().max(DpiRange::MIN);
    for (i, dpi) in stages.stages.iter().take(count).enumerate() {
        let at = 3 + i * STAGE_LEN;
        args[at] = i as u8 + 1;
        args[at + 1..at + 3].copy_from_slice(&dpi.x.clamp(DpiRange::MIN, max).to_be_bytes());
        args[at + 3..at + 5].copy_from_slice(&dpi.y.clamp(DpiRange::MIN, max).to_be_bytes());
    }
    Report::new(0x04, 0x06, 0x26, &args).into()
}

/// Classic rates are 1000 Hz divided by the code, HyperPolling's 8000 Hz.
fn rate_code(base: u16, rate: u16) -> u8 {
    (base / rate.max(1)).clamp(1, 0x40) as u8
}

fn code_rate(base: u16, code: u8) -> u16 {
    if code == 0 { 0 } else { base / u16::from(code) }
}

pub fn get_poll(m: &Model) -> Option<Query<u16>> {
    match m.poll {
        Poll::None => None,
        Poll::Classic(_) => Some(query(Report::new(0x00, 0x85, 0x01, &[]), |r| {
            code_rate(1000, r.args[0])
        })),
        Poll::Hyper(_) => Some(query(Report::new(0x00, 0xC0, 0x01, &[]), |r| {
            code_rate(8000, r.args[1])
        })),
    }
}

pub fn set_poll(m: &Model, rate: u16) -> Batch {
    if !m.poll.rates().contains(&rate) {
        return Batch::EMPTY;
    }
    match m.poll {
        Poll::None => Batch::EMPTY,
        Poll::Classic(_) => Report::new(0x00, 0x05, 0x01, &[rate_code(1000, rate)]).into(),
        // Once for the wired link and once for the wireless one.
        Poll::Hyper(_) => {
            let code = rate_code(8000, rate);
            Batch::of(&[
                Report::new(0x00, 0x40, 0x02, &[0x00, code]),
                Report::new(0x00, 0x40, 0x02, &[0x01, code]),
            ])
        }
    }
}

fn to_percent(v: u8) -> u8 {
    ((u16::from(v) * 100 + 127) / 255) as u8
}

pub fn battery() -> Query<u8> {
    query(Report::new(0x07, 0x80, 0x02, &[]), |r| to_percent(r.args[1]))
}

pub fn charging() -> Query<bool> {
    query(Report::new(0x07, 0x84, 0x02, &[]), |r| r.args[1] != 0)
}

pub const IDLE_RANGE: (u16, u16) = (60, 900);

pub fn get_idle() -> Query<u16> {
    query(Report::new(0x07, 0x83, 0x02, &[]), |r| r.be16(0))
}

pub fn set_idle(seconds: u16) -> Batch {
    let [h, l] = seconds.clamp(IDLE_RANGE.0, IDLE_RANGE.1).to_be_bytes();
    Report::new(0x07, 0x03, 0x02, &[h, l]).into()
}

pub const LOW_BATTERY_RANGE: (u8, u8) = (5, 50);

pub fn get_low_battery() -> Query<u8> {
    query(Report::new(0x07, 0x81, 0x01, &[]), |r| to_percent(r.args[0]))
}

pub fn set_low_battery(percent: u8) -> Batch {
    let p = u16::from(percent.clamp(LOW_BATTERY_RANGE.0, LOW_BATTERY_RANGE.1));
    Report::new(0x07, 0x01, 0x01, &[((p * 255 + 50) / 100) as u8]).into()
}

/// What the HyperPolling dongle's single LED shows: 1 connection, 2 battery,
/// 3 low battery warning.
pub fn get_dongle_led() -> Query<u8> {
    query(Report::new(0x07, 0x90, 0x01, &[]), |r| r.args[0])
}

pub fn set_dongle_led(mode: u8) -> Batch {
    Report::new(0x07, 0x10, 0x01, &[mode.clamp(1, 3)]).into()
}

/// What each of the dongle's three LEDs shows: 0 off, 1 battery,
/// 2 connection, 3 polling rate, 4 DPI.
pub fn get_dongle_leds() -> Query<[u8; 3]> {
    query(Report::new(0x07, 0x95, 0x03, &[]), |r| {
        [r.args[0], r.args[1], r.args[2]]
    })
}

pub fn set_dongle_leds(modes: [u8; 3]) -> Batch {
    Report::new(0x07, 0x15, 0x03, &modes.map(|m| m.min(4))).into()
}

pub fn get_scroll(setting: ScrollSetting) -> Query<bool> {
    query(Report::new(0x02, setting.command() | 0x80, 0x02, &[VARSTORE]), |r| {
        r.args[1] != 0
    })
}

pub fn set_scroll(setting: ScrollSetting, on: bool) -> Batch {
    Report::new(0x02, setting.command(), 0x02, &[VARSTORE, u8::from(on)]).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Report;

    fn model(pid: u16) -> &'static Model {
        Model::find(pid).unwrap()
    }

    #[test]
    fn reads_stages_as_the_viper_v4_pro_sends_them() {
        let mut r = Report::new(0x04, 0x86, 0x26, &[]);
        r.args[..17].copy_from_slice(&[
            1, 1, 2, 1, 0x06, 0x40, 0x06, 0x40, 0, 0, 2, 0x0C, 0x80, 0x0C, 0x80, 0, 0,
        ]);
        let stages = (get_stages().parse)(&r);
        assert_eq!(
            stages,
            DpiStages {
                active: 0,
                stages: vec![Dpi::square(1600), Dpi::square(3200)]
            }
        );
    }

    #[test]
    fn writes_stages_in_the_layout_it_reads() {
        let stages = DpiStages {
            active: 1,
            stages: vec![Dpi::square(800), Dpi { x: 1600, y: 1200 }],
        };
        let batch = set_stages(model(0x00E6), &stages);
        let mut answer = batch[0];
        answer.id = 0x86;
        assert_eq!((get_stages().parse)(&answer), stages);
    }

    #[test]
    fn poll_rates_round_trip() {
        for &rate in model(0x00E6).poll.rates() {
            assert_eq!(code_rate(8000, rate_code(8000, rate)), rate);
        }
        assert_eq!(rate_code(1000, 250), 4);
        assert!(set_poll(model(0x00E6), 333).is_empty());
        assert_eq!(set_poll(model(0x00E6), 8000).len(), 2);
    }

    #[test]
    fn byte_dpi_follows_razercfg() {
        assert_eq!(dpi_to_byte(1000), 36);
        assert_eq!(dpi_to_byte(5600), 220);
        assert_eq!(dpi_to_byte(1840), 68);
        for dpi in (100..=BYTE_DPI_MAX).step_by(100) {
            assert_eq!(byte_to_dpi(dpi_to_byte(dpi)), dpi);
        }
        assert_eq!(dpi_to_byte(u16::MAX), 252);
    }

    #[test]
    fn percentages_round_trip() {
        assert_eq!(to_percent(0x78), 47);
        assert_eq!(to_percent(255), 100);
        let mut r = Report::new(0x07, 0x81, 0x01, &[]);
        r.args[0] = set_low_battery(30)[0].args[0];
        assert_eq!((get_low_battery().parse)(&r), 30);
    }
}
