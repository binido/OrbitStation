//! The device page: what the device reports, its name, the active profile and the factory reset.

use eframe::egui::{self, RichText};

use super::super::storage;
use super::super::view::DeviceView;
use super::super::widgets::{self as ui_kit, icon};
use crate::station::{Device, Error};

pub fn show(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
    let Some(info) = view.info.clone() else {
        ui.label(t!("No data from the gamepad."));
        return;
    };
    ui_kit::card(ui, icon::INFO, t!("Gamepad"), |ui| {
        ui_kit::grid(ui, "info", |ui| {
            ui_kit::key(ui, t!("Model"));
            ui.strong(crate::i18n::tr(dev.model().name));
            ui.end_row();
            ui_kit::key(ui, t!("Gamepad name"));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut view.nickname).hint_text(t!("no name")).desired_width(200.0));
                if ui.button(ui_kit::label(icon::FLOPPY, t!("Save name"))).clicked() {
                    let r = dev.set_nickname(&view.nickname);
                    view.report(r, t!("gamepad name"));
                }
            });
            ui.end_row();
            ui_kit::key(ui, t!("Battery"));
            ui.label(if info.charging { t!("Charging").to_string() } else { format!("{} / 5", info.battery) });
            ui.end_row();
            ui_kit::key(ui, t!("Firmware"));
            ui.label(&info.firmware);
            ui.end_row();
            ui_kit::key(ui, t!("Radio module"));
            ui.label(&info.radio);
            ui.end_row();
            ui_kit::key(ui, t!("Device ID"));
            ui.label(info.device_id.to_string());
            ui.end_row();
            ui_kit::key(ui, t!("Device node"));
            ui.monospace(dev.path().display().to_string());
            ui.end_row();
        });
    });
    let slots = dev.model().profile_slots;
    if dev.profiles().is_none() {
        return;
    }
    ui_kit::card(ui, icon::SLIDERS, t!("Active profile"), |ui| {
        ui.horizontal(|ui| {
            for p in 0..slots {
                if ui.selectable_label(view.active == p, t!("Profile {}", p + 1)).clicked() && view.active != p {
                    let r = dev.profiles().map_or(Ok(()), |s| s.activate(p));
                    if r.is_ok() {
                        view.active = p;
                    }
                    view.report(r, &t!("profile {}", p + 1));
                }
            }
            ui.add_space(12.0);
            if ui.button(ui_kit::label(icon::REFRESH, t!("Refresh"))).clicked() {
                view.refresh(dev);
            }
        });
    });
    ui_kit::card(ui, icon::WARNING, t!("Factory reset of all profiles"), |ui| {
        ui.label(t!(
            "Like 'Restore default' in Space Station: all 4 profiles go back to factory settings. General settings stay as they are."
        ));
        ui.label(t!("Before the reset the app saves copies of all profiles, you can load them back from a file."));
        ui.checkbox(&mut view.confirm_reset_all, t!("I understand that all 4 profiles will be erased"));
        let text =
            RichText::new(ui_kit::label(icon::TRASH, t!("Reset all profiles"))).color(ui.visuals().error_fg_color);
        if ui.add_enabled(view.confirm_reset_all, egui::Button::new(text)).clicked() {
            view.confirm_reset_all = false;
            view.note = match reset_all(dev, view.active) {
                Ok(()) => t!("All profiles reset, copies are in {}", storage::backups_dir().display()),
                Err(e) => e,
            };
            view.mapping = None;
            view.refresh(dev);
        }
    });
}

/// Backs up every slot, then resets them all. Reading a slot switches to it, so the active one comes
/// back at the end either way.
fn reset_all(dev: &mut dyn Device, active: u8) -> Result<(), String> {
    let model = dev.model();
    let Some(store) = dev.profiles() else { return Ok(()) };
    let result = (|| {
        for slot in 0..model.profile_slots {
            let p = store.read(slot).map_err(|e| {
                t!("Could not back up profile {}, the reset is cancelled", slot + 1) + ": " + &e.to_string()
            })?;
            storage::backup_profile(model, slot, &p).map_err(|e| {
                t!("Could not back up profile {}, the reset is cancelled", slot + 1) + ": " + &e.to_string()
            })?;
        }
        store.reset_all().map_err(|e: Error| e.to_string())
    })();
    // the reset already happened or was refused; a failed switch back shows on the next refresh
    let _ = store.activate(active);
    result
}
