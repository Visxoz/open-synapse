//! Each connected model's picture, loaded off the UI thread.

use std::collections::HashMap;

use slint::Image;

use super::{App, with_app};
use crate::artwork::{self, Art};

pub(super) enum Picture {
    Loading,
    Ready {
        outline: Image,
        details: Image,
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
        let picture = match art {
            Some(art) => Picture::Ready {
                outline: Image::from_rgba8(art.outline),
                details: Image::from_rgba8(art.details),
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
