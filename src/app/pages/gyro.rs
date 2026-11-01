//! Gyro page: tilt drives a stick while an enable key turns it on.

use eframe::egui;

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::profile::gyro;

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    let old = gyro::get(&m.map);
    let mut g = old;
    let keys: Vec<(u8, &str)> = [(gyro::NO_KEY, t!("None"))].into_iter().chain(m.model.keys.iter().copied()).collect();
    ui_kit::card(ui, icon::TARGET, t!("Aiming"), |ui| {
        ui_kit::key(
            ui,
            t!("Tilting the gamepad moves the chosen stick. It works in any game with no software on the computer,"),
        );
        ui_kit::key(ui, t!("but only while the gyro is turned on with a button."));
        ui.add_space(6.0);
        ui_kit::grid(ui, "gyro", |ui| {
            ui_kit::key(ui, t!("Gyro controls"));
            pick(ui, "target", &mut g.target, &gyro::TARGETS, 220.0);
            ui.end_row();
            ui_kit::key(ui, t!("Activation button"));
            pick(ui, "key", &mut g.key, &keys, 220.0);
            ui.end_row();
            ui_kit::key(ui, t!("Second button"));
            pick(ui, "key2", &mut g.key2, &keys, 220.0);
            ui.end_row();
            ui_kit::key(ui, t!("Activation mode"));
            pick(ui, "enable", &mut g.enable, &gyro::ENABLE, 220.0);
            ui.end_row();
            ui_kit::key(ui, t!("Sensitivity"));
            ui.add(egui::Slider::new(&mut g.sensitivity, 0..=100));
            ui.end_row();
            ui_kit::key(ui, t!("Dead zone"));
            ui.add(egui::Slider::new(&mut g.dead_zone, 0..=100));
            ui.end_row();
        });
        if g.target != 0 && g.key == gyro::NO_KEY && g.key2 == gyro::NO_KEY {
            let warning = ui_kit::label(icon::WARNING, t!("Without an activation button the gyro will not work."));
            ui.colored_label(ui.visuals().warn_fg_color, warning);
        }
    });
    if g != old {
        gyro::set(&mut m.map, g);
    }
}
