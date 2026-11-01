//! Lighting page: the effect, its colours and the frames the device plays.

use eframe::egui::{self, Color32, Vec2};

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::profile::rgb;

/// Effects without colours of their own to pick: lights off and rainbow.
const NO_COLORS: [u8; 2] = [0, 6];
const NEW_COLOR: rgb::Rgb = [255, 255, 255];
const CELL: f32 = 18.0;

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    ui_kit::card(ui, icon::PALETTE, t!("Effect"), |ui| {
        let mut effect = m.rgb[rgb::MODE];
        let old_colors = m.colors.clone();
        ui_kit::grid(ui, "effect", |ui| {
            ui_kit::key(ui, t!("Effect"));
            pick(ui, "effect", &mut effect, &rgb::EFFECTS, 200.0);
            ui.end_row();
            if !NO_COLORS.contains(&effect) {
                ui_kit::key(ui, t!("Colors"));
                ui.horizontal(|ui| colors(ui, &mut m.colors));
                ui.end_row();
            }
        });
        if effect != m.rgb[rgb::MODE] || m.colors != old_colors {
            rgb::apply(&mut m.rgb, effect, &m.colors);
        }
        ui.add(egui::Slider::new(&mut m.rgb[rgb::BRIGHTNESS], 0..=100).text(t!("brightness, %")));
        ui.add(egui::Slider::new(&mut m.rgb[rgb::PERIOD], 1..=30).text(t!("cycle time (higher = slower)")));
        let mut sync = m.rgb[rgb::GRIP_SYNC] != 0;
        if ui.checkbox(&mut sync, t!("Lighting reacts to vibration")).changed() {
            m.rgb[rgb::GRIP_SYNC] = u8::from(sync);
        }
    });
    ui_kit::card(ui, icon::SPARKLE, t!("Animation preview"), |ui| {
        ui_kit::key(ui, t!("Animation frames the gamepad plays (top to bottom):"));
        let (leds, frames) = (rgb::leds(&m.rgb), rgb::frames(&m.rgb));
        let size = Vec2::new(CELL * leds as f32, CELL * frames as f32);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        for f in 0..frames {
            for l in 0..leds {
                let [r, g, b] = rgb::led(&m.rgb, f, l);
                let min = rect.min + Vec2::new(l as f32 * CELL, f as f32 * CELL);
                let cell = egui::Rect::from_min_size(min, Vec2::splat(CELL - 3.0));
                ui.painter().rect_filled(cell, 4.0, Color32::from_rgb(r, g, b));
            }
        }
    });
}

fn colors(ui: &mut egui::Ui, list: &mut Vec<rgb::Rgb>) {
    let (mut drop, many) = (None, list.len() > 1);
    for (i, c) in list.iter_mut().enumerate() {
        ui.color_edit_button_srgb(c);
        if many && ui.small_button(icon::X).clicked() {
            drop = Some(i);
        }
    }
    if let Some(i) = drop {
        list.remove(i);
    }
    if list.len() < rgb::MAX_COLORS && ui.button(ui_kit::label(icon::PLUS, t!("Add color"))).clicked() {
        list.push(NEW_COLOR);
    }
}
