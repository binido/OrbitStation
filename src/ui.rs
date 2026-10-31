//! Building blocks of the window: Phosphor icons, page headers, cards and sidebar entries.

use eframe::egui::{self, RichText, Vec2};

/// Phosphor Regular codepoints, the glyphs kept in assets/fonts/Phosphor-Regular-subset.ttf.
pub mod icon {
    pub const GAMEPAD: &str = "\u{e26e}";
    pub const HAND_TAP: &str = "\u{ec90}";
    pub const SWAP: &str = "\u{e83c}";
    pub const JOYSTICK: &str = "\u{ea5e}";
    pub const COMPASS: &str = "\u{e1c8}";
    pub const VIBRATE: &str = "\u{e4d8}";
    pub const LIGHTBULB: &str = "\u{e2dc}";
    pub const GEAR: &str = "\u{e272}";
    pub const FLOPPY: &str = "\u{e248}";
    pub const REFRESH: &str = "\u{e094}";
    pub const FOLDER_OPEN: &str = "\u{e256}";
    pub const FILE_SAVE: &str = "\u{e232}";
    pub const FILE_LOAD: &str = "\u{e61e}";
    pub const FACTORY: &str = "\u{e760}";
    pub const WARNING: &str = "\u{e4e0}";
    pub const PLUS: &str = "\u{e3d4}";
    pub const TRASH: &str = "\u{e4a6}";
    pub const X: &str = "\u{e4f6}";
    pub const INFO: &str = "\u{e2ce}";
    pub const DOWNLOAD: &str = "\u{e20c}";
    pub const UNDO: &str = "\u{e038}";
    pub const TRANSLATE: &str = "\u{e4a2}";
    pub const SLIDERS: &str = "\u{e434}";
    pub const GAUGE: &str = "\u{e628}";
    pub const WAVEFORM: &str = "\u{e802}";
    pub const KEYBOARD: &str = "\u{e2d8}";
    pub const SPARKLE: &str = "\u{e6a2}";
    pub const PALETTE: &str = "\u{e6c8}";
    pub const TARGET: &str = "\u{e47c}";
    pub const PLUGS: &str = "\u{eb5a}";
    pub const CURSOR: &str = "\u{e7c8}";
    pub const BATTERY: &str = "\u{e0c2}";
}

pub const FONT: &[u8] = include_bytes!("../assets/fonts/Phosphor-Regular-subset.ttf");

/// `icon  text`, for buttons.
pub fn label(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

/// Big title on top of a page.
pub fn page_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(label(icon, title)).size(24.0).strong());
    ui.add_space(12.0);
}

/// A block of related settings on its own background, the way GNOME groups preferences.
pub fn card<R>(ui: &mut egui::Ui, icon: &str, title: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let r = egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, ui.visuals().text_color().gamma_multiply(0.07)))
        .corner_radius(12)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label(icon, title)).size(17.0).strong());
            ui.add_space(10.0);
            body(ui)
        })
        .inner;
    ui.add_space(14.0);
    r
}

/// Label in the left column of a card's grid, dimmed like GTK's row subtitles.
pub fn key(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(ui.visuals().weak_text_color()));
}

/// Two-column grid inside a card.
pub fn grid<R>(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Grid::new(id).num_columns(2).spacing([28.0, 12.0]).show(ui, body).inner
}

/// Small grey heading over a group of sidebar entries.
pub fn nav_section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(title.to_uppercase()).size(11.5).color(ui.visuals().weak_text_color()));
    ui.add_space(2.0);
}

/// A full-width sidebar entry with its icon.
pub fn nav_item(ui: &mut egui::Ui, selected: bool, icon: &str, text: &str) -> bool {
    let atoms = (RichText::new(icon).size(18.0), text, egui::Atom::grow());
    ui.add(egui::Button::selectable(selected, atoms).min_size(Vec2::new(ui.available_width(), 36.0)))
        .clicked()
}
