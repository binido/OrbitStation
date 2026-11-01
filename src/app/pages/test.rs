//! The test page: live sticks, triggers, buttons and motion sensors, and the motors.

use std::time::Duration;

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};

use super::super::view::DeviceView;
use super::super::widgets::{self as ui_kit, icon};
use crate::station::Device;

/// Bits of the stick clicks in the input report, they light the stick dot.
const L3_BIT: u32 = 1 << 14;
const R3_BIT: u32 = 1 << 15;

pub fn show(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
    let buttons = dev.model().buttons;
    let Some(live) = dev.live() else { return };
    let input = live.input().unwrap_or_default();
    ui.ctx().request_repaint_after(Duration::from_millis(16));
    ui_kit::card(ui, icon::JOYSTICK, t!("Sticks and triggers"), |ui| {
        ui.horizontal(|ui| {
            stick(ui, t!("Left stick"), input.lx, input.ly, input.buttons & L3_BIT != 0);
            ui.add_space(16.0);
            stick(ui, t!("Right stick"), input.rx, input.ry, input.buttons & R3_BIT != 0);
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui_kit::key(ui, t!("Triggers"));
                for (name, v) in [("LT", input.lt), ("RT", input.rt)] {
                    ui.horizontal(|ui| {
                        ui.monospace(name);
                        ui.add(egui::ProgressBar::new(f32::from(v) / 255.0).desired_width(180.0));
                        ui.monospace(format!("{v:>3}"));
                    });
                }
            });
        });
    });
    ui_kit::card(ui, icon::CURSOR, t!("Buttons"), |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, name) in buttons.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
                let on = input.buttons & 1 << i != 0;
                let text = RichText::new(*name).monospace();
                let sel = ui.visuals().selection;
                let fill = if on { sel.bg_fill } else { ui.visuals().widgets.inactive.bg_fill };
                ui.add(
                    egui::Button::new(if on { text.color(sel.stroke.color) } else { text })
                        .fill(fill)
                        .min_size(Vec2::new(52.0, 32.0)),
                );
            }
        });
    });
    ui_kit::card(ui, icon::COMPASS, t!("Motion sensors"), |ui| {
        let row = |name: &str, v: [i16; 3]| format!("{name:<14}{:>7} {:>7} {:>7}", v[0], v[1], v[2]);
        ui.monospace(row(t!("Gyro"), input.gyro));
        ui.monospace(row(t!("Accelerometer"), input.accel));
    });
    ui_kit::card(ui, icon::VIBRATE, t!("Vibration"), |ui| {
        let old = view.rumble;
        for (v, name) in view.rumble.iter_mut().zip([t!("left grip"), t!("right grip"), t!("LT motor"), t!("RT motor")])
        {
            ui.add(egui::Slider::new(v, 0..=255).text(name));
        }
        if old != view.rumble {
            view.rumble_stop = None;
            if let Err(e) = live.rumble(view.rumble) {
                view.note = e.to_string();
            }
        }
    });
}

fn stick(ui: &mut egui::Ui, title: &str, x: i16, y: i16, pressed: bool) {
    ui.vertical(|ui| {
        ui_kit::key(ui, title);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), egui::Sense::hover());
        let p = ui.painter();
        let r = rect.width() / 2.0 - 4.0;
        p.circle_stroke(rect.center(), r, Stroke::new(1.5, ui.visuals().weak_text_color()));
        let dot = rect.center() + Vec2::new(f32::from(x), -f32::from(y)) / 32768.0 * r;
        let color: Color32 = if pressed { ui.visuals().selection.bg_fill } else { ui.visuals().strong_text_color() };
        p.circle_filled(Pos2::new(dot.x, dot.y), 7.0, color);
        ui.monospace(format!("{x:>6} {y:>6}"));
    });
}
