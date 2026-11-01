//! Sticks page: response curves, dead zones and trigger travel.

use eframe::egui::{self, Pos2, Stroke, Vec2};

use super::super::view::Mapping;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::profile::{stick, trigger};

fn pct(b: u8) -> u8 {
    (u32::from(b) * 100 / 255) as u8
}

fn byte(p: u8) -> u8 {
    (u32::from(p) * 255 / 100) as u8
}

pub fn editor(ui: &mut egui::Ui, m: &mut Mapping) {
    if m.model.features.sticks {
        ui.columns(2, |cols| {
            for (side, ui) in cols.iter_mut().enumerate() {
                stick_card(ui, m, side);
            }
        });
    }
    if m.model.features.triggers {
        ui_kit::card(ui, icon::GAUGE, t!("Trigger travel"), |ui| {
            ui_kit::key(
                ui,
                t!("The game gets a full press at the end of the chosen range and nothing before its start."),
            );
            ui_kit::grid(ui, "triggers", |ui| {
                for (side, name) in trigger::SIDES.iter().enumerate() {
                    let (old_start, old_end) = trigger::get(&m.map, side);
                    let (mut start, mut end) = (pct(old_start), pct(old_end));
                    ui.strong(*name);
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut start, 0..=end).text(t!("start, %")));
                        ui.add_space(12.0);
                        ui.add(egui::Slider::new(&mut end, start..=100).text(t!("full press, %")));
                    });
                    ui.end_row();
                    // Untouched values keep their exact bytes, the percent view rounds them.
                    if (start, end) != (pct(old_start), pct(old_end)) {
                        trigger::set(&mut m.map, side, byte(start), byte(end));
                    }
                }
            });
        });
    }
}

fn stick_card(ui: &mut egui::Ui, m: &mut Mapping, side: usize) {
    let old = stick::get(&m.map, side);
    let mut s = old;
    ui_kit::card(ui, icon::JOYSTICK, crate::i18n::tr(stick::SIDES[side]), |ui| {
        curve_plot(ui, &stick::bank(&m.map, side));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui_kit::key(ui, t!("Curve"));
            pick(ui, ("curve", side), &mut s.kind, &stick::PRESETS, 130.0);
        });
        // Preset points are applied on a change only, a stored curve stays byte for byte.
        if let Some((p1, p2)) = stick::preset(s.kind) {
            if s.kind != old.kind {
                (s.p1, s.p2) = (p1, p2);
            }
        } else {
            ui.add(egui::Slider::new(&mut s.p1.0, 0..=127).text(t!("point 1: travel")));
            ui.add(egui::Slider::new(&mut s.p1.1, 0..=127).text(t!("point 1: output")));
            ui.add(egui::Slider::new(&mut s.p2.0, 0..=127).text(t!("point 2: travel")));
            ui.add(egui::Slider::new(&mut s.p2.1, 0..=127).text(t!("point 2: output")));
        }
        ui.add(egui::Slider::new(&mut s.center, 0..=100 - s.edge).text(t!("center dead zone, %")));
        ui.add(egui::Slider::new(&mut s.edge, 0..=100 - s.center).text(t!("outer dead zone, %")));
        ui.checkbox(&mut s.circular, t!("Circular travel area"));
    });
    if s != old {
        stick::set(&mut m.map, side, s);
    }
}

/// Draws the 9-point table the pad plays: travel to the right, output up, the diagonal for reference.
fn curve_plot(ui: &mut egui::Ui, bank: &[u8; 9]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(180.0), egui::Sense::hover());
    let p = ui.painter();
    let weak = Stroke::new(1.0, ui.visuals().weak_text_color());
    p.rect_stroke(rect, 2.0, weak, egui::StrokeKind::Inside);
    let at =
        |x: f32, y: f32| Pos2::new(rect.left() + x * rect.width(), rect.bottom() - y.clamp(0.0, 1.0) * rect.height());
    p.line_segment([at(0.0, 0.0), at(1.0, 1.0)], weak);
    let pts: Vec<Pos2> =
        bank.iter().enumerate().map(|(i, v)| at(i as f32 / 8.0, (f32::from(*v) - 50.0) / 100.0)).collect();
    p.add(egui::Shape::line(pts, Stroke::new(2.0, ui.visuals().selection.bg_fill)));
}
