//! Each connected model's picture, loaded off the UI thread.

use std::collections::HashMap;

use slint::Image;

use super::{App, with_app};
use crate::{
    ArtLight,
    artwork::{self, Art, Light},
};

pub(super) enum Picture {
    Loading,
    Ready {
        outline: Image,
        details: Image,
        /// The logo, wheel, sides and underglow as the picture shows them lit.
        lights: Box<[ArtLight; 4]>,
        /// The top and bottom of the mouse's body, as fractions of the drawing.
        body: (f32, f32),
    },
    /// No picture, or it could not be fetched; the generic drawing stands in.
    Missing,
}

pub(super) type Pictures = HashMap<u16, Picture>;

impl App {
    /// Starts loading the pictures of connected models that lack one.
    pub(super) fn load_pictures(&self) {
        let mut st = self.state.borrow_mut();
        if !st.settings.pictures {
            return;
        }
        let wanted: Vec<_> = st
            .devices
            .iter()
            .map(|d| d.model)
            .filter(|m| !st.pictures.contains_key(&m.pid))
            .collect();
        for model in wanted {
            let url = model.image_url();
            if url.is_none() && !artwork::cached(model.pid) {
                st.pictures.insert(model.pid, Picture::Missing);
                continue;
            }
            st.pictures.insert(model.pid, Picture::Loading);
            let pid = model.pid;
            let spawned = std::thread::Builder::new().name("picture".into()).spawn(move || {
                let art = artwork::load(pid, url)
                    .inspect_err(|e| log::warn!("no picture for {pid:04X}: {e}"))
                    .ok();
                let _ = slint::invoke_from_event_loop(move || with_app(|app| app.picture_loaded(pid, art)));
            });
            if spawned.is_err() {
                st.pictures.insert(pid, Picture::Missing);
            }
        }
    }

    fn picture_loaded(&self, pid: u16, art: Option<Art>) {
        let light = |light: Option<Light>| {
            light.map_or_else(ArtLight::default, |l| ArtLight {
                image: Image::from_rgba8(l.mask),
                cover: Image::from_rgba8(l.cover),
                x: l.at.0,
                y: l.at.1,
                right: l.right,
                left: l.bounds[0],
                top: l.bounds[1],
                width: l.bounds[2],
                height: l.bounds[3],
            })
        };
        let picture = match art {
            Some(art) => Picture::Ready {
                outline: Image::from_rgba8(art.outline),
                details: Image::from_rgba8(art.details),
                lights: Box::new([
                    light(art.lights.logo),
                    light(art.lights.wheel),
                    light(art.lights.sides),
                    light(art.lights.under),
                ]),
                body: art.lights.body,
            },
            None => Picture::Missing,
        };
        let mut st = self.state.borrow_mut();
        if st.settings.pictures {
            st.pictures.insert(pid, picture);
        }
        drop(st);
        self.refresh_selected();
    }

    pub(super) fn set_pictures(&self, on: bool) {
        self.set_settings(|s| s.pictures = on);
        if on {
            self.load_pictures();
        } else {
            self.state.borrow_mut().pictures.clear();
        }
        self.refresh_selected();
        self.refresh_settings();
    }
}
