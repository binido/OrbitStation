//! GTK colours: libadwaita's palette, the GNOME accent and the `@define-color` overrides in gtk.css.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Theme};
use std::collections::HashMap;
use std::path::Path;

/// The libadwaita named colours the window is drawn with.
#[derive(Clone, Copy)]
pub struct Palette {
    pub window_bg: Color32,
    pub window_fg: Color32,
    pub view_bg: Color32,
    pub sidebar_bg: Color32,
    pub popover_bg: Color32,
    pub card_bg: Color32,
    pub accent: Color32,
    pub accent_bg: Color32,
    pub accent_fg: Color32,
    pub warning: Color32,
    pub error: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// libadwaita 1.6 defaults.
const DARK: Palette = Palette {
    window_bg: hex(0x222226),
    window_fg: hex(0xffffff),
    view_bg: hex(0x1d1d20),
    sidebar_bg: hex(0x2e2e32),
    popover_bg: hex(0x36363a),
    card_bg: hex(0x2e2e32),
    accent: hex(0x78aeed),
    accent_bg: hex(0x3584e4),
    accent_fg: hex(0xffffff),
    warning: hex(0xf8e45c),
    error: hex(0xff7b63),
};

const LIGHT: Palette = Palette {
    window_bg: hex(0xfafafb),
    window_fg: hex(0x323233),
    view_bg: hex(0xffffff),
    sidebar_bg: hex(0xebebed),
    popover_bg: hex(0xffffff),
    card_bg: hex(0xffffff),
    accent: hex(0x1c71d8),
    accent_bg: hex(0x3584e4),
    accent_fg: hex(0xffffff),
    warning: hex(0x9c6e03),
    error: hex(0xc01c28),
};

/// GNOME 47 accent colours by their gsettings name.
const ACCENTS: [(&str, u32); 9] = [
    ("blue", 0x3584e4), ("teal", 0x2190a4), ("green", 0x3a944a), ("yellow", 0xc88800), ("orange", 0xed5b00),
    ("red", 0xe62d42), ("pink", 0xd56199), ("purple", 0x9141ac), ("slate", 0x6f8396),
];

pub struct Colors {
    pub dark: Palette,
    pub light: Palette,
}

impl Colors {
    pub fn get(&self, dark: bool) -> &Palette {
        if dark { &self.dark } else { &self.light }
    }
}

/// Reads the desktop's colours and styles both egui themes with them; eframe picks the theme the
/// system asks for.
pub fn apply(ctx: &egui::Context) -> Colors {
    let accent = gnome_accent();
    let css = gtk_css();
    let colors = Colors { dark: palette(DARK, accent, &css), light: palette(LIGHT, accent, &css) };
    ctx.style_mut_of(Theme::Dark, |s| style(&mut s.visuals, &colors.dark));
    ctx.style_mut_of(Theme::Light, |s| style(&mut s.visuals, &colors.light));
    colors
}

/// GTK applies gtk.css whatever the colour scheme, so it overrides both palettes.
fn palette(mut p: Palette, accent: Option<Color32>, css: &HashMap<String, Color32>) -> Palette {
    if let Some(a) = accent {
        (p.accent, p.accent_bg) = (a, a);
    }
    let fields: [(&str, &mut Color32); 11] = [
        ("window_bg_color", &mut p.window_bg),
        ("window_fg_color", &mut p.window_fg),
        ("view_bg_color", &mut p.view_bg),
        ("sidebar_bg_color", &mut p.sidebar_bg),
        ("popover_bg_color", &mut p.popover_bg),
        ("card_bg_color", &mut p.card_bg),
        ("accent_color", &mut p.accent),
        ("accent_bg_color", &mut p.accent_bg),
        ("accent_fg_color", &mut p.accent_fg),
        ("warning_color", &mut p.warning),
        ("error_color", &mut p.error),
    ];
    for (name, field) in fields {
        if let Some(c) = css.get(name) {
            *field = *c;
        }
    }
    p
}

/// `a` over `b` at `t`, how libadwaita's `alpha(currentColor, t)` comes out on a flat background.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Widget shades follow libadwaita: buttons are the text colour at 10%, 15% hovered, 30% pressed.
fn style(v: &mut egui::Visuals, p: &Palette) {
    let (bg, fg) = (p.window_bg, p.window_fg);
    v.panel_fill = bg;
    v.window_fill = p.popover_bg;
    v.window_stroke = Stroke::new(1.0, mix(fg, p.popover_bg, 0.12));
    v.faint_bg_color = p.card_bg;
    v.extreme_bg_color = mix(fg, p.view_bg, 0.08);
    v.code_bg_color = mix(fg, bg, 0.08);
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.error;
    v.selection.bg_fill = p.accent_bg;
    v.selection.stroke = Stroke::new(1.0, p.accent_fg);
    v.text_cursor.stroke.color = p.accent;
    v.slider_trailing_fill = true;
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = bg;
    w.noninteractive.weak_bg_fill = bg;
    w.noninteractive.bg_stroke = Stroke::new(1.0, mix(fg, bg, 0.15));
    w.noninteractive.fg_stroke = Stroke::new(1.0, mix(fg, bg, 0.85));
    for (s, t) in [(&mut w.inactive, 0.10), (&mut w.hovered, 0.15), (&mut w.active, 0.30), (&mut w.open, 0.15)] {
        s.bg_fill = mix(fg, bg, t);
        s.weak_bg_fill = s.bg_fill;
        s.bg_stroke = Stroke::NONE;
        s.fg_stroke = Stroke::new(1.0, fg);
        s.corner_radius = CornerRadius::same(6);
    }
    w.hovered.bg_stroke = Stroke::new(1.0, mix(fg, bg, 0.25));
    w.noninteractive.corner_radius = CornerRadius::same(6);
}

/// `org.gnome.desktop.interface accent-color`, when the desktop has one.
fn gnome_accent() -> Option<Color32> {
    let out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "accent-color"])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string();
    ACCENTS.iter().find(|a| a.0 == name).map(|a| hex(a.1))
}

/// `@define-color` entries of the user's gtk.css, GTK 4 first, imports followed.
fn gtk_css() -> HashMap<String, Color32> {
    let home = crate::config_home();
    ["gtk-4.0/gtk.css", "gtk-3.0/gtk.css"]
        .iter()
        .map(|f| {
            let mut text = String::new();
            read_css(&home.join(f), &mut text, 0);
            parse_css(&text)
        })
        .find(|c| !c.is_empty())
        .unwrap_or_default()
}

/// Inlines `@import url("x.css");` lines, theme generators put their colours in an imported file.
fn read_css(path: &Path, out: &mut String, depth: u8) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    for line in text.lines() {
        let import = line.trim().strip_prefix("@import url(").and_then(|r| r.split(')').next());
        match import.map(|r| r.trim_matches(['"', '\''])) {
            Some(file) if depth < 4 => read_css(&path.with_file_name(file), out, depth + 1),
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
}

fn parse_css(css: &str) -> HashMap<String, Color32> {
    let raw: HashMap<&str, &str> = css
        .split(';')
        .filter_map(|d| d.trim().rsplit("@define-color").next().filter(|_| d.contains("@define-color")))
        .filter_map(|d| d.trim().split_once(char::is_whitespace))
        .map(|(name, value)| (name, value.trim()))
        .collect();
    raw.iter().filter_map(|(k, v)| Some((k.to_string(), resolve(&raw, v)?))).collect()
}

/// `@name` points at another colour.
fn resolve<'a>(raw: &HashMap<&str, &'a str>, mut value: &'a str) -> Option<Color32> {
    for _ in 0..8 {
        match value.strip_prefix('@') {
            Some(r) => value = raw.get(r)?,
            None => return color(value),
        }
    }
    None
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(…)` and `rgba(…)`; GTK functions like `mix()` are skipped.
fn color(v: &str) -> Option<Color32> {
    if let Some(h) = v.strip_prefix('#') {
        let n = |i: usize, w: usize| u8::from_str_radix(h.get(i..i + w)?, 16).ok();
        return match h.len() {
            3 => Some(Color32::from_rgb(n(0, 1)? * 17, n(1, 1)? * 17, n(2, 1)? * 17)),
            6 | 8 => Some(Color32::from_rgb(n(0, 2)?, n(2, 2)?, n(4, 2)?)),
            _ => None,
        };
    }
    let args = v.strip_prefix("rgba(").or_else(|| v.strip_prefix("rgb("))?.strip_suffix(')')?;
    let mut it = args.split(',').map(|a| a.trim().parse::<f32>().ok());
    let (r, g, b) = (it.next()??, it.next()??, it.next()??);
    Some(Color32::from_rgb(r as u8, g as u8, b as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_noctalia_style_css() {
        let css = parse_css(
            "/* generated */\n@define-color accent_bg_color #ffb3b6;\n@define-color window_bg_color #1a1111;\n\
             @define-color headerbar_backdrop_color @window_bg_color;\n@define-color shade rgba(0, 0, 0, 0.36);\n\
             @define-color short #fa0;\n@define-color skipped mix(#fff, #000, 0.5);\n:root { --accent-color: #ffb3b6; }",
        );
        assert_eq!(css["accent_bg_color"], hex(0xffb3b6));
        assert_eq!(css["headerbar_backdrop_color"], hex(0x1a1111));
        assert_eq!(css["shade"], hex(0x000000));
        assert_eq!(css["short"], hex(0xffaa00));
        assert!(!css.contains_key("skipped"));
        let p = palette(DARK, None, &css);
        assert_eq!((p.accent_bg, p.window_bg, p.view_bg), (hex(0xffb3b6), hex(0x1a1111), DARK.view_bg));
    }
}
