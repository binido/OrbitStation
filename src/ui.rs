//! Building blocks of the window: Phosphor icons, page headers, cards and sidebar entries.

use eframe::egui::{self, Align, CursorIcon, Layout, PointerButton, ResizeDirection, RichText, Vec2, ViewportCommand};

/// Phosphor Regular codepoints, the glyphs kept in assets/fonts/Phosphor-Regular-subset.ttf.
pub mod icon {
    pub const GAMEPAD: &str = "\u{e26e}";
    pub const HAND_TAP: &str = "\u{ec90}";
    pub const SWAP: &str = "\u{e83c}";
    pub const JOYSTICK: &str = "\u{ea5e}";
    pub const COMPASS: &str = "\u{e1c8}";
    pub const VIBRATE: &str = "\u{e4d8}";
    pub const LIGHTBULB: &str = "\u{e2dc}";
    pub const GEAR: &str = "\u{e272}";
    pub const FLOPPY: &str = "\u{e248}";
    pub const REFRESH: &str = "\u{e094}";
    pub const FOLDER_OPEN: &str = "\u{e256}";
    pub const FILE_SAVE: &str = "\u{e232}";
    pub const FILE_LOAD: &str = "\u{e61e}";
    pub const FACTORY: &str = "\u{e760}";
    pub const WARNING: &str = "\u{e4e0}";
    pub const PLUS: &str = "\u{e3d4}";
    pub const TRASH: &str = "\u{e4a6}";
    pub const X: &str = "\u{e4f6}";
    pub const INFO: &str = "\u{e2ce}";
    pub const DOWNLOAD: &str = "\u{e20c}";
    pub const UNDO: &str = "\u{e038}";
    pub const TRANSLATE: &str = "\u{e4a2}";
    pub const SLIDERS: &str = "\u{e434}";
    pub const GAUGE: &str = "\u{e628}";
    pub const WAVEFORM: &str = "\u{e802}";
    pub const KEYBOARD: &str = "\u{e2d8}";
    pub const SPARKLE: &str = "\u{e6a2}";
    pub const PALETTE: &str = "\u{e6c8}";
    pub const TARGET: &str = "\u{e47c}";
    pub const PLUGS: &str = "\u{eb5a}";
    pub const CURSOR: &str = "\u{e7c8}";
    pub const BATTERY: &str = "\u{e0c2}";
    pub const MINUS: &str = "\u{e32a}";
    pub const SQUARE: &str = "\u{e45e}";
    pub const RESTORE: &str = "\u{e1ca}";
}

pub const FONT: &[u8] = include_bytes!("../assets/fonts/Phosphor-Regular-subset.ttf");

/// `icon  text`, for buttons.
pub fn label(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// Moves the window on a drag and maximizes it on a double click, like a desktop's own title bar.
pub fn drag_window(ctx: &egui::Context, r: &egui::Response) {
    if r.drag_started_by(PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if r.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(ctx)));
    }
}

/// The window has no system decorations: this bar carries the page title and the window buttons.
pub fn title_bar(ui: &mut egui::Ui, icon: &str, title: &str) {
    let bar = ui.interact(ui.max_rect(), ui.scope_id().with("drag"), egui::Sense::click_and_drag());
    drag_window(ui.ctx(), &bar);
    ui.horizontal_centered(|ui| {
        ui.label(RichText::new(label(icon, title)).size(19.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let ctx = ui.ctx().clone();
            if window_button(ui, icon::X, true).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            let max = maximized(&ctx);
            if window_button(ui, if max { icon::RESTORE } else { icon::SQUARE }, false).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
            }
            if window_button(ui, icon::MINUS, false).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
        });
    });
}

/// Round frameless button; the close one turns red under the pointer.
fn window_button(ui: &mut egui::Ui, icon: &str, close: bool) -> egui::Response {
    ui.scope(|ui| {
        if close {
            let red = ui.visuals().error_fg_color;
            let w = &mut ui.visuals_mut().widgets;
            (w.hovered.weak_bg_fill, w.active.weak_bg_fill) = (red.gamma_multiply(0.35), red.gamma_multiply(0.55));
        }
        let button = egui::Button::new(RichText::new(icon).size(15.0)).frame_when_inactive(false).corner_radius(15);
        ui.add(button.min_size(Vec2::new(32.0, 30.0)))
    })
    .inner
}

/// Without decorations the compositor gives no resize border, so the window's own edges start one.
pub fn resize_edges(ctx: &egui::Context) {
    const EDGE: f32 = 6.0;
    let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    if full || maximized(ctx) {
        return;
    }
    let r = ctx.content_rect();
    let (n, s) = (pos.y - r.top() < EDGE, r.bottom() - pos.y < EDGE);
    let (w, e) = (pos.x - r.left() < EDGE, r.right() - pos.x < EDGE);
    use ResizeDirection::*;
    let dir = match (n, s, w, e) {
        (true, _, true, _) => NorthWest,
        (true, _, _, true) => NorthEast,
        (_, true, true, _) => SouthWest,
        (_, true, _, true) => SouthEast,
        (true, ..) => North,
        (_, true, ..) => South,
        (_, _, true, _) => West,
        (_, _, _, true) => East,
        _ => return,
    };
    ctx.set_cursor_icon(match dir {
        North | South => CursorIcon::ResizeVertical,
        East | West => CursorIcon::ResizeHorizontal,
        NorthWest | SouthEast => CursorIcon::ResizeNwSe,
        NorthEast | SouthWest => CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

/// A block of related settings on its own background, the way GNOME groups preferences.
pub fn card<R>(ui: &mut egui::Ui, icon: &str, title: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let r = egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, ui.visuals().text_color().gamma_multiply(0.07)))
        .corner_radius(12)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label(icon, title)).size(17.0).strong());
            ui.add_space(10.0);
            body(ui)
        })
        .inner;
    ui.add_space(14.0);
    r
}

/// Label in the left column of a card's grid, dimmed like GTK's row subtitles.
pub fn key(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(ui.visuals().weak_text_color()));
}

/// Two-column grid inside a card.
pub fn grid<R>(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Grid::new(id).num_columns(2).spacing([28.0, 12.0]).show(ui, body).inner
}

/// Small grey heading over a group of sidebar entries.
pub fn nav_section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(title.to_uppercase()).size(11.5).color(ui.visuals().weak_text_color()));
    ui.add_space(2.0);
}

/// A full-width sidebar entry with its icon.
pub fn nav_item(ui: &mut egui::Ui, selected: bool, icon: &str, text: &str) -> bool {
    let atoms = (RichText::new(icon).size(18.0), text, egui::Atom::grow());
    ui.add(egui::Button::selectable(selected, atoms).min_size(Vec2::new(ui.available_width(), 36.0)))
        .clicked()
}
