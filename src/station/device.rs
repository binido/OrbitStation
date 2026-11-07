//! What the app can ask of a connected device. [`Device`] is the part every device has, the
//! capability traits are the parts a model may or may not have; the app shows a page per capability.

use std::path::Path;
use std::time::Duration;

use super::{Error, Model};
use crate::flydigi::dock_led::LedConfig;
use crate::flydigi::profile::Profile;

/// Charge of a battery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    /// Level out of 5, 20 % a step; no device reports a finer figure. `None` while charging on a
    /// device that then reports no level at all.
    pub level: Option<u8>,
    pub charging: bool,
}

impl Battery {
    /// Highest level a device reports.
    pub const MAX_LEVEL: u8 = 5;

    /// The level as a percentage, in 20 % steps.
    pub fn percent(self) -> Option<u8> {
        self.level.map(|l| l.min(Battery::MAX_LEVEL) * (100 / Battery::MAX_LEVEL))
    }
}

/// State the device reports about itself.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Info {
    /// Model id from the device, see [`Model::device_ids`].
    pub device_id: u8,
    pub firmware: String,
    /// Firmware of the radio module, for devices that have one.
    pub radio: Option<String>,
    /// The device's own battery, `None` for one without.
    pub battery: Option<Battery>,
    /// For a dock: `Some(None)` when it is empty, `Some(Some(_))` with the seated controller's charge.
    pub docked: Option<Option<Battery>>,
}

/// One live input report: sticks, triggers, buttons and motion sensors.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Input {
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
    /// One bit per button, named by [`Model::buttons`].
    pub buttons: u32,
    pub lt: u8,
    pub rt: u8,
    pub gyro: [i16; 3],
    pub accel: [i16; 3],
}

/// On/off settings kept in the device, outside the profiles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Toggle {
    // a controller's
    QuickSwitchConfig,
    XboxHomeButton,
    MotionDebounce,
    MappingSwitch,
    JoystickDebounce,
    JoystickAutoCalibration,
    JoystickRebound,
    ScreenStatusBar,
    /// Space Station calls it `OffScreen`, but on the bit the picture stays up rather than going dark.
    AlwaysOnDisplay,
    Audio,
    // a dock's
    /// Space Station calls it "Intelligent start"; in practice it turns the lights of the dock and of
    /// the seated controller off while it charges.
    DockLightsOffWhileCharging,
    DockLightingSync,
    DockOffWithComputer,
    DockChargeAnimation,
}

impl Toggle {
    /// A controller's toggles, in the order the settings page lists them.
    pub const PAD: [Toggle; 10] = [
        Toggle::JoystickAutoCalibration,
        Toggle::JoystickDebounce,
        Toggle::JoystickRebound,
        Toggle::MotionDebounce,
        Toggle::QuickSwitchConfig,
        Toggle::MappingSwitch,
        Toggle::XboxHomeButton,
        Toggle::ScreenStatusBar,
        Toggle::AlwaysOnDisplay,
        Toggle::Audio,
    ];

    /// A dock's toggles.
    pub const DOCK: [Toggle; 4] = [
        Toggle::DockLightsOffWhileCharging,
        Toggle::DockLightingSync,
        Toggle::DockOffWithComputer,
        Toggle::DockChargeAnimation,
    ];

    /// Translated name for the settings page.
    pub fn label(self) -> &'static str {
        match self {
            Toggle::QuickSwitchConfig => t!("Quick profile switching"),
            Toggle::XboxHomeButton => t!("Home button as Xbox Guide"),
            Toggle::MotionDebounce => t!("Gyro jitter filter"),
            Toggle::MappingSwitch => t!("Layout switching"),
            Toggle::JoystickDebounce => t!("Stick jitter filter"),
            Toggle::JoystickAutoCalibration => t!("Stick auto-calibration"),
            Toggle::JoystickRebound => t!("Stick rebound algorithm"),
            Toggle::ScreenStatusBar => t!("Always show the on-screen status bar"),
            Toggle::AlwaysOnDisplay => t!("Keep the picture on the screen"),
            Toggle::Audio => t!("Audio through the gamepad"),
            Toggle::DockLightsOffWhileCharging => t!("Lights off while a controller charges"),
            Toggle::DockLightingSync => t!("Lighting in step with the controller"),
            Toggle::DockOffWithComputer => t!("Turn off with the computer"),
            Toggle::DockChargeAnimation => t!("Charging animation"),
        }
    }
}

/// Settings with a fixed list of values, kept in the device outside the profiles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    ReportRate,
    Precision,
    Sensitivity,
    SleepTime,
}

impl Choice {
    /// Translated name for the settings page.
    pub fn label(self) -> &'static str {
        match self {
            Choice::ReportRate => t!("Stick polling rate"),
            Choice::Precision => t!("Stick precision"),
            Choice::Sensitivity => t!("Sensitivity near the stick center"),
            Choice::SleepTime => t!("Time until sleep"),
        }
    }

    /// Values as (raw value, untranslated name); names go through [`crate::i18n::tr`] when shown.
    pub fn options(self) -> &'static [(u8, &'static str)] {
        match self {
            Choice::ReportRate => &[(1, "1000 Hz"), (2, "500 Hz"), (3, "250 Hz"), (4, "125 Hz")],
            Choice::Precision => {
                &[(1, "8 bit"), (4, "9 bit"), (2, "10 bit"), (5, "11 bit"), (3, "12 bit"), (6, "14 bit"), (7, "16 bit")]
            }
            Choice::Sensitivity => &[
                (14, "Highest"),
                (15, "High"),
                (16, "Above average"),
                (17, "Average"),
                (18, "Below average"),
                (19, "Low"),
                (20, "Lowest"),
            ],
            Choice::SleepTime => {
                &[(1, "1 min"), (5, "5 min"), (15, "15 min"), (60, "1 hour"), (180, "3 hours"), (0, "Never")]
            }
        }
    }
}

/// The device's own settings: only those it supports are listed.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Settings {
    pub toggles: Vec<(Toggle, bool)>,
    pub choices: Vec<(Choice, u8)>,
}

/// A connected device. A model of a known family is described by a [`Model`]; a new protocol family
/// implements this trait and registers itself in the station.
pub trait Device: Send {
    /// hidraw node the device talks through, unique while it is connected.
    fn path(&self) -> &Path;

    fn model(&self) -> &'static Model;

    /// False once the device was unplugged; the station drops it on its next scan.
    fn is_alive(&self) -> bool;

    /// Battery, firmware and the model id.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn info(&mut self) -> Result<Info, Error>;

    /// The name the owner gave the device, empty when there is none.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn nickname(&mut self) -> Result<String, Error>;

    /// Stores a new name, an empty one restores the factory state.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn set_nickname(&mut self, name: &str) -> Result<(), Error>;

    /// Settings kept in the device, `None` when it has none.
    fn settings(&mut self) -> Option<&mut dyn HardwareSettings> {
        None
    }

    /// Profile slots, `None` when the device keeps no profiles.
    fn profiles(&mut self) -> Option<&mut dyn ProfileStore> {
        None
    }

    /// Live input and rumble, `None` when the device has no controls of its own.
    fn live(&mut self) -> Option<&mut dyn LiveInput> {
        None
    }

    /// Triggers that push back, `None` for a device without them.
    fn force_triggers(&mut self) -> Option<&mut dyn ForceTriggers> {
        None
    }

    /// Lighting kept outside any profile and uploaded as frames, a dock's.
    fn dock_lighting(&mut self) -> Option<&mut dyn DockLighting> {
        None
    }
}

/// Force triggers. Their effects live in the profile and start only when sent live.
pub trait ForceTriggers {
    /// Starts the effects a mapping blob holds, without storing them.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device did not take an effect.
    fn engage(&mut self, map: &[u8]) -> Result<(), Error>;
}

/// Lighting a device plays from frames the host computes. An upload takes seconds, so it is sent a
/// little at a time and the window stays responsive.
pub trait DockLighting {
    /// The effect the device has, without its frames: the device never returns them.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn read_lighting(&mut self) -> Result<LedConfig, Error>;

    /// Starts writing `config`; [`DockLighting::step_lighting`] sends it.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device did not take the start.
    fn start_lighting(&mut self, config: &LedConfig) -> Result<(), Error>;

    /// Sends what fits in `budget` and returns the share sent, 1.0 once the write is complete.
    ///
    /// # Errors
    /// [`Error::NoReply`] when a pack was not acknowledged; the write is then abandoned.
    fn step_lighting(&mut self, budget: Duration) -> Result<f32, Error>;
}

/// Settings stored in the device outside the profiles.
pub trait HardwareSettings {
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn read(&mut self) -> Result<Settings, Error>;

    /// # Errors
    /// [`Error::NoReply`] when the device did not take the change.
    fn set_toggle(&mut self, toggle: Toggle, on: bool) -> Result<(), Error>;

    /// # Errors
    /// [`Error::NoReply`] when the device did not take the change.
    fn set_choice(&mut self, choice: Choice, value: u8) -> Result<(), Error>;
}

/// Profile slots in the device's flash.
pub trait ProfileStore {
    /// The slot the device plays now.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device stays silent.
    fn active(&mut self) -> Result<u8, Error>;

    /// Makes `slot` the one the device plays.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device did not switch.
    fn activate(&mut self, slot: u8) -> Result<(), Error>;

    /// Reads a slot. Devices may switch to the slot while answering, callers restore the active one.
    ///
    /// # Errors
    /// [`Error::NoReply`] when a part of the profile did not arrive.
    fn read(&mut self, slot: u8) -> Result<Profile, Error>;

    /// Writes the parts of `new` that differ from `old`, commits them to flash and plays the slot.
    ///
    /// # Errors
    /// [`Error::NoReply`] when a packet or the commit was not confirmed; the slot may then hold a
    /// mix of both, the caller keeps a backup.
    fn write(&mut self, slot: u8, old: &Profile, new: &Profile) -> Result<(), Error>;

    /// Puts every slot back to factory settings.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the reset was not confirmed.
    fn reset_all(&mut self) -> Result<(), Error>;
}

/// Live controls for the test page.
pub trait LiveInput {
    /// Turns the stream of extended input reports on or off.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device did not switch.
    fn stream(&mut self, on: bool) -> Result<(), Error>;

    /// The newest report, `None` before the first one arrives.
    fn input(&self) -> Option<Input>;

    /// Runs the motors: left grip, right grip, left trigger, right trigger; 0 stops one.
    ///
    /// # Errors
    /// [`Error::NoReply`] when the device did not take the command.
    fn rumble(&mut self, motors: [u8; 4]) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_is_a_fifth_of_the_charge() {
        let at = |level| Battery { level, charging: false }.percent();
        assert_eq!((at(Some(5)), at(Some(3)), at(Some(0)), at(None)), (Some(100), Some(60), Some(0), None));
        assert_eq!(at(Some(9)), Some(100));
    }
}
