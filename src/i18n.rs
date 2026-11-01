//! UI strings are written in English and looked up in a per-language table, English is the fallback.

mod ru;

use std::fmt::Display;
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

/// The name of `lang` in its own language.
pub fn name(lang: Lang) -> &'static str {
    LANGS.iter().find(|l| l.0 == lang).map_or("English", |l| l.2)
}

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

/// The language picked in the settings, or the system one until something is picked.
pub fn load() -> Lang {
    let saved = crate::app::read_config("language").unwrap_or_default();
    LANGS.iter().find(|l| l.1 == saved).map_or_else(system, |l| l.0)
}

pub fn save(lang: Lang) -> std::io::Result<()> {
    let code = LANGS.iter().find(|l| l.0 == lang).map_or("en", |l| l.1);
    crate::app::write_config("language", code)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::flydigi::profile::{gyro, macros, map, rgb, stick};
    use crate::models;
    use crate::station::{Choice, Toggle};

    /// Rust sources under `dir`, read from disk so a new file cannot slip by.
    fn sources(dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("src is readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("i18n.rs") {
                out.push(std::fs::read_to_string(&path).expect("a source file is readable"));
            }
        }
    }

    /// Every `t!("…")` in the sources.
    fn literals() -> Vec<String> {
        let mut files = vec![];
        sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let mut out = vec![];
        for src in &files {
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

    /// Names kept untranslated in tables and translated where they are shown.
    fn table_names() -> Vec<&'static str> {
        let tables: [&[(u8, &str)]; 11] = [
            &models::VADER_5_PRO.targets[..1],
            &map::TURBO,
            &stick::PRESETS,
            &gyro::TARGETS,
            &gyro::ENABLE,
            &rgb::EFFECTS,
            &macros::KINDS,
            &macros::EVENTS,
            &macros::DIRECTIONS[8..],
            Choice::SleepTime.options(),
            Choice::Sensitivity.options(),
        ];
        let mut all: Vec<&str> = tables.iter().flat_map(|t| t.iter().map(|e| e.1)).collect();
        all.extend(stick::SIDES);
        all.push(models::UNKNOWN.name);
        all.extend(Toggle::ALL.iter().map(|t| t.label()));
        all
    }

    #[test]
    fn every_string_has_russian() {
        let mut all = literals();
        all.extend(table_names().into_iter().map(String::from));
        let missing: Vec<_> = all.iter().filter(|s| ru::ru(s).is_none()).collect();
        assert!(missing.is_empty(), "no Russian for {missing:?}");
    }

    #[test]
    fn translations_keep_their_placeholders() {
        let mut all = literals();
        all.extend(table_names().into_iter().map(String::from));
        for s in all.iter().filter_map(|s| Some((s, ru::ru(s)?))) {
            assert_eq!(s.0.matches("{}").count(), s.1.matches("{}").count(), "placeholders differ in {:?}", s.0);
        }
    }

    #[test]
    fn fills_placeholders_in_order() {
        assert_eq!(fill("{} of {}", &[&1, &"two"]), "1 of two");
        assert_eq!(fill("no args", &[]), "no args");
    }
}
