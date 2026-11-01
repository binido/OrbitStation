//! Flydigi's Space Station protocol ("NewXInput"), spoken on a vendor HID interface. One
//! [`FlydigiDevice`] serves every controller of the family, the [`Model`] it reports picks what it
//! can do; [`dock`] is the charging dock, which shares the envelope and little else.

pub mod dock;
pub mod dock_led;
pub mod frame;
mod parse;
pub mod profile;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use frame::{PKT, cmd, request};
use profile::{Profile, map};

use crate::models;
use crate::station::hid::{Link, Node, Reports};
use crate::station::{
    Choice, Device, Error, Family, HardwareSettings, Info, Input, LiveInput, Model, ProfileStore, Settings, Toggle,
};

/// The controller family for the station.
pub const PAD_FAMILY: Family = Family { claims, open };

/// Flydigi's USB vendor id. Only the `5a a5` generation carries it, older pads are plain XInput.
pub const VENDOR: u16 = 0x37d7;
/// The product id's top nibble tells kinds of device apart, as Space Station's HID managers do:
/// 2 for controllers, 6 for charging docks.
const PAD_NIBBLE: u16 = 2;
const DOCK_NIBBLE: u16 = 6;

/// Every request but the flash commit is answered within this time.
const REPLY_WAIT: Duration = Duration::from_millis(500);
/// The SDK gives the flash write and the reset 10 s.
const FLASH_WAIT: Duration = Duration::from_secs(10);
/// A device that does not answer its first heartbeat within this time is not taken.
const PROBE_WAIT: Duration = Duration::from_millis(300);
/// A bad packet is answered with silence, so a missing reply is retried.
const TRIES: usize = 3;

/// A controller's command interface: Flydigi's ids and the vendor collection, which keeps its
/// keyboard and mouse interface out. The model is told by the id the controller reports.
fn claims(node: &Node) -> bool {
    node.vendor == VENDOR && node.product >> 12 == PAD_NIBBLE && node.reports.is_some()
}

fn open(node: &Node) -> Result<Box<dyn Device>, Error> {
    let reports = node.reports.ok_or_else(|| Error::Silent(node.path.clone()))?;
    FlydigiDevice::open(&node.path, reports).map(|d| Box::new(d) as Box<dyn Device>)
}

/// A connected device of the Space Station family.
pub struct FlydigiDevice {
    path: PathBuf,
    model: &'static Model,
    link: Link,
    input: Arc<Mutex<Option<Input>>>,
}

impl FlydigiDevice {
    /// Opens the node and asks the device what it is.
    ///
    /// # Errors
    /// [`Error::Access`] when the node cannot be opened, [`Error::Silent`] when nothing answers:
    /// a receiver without its pad, or a second pad that hung.
    pub fn open(path: &Path, reports: Reports) -> Result<FlydigiDevice, Error> {
        let input = Arc::new(Mutex::new(None));
        let sink = input.clone();
        let link = Link::open(path, reports, move |p| match parse::input(p) {
            Some(i) => {
                *sink.lock().unwrap_or_else(PoisonError::into_inner) = Some(i);
                true
            }
            None => false,
        })?;
        let mut device = FlydigiDevice { path: path.to_path_buf(), model: &models::UNKNOWN, link, input };
        let hello = request(cmd::HEARTBEAT, &[]);
        let reply = device.exchange(&hello, PROBE_WAIT, 1).map_err(|_| Error::Silent(path.to_path_buf()))?;
        device.model = models::pad(parse::info(&reply).device_id);
        Ok(device)
    }

    /// Sends `req` and waits for the reply with the same command id, retrying `tries` times.
    fn exchange(&self, req: &[u8; PKT], wait: Duration, tries: usize) -> Result<Vec<u8>, Error> {
        for _ in 0..tries {
            self.link.send(req)?;
            if let Some(reply) = self.link.reply(wait, |p| frame::is_reply(p, req[2])) {
                return Ok(reply);
            }
        }
        Err(Error::NoReply(req[2]))
    }

    fn call(&self, req: &[u8; PKT]) -> Result<Vec<u8>, Error> {
        self.exchange(req, REPLY_WAIT, TRIES)
    }

    /// Reads a stored blob, as many packets as the device says it has.
    fn read_blob(&self, command: u8, slot: u8) -> Result<Vec<u8>, Error> {
        for _ in 0..TRIES {
            self.link.send(&request(command, &[slot, map::PKG as u8]))?;
            if let Some(blob) = self.collect_blob(command) {
                return Ok(blob);
            }
        }
        Err(Error::NoReply(command))
    }

    fn collect_blob(&self, command: u8) -> Option<Vec<u8>> {
        let (mut blob, mut got) = (vec![], 0);
        while let Some(r) = self.link.reply(REPLY_WAIT, |p| frame::is_reply(p, command)) {
            let (total, idx) = (r[3] as usize, r[4] as usize);
            if idx >= total || (got > 0 && blob.len() != total * map::PKG) {
                return None;
            }
            blob.resize(total * map::PKG, 0);
            blob[idx * map::PKG..][..map::PKG].copy_from_slice(&r[6..6 + map::PKG]);
            got += 1;
            if got == total {
                return Some(blob);
            }
        }
        None
    }

    /// Sends the packets that differ from `old`: a start command with the span, then numbered packets.
    fn write_blob(&self, start_cmd: u8, pack_cmd: u8, slot: u8, old: &[u8], new: &[u8]) -> Result<(), Error> {
        let Some((start, count)) = map::changed_span(old, new) else {
            return Ok(());
        };
        self.call(&request(start_cmd, &[slot, start as u8, count as u8, map::PKG as u8]))?;
        for i in 0..count {
            let mut pack = [0u8; map::PKG + 1];
            pack[0] = i as u8;
            pack[1..].copy_from_slice(&new[(start + i) * map::PKG..][..map::PKG]);
            self.call(&request(pack_cmd, &pack))?;
        }
        Ok(())
    }
}

impl Device for FlydigiDevice {
    fn path(&self) -> &Path {
        &self.path
    }

    fn model(&self) -> &'static Model {
        self.model
    }

    fn is_alive(&self) -> bool {
        self.link.is_alive()
    }

    fn info(&mut self) -> Result<Info, Error> {
        self.call(&request(cmd::HEARTBEAT, &[])).map(|r| parse::info(&r))
    }

    fn nickname(&mut self) -> Result<String, Error> {
        self.call(&request(cmd::NICKNAME, &[])).map(|r| parse::nickname(&r))
    }

    fn set_nickname(&mut self, name: &str) -> Result<(), Error> {
        self.call(&parse::nickname_request(name)).map(drop)
    }

    fn settings(&mut self) -> Option<&mut dyn HardwareSettings> {
        self.model.settings.then_some(self as &mut dyn HardwareSettings)
    }

    fn profiles(&mut self) -> Option<&mut dyn ProfileStore> {
        (self.model.profile_slots > 0).then_some(self as &mut dyn ProfileStore)
    }

    fn live(&mut self) -> Option<&mut dyn LiveInput> {
        self.model.live_input.then_some(self as &mut dyn LiveInput)
    }
}

impl HardwareSettings for FlydigiDevice {
    fn read(&mut self) -> Result<Settings, Error> {
        self.call(&request(cmd::HW_STATUS, &[])).map(|r| parse::settings(&r))
    }

    fn set_toggle(&mut self, toggle: Toggle, on: bool) -> Result<(), Error> {
        let req = parse::toggle(toggle, on).ok_or(Error::Unsupported)?;
        self.call(&req).map(drop)
    }

    fn set_choice(&mut self, choice: Choice, value: u8) -> Result<(), Error> {
        self.call(&parse::choice(choice, value)).map(drop)
    }
}

impl ProfileStore for FlydigiDevice {
    fn active(&mut self) -> Result<u8, Error> {
        self.call(&request(cmd::PROFILE_INFO, &[])).map(|r| parse::active_profile(&r))
    }

    fn activate(&mut self, slot: u8) -> Result<(), Error> {
        self.call(&request(cmd::APPLY_PROFILE, &[slot])).map(drop)
    }

    /// The pad switches to the slot while answering.
    fn read(&mut self, slot: u8) -> Result<Profile, Error> {
        let map = self.read_blob(cmd::MAP_READ, slot)?;
        if map.len() != map::SIZE {
            return Err(Error::NoReply(cmd::MAP_READ));
        }
        // protocol 3.1 keeps the macros inside the mapping blob and has no store to read
        let macros = if self.model.macro_store { self.read_blob(cmd::MACRO_READ, slot)? } else { vec![] };
        let rgb = self.read_blob(cmd::RGB_READ, slot)?;
        Ok(Profile { map, macros, rgb })
    }

    /// Same order as Space Station (mapping, macros, lighting), then the running profile is committed
    /// to flash and applied, since macros only start playing after an apply.
    fn write(&mut self, slot: u8, old: &Profile, new: &Profile) -> Result<(), Error> {
        self.write_blob(cmd::MAP_WRITE_START, cmd::MAP_WRITE_PACK, slot, &old.map, &new.map)?;
        if self.model.macro_store {
            self.write_blob(cmd::MACRO_WRITE_START, cmd::MACRO_WRITE_PACK, slot, &old.macros, &new.macros)?;
        }
        self.write_blob(cmd::RGB_WRITE_START, cmd::RGB_WRITE_PACK, slot, &old.rgb, &new.rgb)?;
        let version = map::version(&new.map).to_le_bytes();
        self.exchange(&request(cmd::MAP_SAVE, &version), FLASH_WAIT, TRIES)?;
        self.activate(slot)
    }

    /// Space Station's "Restore default" (ResetMappingConfigByCfgIdCommand). Measured on a Vader 5 Pro:
    /// whatever profile it names, **all four** profiles go back to factory.
    fn reset_all(&mut self) -> Result<(), Error> {
        self.exchange(&request(cmd::MAP_RESET, &[0]), FLASH_WAIT, TRIES).map(drop)
    }
}

impl LiveInput for FlydigiDevice {
    fn stream(&mut self, on: bool) -> Result<(), Error> {
        self.call(&request(cmd::TEST_MODE, &[0xff, on as u8, 0xff, 0xff, 0xff])).map(drop)
    }

    fn input(&self) -> Option<Input> {
        *self.input.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn rumble(&mut self, motors: [u8; 4]) -> Result<(), Error> {
        self.call(&request(cmd::VIBRATION, &motors)).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flydigi::profile::{gyro, macros, rgb, stick, trigger, vibration};

    fn readback_matches(device: &mut FlydigiDevice, want: &Profile) -> Result<(), String> {
        let back = ProfileStore::read(device, 3).map_err(|e| e.to_string())?;
        let diff = |a: &[u8], b: &[u8]| -> Vec<(usize, u8, u8)> {
            a.iter().zip(b).enumerate().filter(|(_, (x, y))| x != y).map(|(i, (x, y))| (i, *x, *y)).collect()
        };
        match (diff(&want.map, &back.map), diff(&want.macros, &back.macros)) {
            (m, k) if m.is_empty() && k.is_empty() && back.macros.len() == want.macros.len() => Ok(()),
            (m, k) => Err(format!("readback differs (index, wanted, read): map {m:?} macros {k:?}")),
        }
    }

    /// Needs a wired Vader 5 Pro: `cargo test -- --ignored`. Profile 4 is changed and restored.
    #[test]
    #[ignore]
    fn profile_round_trips_on_hardware() {
        let node = crate::station::hid::nodes().into_iter().find(claims).expect("a Flydigi device is connected");
        let reports = node.reports.expect("a claimed node has vendor reports");
        let mut d = FlydigiDevice::open(&node.path, reports).expect("the device answers");
        let active = ProfileStore::active(&mut d).expect("the active slot");
        let orig = ProfileStore::read(&mut d, 3).expect("slot 4 reads");
        let mut new = orig.clone();
        map::set_key(&mut new.map, 4, map::Key { target: map::IDENTITY, turbo: 1, hz: 20 });
        let left = stick::get(&orig.map, 0);
        stick::set(&mut new.map, 0, stick::Stick { kind: 1, center: 20, edge: 10, p1: (64, 96), ..left });
        trigger::set(&mut new.map, 1, 30, 200);
        let g = gyro::get(&orig.map);
        gyro::set(&mut new.map, gyro::Gyro { target: 2, key: 10, enable: 1, sensitivity: 50, ..g });
        let v = vibration::get(&orig.map);
        vibration::set(&mut new.map, vibration::Vibration { grip_strength: 70, trig_on: !v.trig_on, ..v });
        rgb::apply(&mut new.rgb, 1, &[[255, 0, 0], [0, 0, 255]]);
        map::set_version(&mut new.map, 0x1234);
        map::set_title(&mut new.map, "Test");
        let tap = vec![
            macros::Step { delay_ms: 0, key: 7, event: 1 },
            macros::Step { delay_ms: 40, key: 7, event: 0 },
            macros::Step { delay_ms: 20, key: 161, event: 3 },
            macros::Step { delay_ms: 200, key: 160, event: 3 },
        ];
        let list = [macros::Macro { key: 18, kind: 1, interval_ms: 0, steps: tap }];
        new.macros = macros::encode(&orig.macros, &list).expect("one macro fits");
        profile::bind_macros(&mut new.map, d.model().keys, &list).expect("one macro per key");
        let written = ProfileStore::write(&mut d, 3, &orig, &new)
            .map_err(|e| e.to_string())
            .and_then(|()| readback_matches(&mut d, &new));
        let restored = ProfileStore::write(&mut d, 3, &new, &orig)
            .map_err(|e| e.to_string())
            .and_then(|()| readback_matches(&mut d, &orig));
        ProfileStore::activate(&mut d, active).expect("the active slot comes back");
        written.expect("the changed profile reads back");
        restored.expect("the original profile reads back");
    }
}
