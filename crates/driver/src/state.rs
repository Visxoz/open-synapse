use razer_protocol::{Dpi, DpiStages, Features, Led, Power, Query, ScrollSetting, Setting, commands, lighting};

use crate::device::Device;

/// What a mouse reports about itself; `None` where it lacks the setting or
/// did not answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The device answered; a receiver's mouse may be asleep or off.
    pub awake: bool,
    pub dpi: Option<Dpi>,
    pub stages: Option<DpiStages>,
    pub poll: Option<u16>,
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    pub idle: Option<u16>,
    pub low_battery: Option<u8>,
    pub dongle_led: Option<u8>,
    pub dongle_leds: Option<[u8; 3]>,
    pub brightness: Vec<(Led, u8)>,
    pub scroll: Vec<(ScrollSetting, bool)>,
}

impl State {
    /// The reported values as settings, to be kept in a profile.
    pub fn settings(&self) -> Vec<Setting> {
        let mut out = Vec::new();
        match (&self.stages, self.dpi) {
            (Some(stages), _) => out.push(Setting::Stages(stages.clone())),
            (None, Some(dpi)) => out.push(Setting::Dpi(dpi)),
            _ => {}
        }
        out.extend(self.poll.map(Setting::Poll));
        out.extend(self.idle.map(Setting::Idle));
        out.extend(self.low_battery.map(Setting::LowBattery));
        out.extend(self.dongle_led.map(Setting::DongleLed));
        out.extend(self.dongle_leds.map(Setting::DongleLeds));
        out.extend(self.brightness.iter().map(|&(led, b)| Setting::Brightness(led, b)));
        out.extend(self.scroll.iter().map(|&(s, on)| Setting::Scroll(s, on)));
        out
    }

    /// Takes on `setting` as though the device had reported it.
    pub fn apply(&mut self, setting: &Setting) {
        fn put<K: PartialEq, V>(list: &mut Vec<(K, V)>, key: K, value: V) {
            match list.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = value,
                None => list.push((key, value)),
            }
        }
        match setting {
            Setting::Dpi(dpi) => self.dpi = Some(*dpi),
            Setting::Stages(stages) => {
                self.dpi = stages.stages.get(stages.active).copied().or(self.dpi);
                self.stages = Some(stages.clone());
            }
            Setting::Poll(rate) => self.poll = Some(*rate),
            Setting::Idle(seconds) => self.idle = Some(*seconds),
            Setting::LowBattery(percent) => self.low_battery = Some(*percent),
            Setting::DongleLed(mode) => self.dongle_led = Some(*mode),
            Setting::DongleLeds(modes) => self.dongle_leds = Some(*modes),
            Setting::Brightness(led, level) => put(&mut self.brightness, *led, *level),
            Setting::Scroll(s, on) => put(&mut self.scroll, *s, *on),
            Setting::Effect(..) => {}
        }
    }
}

impl Device {
    /// Asks `q` if the model has what it reads, `None` otherwise or on silence.
    fn read<T>(&self, has: bool, q: Query<T>) -> Option<T> {
        if has { self.query(q).ok() } else { None }
    }

    /// Everything the mouse can report.
    pub fn read_state(&self) -> State {
        let m = self.model;
        let has = |f| m.features.contains(f);
        let power = |p| m.power.contains(p);
        let mut state = State {
            poll: commands::get_poll(m).and_then(|q| self.query(q).ok()),
            idle: self.read(power(Power::IDLE), commands::get_idle()),
            low_battery: self.read(power(Power::LOW_BATTERY), commands::get_low_battery()),
            dongle_led: self.read(has(Features::DONGLE_LED), commands::get_dongle_led()),
            dongle_leds: self.read(has(Features::DONGLE_LEDS), commands::get_dongle_leds()),
            brightness: m
                .lighting
                .zones
                .iter()
                .filter_map(|z| Some((z.led, self.query(lighting::get_brightness(m, z.led)?).ok()?)))
                .collect(),
            scroll: ScrollSetting::ALL
                .into_iter()
                .filter(|s| has(s.feature()))
                .filter_map(|s| Some((s, self.query(commands::get_scroll(s)).ok()?)))
                .collect(),
            ..State::default()
        };
        self.read_live(&mut state);
        state.awake |= state.poll.is_some();
        state
    }

    /// The values that change without us: the DPI button and the battery.
    pub fn read_live(&self, state: &mut State) {
        let m = self.model;
        state.dpi = commands::get_dpi(m).and_then(|q| self.query(q).ok());
        state.stages = self.read(m.features.contains(Features::DPI_STAGES), commands::get_stages());
        state.battery = self.read(m.power.contains(Power::BATTERY), commands::battery());
        state.charging = self.read(m.power.contains(Power::CHARGING), commands::charging());
        state.awake = state.dpi.is_some() || state.stages.is_some() || state.battery.is_some();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_reproduce_the_state() {
        let state = State {
            dpi: Some(Dpi::square(800)),
            stages: Some(DpiStages {
                active: 0,
                stages: vec![Dpi::square(800), Dpi::square(1600)],
            }),
            poll: Some(1000),
            idle: Some(300),
            low_battery: Some(20),
            dongle_leds: Some([2, 1, 3]),
            brightness: vec![(Led::Logo, 128)],
            scroll: vec![(ScrollSetting::SmartReel, true)],
            ..State::default()
        };
        let mut rebuilt = State::default();
        for s in state.settings() {
            rebuilt.apply(&s);
        }
        assert_eq!(rebuilt, state);
    }
}
