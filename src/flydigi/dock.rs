//! Flydigi's charging docks (CD2). The `5a a5` envelope on an unnumbered 64-byte report, with a
//! command set of its own: a heartbeat with four switches, a name, and a status report pushed about
//! once a second that says whether a controller is seated and how charged it is.
//! Facts from openflydigi's measurements of a CD2 on firmware 0.0.3.9. Its lighting is
//! [`super::dock_led`].

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use super::dock_led::LedConfig;
use super::frame::{PKT, request_long};
use super::{DOCK_NIBBLE, VENDOR};
use crate::models;
use crate::station::hid::{Link, Node, Reports};
use crate::station::{
    Battery, Choice, Device, DockLighting, Error, Family, HardwareSettings, Info, Model, Settings, Toggle,
};

/// The dock family for the station.
pub const FAMILY: Family = Family { claims, open };

mod cmd {
    pub const HEARTBEAT: u8 = 1;
    pub const NICKNAME: u8 = 2;
    pub const LIGHTS_OFF_WHILE_CHARGING: u8 = 17;
    pub const LIGHTING_SYNC: u8 = 18;
    pub const OFF_WITH_COMPUTER: u8 = 19;
    pub const READ_LED: u8 = 20;
    pub const SET_NICKNAME: u8 = 24;
    pub const CHARGE_ANIMATION: u8 = 25;
    pub const WRITE_LED_START: u8 = 97;
    pub const WRITE_LED_PACK: u8 = 98;
    /// Pushed by the dock, never asked for.
    pub const STATUS: u8 = 239;
}

/// One unnumbered 64-byte report each way, as the CD2 declares; also used when a dock's descriptor
/// could not be read, since Flydigi's own dock manager does not look at it.
const REPORTS: Reports = Reports { output: 0, output_len: 64, input: 0 };
const REPLY_WAIT: Duration = Duration::from_millis(500);
const PROBE_WAIT: Duration = Duration::from_millis(300);
const TRIES: usize = 3;
/// Name bytes a request has room for.
const NICKNAME_BYTES: usize = PKT - 6;
/// Battery byte of the status report from which the seated controller counts as charging.
const CHARGING_LEVEL: u8 = 6;
/// Battery levels a controller reports, 0..5.
const MAX_LEVEL: u8 = 5;
/// Data bytes per lighting pack; every pack waits for its own ack.
const PACK_BYTES: usize = 50;
/// First byte of the lighting write's start command, as Space Station sends it.
const LED_CONFIG_ID: u8 = 10;

fn claims(node: &Node) -> bool {
    node.vendor == VENDOR && node.product >> 12 == DOCK_NIBBLE
}

fn open(node: &Node) -> Result<Box<dyn Device>, Error> {
    DockDevice::open(&node.path, node.reports.unwrap_or(REPORTS)).map(|d| Box::new(d) as Box<dyn Device>)
}

/// A connected charging dock.
pub struct DockDevice {
    path: PathBuf,
    model: &'static Model,
    link: Link,
    /// The seated controller's charge, `None` while the dock is empty, from the last status report.
    seated: Arc<Mutex<Option<Battery>>>,
    /// A lighting write in progress: its packs and the next one to send.
    upload: Option<(Vec<Vec<u8>>, usize)>,
}

impl DockDevice {
    /// Opens the node and asks the dock what it is.
    ///
    /// # Errors
    /// [`Error::Access`] when the node cannot be opened, [`Error::Silent`] when nothing answers.
    pub fn open(path: &Path, reports: Reports) -> Result<DockDevice, Error> {
        let seated = Arc::new(Mutex::new(None));
        let sink = seated.clone();
        let link = Link::open(path, reports, move |p| match parse_status(p) {
            Some(s) => {
                *sink.lock().unwrap_or_else(PoisonError::into_inner) = s;
                true
            }
            None => false,
        })?;
        let mut dock =
            DockDevice { path: path.to_path_buf(), model: &models::UNKNOWN_DOCK, link, seated, upload: None };
        let reply = dock.exchange(cmd::HEARTBEAT, &[], PROBE_WAIT, 1).map_err(|_| Error::Silent(path.to_path_buf()))?;
        dock.model = models::dock(reply.get(6).copied().unwrap_or(u8::MAX));
        Ok(dock)
    }

    fn exchange(&self, command: u8, payload: &[u8], wait: Duration, tries: usize) -> Result<Vec<u8>, Error> {
        let req = request_long(command, payload);
        for _ in 0..tries {
            self.link.send(&req)?;
            // a dock's reply is as long as its payload, not a full report
            if let Some(reply) = self.link.reply(wait, |p| p.len() > 4 && p[..3] == [0x5a, 0xa5, command]) {
                return Ok(reply);
            }
        }
        Err(Error::NoReply(command))
    }

    fn call(&self, command: u8, payload: &[u8]) -> Result<Vec<u8>, Error> {
        self.exchange(command, payload, REPLY_WAIT, TRIES)
    }

    fn heartbeat(&self) -> Result<Heartbeat, Error> {
        let reply = self.call(cmd::HEARTBEAT, &[])?;
        parse_heartbeat(&reply).ok_or(Error::NoReply(cmd::HEARTBEAT))
    }
}

/// What the dock's heartbeat carries.
#[derive(Debug, PartialEq, Eq)]
struct Heartbeat {
    charger_type: u8,
    firmware: String,
    /// Lights off while charging, lighting sync, off with the computer, charge animation.
    switches: [bool; 4],
}

/// Offsets are Space Station's `HeartBeatCommand.ParseAckData`; the firmware is packed nibbles.
fn parse_heartbeat(p: &[u8]) -> Option<Heartbeat> {
    let b = p.get(..22)?;
    Some(Heartbeat {
        charger_type: b[6],
        firmware: format!("{}.{}.{}.{}", b[16] >> 4, b[16] & 15, b[17] >> 4, b[17] & 15),
        switches: [b[18] == 1, b[19] == 1, b[20] == 1, b[21] == 1],
    })
}

/// The pushed status: `Some(None)` for an empty dock, `Some(Some(_))` with the seated controller's
/// charge, `None` for any other packet.
fn parse_status(p: &[u8]) -> Option<Option<Battery>> {
    let b = p.get(..9).filter(|b| b[..3] == [0x5a, 0xa5, cmd::STATUS])?;
    let level = b[8];
    Some((b[7] != 0).then_some(Battery { level: level.min(MAX_LEVEL), charging: level >= CHARGING_LEVEL }))
}

/// The name sits at 6 and runs `len - 4` bytes; a first byte of 0 or ff means none.
fn parse_nickname(p: &[u8]) -> String {
    let Some(&len) = p.get(3) else { return String::new() };
    let raw = p.get(6..6 + usize::from(len.saturating_sub(4))).unwrap_or_default();
    match raw.first() {
        None | Some(0 | 0xff) => String::new(),
        Some(_) => String::from_utf8_lossy(raw).trim().to_string(),
    }
}

/// The request bytes of a name, cut to whole characters within the room a request has.
fn nickname_bytes(name: &str) -> Vec<u8> {
    let mut out = vec![];
    for c in name.chars() {
        let mut buf = [0; 4];
        let enc = c.encode_utf8(&mut buf).as_bytes();
        if out.len() + enc.len() > NICKNAME_BYTES {
            break;
        }
        out.extend_from_slice(enc);
    }
    out
}

impl Device for DockDevice {
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
        let hb = self.heartbeat()?;
        let seated = *self.seated.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(Info { device_id: hb.charger_type, firmware: hb.firmware, radio: None, battery: None, docked: Some(seated) })
    }

    fn nickname(&mut self) -> Result<String, Error> {
        self.call(cmd::NICKNAME, &[]).map(|r| parse_nickname(&r))
    }

    /// Never tried on a dock by openflydigi; the request is framed as every other one.
    fn set_nickname(&mut self, name: &str) -> Result<(), Error> {
        let bytes = nickname_bytes(name);
        if bytes.is_empty() {
            return Err(Error::EmptyName);
        }
        self.call(cmd::SET_NICKNAME, &bytes).map(drop)
    }

    fn settings(&mut self) -> Option<&mut dyn HardwareSettings> {
        self.model.settings.then_some(self as &mut dyn HardwareSettings)
    }

    fn dock_lighting(&mut self) -> Option<&mut dyn DockLighting> {
        self.model.features.lighting.then_some(self as &mut dyn DockLighting)
    }
}

impl DockLighting for DockDevice {
    fn read_lighting(&mut self) -> Result<LedConfig, Error> {
        let reply = self.call(cmd::READ_LED, &[])?;
        LedConfig::from_header(&reply).ok_or(Error::NoReply(cmd::READ_LED))
    }

    /// The pack count is the real one; Space Station advertises one more when the data divides by 50.
    fn start_lighting(&mut self, config: &LedConfig) -> Result<(), Error> {
        let packs: Vec<Vec<u8>> = config.serialise().chunks(PACK_BYTES).map(<[u8]>::to_vec).collect();
        let [hi, lo] = (packs.len() as u16).to_be_bytes();
        self.call(cmd::WRITE_LED_START, &[LED_CONFIG_ID, 0, 0, hi, lo])?;
        self.upload = Some((packs, 0));
        Ok(())
    }

    fn step_lighting(&mut self, budget: Duration) -> Result<f32, Error> {
        let Some((packs, next)) = self.upload.as_mut() else { return Ok(1.0) };
        let deadline = Instant::now() + budget;
        let total = packs.len();
        let [hi, lo] = (total as u16).to_be_bytes();
        while *next < total && Instant::now() < deadline {
            let [ihi, ilo] = (*next as u16).to_be_bytes();
            let pack = &packs[*next];
            let mut payload = vec![hi, lo, ihi, ilo, pack.len() as u8];
            payload.extend_from_slice(pack);
            let req = request_long(cmd::WRITE_LED_PACK, &payload);
            let mut acked = false;
            for _ in 0..TRIES {
                self.link.send(&req)?;
                if self.link.reply(REPLY_WAIT, |p| p.len() > 4 && p[..3] == [0x5a, 0xa5, cmd::WRITE_LED_PACK]).is_some()
                {
                    acked = true;
                    break;
                }
            }
            if !acked {
                // the dock's frame memory now holds part of an animation; a whole new write fixes it
                self.upload = None;
                return Err(Error::NoReply(cmd::WRITE_LED_PACK));
            }
            *next += 1;
        }
        let done = *next as f32 / total.max(1) as f32;
        if *next >= total {
            self.upload = None;
        }
        Ok(done)
    }
}

impl HardwareSettings for DockDevice {
    fn read(&mut self) -> Result<Settings, Error> {
        let hb = self.heartbeat()?;
        Ok(Settings { toggles: Toggle::DOCK.into_iter().zip(hb.switches).collect(), choices: vec![] })
    }

    fn set_toggle(&mut self, toggle: Toggle, on: bool) -> Result<(), Error> {
        let command = match toggle {
            Toggle::DockLightsOffWhileCharging => cmd::LIGHTS_OFF_WHILE_CHARGING,
            Toggle::DockLightingSync => cmd::LIGHTING_SYNC,
            Toggle::DockOffWithComputer => cmd::OFF_WITH_COMPUTER,
            Toggle::DockChargeAnimation => cmd::CHARGE_ANIMATION,
            _ => return Err(Error::Unsupported),
        };
        self.call(command, &[u8::from(on)]).map(drop)
    }

    fn set_choice(&mut self, _choice: Choice, _value: u8) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_reads_type_firmware_and_switches() {
        let mut p = vec![0u8; 32];
        p[..4].copy_from_slice(&[0x5a, 0xa5, 1, 22]);
        p[6] = 1;
        (p[16], p[17]) = (0x03, 0x09);
        p[18..22].copy_from_slice(&[1, 0, 1, 0]);
        let hb = parse_heartbeat(&p).expect("a full heartbeat");
        assert_eq!((hb.charger_type, hb.firmware.as_str()), (1, "0.3.0.9"));
        assert_eq!(hb.switches, [true, false, true, false]);
    }

    #[test]
    fn an_unnamed_dock_has_no_name() {
        // the unset reply openflydigi measured
        assert_eq!(parse_nickname(&[0x5a, 0xa5, 2, 4, 1, 0, 7]), "");
    }

    #[test]
    fn a_name_runs_up_to_its_checksum() {
        let mut p = vec![0x5a, 0xa5, 2, 8, 1, 0];
        p.extend_from_slice(b"Desk");
        p.push(0x5e);
        assert_eq!(parse_nickname(&p), "Desk");
    }

    #[test]
    fn status_tells_an_empty_dock_from_a_charging_controller() {
        assert_eq!(parse_status(&[0x5a, 0xa5, 239, 0, 0, 0, 0, 0, 0]), Some(None));
        let seated = parse_status(&[0x5a, 0xa5, 239, 0, 0, 0, 0, 1, 6]);
        assert_eq!(seated, Some(Some(Battery { level: 5, charging: true })));
        assert_eq!(parse_status(&[0x5a, 0xa5, 1, 0, 0, 0, 0, 1, 6]), None);
    }

    #[test]
    fn long_names_are_cut_at_a_character() {
        assert_eq!(nickname_bytes(&"я".repeat(20)).len(), NICKNAME_BYTES);
    }
}
