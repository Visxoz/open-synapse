//! Devices coming and going, and settings written to them.

use std::time::{Duration, Instant};

use razer_protocol::{Dpi, DpiRange, DpiStages, MAX_STAGES, Setting};

use super::App;
use crate::{
    profiles::Profiles,
    worker::{DeviceId, Info},
};

/// How long after a change of the user's polled values are ignored, as
/// they may predate it.
const SETTLE: Duration = Duration::from_millis(1500);

impl App {
    /// Brings `device` in line with the active profile, or lends the profile
    /// its settings if it has none for this model yet.
    pub(super) fn sync_profile(&self, profiles: &mut Profiles, device: &mut Info) {
        if !device.state.awake {
            return;
        }
        let pid = device.model.pid;
        match profiles.active().settings(pid).map(<[Setting]>::to_vec) {
            Some(settings) => {
                for s in &settings {
                    device.state.apply(s);
                }
                self.worker.apply(device.id, settings);
            }
            None => {
                for s in device.state.settings() {
                    profiles.active_mut().store(pid, s);
                }
                self.save_soon();
            }
        }
    }

    pub(super) fn connected(&self, mut info: Info) {
        let mut st = self.state.borrow_mut();
        self.sync_profile(&mut st.profiles, &mut info);
        st.devices.push(info);
        if st.devices.len() == 1 {
            st.selected = 0;
            st.zone = 0;
            st.load_params();
        }
        drop(st);
        // First, so the picture counts as loading when the device is shown.
        self.load_pictures();
        self.refresh();
    }

    pub(super) fn disconnected(&self, id: DeviceId) {
        let mut guard = self.state.borrow_mut();
        let st = &mut *guard;
        let Some(index) = st.devices.iter().position(|d| d.id == id) else {
            return;
        };
        st.devices.remove(index);
        // Pictures of models no longer connected are not worth their memory.
        let devices = &st.devices;
        st.pictures.retain(|pid, _| devices.iter().any(|d| d.model.pid == *pid));
        if st.selected > index {
            st.selected -= 1;
        } else if st.selected == index {
            // The next device takes its place, with its own zones.
            st.selected = index.min(devices.len().saturating_sub(1));
            st.zone = 0;
            st.load_params();
        }
        drop(guard);
        self.refresh();
    }

    pub(super) fn changed(&self, info: Info) {
        {
            let mut guard = self.state.borrow_mut();
            let st = &mut *guard;
            let recent = st.touched.is_some_and(|t| t.elapsed() < SETTLE);
            let Some(device) = st.devices.iter_mut().find(|d| d.id == info.id) else {
                return;
            };
            let woke = !device.state.awake && info.state.awake;
            if recent && !woke {
                device.state.battery = info.state.battery;
                device.state.charging = info.state.charging;
                device.state.awake = info.state.awake;
            } else {
                *device = info;
            }
            if woke {
                self.sync_profile(&mut st.profiles, device);
                st.load_params();
            }
        }
        self.refresh_devices();
        self.refresh_selected();
    }

    pub(super) fn select_device(&self, index: usize) {
        let mut st = self.state.borrow_mut();
        if index < st.devices.len() && index != st.selected {
            st.selected = index;
            st.zone = 0;
            st.load_params();
        }
        drop(st);
        self.refresh_devices();
        self.refresh_selected();
    }

    /// Writes `setting` to the selected device and keeps it in the active
    /// profile.
    pub(super) fn change(&self, setting: Setting) {
        let mut st = self.state.borrow_mut();
        let selected = st.selected;
        let Some(device) = st.devices.get_mut(selected) else {
            return;
        };
        device.state.apply(&setting);
        let (id, pid) = (device.id, device.model.pid);
        st.profiles.active_mut().store(pid, setting.clone());
        st.touched = Some(Instant::now());
        drop(st);
        self.worker.apply(id, vec![setting]);
        self.save_soon();
        self.refresh_selected();
    }

    /// Edits the selected device's DPI stages.
    pub(super) fn edit_stages(&self, edit: impl FnOnce(&mut DpiStages, DpiRange)) {
        let st = self.state.borrow();
        let Some(device) = st.devices.get(st.selected) else {
            return;
        };
        let Some(mut stages) = device.state.stages.clone() else {
            return;
        };
        let range = device.model.dpi;
        drop(st);
        edit(&mut stages, range);
        if stages.stages.is_empty() {
            return;
        }
        stages.active = stages.active.min(stages.stages.len() - 1);
        self.change(Setting::Stages(stages));
    }

    pub(super) fn add_stage(&self) {
        self.edit_stages(|s, range| {
            if s.stages.len() < MAX_STAGES {
                let last = s.stages.last().map_or(800, |d| d.x);
                s.stages.push(Dpi::square(range.clamp(last.saturating_mul(2))));
                s.active = s.stages.len() - 1;
            }
        });
    }

    pub(super) fn remove_stage(&self, index: usize) {
        self.edit_stages(|s, _| {
            if s.stages.len() > 1 && index < s.stages.len() {
                s.stages.remove(index);
                if s.active > index || s.active == s.stages.len() {
                    s.active = s.active.saturating_sub(1);
                }
            }
        });
    }
}
