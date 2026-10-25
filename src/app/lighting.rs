//! The lighting page: zones, effects and their colours.

use razer_protocol::{Effect, EffectParams, Effects, Led, Rgb, Setting, Zone};

use super::{App, State};

impl State {
    pub(super) fn zone(&self) -> Option<Zone> {
        let device = self.devices.get(self.selected)?;
        device.model.lighting.zones.get(self.zone).copied()
    }

    /// The effect the active profile keeps for `led` on the selected device.
    pub(super) fn effect(&self, led: Led) -> Option<Effect> {
        let pid = self.devices.get(self.selected)?.model.pid;
        self.profiles.active().settings(pid)?.iter().find_map(|s| match s {
            Setting::Effect(l, e) if *l == led => Some(*e),
            _ => None,
        })
    }

    pub(super) fn load_params(&mut self) {
        let effect = self.zone().and_then(|z| self.effect(z.led));
        self.params = effect.map_or_else(EffectParams::default, |e| e.params(EffectParams::default()));
    }
}

pub fn rgb(c: slint::Color) -> Rgb {
    Rgb::new(c.red(), c.green(), c.blue())
}

pub fn colour(c: Rgb) -> slint::Color {
    slint::Color::from_rgb_u8(c.r, c.g, c.b)
}

impl App {
    pub(super) fn select_zone(&self, index: usize) {
        let mut st = self.state.borrow_mut();
        st.zone = index;
        st.load_params();
        drop(st);
        self.refresh_selected();
    }

    pub(super) fn set_effect(&self, kind: Effects) {
        let st = self.state.borrow();
        let Some(zone) = st.zone() else { return };
        let effect = Effect::build(kind, &st.params);
        drop(st);
        if let Some(effect) = effect {
            self.change(Setting::Effect(zone.led, effect));
        }
    }

    /// Changes the parameters, then shows the zone's effect with them.
    pub(super) fn edit_params(&self, edit: impl FnOnce(&mut EffectParams)) {
        let mut st = self.state.borrow_mut();
        edit(&mut st.params);
        let Some(zone) = st.zone() else { return };
        let old = st.effect(zone.led);
        let kind = old.map_or(Effects::STATIC, |e| e.kind());
        let new = Effect::build(kind, &st.params).filter(|n| zone.effects.contains(kind) && Some(*n) != old);
        drop(st);
        match new {
            Some(effect) => self.change(Setting::Effect(zone.led, effect)),
            None => self.refresh_selected(),
        }
    }

    pub(super) fn set_brightness(&self, level: u8) {
        let zone = self.state.borrow().zone();
        if let Some(zone) = zone.filter(|z| z.brightness) {
            self.change(Setting::Brightness(zone.led, level));
        }
    }
}
