//! Remapping page: what each button sends, turbo and macros.

use eframe::egui::{self, RichText};

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::profile::macros::{self, Macro, Step};
use crate::flydigi::profile::map;

/// Most steps a new macro or a "button press" adds: press, then release 50 ms later.
const TAP: [Step; 2] = [Step { delay_ms: 0, key: 4, event: 1 }, Step { delay_ms: 50, key: 4, event: 0 }];
/// Key a macro is bound to when every other key has one, M1 on a Vader 5 Pro.
const FALLBACK_KEY: u8 = 18;

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    if m.model.features.remap {
        ui_kit::card(ui, icon::SWAP, t!("Buttons"), |ui| {
            ui_kit::key(ui, t!("The button on the left presses what is chosen on the right."));
            keys_grid(ui, m);
            ui.add_space(4.0);
            if ui.button(ui_kit::label(icon::UNDO, t!("Default buttons"))).clicked() {
                for &(id, _) in m.model.keys {
                    map::set_key(&mut m.map, id, map::Key { target: map::IDENTITY, turbo: 0, hz: 0 });
                }
                m.macros.clear();
            }
        });
    }
    if m.model.features.macros {
        ui_kit::card(ui, icon::KEYBOARD, t!("Macros"), |ui| macros_editor(ui, m));
    }
}

fn keys_grid(ui: &mut egui::Ui, m: &mut Mapping) {
    egui::Grid::new("remap").num_columns(4).spacing([20.0, 8.0]).show(ui, |ui| {
        ui_kit::key(ui, t!("Button"));
        ui_kit::key(ui, t!("Presses"));
        ui_kit::key(ui, t!("Turbo"));
        ui_kit::key(ui, t!("Presses per second"));
        ui.end_row();
        for &(id, name) in m.model.keys {
            let mut k = map::key(&m.map, id);
            let changed = k != map::key(&m.device.map, id);
            ui.label(if changed { RichText::new(name).strong() } else { RichText::new(name) });
            if m.macros.iter().any(|mc| mc.key == id) {
                ui.label(t!("Macro"));
                ui.end_row();
                continue;
            }
            pick(ui, ("target", id), &mut k.target, m.model.targets, 140.0);
            pick(ui, ("turbo", id), &mut k.turbo, &map::TURBO, 170.0);
            if k.turbo > 0 {
                k.hz = k.hz.max(1);
                ui.add(egui::Slider::new(&mut k.hz, 1..=map::TURBO_MAX_HZ));
            }
            map::set_key(&mut m.map, id, k);
            ui.end_row();
        }
    });
}

fn macros_editor(ui: &mut egui::Ui, m: &mut Mapping) {
    // a step presses a real button, "unchanged" means nothing there
    let step_keys: Vec<(u8, &str)> = m.model.targets.iter().copied().filter(|t| t.0 != map::IDENTITY).collect();
    let mut remove = None;
    for (i, mc) in m.macros.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().text_color().gamma_multiply(0.04))
                .corner_radius(8)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if macro_header(ui, mc, m.model.keys) {
                        remove = Some(i);
                    }
                    ui.add_space(4.0);
                    macro_steps(ui, mc, &step_keys);
                });
        });
        ui.add_space(8.0);
    }
    if let Some(i) = remove {
        m.macros.remove(i);
    }
    let free = m.model.keys.iter().rev().map(|k| k.0).find(|k| m.macros.iter().all(|mc| mc.key != *k));
    if m.macros.len() < m.max_macros() && ui.button(ui_kit::label(icon::PLUS, t!("Add macro"))).clicked() {
        m.macros.push(Macro { key: free.unwrap_or(FALLBACK_KEY), kind: 1, interval_ms: 0, steps: TAP.to_vec() });
    }
}

/// Key, mode and repeat pause of a macro; returns whether it should be deleted.
fn macro_header(ui: &mut egui::Ui, mc: &mut Macro, keys: &[(u8, &str)]) -> bool {
    let mut delete = false;
    ui.horizontal_wrapped(|ui| {
        ui_kit::key(ui, t!("Button"));
        pick(ui, "key", &mut mc.key, keys, 80.0);
        ui.add_space(8.0);
        ui_kit::key(ui, t!("Mode"));
        pick(ui, "kind", &mut mc.kind, &macros::KINDS, 170.0);
        if mc.kind != 1 {
            ui.add_space(8.0);
            ui_kit::key(ui, t!("Pause between repeats, ms"));
            ui.add(egui::DragValue::new(&mut mc.interval_ms).range(0..=10_000));
        }
        ui.add_space(8.0);
        let text = RichText::new(ui_kit::label(icon::TRASH, t!("Delete macro"))).color(ui.visuals().error_fg_color);
        delete = ui.button(text).clicked();
    });
    delete
}

fn macro_steps(ui: &mut egui::Ui, mc: &mut Macro, step_keys: &[(u8, &str)]) {
    let mut drop = None;
    egui::Grid::new("steps").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
        for (j, st) in mc.steps.iter_mut().enumerate() {
            ui.push_id(j, |ui| {
                ui.add(egui::DragValue::new(&mut st.delay_ms).range(0..=10_000).suffix(t!(" ms")));
                let was_stick = macros::is_stick(st.event);
                pick(ui, "ev", &mut st.event, &macros::EVENTS, 120.0);
                if was_stick != macros::is_stick(st.event) {
                    st.key = if macros::is_stick(st.event) { macros::DIRECTIONS[0].0 } else { TAP[0].key };
                }
                if macros::is_stick(st.event) {
                    pick(ui, "k", &mut st.key, &macros::DIRECTIONS, 90.0);
                } else {
                    pick(ui, "k", &mut st.key, step_keys, 90.0);
                }
                if ui.small_button(icon::X).clicked() {
                    drop = Some(j);
                }
            });
            ui.end_row();
        }
    });
    if let Some(j) = drop {
        mc.steps.remove(j);
    }
    if ui.button(ui_kit::label(icon::PLUS, t!("Add a button press"))).clicked() {
        mc.steps.push(Step { delay_ms: 30, ..TAP[0].clone() });
        mc.steps.push(TAP[1].clone());
    }
}
