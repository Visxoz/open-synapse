use std::collections::BTreeMap;

use razer_protocol::Setting;
use serde::{Deserialize, Serialize};

const FILE: &str = "profiles.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    /// Lower-case executable names that bring this profile to the front.
    #[serde(default)]
    pub programs: Vec<String>,
    /// Each mouse's settings, by USB product ID, one per slot.
    #[serde(default)]
    pub devices: BTreeMap<u16, Vec<Setting>>,
}

impl Profile {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn settings(&self, pid: u16) -> Option<&[Setting]> {
        self.devices.get(&pid).map(Vec::as_slice)
    }

    /// Keeps `setting`, replacing any earlier one for the same slot.
    pub fn store(&mut self, pid: u16, setting: Setting) {
        let list = self.devices.entry(pid).or_default();
        match list.iter_mut().find(|s| s.slot() == setting.slot()) {
            Some(old) => *old = setting,
            None => list.push(setting),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profiles {
    pub active: usize,
    pub list: Vec<Profile>,
}

impl Default for Profiles {
    fn default() -> Self {
        Self {
            active: 0,
            list: vec![Profile::named("Default")],
        }
    }
}

impl Profiles {
    pub fn load() -> Self {
        let mut profiles: Self = crate::storage::load(FILE);
        if profiles.list.is_empty() {
            profiles = Self::default();
        }
        profiles.active = profiles.active.min(profiles.list.len() - 1);
        profiles
    }

    pub fn save(&self) {
        // A demo mouse must not end up in the real profiles.
        if cfg!(debug_assertions) && std::env::var_os("RAZER_DEMO").is_some() {
            return;
        }
        crate::storage::save(FILE, self);
    }

    pub fn active(&self) -> &Profile {
        &self.list[self.active]
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        &mut self.list[self.active]
    }

    pub fn for_program(&self, exe: &str) -> Option<usize> {
        self.list.iter().position(|p| p.programs.iter().any(|e| e == exe))
    }

    /// A copy of the active profile under a fresh name, made active.
    pub fn duplicate_active(&mut self) -> usize {
        let name = (self.list.len() + 1..)
            .map(|n| format!("Profile {n}"))
            .find(|n| self.list.iter().all(|p| &p.name != n))
            .expect("a free name");
        let mut copy = self.active().clone();
        copy.name = name;
        copy.programs.clear();
        self.list.push(copy);
        self.active = self.list.len() - 1;
        self.active
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if self.list.len() <= 1 || index >= self.list.len() {
            return false;
        }
        self.list.remove(index);
        if self.active > index || self.active == self.list.len() {
            self.active = self.active.saturating_sub(1);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use razer_protocol::{Dpi, DpiStages};

    #[test]
    fn storing_replaces_the_same_slot() {
        let mut p = Profile::named("A");
        p.store(1, Setting::Poll(500));
        p.store(1, Setting::Idle(300));
        p.store(1, Setting::Poll(1000));
        assert_eq!(p.settings(1), Some(&[Setting::Poll(1000), Setting::Idle(300)][..]));
    }

    #[test]
    fn duplicates_get_unique_names() {
        let mut ps = Profiles::default();
        ps.list[0].programs.push("game.exe".into());
        ps.duplicate_active();
        ps.active = 0;
        ps.duplicate_active();
        let names: Vec<_> = ps.list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Default", "Profile 2", "Profile 3"]);
        assert!(ps.list[1].programs.is_empty());
    }

    #[test]
    fn removing_keeps_the_active_one() {
        let mut ps = Profiles::default();
        ps.duplicate_active();
        ps.duplicate_active();
        ps.active = 2;
        assert!(ps.remove(0));
        assert_eq!(ps.active().name, "Profile 3");
        assert!(ps.remove(1));
        assert!(!ps.remove(0));
    }

    #[test]
    fn survives_a_round_trip() {
        let mut ps = Profiles::default();
        ps.active_mut().store(
            0x00E6,
            Setting::Stages(DpiStages {
                active: 1,
                stages: vec![Dpi::square(800)],
            }),
        );
        let json = serde_json::to_string(&ps).unwrap();
        assert_eq!(serde_json::from_str::<Profiles>(&json).unwrap(), ps);
    }
}
