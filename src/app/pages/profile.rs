//! What every profile page shares: loading the slot, its name and files, and the write bar.

use std::time::{Duration, Instant};

use eframe::egui::{self, RichText};

use super::super::storage;
use super::super::view::{DeviceView, Mapping};
use super::super::widgets::{self as ui_kit, icon};
use crate::flydigi::profile::{Profile, map};
use crate::station::Device;

/// Motor strength of the one-second vibration test.
const TEST_RUMBLE: [u8; 4] = [160; 4];
const TEST_RUMBLE_FOR: Duration = Duration::from_secs(1);

/// Draws `editor` for the active slot, or the button that loads it.
pub fn show(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView, editor: fn(&mut egui::Ui, &mut Mapping)) {
    if !view.has_mapping() {
        ui_kit::card(ui, icon::DOWNLOAD, &t!("Profile {}", view.active + 1), |ui| {
            ui.label(t!("Profile {} is not loaded yet.", view.active + 1));
            if ui.button(ui_kit::label(icon::DOWNLOAD, t!("Load from gamepad"))).clicked() {
                view.load_mapping(dev);
            }
        });
        return;
    }
    files(ui, view);
    let active = view.active;
    let Some(m) = view.mapping.as_mut().filter(|m| m.slot == active) else {
        return;
    };
    editor(ui, m);
    if std::mem::take(&mut m.test_vibration)
        && let Some(live) = dev.live()
    {
        match live.rumble(TEST_RUMBLE) {
            Ok(()) => view.rumble_stop = Some(Instant::now() + TEST_RUMBLE_FOR),
            Err(e) => view.note = e.to_string(),
        }
    }
    if std::mem::take(&mut m.try_triggers)
        && let Some(triggers) = dev.force_triggers()
    {
        view.note = match triggers.engage(&m.map) {
            Ok(()) => t!("The effects are on the triggers until the profile is applied again").into(),
            Err(e) => e.to_string(),
        };
    }
}

/// Write, discard and reload, in a bar that stays in view however long the page is.
pub fn actions(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
    let Some(m) = view.mapping_mut() else { return };
    let built = m.build();
    let dirty = built.as_ref().is_ok_and(|p| *p != m.device);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let sel = ui.visuals().selection;
        let text = RichText::new(ui_kit::label(icon::FLOPPY, t!("Write to gamepad"))).color(sel.stroke.color);
        if ui.add_enabled(dirty, egui::Button::new(text).fill(sel.bg_fill)).clicked() {
            view.save_mapping(dev);
        }
        let discard = egui::Button::new(ui_kit::label(icon::UNDO, t!("Discard changes")));
        if ui.add_enabled(dirty, discard).clicked()
            && let Some(m) = view.mapping_mut()
        {
            *m = Mapping::new(m.model, m.slot, m.device.clone());
        }
        if ui.button(ui_kit::label(icon::REFRESH, t!("Reload"))).clicked() {
            view.load_mapping(dev);
        }
        match &built {
            Err(e) => {
                ui.colored_label(ui.visuals().warn_fg_color, ui_kit::label(icon::WARNING, &e.to_string()));
            }
            Ok(_) if dirty => {
                ui.colored_label(ui.visuals().warn_fg_color, t!("Unsaved changes"));
            }
            Ok(_) => {}
        }
    });
    ui.add_space(4.0);
}

/// Profile name, files and the factory template.
fn files(ui: &mut egui::Ui, view: &mut DeviceView) {
    let active = view.active;
    let Some(m) = view.mapping.as_mut().filter(|m| m.slot == active) else {
        return;
    };
    let slot = m.slot;
    let mut note = None;
    ui_kit::card(ui, icon::SLIDERS, &t!("Profile {}", slot + 1), |ui| {
        ui_kit::grid(ui, "profile", |ui| {
            ui_kit::key(ui, t!("Name"));
            ui.horizontal(|ui| {
                let mut title = map::title(&m.map);
                let edit = egui::TextEdit::singleline(&mut title).char_limit(map::TITLE_CHARS).desired_width(160.0);
                if ui.add(edit).on_hover_text(t!("Profile name, up to 10 characters")).changed() {
                    map::set_title(&mut m.map, &title);
                }
                if let Some(factory) = Profile::factory(m.model, slot) {
                    let button = ui.button(ui_kit::label(icon::FACTORY, t!("Factory profile")));
                    if button.on_hover_text(t!("Only this profile, the others stay untouched")).clicked() {
                        note = Some(match m.replace(&factory) {
                            Ok(()) => t!("Factory values filled in, press 'Write to gamepad'").to_string(),
                            Err(e) => e.to_string(),
                        });
                    }
                }
            });
            ui.end_row();
            ui_kit::key(ui, t!("Files"));
            ui.horizontal_wrapped(|ui| {
                if ui.button(ui_kit::label(icon::FILE_SAVE, t!("Save to file"))).clicked() {
                    note = Some(match m.build().map(|p| storage::export_profile(slot, &p)) {
                        Ok(Ok(path)) => t!("Saved to {}", path.display()),
                        Ok(Err(e)) => t!("Could not save: {}", e),
                        Err(e) => t!("Could not save: {}", e),
                    });
                }
                let label =
                    view.import_pick.as_deref().map_or_else(|| t!("choose a file").to_string(), storage::file_label);
                egui::ComboBox::from_id_salt("import").selected_text(label).width(240.0).show_ui(ui, |ui| {
                    for f in storage::profile_files() {
                        let text = storage::file_label(&f);
                        ui.selectable_value(&mut view.import_pick, Some(f), text);
                    }
                });
                let load = egui::Button::new(ui_kit::label(icon::FILE_LOAD, t!("Load from file")));
                if let Some(path) = view.import_pick.as_deref() {
                    if ui.add(load).clicked() {
                        let loaded = std::fs::read(path)
                            .map_err(|_| crate::flydigi::profile::ProfileError::BadFile)
                            .and_then(|d| Profile::from_file(&d));
                        note = Some(match loaded.and_then(|p| m.replace(&p)) {
                            Ok(()) => t!("Loaded from {}, press 'Write to gamepad'", storage::file_label(path)),
                            Err(e) => e.to_string(),
                        });
                    }
                } else {
                    ui.add_enabled(false, load);
                }
                if ui.button(ui_kit::label(icon::FOLDER_OPEN, t!("Open folder"))).clicked() {
                    storage::open_data_dir();
                }
            });
            ui.end_row();
        });
    });
    if let Some(note) = note {
        view.note = note;
    }
}
