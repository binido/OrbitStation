//! Vibration page: grip and trigger motors of the profile.

use eframe::egui;

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use crate::flydigi::profile::{trigger, vibration};

fn pct(b: u8) -> u8 {
    (u32::from(b) * 100 / 255) as u8
}

fn byte(p: u8) -> u8 {
    (u32::from(p) * 255 / 100) as u8
}

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    let old = vibration::get(&m.map);
    let mut v = old;
    ui_kit::card(ui, icon::VIBRATE, t!("Grips"), |ui| {
        ui.checkbox(&mut v.grip_on, t!("Grip vibration"));
        let strength = egui::Slider::new(&mut v.grip_strength, 1..=100).text(t!("strength, % (50 = like Xbox)"));
        ui.add_enabled(v.grip_on, strength);
    });
    let trigger_motors = m.model.features.trigger_motors;
    ui_kit::card_if(trigger_motors, ui, icon::WAVEFORM, t!("Triggers"), |ui| {
        ui.checkbox(&mut v.trig_on, t!("Trigger vibration"));
        ui.add_enabled_ui(v.trig_on, |ui| {
            ui.add(egui::Slider::new(&mut v.trig_strength, 1..=100).text(t!("strength, %")));
            ui.add(egui::Slider::new(&mut v.trig_threshold, 0..=100).text(t!("threshold: weaker signals stay silent")));
            ui_kit::grid(ui, "windows", |ui| {
                for (side, name) in trigger::SIDES.iter().enumerate() {
                    let (lo, hi) = v.trig_window[side];
                    let (mut a, mut b) = (pct(lo), pct(hi));
                    ui.strong(*name);
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut a, 0..=b).text(t!("min. amplitude, %")));
                        ui.add_space(12.0);
                        ui.add(egui::Slider::new(&mut b, a..=100).text(t!("max. amplitude, %")));
                    });
                    ui.end_row();
                    // Untouched values keep their exact bytes, the percent view rounds them.
                    if (a, b) != (pct(lo), pct(hi)) {
                        v.trig_window[side] = (byte(a), byte(b));
                    }
                }
            });
        });
    });
    if v != old {
        vibration::set(&mut m.map, v);
    }
    let test = ui.button(ui_kit::label(icon::VIBRATE, t!("Test vibration (1 second)")));
    if test.on_hover_text(t!("Strength and ranges apply after writing to the gamepad")).clicked() {
        m.test_vibration = true;
    }
}
