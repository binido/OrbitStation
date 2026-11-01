//! What the window remembers about each connected device: what it last reported and what is being
//! edited for it, so switching between devices keeps every page as it was.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::storage;
use crate::flydigi::profile::{self, Profile, ProfileError, macros, rgb};
use crate::station::{Device, Error, Info, Model, Settings};

/// How often a connected device is asked whether it is still there.
const POLL: Duration = Duration::from_secs(10);

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
            macros: macros::parse(&device.macros),
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
        self.macros = macros::parse(&p.macros);
        self.rgb.clone_from(&p.rgb);
        Ok(())
    }

    /// The profile to write.
    ///
    /// # Errors
    /// [`ProfileError`] when the macros do not fit or two share a key.
    pub fn build(&self) -> Result<Profile, ProfileError> {
        let mut map = self.map.clone();
        profile::bind_macros(&mut map, self.model.keys, &self.macros)?;
        Ok(Profile { map, macros: macros::encode(&self.device.macros, &self.macros)?, rgb: self.rgb.clone() })
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

    /// Turns live input on while the test page of this device is open, and stops a timed rumble.
    pub fn tick(&mut self, dev: &mut dyn Device, testing: bool) {
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
