//! Wires the window's and tray's callbacks to the app.

use std::rc::Rc;

use razer_protocol::{Dpi, Effects, ScrollSetting, Setting};
use slint::ComponentHandle;

use super::{App, lighting::rgb};
use crate::{
    AppWindow, Backend,
    platform::{autostart, pointer},
};

fn u16_of(v: i32) -> u16 {
    v.clamp(0, i32::from(u16::MAX)) as u16
}

impl App {
    pub(super) fn bind(self: &Rc<Self>, ui: &AppWindow) {
        // `on!(target.on_x, |app, args| body)` binds a callback to a weak app.
        macro_rules! on {
            ($target:expr, $setter:ident, |$app:ident $(, $arg:ident)*| $body:expr) => {{
                let weak = Rc::downgrade(self);
                $target.$setter(move |$($arg),*| {
                    if let Some($app) = weak.upgrade() {
                        $body
                    }
                });
            }};
        }
        let b = ui.global::<Backend>();
        b.set_version(env!("CARGO_PKG_VERSION").into());
        b.set_has_pointer(pointer::AVAILABLE);

        on!(b, on_select_device, |app, i| app.select_device(i as usize));
        on!(b, on_activate_stage, |app, i| app
            .edit_stages(|s, _| s.active = i as usize));
        on!(b, on_set_stage, |app, i, x, y| app.edit_stages(|s, _| {
            if let Some(stage) = s.stages.get_mut(i as usize) {
                *stage = Dpi {
                    x: u16_of(x),
                    y: u16_of(y),
                };
            }
        }));
        on!(b, on_add_stage, |app| app.add_stage());
        on!(b, on_remove_stage, |app, i| app.remove_stage(i as usize));
        on!(b, on_set_dpi, |app, x, y| app.change(Setting::Dpi(Dpi {
            x: u16_of(x),
            y: u16_of(y)
        })));
        on!(b, on_set_poll, |app, rate| app.change(Setting::Poll(u16_of(rate))));
        on!(b, on_set_scroll, |app, i, on| {
            if let Some(&setting) = ScrollSetting::ALL.get(i as usize) {
                app.change(Setting::Scroll(setting, on));
            }
        });

        // Each write is broadcast to every window, so sliders send only when released.
        on!(b, on_set_pointer_speed, |app, v| {
            pointer::set_speed(v.max(0) as u32);
            app.refresh_pointer();
        });
        on!(b, on_set_pointer_precision, |app, on| {
            pointer::set_precision(on);
            app.refresh_pointer();
        });
        on!(b, on_set_scroll_lines, |app, v| {
            pointer::set_scroll_lines(v.max(1) as u32);
            app.refresh_pointer();
        });
        on!(b, on_set_double_click, |app, v| {
            pointer::set_double_click(v.max(0) as u32);
            app.refresh_pointer();
        });

        on!(b, on_select_zone, |app, i| app.select_zone(i as usize));
        on!(b, on_set_effect, |app, kind| app
            .set_effect(Effects::from_index(kind as u32)));
        on!(b, on_set_colour, |app, which, c| app.edit_params(|p| {
            if which == 0 { p.a = rgb(c) } else { p.b = rgb(c) }
        }));
        on!(b, on_set_speed, |app, v| app.edit_params(|p| p.speed = v as u8));
        on!(b, on_set_direction, |app, v| app.edit_params(|p| p.direction = v as u8));
        on!(b, on_set_brightness, |app, v| app.set_brightness(v.clamp(0, 255) as u8));

        on!(b, on_set_idle, |app, seconds| app
            .change(Setting::Idle(u16_of(seconds))));
        on!(b, on_set_low_battery, |app, percent| app
            .change(Setting::LowBattery(percent.clamp(0, 100) as u8)));
        on!(b, on_set_dongle_led, |app, i, mode| app
            .set_dongle_led(i as usize, mode.clamp(0, 4) as u8));

        on!(b, on_select_profile, |app, i| app.activate_profile(i as usize, true));
        on!(b, on_add_profile, |app| app.add_profile());
        on!(b, on_rename_profile, |app, i, name| app
            .rename_profile(i as usize, &name));
        on!(b, on_delete_profile, |app, i| app.delete_profile(i as usize));
        on!(b, on_list_programs, |app| app.list_programs());
        on!(b, on_add_program, |app, i, exe| app.edit_programs(i as usize, |list| {
            let exe = exe.to_lowercase();
            if !list.contains(&exe) {
                list.push(exe);
            }
        }));
        on!(b, on_remove_program, |app, i, p| app.edit_programs(
            i as usize,
            |list| {
                if (p as usize) < list.len() {
                    list.remove(p as usize);
                }
            }
        ));

        on!(b, on_set_start_at_login, |app, on| {
            if let Err(e) = autostart::set(on) {
                app.notify(&format!("Could not change start-up: {e}"));
            }
            app.refresh_settings();
        });
        on!(b, on_set_close_to_tray, |app, on| {
            app.set_settings(|s| s.close_to_tray = on);
            app.refresh_settings();
        });
        on!(b, on_set_auto_switch, |app, on| {
            app.set_settings(|s| s.auto_switch = on);
            app.watch_foreground();
            app.refresh_settings();
        });

        on!(b, on_set_pictures, |app, on| app.set_pictures(on));
        on!(b, on_set_gpu, |app, on| {
            app.set_settings(|s| s.gpu = on);
            app.notify("Restart Open Synapse to apply hardware acceleration.");
            app.refresh_settings();
        });

        on!(b, on_set_check_updates, |app, on| {
            app.set_settings(|s| s.check_updates = on);
            app.refresh_settings();
            app.check_for_update();
        });
        on!(b, on_install_update, |app| app.install_update());

        on!(self.tray, on_panel, |app| app.toggle_panel());
        on!(self.tray, on_open, |app| app.show());
        on!(self.tray, on_choose, |app, i| app.activate_profile(i as usize, true));
        on!(self.tray, on_quit, |app| app.quit());

        on!(self.panel, on_select_device, |app, i| app.select_device(i as usize));
        on!(self.panel, on_activate_stage, |app, i| app
            .edit_stages(|s, _| s.active = i as usize));
        on!(self.panel, on_set_poll, |app, rate| app
            .change(Setting::Poll(u16_of(rate))));
        on!(self.panel, on_select_profile, |app, i| app
            .activate_profile(i as usize, true));
        on!(self.panel, on_open, |app| {
            app.hide_panel();
            app.show();
        });
        on!(self.panel, on_quit, |app| app.quit());
        on!(self.panel, on_dismiss, |app| app.hide_panel());
    }

    fn set_dongle_led(&self, index: usize, mode: u8) {
        let st = self.state.borrow();
        let Some(device) = st.devices.get(st.selected) else {
            return;
        };
        let setting = match (device.state.dongle_leds, device.state.dongle_led) {
            (Some(mut modes), _) if index < 3 => {
                modes[index] = mode;
                Setting::DongleLeds(modes)
            }
            (None, Some(_)) => Setting::DongleLed(mode),
            _ => return,
        };
        drop(st);
        self.change(setting);
    }
}
