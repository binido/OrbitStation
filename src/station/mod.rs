//! The station: finds supported devices on the HID bus and keeps one handle per connected device.
//! It knows nothing about windows, the app asks it for devices and talks to them through [`Device`].

mod device;
mod error;
pub mod hid;
mod model;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub use device::{
    Battery, Choice, Device, DockLighting, HardwareSettings, Info, Input, LiveInput, ProfileStore, Settings, Toggle,
};
pub use error::Error;
pub use model::{Factory, Features, Kind, Model};

use crate::flydigi;

/// A protocol family: which HID nodes it speaks to and how to open one.
pub struct Family {
    pub claims: fn(&hid::Node) -> bool,
    pub open: fn(&hid::Node) -> Result<Box<dyn Device>, Error>,
}

/// Every family the station talks to. A device with a protocol of its own adds a family here.
const FAMILIES: &[Family] = &[flydigi::PAD_FAMILY, flydigi::dock::FAMILY];

/// How long a node that failed to open is left alone; probing a silent one stalls the window briefly.
const RETRY: Duration = Duration::from_secs(10);

/// Connected devices and the nodes that failed to open.
#[derive(Default)]
pub struct Station {
    devices: Vec<Box<dyn Device>>,
    failed: HashMap<PathBuf, Instant>,
}

impl Station {
    /// Connected devices in the order they were found.
    pub fn devices(&self) -> impl Iterator<Item = &dyn Device> {
        self.devices.iter().map(|d| d.as_ref())
    }

    /// The device on `path`, if it is still connected.
    pub fn get(&mut self, path: &Path) -> Option<&mut (dyn Device + 'static)> {
        self.devices.iter_mut().find(|d| d.path() == path).map(|d| d.as_mut())
    }

    /// Forgets a device that stopped answering; the next scan opens it again if it is still there.
    pub fn drop_device(&mut self, path: &Path) {
        self.devices.retain(|d| d.path() != path);
    }

    /// Drops unplugged devices and opens newly connected ones.
    ///
    /// Returns why nodes a family claimed could not be opened, for the status line.
    pub fn scan(&mut self) -> Vec<Error> {
        self.devices.retain(|d| d.is_alive());
        let now = Instant::now();
        self.failed.retain(|_, until| *until > now);
        let mut errors = vec![];
        for node in hid::nodes() {
            let known = self.devices.iter().any(|d| d.path() == node.path) || self.failed.contains_key(&node.path);
            let Some(family) = FAMILIES.iter().find(|f| !known && (f.claims)(&node)) else {
                continue;
            };
            match (family.open)(&node) {
                Ok(device) => self.devices.push(device),
                Err(e) => {
                    self.failed.insert(node.path, now + RETRY);
                    errors.push(e);
                }
            }
        }
        errors
    }
}
