#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod artwork;
mod logger;
mod profiles;
mod settings;
mod storage;
mod update;
mod win;
mod worker;

use std::cell::Cell;

use i_slint_backend_winit::winit::{
    platform::windows::{CornerPreference, WindowAttributesExtWindows},
    window::WindowAttributes,
};
use slint::{CloseRequestResponse, ComponentHandle, PhysicalPosition, PhysicalSize};

use crate::{app::App, settings::WindowGeometry};

slint::include_modules!();

/// The window's background, matched by the title bar.
const BACKGROUND: slint::Color = slint::Color::from_rgb_u8(0x0b, 0x0b, 0x0e);

thread_local! {
    /// Set just before the tray panel is made, so the backend builds its
    /// window as a pop-up rather than another app window.
    static POPUP_NEXT: Cell<bool> = const { Cell::new(false) };
}

/// A pop-up: no frame, square corners and no button in the taskbar.
fn popup_attributes(attributes: WindowAttributes) -> WindowAttributes {
    attributes
        .with_decorations(false)
        .with_resizable(false)
        .with_skip_taskbar(true)
        .with_corner_preference(CornerPreference::DoNotRound)
}

fn restore_geometry(ui: &AppWindow, geometry: Option<WindowGeometry>) {
    let Some(g) = geometry else { return };
    if g.width >= 800 && g.height >= 500 {
        ui.window().set_size(PhysicalSize::new(g.width, g.height));
        ui.window().set_position(PhysicalPosition::new(g.x, g.y));
    }
    if g.maximized {
        ui.window().set_maximized(true);
    }
}

fn current_geometry(ui: &AppWindow, previous: Option<WindowGeometry>) -> Option<WindowGeometry> {
    let window = ui.window();
    if window.is_minimized() {
        return previous;
    }
    if window.is_maximized() {
        return Some(WindowGeometry {
            maximized: true,
            ..previous.unwrap_or_default()
        });
    }
    let (size, position) = (window.size(), window.position());
    Some(WindowGeometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: false,
    })
}

/// Skia on the CPU, which keeps the GPU driver and its memory out of the
/// process, or, when chosen, on the GPU through Direct3D 12 rather than the
/// Vulkan wgpu might otherwise pick.
fn select_renderer(gpu: bool) {
    if gpu && std::env::var_os("WGPU_BACKEND").is_none() {
        // SAFETY: called first thing in `main`, before any other thread starts.
        unsafe { std::env::set_var("WGPU_BACKEND", "dx12") };
    }
    // Slint honours the variable itself, with no pop-up attributes.
    if std::env::var_os("SLINT_BACKEND").is_some() {
        return;
    }
    let backend = i_slint_backend_winit::Backend::builder()
        .with_renderer_name(if gpu { "skia" } else { "skia-software" })
        .with_window_attributes_hook(|attributes| {
            if POPUP_NEXT.take() {
                popup_attributes(attributes)
            } else {
                attributes
            }
        })
        .build();
    if let Err(e) = backend
        .map_err(|e| e.to_string())
        .and_then(|b| slint::platform::set_platform(Box::new(b)).map_err(|e| e.to_string()))
    {
        log::warn!("could not select the renderer: {e}");
    }
}

fn main() {
    logger::init();
    let settings = settings::Settings::load();
    select_renderer(settings.gpu);
    let Some(mut instance) = win::single_instance::acquire() else {
        return;
    };
    update::clean_up();

    let ui = AppWindow::new().expect("window opens");
    let tray = Tray::new().expect("tray icon is created");
    POPUP_NEXT.set(true);
    let panel = TrayPanel::new().expect("tray panel is created");
    POPUP_NEXT.set(false);
    restore_geometry(&ui, settings.window);
    let app = App::new(&ui, tray, panel, settings);
    instance.on_activate(|| drop(slint::invoke_from_event_loop(app::activate)));

    let weak = ui.as_weak();
    let close_app = app.clone();
    ui.window().on_close_requested(move || {
        if let Some(ui) = weak.upgrade() {
            let window = current_geometry(&ui, close_app.settings().window);
            close_app.set_settings(|s| s.window = window);
        }
        if close_app.settings().close_to_tray {
            close_app.hide();
        } else {
            close_app.quit();
        }
        CloseRequestResponse::HideWindow
    });

    if std::env::args().any(|a| a == win::autostart::TRAY_ARG) {
        app.hide();
    } else {
        app.show();
    }
    slint::run_event_loop_until_quit().expect("event loop runs");
    drop(app);
    drop(instance);
    // A staged update starts only once this copy no longer holds the lock.
    update::launch_pending();
}
