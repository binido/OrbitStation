//! OrbitStation: settings for Flydigi controllers on Linux, a native take on Flydigi Space Station.
//!
//! - [`station`] finds devices and talks to them through the [`station::Device`] trait.
//! - [`flydigi`] implements that trait for the Space Station protocol family.
//! - [`models`] describes each model as data.
//! - [`app`] is the window.

#[macro_use]
mod i18n;
mod app;
mod flydigi;
mod models;
mod station;

use eframe::egui;

fn main() -> eframe::Result {
    i18n::set(i18n::load());
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/orbitstation-128.png")).unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("OrbitStation")
            // the app draws its own title bar, see app::widgets::title_bar
            .with_decorations(false)
            // Matches orbitstation.desktop, so docks and task bars pick up the installed icon.
            .with_app_id("orbitstation")
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("OrbitStation", options, Box::new(|cc| Ok(Box::new(app::App::new(&cc.egui_ctx)))))
}
