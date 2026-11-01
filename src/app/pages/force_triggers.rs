//! Force triggers page: the effect each trigger pushes back with, stored in the profile.

use eframe::egui;

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::profile::force_trigger::{self, mode};
use crate::i18n;

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    ui.columns(2, |cols| {
        for (side, ui) in cols.iter_mut().enumerate() {
            trigger_card(ui, m, side);
        }
    });
    let try_it = ui.button(ui_kit::label(icon::TARGET, t!("Try on the controller")));
    if try_it.on_hover_text(t!("Starts the effects without writing them; writing the profile keeps them")).clicked() {
        m.try_triggers = true;
    }
}

fn trigger_card(ui: &mut egui::Ui, m: &mut Mapping, side: usize) {
    let (old_effect, old_values) = force_trigger::get(&m.map, side);
    let (mut effect, mut values) = (old_effect, old_values.clone());
    ui_kit::card(ui, icon::GAUGE, force_trigger::SIDES[side], |ui| {
        ui.horizontal(|ui| {
            ui_kit::key(ui, t!("Effect"));
            pick(ui, ("force", side), &mut effect, &force_trigger::EFFECTS, 220.0);
        });
        if effect != old_effect {
            values = force_trigger::knobs(effect).iter().map(|k| k.default).collect();
        }
        if effect == mode::LOCK {
            ui_kit::key(ui, t!("Past this point the trigger reads fully pressed at once."));
        }
        for (k, v) in force_trigger::knobs(effect).iter().zip(&mut values) {
            if k.switch {
                let mut on = *v == 1;
                if ui.checkbox(&mut on, i18n::tr(k.name)).changed() {
                    *v = u8::from(on);
                }
            } else {
                ui.add(egui::Slider::new(v, k.min..=k.max).text(i18n::tr(k.name)));
            }
        }
    });
    if (effect, &values) != (old_effect, &old_values) {
        force_trigger::set(&mut m.map, side, effect, &values);
    }
}
