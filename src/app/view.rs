//! What the window remembers about each connected device: what it last reported and what is being
//! edited for it, so switching between devices keeps every page as it was.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::storage;
use crate::flydigi::dock_led::LedConfig;
use crate::flydigi::profile::{self, Profile, ProfileError, macro_page, macros, rgb};
use crate::station::{Device, Error, Info, Model, Settings};

/// How often a connected device is asked whether it is still there.
const POLL: Duration = Duration::from_secs(10);
/// Time per window frame a lighting write may take, so the window keeps drawing.
const UPLOAD_SLICE: Duration = Duration::from_millis(30);

/// A profile as read from the device and as edited in the window.
pub struct Mapping {
    pub model: &'static Model,
    pub slot: u8,
    /// The copy on the device, the editor compares against it.
    pub device: Profile,
    pub map: Vec<u8>,
    pub macros: Vec<macros::Macro>,
    pub rgb: Vec<u8>,
    /// Colours the lighting effect is generated from; the device stores only frames.
    pub colors: Vec<rgb::Rgb>,
    /// Set by the vibration editor, the app runs the motors for a second.
    pub test_vibration: bool,
}

impl Mapping {
    pub fn new(model: &'static Model, slot: u8, device: Profile) -> Mapping {
        Mapping {
            model,
            slot,
            map: device.map.clone(),
            macros: read_macros(model, &device),
            rgb: device.rgb.clone(),
            colors: vec![[0, 116, 255]],
            test_vibration: false,
            device,
        }
    }

    /// Puts a profile from a file or the factory template into the editor, the device copy stays.
    ///
    /// # Errors
    /// [`ProfileError::OtherDevice`] when the profile is from a device with other blob sizes.
    pub fn replace(&mut self, p: &Profile) -> Result<(), ProfileError> {
        if !self.device.fits(p) {
            return Err(ProfileError::OtherDevice);
        }
        self.map.clone_from(&p.map);
        self.macros = read_macros(self.model, p);
        self.rgb.clone_from(&p.rgb);
        Ok(())
    }

    /// The profile to write. Keys bound to a macro follow the macro editor on models that have one.
    ///
    /// # Errors
    /// [`ProfileError`] when the macros do not fit or two share a key.
    pub fn build(&self) -> Result<Profile, ProfileError> {
        if !self.model.features.macros {
            // macros the editor does not show (protocol 3.1 keeps them in the mapping blob) stay as
            // the device has them, bindings included
            return Ok(Profile { map: self.map.clone(), macros: self.device.macros.clone(), rgb: self.rgb.clone() });
        }
        let mut map = self.map.clone();
        if self.model.macro_store {
            profile::bind_macros(&mut map, self.model.keys, &self.macros)?;
            let store = macros::encode(&self.device.macros, &self.macros)?;
            return Ok(Profile { map, macros: store, rgb: self.rgb.clone() });
        }
        // An untouched page keeps its bytes: re-encoding would clear unset intervals and orphaned
        // bindings the user did not ask about.
        if self.macros != macro_page::parse(&map) {
            profile::bind_macros(&mut map, self.model.keys, &self.macros)?;
            macro_page::encode(&mut map, &self.macros)?;
        }
        Ok(Profile { map, macros: self.device.macros.clone(), rgb: self.rgb.clone() })
    }

    /// Most macros the model's profile holds.
    pub fn max_macros(&self) -> usize {
        if self.model.macro_store { macros::MAX } else { macro_page::MAX }
    }
}

/// Macros as the device holds them, wherever the model keeps them.
fn read_macros(model: &Model, p: &Profile) -> Vec<macros::Macro> {
    match (model.features.macros, model.macro_store) {
        (false, _) => vec![],
        (true, true) => macros::parse(&p.macros),
        (true, false) => macro_page::parse(&p.map),
    }
}

/// Window state of one connected device.
#[derive(Default)]
pub struct DeviceView {
    pub info: Option<Info>,
    pub settings: Option<Settings>,
    /// The profile slot the device plays.
    pub active: u8,
    pub nickname: String,
    pub mapping: Option<Mapping>,
    pub import_pick: Option<PathBuf>,
    pub confirm_reset_all: bool,
    /// Motors on the test page: grip left/right, trigger left/right.
    pub rumble: [u8; 4],
    pub rumble_stop: Option<Instant>,
    /// Whether the device sends live input now.
    pub streaming: bool,
    /// Last outcome, for the status line.
    pub note: String,
    /// The dock lighting being edited, read from the device first.
    pub dock_led: Option<LedConfig>,
    /// Share of a dock lighting write sent so far, while one runs.
    pub upload: Option<f32>,
    last_poll: Option<Instant>,
}

impl DeviceView {
    /// Asks the device for everything the pages show.
    pub fn refresh(&mut self, dev: &mut dyn Device) {
        self.last_poll = Some(Instant::now());
        self.note = match self.load(dev) {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
    }

    fn load(&mut self, dev: &mut dyn Device) -> Result<(), Error> {
        self.info = Some(dev.info()?);
        self.nickname = dev.nickname()?;
        if let Some(s) = dev.settings() {
            self.settings = Some(s.read()?);
        }
        if let Some(p) = dev.profiles() {
            self.active = p.active()?;
        }
        Ok(())
    }

    /// Checks in with the device every [`POLL`].
    ///
    /// # Errors
    /// When the device did not answer; the caller lets it go so a fresh scan can open it again.
    pub fn poll(&mut self, dev: &mut dyn Device) -> Result<(), Error> {
        if self.last_poll.is_some_and(|t| t.elapsed() < POLL) {
            return Ok(());
        }
        self.last_poll = Some(Instant::now());
        self.info = Some(dev.info()?);
        Ok(())
    }

    /// Whether the active slot was loaded into the editor.
    pub fn has_mapping(&self) -> bool {
        self.mapping.as_ref().is_some_and(|m| m.slot == self.active)
    }

    /// The editor of the active slot, if it was loaded.
    pub fn mapping_mut(&mut self) -> Option<&mut Mapping> {
        self.mapping.as_mut().filter(|m| m.slot == self.active)
    }

    /// Reads the active slot into the editor.
    pub fn load_mapping(&mut self, dev: &mut dyn Device) {
        let slot = self.active;
        let model = dev.model();
        let Some(store) = dev.profiles() else { return };
        self.note = match store.read(slot) {
            Ok(p) => {
                self.mapping = Some(Mapping::new(model, slot, p));
                t!("Profile {} loaded", slot + 1)
            }
            Err(e) => t!("Could not read the profile").to_string() + ": " + &e.to_string(),
        };
    }

    /// Writes the editor to the device after backing up what the device held.
    pub fn save_mapping(&mut self, dev: &mut dyn Device) {
        let Some(m) = self.mapping.as_mut() else { return };
        let Some(store) = dev.profiles() else { return };
        let mut new = match m.build() {
            Ok(p) => p,
            Err(e) => {
                self.note = e.to_string();
                return;
            }
        };
        // Lighting has no tag of its own, a lighting-only save keeps the profile's tag.
        if new.map != m.device.map || new.macros != m.device.macros {
            profile::map::set_version(&mut new.map, storage::new_version(&m.device.map));
        }
        let backup = storage::backup_profile(m.model, m.slot, &m.device);
        self.note = match store.write(m.slot, &m.device, &new) {
            Err(e) => t!("Write failed: {}", e),
            Ok(()) if store.read(m.slot).ok().as_ref() != Some(&new) => {
                t!("Written, but the check read did not match, load the profile again").into()
            }
            Ok(()) => {
                *m = Mapping::new(m.model, m.slot, new);
                match backup {
                    Ok(path) => t!("Profile saved, previous copy: {}", path.display()),
                    Err(e) => t!("Profile saved, but the copy could not be made: {}", e),
                }
            }
        };
    }

    /// Sends a change and reports the outcome in the status line.
    pub fn report(&mut self, result: Result<(), Error>, what: &str) {
        self.note = match result {
            Ok(()) => t!("Saved: {}", what),
            Err(e) => t!("Could not apply: {}", what) + " (" + &e.to_string() + ")",
        };
    }

    /// Turns live input on while the test page of this device is open, stops a timed rumble and
    /// sends the next part of a lighting write.
    pub fn tick(&mut self, dev: &mut dyn Device, testing: bool) {
        if self.upload.is_some()
            && let Some(l) = dev.dock_lighting()
        {
            match l.step_lighting(UPLOAD_SLICE) {
                Ok(done) if done >= 1.0 => {
                    self.upload = None;
                    self.note = t!("Lighting written").into();
                }
                Ok(done) => self.upload = Some(done),
                Err(e) => {
                    self.upload = None;
                    self.note = e.to_string();
                }
            }
        }
        let stop = self.rumble_stop.is_some_and(|t| Instant::now() >= t);
        let Some(live) = dev.live() else { return };
        if stop {
            self.rumble_stop = None;
            self.report_err(live.rumble([0; 4]));
        }
        if self.streaming != testing {
            self.streaming = testing;
            self.report_err(live.stream(testing));
        }
    }

    /// Stops everything the app started on the device, before the window closes.
    pub fn shut_down(&mut self, dev: &mut dyn Device) {
        if let Some(live) = dev.live() {
            // the window is closing, there is no status line left to report to
            let _ = live.rumble([0; 4]);
            let _ = live.stream(false);
        }
    }

    fn report_err(&mut self, r: Result<(), Error>) {
        if let Err(e) = r {
            self.note = e.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flydigi::profile::map;
    use crate::models::APEX_5;

    fn apex_profile() -> Profile {
        let mut m = vec![0xff; map::SIZE];
        // no macros, every repeat interval at the factory 30 ms, and a stale macro binding on M1
        m[820..825].fill(3);
        map::set_key(&mut m, 18, map::Key { target: map::MACRO, turbo: 0, hz: 0 });
        Profile { map: m, macros: vec![], rgb: vec![0; 380] }
    }

    #[test]
    fn an_untouched_apex_profile_builds_back_unchanged() {
        let m = Mapping::new(&APEX_5, 0, apex_profile());
        assert_eq!(m.build(), Ok(apex_profile()));
    }

    #[test]
    fn an_apex_macro_goes_into_the_mapping_blob_and_binds_its_key() {
        let mut m = Mapping::new(&APEX_5, 0, apex_profile());
        let tap = vec![macros::Step { delay_ms: 0, key: 4, event: 1 }, macros::Step { delay_ms: 50, key: 4, event: 0 }];
        m.macros.push(macros::Macro { key: 19, kind: 1, interval_ms: 0, steps: tap });
        let p = m.build().expect("one macro fits");
        assert_eq!(macro_page::parse(&p.map), m.macros);
        assert_eq!(map::key(&p.map, 19).target, map::MACRO);
        assert_eq!(map::key(&p.map, 18).target, map::IDENTITY);
        assert!(p.macros.is_empty());
    }
}
