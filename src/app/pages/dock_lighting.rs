//! The dock's lighting page: effect, colours, direction, speed and brightness, a live preview of the
//! wedge, and the write, which runs a little per window frame.

use eframe::egui::{self, Color32, Pos2, RichText, Vec2};

use super::super::view::DeviceView;
use super::super::widgets::{self as ui_kit, icon};
use super::pick;
use crate::flydigi::dock_led::{self, LedConfig, mode};
use crate::station::Device;

/// Width the 450x420 preview is drawn at.
const PREVIEW_WIDTH: f32 = 360.0;
const LED_RADIUS: f32 = 7.0;
const DEFAULT_BRIGHTNESS: u8 = 50;

pub fn show(ui: &mut egui::Ui, dev: &mut dyn Device, view: &mut DeviceView) {
    let Some(lighting) = dev.dock_lighting() else { return };
    if view.dock_led.is_none() {
        view.dock_led = Some(match lighting.read_lighting() {
            Ok(c) => c,
            Err(e) => {
                view.note = e.to_string();
                LedConfig::defaults(mode::PULSE, DEFAULT_BRIGHTNESS)
            }
        });
    }
    let Some(c) = view.dock_led.as_mut() else { return };
    ui_kit::card(ui, icon::PALETTE, t!("Effect"), |ui| {
        let computable = dock_led::EFFECTS.iter().any(|e| e.0 == c.mode);
        if !computable {
            ui_kit::key(ui, t!("The dock plays a picture or Flydigi's own animation; pick an effect to replace it."));
        }
        ui_kit::grid(ui, "dock-effect", |ui| {
            ui_kit::key(ui, t!("Effect"));
            let mut effect = c.mode;
            pick(ui, "dock-effect", &mut effect, &dock_led::EFFECTS, 200.0);
            if effect != c.mode {
                *c = LedConfig::defaults(effect, c.brightness.max(1));
            }
            ui.end_row();
            let wanted = dock_led::colors_for(c.mode);
            if wanted > 0 {
                c.colors.resize(wanted, [255, 255, 255]);
                ui_kit::key(ui, t!("Colors"));
                ui.horizontal(|ui| {
                    for color in &mut c.colors {
                        ui.color_edit_button_srgb(color);
                    }
                });
                ui.end_row();
            }
            if dock_led::has_direction(c.mode) {
                ui_kit::key(ui, t!("Direction"));
                pick(ui, "dock-direction", &mut c.direction, &dock_led::DIRECTIONS, 200.0);
                ui.end_row();
            }
            let (lo, hi) = dock_led::period_range(c.mode);
            if lo < hi {
                ui_kit::key(ui, t!("Speed"));
                c.period = c.period.clamp(lo, hi);
                ui.add(egui::Slider::new(&mut c.period, lo..=hi).text(t!("cycle time (higher = slower)")));
                ui.end_row();
            }
            ui_kit::key(ui, t!("Brightness"));
            ui.add(egui::Slider::new(&mut c.brightness, 1..=100).text("%"));
            ui.end_row();
        });
        ui.add_space(6.0);
        match view.upload {
            Some(done) => {
                ui.add(egui::ProgressBar::new(done).text(t!("Writing the lighting…")).desired_width(320.0));
            }
            None => {
                let sel = ui.visuals().selection;
                let text = RichText::new(ui_kit::label(icon::FLOPPY, t!("Write to the dock"))).color(sel.stroke.color);
                if ui.add_enabled(computable, egui::Button::new(text).fill(sel.bg_fill)).clicked() {
                    view.upload = match lighting.start_lighting(c) {
                        Ok(()) => Some(0.0),
                        Err(e) => {
                            view.note = e.to_string();
                            None
                        }
                    };
                }
            }
        }
    });
    ui_kit::card(ui, icon::SPARKLE, t!("Animation preview"), |ui| preview(ui, c));
}

/// Plays the frames on the wedge at the speed the dock will.
fn preview(ui: &mut egui::Ui, c: &LedConfig) {
    let frames = c.frames();
    let frame_ms = f64::from(u32::from(c.period.max(1)) * dock_led::PERIOD_MS);
    let index = (ui.input(|i| i.time) * 1000.0 / frame_ms) as usize % frames.len().max(1);
    let scale = PREVIEW_WIDTH / 450.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(PREVIEW_WIDTH, 420.0 * scale), egui::Sense::hover());
    let dim = f32::from(c.brightness) / 100.0;
    for (led, (x, y)) in dock_led::preview_positions().into_iter().enumerate() {
        let [r, g, b] = frames.get(index).map_or([0; 3], |f| f[led]);
        let lit = |v: u8| (f32::from(v) * dim) as u8;
        let at = rect.min + Vec2::new(x, y) * scale;
        ui.painter().circle_filled(
            Pos2::new(at.x, at.y),
            LED_RADIUS * scale,
            Color32::from_rgb(lit(r), lit(g), lit(b)),
        );
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(u64::from(dock_led::PERIOD_MS)));
}
