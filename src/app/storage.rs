//! Files the app keeps: settings in `~/.config/orbitstation`, profiles and backups in
//! `~/.local/share/orbitstation`.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::flydigi::profile::{Profile, map};
use crate::station::Model;

const APP_DIR: &str = "orbitstation";
const PROFILES_DIR: &str = "profiles";
const BACKUPS_DIR: &str = "backups";
const PROFILE_EXT: &str = "osp";

/// `$XDG_CONFIG_HOME`, or `~/.config`.
pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_path(name: &str) -> PathBuf {
    config_home().join(APP_DIR).join(name)
}

/// A setting saved with [`write_config`], one small file per setting.
pub fn read_config(name: &str) -> Option<String> {
    std::fs::read_to_string(config_path(name)).ok().map(|s| s.trim().to_string())
}

/// Saves a setting.
///
/// # Errors
/// When the config folder cannot be created or written.
pub fn write_config(name: &str, value: &str) -> io::Result<()> {
    write_file(&config_path(name), value.as_bytes())
}

/// `$XDG_DATA_HOME/orbitstation`, or `~/.local/share/orbitstation`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

/// The folder backups go to.
pub fn backups_dir() -> PathBuf {
    data_dir().join(BACKUPS_DIR)
}

/// Writes `data`, creating the folder first.
///
/// # Errors
/// When the folder cannot be created or the file written.
pub fn write_file(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, data)
}

/// Saves a profile under its title in the profiles folder.
///
/// # Errors
/// When the file cannot be written.
pub fn export_profile(slot: u8, p: &Profile) -> io::Result<PathBuf> {
    let title: String = map::title(&p.map).chars().filter(|c| c.is_alphanumeric() || " -_".contains(*c)).collect();
    let name = match title.trim() {
        "" => format!("profile{}", slot + 1),
        t => t.to_string(),
    };
    let path = data_dir().join(PROFILES_DIR).join(format!("{name}.{PROFILE_EXT}"));
    write_file(&path, &p.to_file())?;
    Ok(path)
}

/// Saves the profile as it was on the device before a write; flash has no other undo.
///
/// # Errors
/// When the file cannot be written.
pub fn backup_profile(model: &Model, slot: u8, p: &Profile) -> io::Result<PathBuf> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let path = backups_dir().join(format!("{}-profile{}-{secs}.{PROFILE_EXT}", model.slug, slot + 1));
    write_file(&path, &p.to_file())?;
    Ok(path)
}

/// Saved profiles first, then backups, newest first.
pub fn profile_files() -> Vec<PathBuf> {
    let list = |dir: &str| {
        let mut v: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(data_dir().join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == PROFILE_EXT))
            .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(UNIX_EPOCH), e.path()))
            .collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.0));
        v.into_iter().map(|(_, p)| p)
    };
    list(PROFILES_DIR).chain(list(BACKUPS_DIR)).collect()
}

/// A file's name for the picker, backups are marked as such.
pub fn file_label(p: &Path) -> String {
    let name = p.file_stem().unwrap_or_default().to_string_lossy();
    if p.parent().is_some_and(|d| d.ends_with(BACKUPS_DIR)) { t!("backup: {}", name) } else { name.into_owned() }
}

/// Opens the data folder in the file manager.
pub fn open_data_dir() {
    let dir = data_dir();
    // the folder is only for looking at, a failure leaves nothing to report
    let _ = std::fs::create_dir_all(dir.join(PROFILES_DIR));
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
}

/// Space Station picks a random tag that differs from the current one; 0xffff means "never written".
pub fn new_version(blob: &[u8]) -> u16 {
    let old = map::version(blob);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let v = (nanos % 0xffff) as u16;
    if v == old { (v + 1) % 0xffff } else { v }
}
