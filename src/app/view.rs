//! Shows the app's state in the window and the tray.

use std::{fmt::Write, rc::Rc};

use razer_protocol::{DpiRange, EffectParams, Features, Led, Link, Model as Mouse, Power, ScrollSetting};
use slint::{Color, ComponentHandle, Model, ModelRc, SharedString, VecModel};

use super::{App, State, lighting::colour, pictures::Picture};
use crate::{
    Backend, Choice, DeviceItem, ProfileItem, StageItem, ZoneLook,
    win::{autostart, pointer},
    worker::Info,
};

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items)))
}

/// Replaces a model only when its rows change, so repeated elements keep
/// their state, such as a slider being dragged.
fn rows<T: Clone + PartialEq + 'static>(current: ModelRc<T>, items: Vec<T>, set: impl FnOnce(ModelRc<T>)) {
    let same = current.row_count() == items.len()
        && items
            .iter()
            .enumerate()
            .all(|(i, item)| current.row_data(i).as_ref() == Some(item));
    if !same {
        set(model(items));
    }
}

fn choice(label: impl Into<SharedString>, value: i32) -> Choice {
    Choice {
        label: label.into(),
        value,
    }
}

/// The model's name without the connection, which is shown beside it.
fn display_name(m: &Mouse) -> &'static str {
    let name = m.name;
    name.rfind(" (").map_or(name, |at| &name[..at])
}

fn link_label(link: Link) -> &'static str {
    match link {
        Link::Wired => "Wired",
        Link::Wireless => "Wireless",
        Link::Bluetooth => "Bluetooth",
    }
}

fn features(m: &Mouse) -> Vec<SharedString> {
    let mut out: Vec<SharedString> = Vec::new();
    if m.dpi != DpiRange::None {
        out.push(format!("Up to {} DPI", m.dpi.max()).into());
    }
    if m.features.contains(Features::DPI_STAGES) {
        out.push("DPI stages".into());
    }
    if let Some(top) = m.poll.rates().last() {
        out.push(format!("Polling up to {top} Hz").into());
    }
    for (power, label) in [
        (Power::BATTERY, "Battery"),
        (Power::IDLE, "Sleep timer"),
        (Power::LOW_BATTERY, "Low power mode"),
    ] {
        if m.power.contains(power) {
            out.push(label.into());
        }
    }
    match m.lighting.zones.len() {
        0 => {}
        1 => out.push("Lighting".into()),
        n => out.push(format!("Lighting in {n} zones").into()),
    }
    if m.features.contains(Features::DONGLE_LED) || m.features.contains(Features::DONGLE_LEDS) {
        out.push("Receiver lights".into());
    }
    if ScrollSetting::ALL.iter().any(|s| m.features.contains(s.feature())) {
        out.push("Scroll wheel".into());
    }
    out
}

/// The logo, scroll wheel, sides and underglow of the picture, each shown
/// as the zone that lights it.
fn looks(st: &State, device: &Info) -> Vec<ZoneLook> {
    let zones = device.model.lighting.zones;
    // Each part of the drawing, its own zones first, then zones whose light reaches it.
    let look = |own: &[Led], spill: &[Led]| {
        let has = |led: &Led| zones.iter().any(|z| z.led == *led);
        let mine = own.iter().find(|led| has(led));
        let Some(&led) = mine.or_else(|| spill.iter().find(|led| has(led))) else {
            return ZoneLook {
                present: false,
                label: "".into(),
                kind: -1,
                a: Color::default(),
                b: Color::default(),
            };
        };
        let label = mine.map_or_else(String::new, |led| led.label().to_uppercase());
        let (kind, p) = st.effect(led).map_or((-1, EffectParams::default()), |e| {
            // One-colour LEDs are drawn in white, as they are sent.
            let e = if device.model.lighting.rgb { e } else { e.in_white() };
            (e.kind().index() as i32, e.params(EffectParams::default()))
        });
        ZoneLook {
            present: true,
            label: label.into(),
            kind,
            a: colour(p.a),
            b: colour(p.b),
        }
    };
    vec![
        look(&[Led::Logo], &[Led::Matrix, Led::Backlight]),
        look(&[Led::Scroll], &[Led::Matrix, Led::Backlight]),
        look(&[Led::Left, Led::Right], &[Led::Matrix, Led::Backlight]),
        look(&[Led::Backlight, Led::Matrix], &[]),
    ]
}

impl App {
    pub(super) fn refresh(&self) {
        self.refresh_devices();
        self.refresh_selected();
        self.refresh_profiles();
        self.refresh_settings();
        self.refresh_pointer();
    }

    pub(super) fn refresh_devices(&self) {
        let ui = self.ui();
        let b = ui.global::<Backend>();
        let st = self.state.borrow();
        let items = st
            .devices
            .iter()
            .map(|d| DeviceItem {
                name: display_name(d.model).trim_start_matches("Razer ").into(),
                link: if d.state.awake {
                    link_label(d.model.link).into()
                } else {
                    "Asleep".into()
                },
                battery: d.state.battery.map_or(-1, i32::from),
                charging: d.state.charging.unwrap_or(false),
            })
            .collect();
        rows(b.get_devices(), items, |m| b.set_devices(m));
        b.set_device(if st.devices.is_empty() { -1 } else { st.selected as i32 });

        let mut tooltip = String::from("Razer Native");
        for d in &st.devices {
            let _ = write!(tooltip, "\n{}", display_name(d.model));
            if let Some(level) = d.state.battery {
                let _ = write!(tooltip, ": {level}%");
            }
        }
        self.tray.set_status(tooltip.into());
    }

    pub(super) fn refresh_selected(&self) {
        let ui = self.ui();
        let b = ui.global::<Backend>();
        let st = self.state.borrow();
        let Some(device) = st.devices.get(st.selected) else {
            return;
        };
        let (m, s) = (device.model, &device.state);

        b.set_name(display_name(m).into());
        b.set_link(link_label(m.link).into());
        b.set_asleep(!s.awake);
        let picture = st.pictures.get(&m.pid);
        let (outline, details, [logo, wheel, sides, under], body) = match picture {
            Some(Picture::Ready {
                outline,
                details,
                lights,
                body,
            }) => (outline.clone(), details.clone(), (**lights).clone(), *body),
            _ => (Default::default(), Default::default(), Default::default(), (0., 1.)),
        };
        b.set_art_pending(matches!(picture, Some(Picture::Loading)));
        b.set_art_outline(outline);
        b.set_art_details(details);
        b.set_art_logo(logo);
        b.set_art_wheel(wheel);
        b.set_art_sides(sides);
        b.set_art_under(under);
        b.set_art_body_top(body.0);
        b.set_art_body_bottom(body.1);
        b.set_firmware(match device.firmware {
            (0, 0) => "unknown".into(),
            (major, minor) => format!("{major}.{minor:02}").into(),
        });
        b.set_serial(device.serial.as_str().into());
        b.set_product_id(format!("{:04X}", m.pid).into());
        b.set_path(device.path.as_str().into());
        rows(b.get_features(), features(m), |v| b.set_features(v));

        b.set_has_dpi(m.dpi != DpiRange::None);
        b.set_dpi_min(DpiRange::MIN.into());
        b.set_dpi_max(m.dpi.max().into());
        let dpi = s.dpi.unwrap_or_default();
        b.set_dpi(StageItem {
            x: dpi.x.into(),
            y: dpi.y.into(),
        });
        b.set_has_stages(s.stages.is_some());
        if let Some(stages) = &s.stages {
            let items = stages
                .stages
                .iter()
                .map(|d| StageItem {
                    x: d.x.into(),
                    y: d.y.into(),
                })
                .collect();
            rows(b.get_stages(), items, |v| b.set_stages(v));
            b.set_active_stage(stages.active as i32);
        }
        let rates = m
            .poll
            .rates()
            .iter()
            .map(|&r| choice(r.to_string(), r.into()))
            .collect();
        rows(b.get_poll_rates(), rates, |v| b.set_poll_rates(v));
        b.set_poll(s.poll.map_or(0, i32::from));

        let scroll: Vec<_> = ScrollSetting::ALL
            .into_iter()
            .enumerate()
            .filter(|(_, setting)| m.features.contains(setting.feature()))
            .collect();
        let options = scroll
            .iter()
            .map(|&(i, setting)| choice(setting.label(), i as i32))
            .collect();
        let values = scroll
            .iter()
            .map(|(_, setting)| s.scroll.iter().any(|(k, on)| k == setting && *on))
            .collect();
        rows(b.get_scroll_options(), options, |v| b.set_scroll_options(v));
        rows(b.get_scroll_values(), values, |v| b.set_scroll_values(v));

        let zones = m.lighting.zones;
        let zone_items = zones
            .iter()
            .enumerate()
            .map(|(i, z)| choice(z.label(), i as i32))
            .collect();
        rows(b.get_zones(), zone_items, |v| b.set_zones(v));
        b.set_zone(st.zone as i32);
        b.set_zone_led(st.zone().map_or(-1, |z| z.led as i32));
        if let Some(zone) = st.zone() {
            let effects = zone
                .effects
                .iter()
                .map(|k| choice(razer_protocol::Effect::label(k), k.index() as i32))
                .collect();
            rows(b.get_effects(), effects, |v| b.set_effects(v));
            b.set_effect(st.effect(zone.led).map_or(-1, |e| e.kind().index() as i32));
            b.set_has_brightness(zone.brightness);
            b.set_has_colour(m.lighting.rgb);
            let level = s
                .brightness
                .iter()
                .find(|(led, _)| *led == zone.led)
                .map_or(255, |(_, l)| *l);
            b.set_brightness(level.into());
        }
        b.set_colour_a(colour(st.params.a));
        b.set_colour_b(colour(st.params.b));
        b.set_speed(st.params.speed.into());
        b.set_direction(st.params.direction.into());
        rows(b.get_looks(), looks(&st, device), |v| b.set_looks(v));

        b.set_has_battery(m.power.contains(Power::BATTERY));
        b.set_battery(s.battery.map_or(0, i32::from));
        b.set_charging(s.charging.unwrap_or(false));
        b.set_has_idle(m.power.contains(Power::IDLE) && s.idle.is_some());
        b.set_idle(s.idle.map_or(300, i32::from));
        b.set_has_low_battery(m.power.contains(Power::LOW_BATTERY) && s.low_battery.is_some());
        b.set_low_battery(s.low_battery.map_or(15, i32::from));
        let (modes, leds): (Vec<Choice>, Vec<i32>) = match (s.dongle_leds, s.dongle_led) {
            (Some(leds), _) => (
                [
                    ("Off", 0),
                    ("Battery", 1),
                    ("Connection", 2),
                    ("Polling", 3),
                    ("DPI", 4),
                ]
                .map(|(l, v)| choice(l, v))
                .to_vec(),
                leds.map(i32::from).to_vec(),
            ),
            (None, Some(led)) => (
                [("Connection", 1), ("Battery", 2), ("Low battery", 3)]
                    .map(|(l, v)| choice(l, v))
                    .to_vec(),
                vec![led.into()],
            ),
            _ => (Vec::new(), Vec::new()),
        };
        rows(b.get_dongle_modes(), modes, |v| b.set_dongle_modes(v));
        rows(b.get_dongle_leds(), leds, |v| b.set_dongle_leds(v));
    }

    pub(super) fn refresh_profiles(&self) {
        let ui = self.ui();
        let b = ui.global::<Backend>();
        let st = self.state.borrow();
        let items = st
            .profiles
            .list
            .iter()
            .map(|p| ProfileItem {
                name: p.name.as_str().into(),
                programs: model(p.programs.iter().map(|e| SharedString::from(e.as_str())).collect()),
            })
            .collect::<Vec<_>>();
        // Profiles hold nested models, which never compare equal, so compare by content.
        let current = b.get_profiles();
        let changed = current.row_count() != items.len()
            || items.iter().enumerate().any(|(i, item)| {
                current
                    .row_data(i)
                    .is_none_or(|old| old.name != item.name || !old.programs.iter().eq(item.programs.iter()))
            });
        if changed {
            b.set_profiles(model(items));
        }
        b.set_profile(st.profiles.active as i32);

        let names = st
            .profiles
            .list
            .iter()
            .map(|p| SharedString::from(p.name.as_str()))
            .collect();
        rows(self.tray.get_profiles(), names, |v| self.tray.set_profiles(v));
        self.tray.set_profile(st.profiles.active as i32);
    }

    /// Windows' pointer settings, which other programs may change too.
    pub(super) fn refresh_pointer(&self) {
        let ui = self.ui();
        let b = ui.global::<Backend>();
        let p = pointer::read();
        b.set_pointer_speed(p.speed as i32);
        b.set_pointer_precision(p.precision);
        b.set_scroll_lines(p.scroll_lines as i32);
        b.set_double_click(p.double_click_ms as i32);
    }

    pub(super) fn refresh_settings(&self) {
        let ui = self.ui();
        let b = ui.global::<Backend>();
        let st = self.state.borrow();
        b.set_start_with_windows(autostart::enabled());
        b.set_close_to_tray(st.settings.close_to_tray);
        b.set_auto_switch(st.settings.auto_switch);
        b.set_gpu(st.settings.gpu);
        b.set_pictures(st.settings.pictures);
    }
}
