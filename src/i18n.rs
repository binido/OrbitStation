//! UI strings are written in English and looked up in a per-language table, English is the fallback.

mod ru;

use std::fmt::Display;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

/// `t!("Text")` is the translated text, `t!("Saved: {}", what)` fills each `{}` in order.
macro_rules! t {
    ($s:literal) => {
        $crate::i18n::tr($s)
    };
    ($s:literal, $($a:expr),+ $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($s), &[$(&$a as &dyn std::fmt::Display),+])
    };
}

#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Ru,
}

/// Each name is written in its own language, so it can be found whatever is shown now.
pub const LANGS: [(Lang, &str, &str); 2] = [(Lang::En, "en", "English"), (Lang::Ru, "ru", "Русский")];

static CURRENT: AtomicU8 = AtomicU8::new(Lang::En as u8);

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::Ru as u8 { Lang::Ru } else { Lang::En }
}

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

/// Strings from tables are translated where they are shown, so this takes any `&str`.
pub fn tr(s: &str) -> &str {
    match current() {
        Lang::En => s,
        Lang::Ru => ru::ru(s).unwrap_or(s),
    }
}

pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut parts = template.split("{}");
    let mut out = parts.next().unwrap_or_default().to_string();
    for (part, arg) in parts.zip(args) {
        out += &arg.to_string();
        out += part;
    }
    out
}

/// The locale the desktop runs in, the same variables gettext reads; anything without a table is English.
pub fn system() -> Lang {
    let locale = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default();
    if locale.starts_with("ru") { Lang::Ru } else { Lang::En }
}

fn config_file() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("orbitstation/language")
}

/// The language picked in the settings, or the system one until something is picked.
pub fn load() -> Lang {
    let saved = std::fs::read_to_string(config_file()).unwrap_or_default();
    LANGS.iter().find(|l| l.1 == saved.trim()).map_or_else(system, |l| l.0)
}

pub fn save(lang: Lang) -> std::io::Result<()> {
    let path = config_file();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, LANGS.iter().find(|l| l.0 == lang).unwrap().1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{self, Toggle, gyro, macros, map, rgb, stick};

    /// Every `t!("…")` in the sources, read from the files so a new string cannot slip by untranslated.
    fn literals() -> Vec<String> {
        let mut out = vec![];
        for src in [include_str!("main.rs"), include_str!("device.rs"), include_str!("proto.rs")] {
            for (at, m) in src.match_indices("t!(\"") {
                // `format!("` ends the same way
                if src[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let mut s = String::new();
                let mut chars = src[at + m.len()..].chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => s.extend(chars.next()),
                        '"' => break,
                        c => s.push(c),
                    }
                }
                out.push(s);
            }
        }
        out
    }

    #[test]
    fn every_string_has_russian() {
        let mut all = literals();
        let tables: [&[(u8, &str)]; 11] = [
            &map::TARGETS[..1], &map::TURBO, &stick::PRESETS, &gyro::TARGETS, &gyro::ENABLE, &rgb::EFFECTS,
            &macros::KINDS, &macros::EVENTS, &macros::DIRECTIONS[8..], &proto::SLEEP_TIMES, &proto::SENSITIVITIES,
        ];
        all.extend(tables.iter().flat_map(|t| t.iter().map(|e| e.1.to_string())));
        all.extend(stick::SIDES.iter().map(|s| s.to_string()));
        all.extend(Toggle::ALL.iter().map(|t| t.label().to_string()));
        let missing: Vec<_> = all.iter().filter(|s| ru::ru(s).is_none()).collect();
        assert!(missing.is_empty(), "no Russian for {missing:?}");
        for s in &all {
            assert_eq!(s.matches("{}").count(), ru::ru(s).unwrap().matches("{}").count(), "placeholders differ in {s:?}");
        }
    }

    #[test]
    fn fills_placeholders_in_order() {
        assert_eq!(fill("{} of {}", &[&1, &"two"]), "1 of two");
        assert_eq!(fill("no args", &[]), "no args");
    }
}
