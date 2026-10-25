//! Switching, editing and linking profiles.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use super::App;
use crate::win::programs::{self, ForegroundHook};

impl App {
    /// Makes profile `index` active and applies it to every device; a
    /// device it has no settings for lends it its current ones.
    pub(super) fn activate_profile(&self, index: usize, chosen: bool) {
        let mut st = self.state.borrow_mut();
        if index >= st.profiles.list.len() {
            return;
        }
        if chosen {
            st.chosen = index;
        }
        if index == st.profiles.active {
            return;
        }
        st.profiles.active = index;
        drop(st);
        self.apply_active();
    }

    fn apply_active(&self) {
        {
            let mut guard = self.state.borrow_mut();
            let st = &mut *guard;
            for device in &mut st.devices {
                self.sync_profile(&mut st.profiles, device);
            }
            st.load_params();
        }
        self.save_soon();
        self.refresh();
    }

    pub(super) fn add_profile(&self) {
        let mut st = self.state.borrow_mut();
        st.chosen = st.profiles.duplicate_active();
        drop(st);
        self.save_soon();
        self.refresh();
    }

    pub(super) fn rename_profile(&self, index: usize, name: &str) {
        let name = name.trim();
        let mut st = self.state.borrow_mut();
        match st.profiles.list.get_mut(index) {
            Some(p) if !name.is_empty() && p.name != name => p.name = name.to_owned(),
            _ => return,
        }
        drop(st);
        self.save_soon();
        self.refresh_profiles();
    }

    pub(super) fn delete_profile(&self, index: usize) {
        let mut st = self.state.borrow_mut();
        let was_active = st.profiles.active;
        if !st.profiles.remove(index) {
            return;
        }
        if st.chosen >= st.profiles.list.len() || st.chosen == index {
            st.chosen = st.profiles.active;
        } else if st.chosen > index {
            st.chosen -= 1;
        }
        drop(st);
        if was_active == index {
            self.apply_active();
        } else {
            self.save_soon();
            self.refresh();
        }
    }

    pub(super) fn edit_programs(&self, index: usize, edit: impl FnOnce(&mut Vec<String>)) {
        let mut st = self.state.borrow_mut();
        let Some(profile) = st.profiles.list.get_mut(index) else {
            return;
        };
        edit(&mut profile.programs);
        drop(st);
        self.save_soon();
        self.refresh_profiles();
    }

    pub(super) fn list_programs(&self) {
        let running: Vec<slint::SharedString> = programs::running().into_iter().map(Into::into).collect();
        self.ui()
            .global::<crate::Backend>()
            .set_running_programs(ModelRc::from(Rc::new(VecModel::from(running))));
    }

    /// Follows the program in front while automatic switching is on.
    pub(super) fn watch_foreground(&self) {
        let on = self.state.borrow().settings.auto_switch;
        let mut hook = self.foreground.borrow_mut();
        if on && hook.is_none() {
            let own = programs::own_exe();
            *hook = ForegroundHook::new(move |exe| {
                if Some(&exe) != own.as_ref() {
                    super::with_app(|app| app.foreground_changed(&exe));
                }
            });
        } else if !on {
            *hook = None;
        }
    }

    fn foreground_changed(&self, exe: &str) {
        let st = self.state.borrow();
        let target = st.profiles.for_program(exe).unwrap_or(st.chosen);
        let current = st.profiles.active;
        drop(st);
        if target != current {
            log::info!("{exe} is in front; switching profile");
            self.activate_profile(target, false);
        }
    }
}
