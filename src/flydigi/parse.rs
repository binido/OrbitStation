//! Replies of the Space Station protocol turned into the station's types, and the requests that
//! carry station values.

use super::frame::{PKT, cmd, request};
use crate::station::{Battery, Choice, Info, Input, Settings, Toggle};

/// Sub-command of 0x13 for a controller's toggle; its bit in the 0x03 reply is this minus one, the
/// upper ones in the second byte pair (Space Station's `ReadHardwareFunctionEnableStatus`). `None`
/// for a dock's toggle.
fn toggle_id(t: Toggle) -> Option<u8> {
    Some(match t {
        Toggle::QuickSwitchConfig => 1,
        Toggle::XboxHomeButton => 2,
        Toggle::MotionDebounce => 3,
        Toggle::MappingSwitch => 4,
        Toggle::JoystickDebounce => 5,
        Toggle::JoystickAutoCalibration => 6,
        Toggle::JoystickRebound => 7,
        Toggle::ScreenStatusBar => 8,
        Toggle::AlwaysOnDisplay => 9,
        Toggle::Audio => 10,
        _ => return None,
    })
}

fn toggle_bit(t: Toggle) -> u16 {
    toggle_id(t).map_or(0, |id| 1 << (id - 1))
}

/// Request that switches a controller's toggle, `None` for a toggle controllers do not have.
pub fn toggle(t: Toggle, on: bool) -> Option<[u8; PKT]> {
    Some(request(cmd::TOGGLE, &[toggle_id(t)?, u8::from(on)]))
}

/// Request that sets a choice.
pub fn choice(c: Choice, value: u8) -> [u8; PKT] {
    let id = match c {
        Choice::ReportRate => cmd::REPORT_RATE,
        Choice::Precision => cmd::PRECISION,
        Choice::Sensitivity => cmd::SENSITIVITY,
        Choice::SleepTime => cmd::SLEEP_TIME,
    };
    request(id, &[value])
}

fn version(hi: u8, lo: u8) -> String {
    format!("{}.{}.{}.{}", hi >> 4, hi & 15, lo >> 4, lo & 15)
}

/// Data starts at 5 for multi-packet replies and at 4 for the last packet, see HeartBeatControllerCommandNewXInput.ParseAckData.
fn data(p: &[u8]) -> &[u8] {
    if p[4] < p[3] { &p[5..] } else { &p[4..] }
}

/// The 0x01 reply.
pub fn info(p: &[u8]) -> Info {
    let d = data(p);
    let bat = d[6];
    Info {
        device_id: d[0],
        battery: Some(Battery { level: bat & 15, charging: bat >> 4 == 1 }),
        firmware: version(d[10], d[11]),
        radio: Some(version(d[22], d[23])),
        docked: None,
    }
}

/// The 0x03 reply: the toggles the device supports and the choices it reports.
pub fn settings(p: &[u8]) -> Settings {
    let usable = u16::from_le_bytes([p[5], p[7]]);
    let enabled = u16::from_le_bytes([p[6], p[8]]);
    let toggles = Toggle::PAD
        .into_iter()
        .filter(|t| usable & toggle_bit(*t) != 0)
        .map(|t| (t, enabled & toggle_bit(t) != 0))
        .collect();
    // 0 means the device has no such setting; sleep time 0 is "never" and always exists
    let choices = [(Choice::ReportRate, p[10]), (Choice::Precision, p[11]), (Choice::Sensitivity, p[12])]
        .into_iter()
        .filter(|c| c.1 != 0)
        .chain([(Choice::SleepTime, p[9])])
        .collect();
    Settings { toggles, choices }
}

/// Active profile 0..=3, as ReadMappingConfigVersionAllCommandNewXInput reads it.
pub fn active_profile(p: &[u8]) -> u8 {
    match p[5] {
        v @ 0..=3 => v,
        v @ 4..=7 => v - 4,
        _ => 0,
    }
}

/// The pad stores the payload of 0x18 as sent, checksum included, so the name ends with a 0.
const NICKNAME_BYTES: usize = 25;

/// Request that stores a nickname, cut to whole characters within 25 bytes.
pub fn nickname_request(name: &str) -> [u8; PKT] {
    let mut bytes: Vec<u8> = vec![];
    for c in name.chars() {
        let mut buf = [0; 4];
        let enc = c.encode_utf8(&mut buf).as_bytes();
        if bytes.len() + enc.len() > NICKNAME_BYTES {
            break;
        }
        bytes.extend_from_slice(enc);
    }
    // An empty name is written as all ff, the factory state; a 0 byte would be shown by Space Station.
    if bytes.is_empty() {
        return request(cmd::SET_NICKNAME, &[0xff; NICKNAME_BYTES + 1]);
    }
    bytes.push(0);
    request(cmd::SET_NICKNAME, &bytes)
}

/// The 0x02 reply.
pub fn nickname(p: &[u8]) -> String {
    let d = &p[5..31];
    let end = d.iter().position(|b| *b == 0 || *b == 0xff).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

/// An extended input report (0xef), sent while the test mode is on.
pub fn input(p: &[u8]) -> Option<Input> {
    if p.len() < 29 || p[..3] != [0x5a, 0xa5, cmd::EXT_INPUT] {
        return None;
    }
    let s = |i: usize| i16::from_le_bytes([p[i], p[i + 1]]);
    Some(Input {
        lx: s(3),
        ly: s(5),
        rx: s(7),
        ry: s(9),
        buttons: u32::from_le_bytes([p[11], p[12], p[13], p[14]]),
        lt: p[15],
        rt: p[16],
        gyro: [s(17), s(19), s(21)],
        accel: [s(23), s(25), s(27)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split(' ').map(|b| u8::from_str_radix(b, 16).expect("test data is hex")).collect()
    }

    #[test]
    fn heartbeat_of_a_vader_5_pro() {
        let hb = hex("5a a5 01 01 00 82 01 00 00 00 00 13 45 01 00 71 50 00 00 35 18 00 00 00 00 00 00 10 26 1f 00 41");
        let info = info(&hb);
        assert_eq!((info.device_id, info.battery), (0x82, Some(Battery { level: 3, charging: true })));
        assert_eq!((info.firmware.as_str(), info.radio.as_deref()), ("7.1.5.0", Some("1.0.2.6")));
    }

    #[test]
    fn heartbeat_of_an_apex_5_without_its_report_id() {
        // openflydigi's capture over the dongle, report id 04 stripped: 31 bytes
        let hb = hex("5a a5 01 01 00 80 02 00 00 00 00 05 45 01 00 70 45 21 31 45 25 00 00 01 28 00 00 11 31 1f 00");
        let info = info(&hb);
        assert_eq!((info.device_id, info.battery), (128, Some(Battery { level: 5, charging: false })));
        assert_eq!(info.firmware, "7.0.4.5");
    }

    #[test]
    fn settings_list_only_usable_toggles() {
        // a Vader 5 Pro: the second pair says always-on display, not audio, as Space Station reads it
        let st = settings(&hex(
            "5a a5 03 01 00 fb 72 01 00 0f 00 02 11 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 94",
        ));
        let has = |t| st.toggles.iter().find(|x| x.0 == t).map(|x| x.1);
        assert_eq!(has(Toggle::JoystickRebound), Some(true));
        assert_eq!(has(Toggle::MotionDebounce), None);
        assert_eq!(has(Toggle::AlwaysOnDisplay), Some(false));
        assert_eq!(has(Toggle::Audio), None);
        assert_eq!(st.choices, vec![(Choice::Precision, 2), (Choice::Sensitivity, 17), (Choice::SleepTime, 15)]);
    }

    #[test]
    fn apex_5_settings_with_its_screen() {
        // openflydigi's capture of a wired Apex 5
        let st = settings(&hex(
            "5a a5 03 01 00 fb 7b 01 00 0f 00 02 11 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
        ));
        let has = |t| st.toggles.iter().find(|x| x.0 == t).map(|x| x.1);
        assert_eq!(has(Toggle::ScreenStatusBar), Some(false));
        assert_eq!(has(Toggle::AlwaysOnDisplay), Some(false));
        assert_eq!(has(Toggle::QuickSwitchConfig), Some(true));
    }

    #[test]
    fn toggle_request_names_its_sub_command() {
        let req = toggle(Toggle::JoystickDebounce, true).expect("a controller toggle");
        assert_eq!(req[..7], [0x5a, 0xa5, 0x13, 0x04, 0x05, 0x01, 0x1d]);
        assert_eq!(toggle(Toggle::DockLightingSync, true), None);
    }

    #[test]
    fn input_report_carries_motion() {
        let p = hex("5a a5 ef 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 04 00 03 00 22 00 f1 ff 0a 10 00 00 23");
        let inp = input(&p).expect("an ef report");
        assert_eq!(inp.accel, [0x22, -15, 4106]);
        assert_eq!(inp.gyro, [1, 4, 3]);
    }

    #[test]
    fn nickname_round_trips_through_the_pad() {
        // what the pad answered after `nickname_request("Вейдер")`
        let p = nickname_request("Вейдер");
        assert_eq!(p[3] as usize, 2 + "Вейдер".len() + 1);
        let mut reply = [0u8; PKT];
        reply[..5].copy_from_slice(&[0x5a, 0xa5, 2, 1, 0]);
        reply[5..5 + 14].copy_from_slice(&p[4..4 + 14]);
        reply[19] = p[4 + 13];
        assert_eq!(nickname(&reply), "Вейдер");
    }

    #[test]
    fn empty_nickname_is_the_factory_state() {
        let empty = nickname_request("");
        assert_eq!((empty[3], empty[4], empty[29]), (28, 0xff, 0xff));
        let mut reply = [0u8; PKT];
        reply[5..31].fill(0xff);
        assert_eq!(nickname(&reply), "");
        assert!(nickname_request(&"я".repeat(20))[3] as usize <= 2 + NICKNAME_BYTES + 1);
    }
}
