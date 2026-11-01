//! Settings page: the app's own settings, then those the selected device keeps.

use eframe::egui;

use super::super::view::DeviceView;
use super::super::widgets::{self as ui_kit, icon};
use crate::i18n;
use crate::station::{Choice, Device};

/// Interface scale steps, on top of the system scale.
const ZOOMS: [f32; 8] = [0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// Language and scale. Returns why the language could not be saved, if it could not.
pub fn app_settings(ui: &mut egui::Ui) -> Option<String> {
    let mut error = None;
    ui_kit::card(ui, icon::TRANSLATE, t!("Application"), |ui| {
        ui_kit::grid(ui, "app", |ui| {
            ui_kit::key(ui, t!("Language"));
            let mut lang = i18n::current();
            egui::ComboBox::from_id_salt("language").selected_text(i18n::name(lang)).width(180.0).show_ui(ui, |ui| {
                for (l, _, n) in i18n::LANGS {
                    ui.selectable_value(&mut lang, l, n);
                }
            });
            if lang != i18n::current() {
                i18n::set(lang);
                error = i18n::save(lang).err().map(|e| t!("Could not save: {}", e));
            }
            ui.end_row();
            ui_kit::key(ui, t!("Interface scale"));
            let mut zoom = ui.ctx().zoom_factor();
            egui::ComboBox::from_id_salt("zoom")
                .selected_text(format!("{:.0}%", zoom * 100.0))
                .width(180.0)
                .show_ui(ui, |ui| {
                    for z in ZOOMS {
                        ui.selectable_value(&mut zoom, z, format!("{:.0}%", z * 100.0));
                    }
                })
                .response
                .on_hover_text(t!("100% is the system scale. Ctrl + and Ctrl - work too, Ctrl 0 goes back to 100%."));
            if zoom != ui.ctx().zoom_factor() {
                ui.ctx().set_zoom_factor(zoom);
            }
            ui.end_row();
        });
    });
    error
}

/// The settings the device keeps, changed on the device as they are clicked.
pub fn device_settings(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
    let Some(mut st) = view.settings.clone() else {
        ui.label(t!("No settings data."));
        return;
    };
    let Some(hw) = dev.settings() else { return };
    if !st.toggles.is_empty() {
        ui_kit::card(ui, icon::GEAR, t!("General"), |ui| {
            for (t, on) in &mut st.toggles {
                if ui.checkbox(on, t.label()).changed() {
                    let r = hw.set_toggle(*t, *on);
                    view.report(r, t.label());
                }
            }
        });
    }
    let mut choice_card = |ui: &mut egui::Ui, icon: &str, title: &str, id: &str, which: &[Choice]| {
        if !st.choices.iter().any(|c| which.contains(&c.0)) {
            return;
        }
        ui_kit::card(ui, icon, title, |ui| {
            ui_kit::grid(ui, id, |ui| {
                for (c, value) in st.choices.iter_mut().filter(|c| which.contains(&c.0)) {
                    ui_kit::key(ui, c.label());
                    ui.horizontal_wrapped(|ui| {
                        for &(v, name) in c.options() {
                            if ui.radio_value(value, v, i18n::tr(name)).clicked() {
                                let r = hw.set_choice(*c, v);
                                view.report(r, c.label());
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });
    };
    let sticks = [Choice::ReportRate, Choice::Precision, Choice::Sensitivity];
    choice_card(ui, icon::JOYSTICK, t!("Sticks"), "sticks", &sticks);
    choice_card(ui, icon::BATTERY, t!("Power"), "power", &[Choice::SleepTime]);
    view.settings = Some(st);
}
