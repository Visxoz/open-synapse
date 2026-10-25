#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod artwork;
mod logger;
mod profiles;
mod settings;
mod storage;
mod win;
mod worker;

use slint::{CloseRequestResponse, ComponentHandle, PhysicalPosition, PhysicalSize};

use crate::{app::App, settings::WindowGeometry};

slint::include_modules!();

/// The window's background, matched by the title bar.
const BACKGROUND: slint::Color = slint::Color::from_rgb_u8(0x0b, 0x0b, 0x0e);

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
    if std::env::var_os("SLINT_BACKEND").is_none()
        && let Err(e) = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name(if gpu { "skia" } else { "skia-software" }.into())
            .select()
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

    let ui = AppWindow::new().expect("window opens");
    let tray = Tray::new().expect("tray icon is created");
    restore_geometry(&ui, settings.window);
    let app = App::new(&ui, tray, settings);
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
}
