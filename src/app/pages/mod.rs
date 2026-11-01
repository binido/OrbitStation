//! Pages of the window. Each device page takes the device and its [`DeviceView`]; which pages a
//! device gets follows from its capabilities and its model's features.

mod device;
mod gyro;
mod lighting;
mod profile;
mod remap;
mod settings;
mod sticks;
mod test;
mod vibration;

use eframe::egui;

pub use profile::actions as profile_actions;
pub use settings::app_settings;

use super::view::DeviceView;
use super::widgets::icon;
use crate::station::{Device, Kind};

/// What to call a kind of device in a heading.
pub fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Gamepad => t!("Gamepad"),
        Kind::Dock => t!("Dock"),
    }
}

/// The sidebar icon of a kind of device.
pub fn kind_icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Gamepad => icon::GAMEPAD,
        Kind::Dock => icon::DOCK,
    }
}

/// A page in the sidebar.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Tab {
    Device,
    Test,
    Remap,
    Sticks,
    Gyro,
    Vibration,
    Lighting,
    Settings,
}

impl Tab {
    /// Pages about the device itself.
    pub const DEVICE: [Tab; 2] = [Tab::Device, Tab::Test];
    /// Pages that edit a profile slot.
    pub const PROFILE: [Tab; 5] = [Tab::Remap, Tab::Sticks, Tab::Gyro, Tab::Vibration, Tab::Lighting];

    pub fn icon(self) -> &'static str {
        match self {
            Tab::Device => icon::GAMEPAD,
            Tab::Test => icon::HAND_TAP,
            Tab::Remap => icon::SWAP,
            Tab::Sticks => icon::JOYSTICK,
            Tab::Gyro => icon::COMPASS,
            Tab::Vibration => icon::VIBRATE,
            Tab::Lighting => icon::LIGHTBULB,
            Tab::Settings => icon::GEAR,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tab::Device => t!("Device"),
            Tab::Test => t!("Button test"),
            Tab::Remap => t!("Remapping"),
            Tab::Sticks => t!("Sticks"),
            Tab::Gyro => t!("Gyro"),
            Tab::Vibration => t!("Vibration"),
            Tab::Lighting => t!("Lighting"),
            Tab::Settings => t!("Settings"),
        }
    }

    /// Whether the page edits the profile stored on the device.
    pub fn edits_profile(self) -> bool {
        Tab::PROFILE.contains(&self)
    }

    /// Whether `dev` has what the page shows.
    pub fn fits(self, dev: &mut dyn Device) -> bool {
        let f = dev.model().features;
        let slots = dev.profiles().is_some();
        match self {
            Tab::Device | Tab::Settings => true,
            Tab::Test => dev.live().is_some() && !dev.model().buttons.is_empty(),
            Tab::Remap => slots && (f.remap || f.macros),
            Tab::Sticks => slots && (f.sticks || f.triggers),
            Tab::Gyro => slots && f.gyro,
            Tab::Vibration => slots && f.vibration,
            Tab::Lighting => slots && f.lighting,
        }
    }

    /// Draws the page for `dev`. The settings page also has app settings, drawn by the caller.
    pub fn show(self, ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
        match self {
            Tab::Device => device::show(ui, dev, view),
            Tab::Test => test::show(ui, dev, view),
            Tab::Remap => profile::show(ui, dev, view, remap::editor),
            Tab::Sticks => profile::show(ui, dev, view, sticks::editor),
            Tab::Gyro => profile::show(ui, dev, view, gyro::editor),
            Tab::Vibration => profile::show(ui, dev, view, vibration::editor),
            Tab::Lighting => profile::show(ui, dev, view, lighting::editor),
            Tab::Settings => settings::device_settings(ui, dev, view),
        }
    }
}

/// `pick` lists `(value, name)` pairs in a combo box; names go through the translation table.
fn pick(ui: &mut egui::Ui, id: impl egui::AsIdSalt, value: &mut u8, list: &[(u8, &str)], width: f32) {
    let shown =
        list.iter().find(|t| t.0 == *value).map_or_else(|| format!("#{value}"), |t| crate::i18n::tr(t.1).into());
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(width).show_ui(ui, |ui| {
        for (v, n) in list {
            ui.selectable_value(value, *v, crate::i18n::tr(n));
        }
    });
}
