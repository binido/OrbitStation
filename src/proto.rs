//! Flydigi "NewXInput" protocol, used by Vader 5 Pro on the vendor HID interface (interface 1).
//! Request: `5a a5 CMD LEN PAYLOAD.. SUM`, LEN = 2 + payload length, SUM = low byte of CMD + LEN + payload.
//! Reply: `5a a5 CMD TOTAL INDEX DATA..`, 32 bytes.

pub const PKT: usize = 32;

pub mod cmd {
    pub const HEARTBEAT: u8 = 0x01;
    pub const HW_STATUS: u8 = 0x03;
    pub const TEST_MODE: u8 = 0x11;
    pub const VIBRATION: u8 = 0x12;
    pub const TOGGLE: u8 = 0x13;
    pub const REPORT_RATE: u8 = 0x14;
    pub const PRECISION: u8 = 0x15;
    pub const SENSITIVITY: u8 = 0x16;
    pub const SLEEP_TIME: u8 = 0x17;
    pub const PROFILE_INFO: u8 = 0xa1;
    pub const APPLY_PROFILE: u8 = 0xa2;
    pub const MAP_READ: u8 = 0xa3;
    pub const MAP_WRITE_START: u8 = 0xa4;
    pub const MAP_WRITE_PACK: u8 = 0xa5;
    pub const MAP_SAVE: u8 = 0xa6;
    pub const EXT_INPUT: u8 = 0xef;
}

/// Profile mapping blob: 42 packets of 20 bytes, layout from MappingConfigParserV30.
pub mod map {
    pub const PKG: usize = 20;
    pub const SIZE: usize = 840;
    const OFF_KEYS: usize = 13;
    const OFF_VERSION: usize = 225;
    /// Key table target meaning "the button does what it says on the shell".
    pub const IDENTITY: u8 = 255;

    /// Remappable Vader 5 Pro buttons, ids from the renderer's device_config_f5.
    pub const KEYS: [(u8, &str); 25] = [
        (4, "A"), (5, "B"), (7, "X"), (8, "Y"),
        (0, "↑"), (2, "↓"), (3, "←"), (1, "→"),
        (10, "LB"), (11, "RB"), (12, "LT"), (13, "RT"),
        (16, "C"), (17, "Z"), (14, "L3"), (15, "R3"),
        (6, "Select"), (9, "Start"), (27, "Home"),
        (18, "M1"), (19, "M2"), (20, "M3"), (21, "M4"), (22, "LM"), (23, "RM"),
    ];

    /// What a button may send: only XInput buttons, the extra ones are invisible to the host.
    pub const TARGETS: [(u8, &str); 18] = [
        (IDENTITY, "По умолчанию"),
        (4, "A"), (5, "B"), (7, "X"), (8, "Y"),
        (0, "↑"), (2, "↓"), (3, "←"), (1, "→"),
        (10, "LB"), (11, "RB"), (12, "LT"), (13, "RT"),
        (14, "L3"), (15, "R3"), (6, "Select"), (9, "Start"), (27, "Home"),
    ];

    pub fn target(blob: &[u8], key: u8) -> u8 {
        blob[OFF_KEYS + key as usize * 3]
    }

    /// Only the target byte changes, turbo settings of the key stay as they are.
    pub fn set_target(blob: &mut [u8], key: u8, target: u8) {
        blob[OFF_KEYS + key as usize * 3] = target;
    }

    /// Change tag other apps compare to detect an edited slot, 0xffff means never written.
    pub fn version(blob: &[u8]) -> u16 {
        u16::from_le_bytes([blob[OFF_VERSION], blob[OFF_VERSION + 1]])
    }

    pub fn set_version(blob: &mut [u8], v: u16) {
        blob[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&v.to_le_bytes());
    }

    /// First changed packet and how many follow it, the official app also resends only the changed span.
    pub fn changed_span(old: &[u8], new: &[u8]) -> Option<(usize, usize)> {
        let differs = |i: usize| old[i * PKG..(i + 1) * PKG] != new[i * PKG..(i + 1) * PKG];
        let first = (0..SIZE / PKG).find(|&i| differs(i))?;
        let last = (0..SIZE / PKG).rev().find(|&i| differs(i))?;
        Some((first, last - first + 1))
    }
}

pub fn request(cmd: u8, payload: &[u8]) -> [u8; PKT] {
    let mut p = [0u8; PKT];
    p[0] = 0x5a;
    p[1] = 0xa5;
    p[2] = cmd;
    p[3] = 2 + payload.len() as u8;
    p[4..4 + payload.len()].copy_from_slice(payload);
    p[4 + payload.len()] = p[2..4 + payload.len()].iter().fold(0u8, |s, b| s.wrapping_add(*b));
    p
}

pub fn is_reply(p: &[u8], cmd: u8) -> bool {
    p.len() >= 5 && p[0] == 0x5a && p[1] == 0xa5 && p[2] == cmd
}

/// Switches stored as bits in the 0x03 reply, the bit number is the 0x13 sub-command minus one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Toggle {
    QuickSwitchConfig = 1,
    XboxHomeButton = 2,
    MotionDebounce = 3,
    MappingSwitch = 4,
    JoystickDebounce = 5,
    JoystickAutoCalibration = 6,
    JoystickRebound = 7,
    ScreenStatusBar = 8,
    Audio = 10,
}

impl Toggle {
    pub const ALL: [Toggle; 9] = [
        Toggle::JoystickAutoCalibration,
        Toggle::JoystickDebounce,
        Toggle::JoystickRebound,
        Toggle::MotionDebounce,
        Toggle::QuickSwitchConfig,
        Toggle::MappingSwitch,
        Toggle::XboxHomeButton,
        Toggle::ScreenStatusBar,
        Toggle::Audio,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Toggle::QuickSwitchConfig => "Быстрое переключение профилей",
            Toggle::XboxHomeButton => "Кнопка Home как Xbox Guide",
            Toggle::MotionDebounce => "Подавление дрожания гироскопа",
            Toggle::MappingSwitch => "Переключение раскладки",
            Toggle::JoystickDebounce => "Подавление дрожания стиков",
            Toggle::JoystickAutoCalibration => "Автокалибровка стиков",
            Toggle::JoystickRebound => "Алгоритм возврата стиков",
            Toggle::ScreenStatusBar => "Строка состояния на экране всегда",
            Toggle::Audio => "Аудио через геймпад",
        }
    }

    pub fn packet(self, on: bool) -> [u8; PKT] {
        request(cmd::TOGGLE, &[self as u8, on as u8])
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct DeviceInfo {
    pub device_id: u8,
    pub connect_type: u8,
    pub battery: u8,
    pub charging: bool,
    pub firmware: String,
    pub nearlink: String,
}

fn version(hi: u8, lo: u8) -> String {
    format!("{}.{}.{}.{}", hi >> 4, hi & 15, lo >> 4, lo & 15)
}

/// Data starts at 5 for multi-packet replies and at 4 for the last packet, see HeartBeatControllerCommandNewXInput.ParseAckData.
fn data(p: &[u8]) -> &[u8] {
    if p[4] < p[3] { &p[5..] } else { &p[4..] }
}

pub fn parse_heartbeat(p: &[u8]) -> DeviceInfo {
    let d = data(p);
    let bat = d[6];
    DeviceInfo {
        device_id: d[0],
        connect_type: d[1],
        charging: bat >> 4 == 1,
        battery: bat & 15,
        firmware: version(d[10], d[11]),
        nearlink: version(d[22], d[23]),
    }
}

pub fn model_name(device_id: u8) -> &'static str {
    match device_id {
        130 | 144 | 145 => "Flydigi Vader 5 Pro",
        _ => "Неизвестный Flydigi",
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct HwStatus {
    pub usable: u16,
    pub enabled: u16,
    pub sleep_time: u8,
    pub report_rate: u8,
    pub precision: u8,
    pub sensitivity: u8,
}

impl HwStatus {
    fn bit(t: Toggle) -> u16 {
        if t == Toggle::Audio { 1 << 8 } else { 1 << (t as u8 - 1) }
    }
    pub fn usable(&self, t: Toggle) -> bool {
        self.usable & Self::bit(t) != 0
    }
    pub fn enabled(&self, t: Toggle) -> bool {
        self.enabled & Self::bit(t) != 0
    }
    pub fn set(&mut self, t: Toggle, on: bool) {
        if on { self.enabled |= Self::bit(t) } else { self.enabled &= !Self::bit(t) }
    }
}

pub fn parse_hw_status(p: &[u8]) -> HwStatus {
    HwStatus {
        usable: p[5] as u16 | ((p[7] as u16 & 1) << 8),
        enabled: p[6] as u16 | ((p[8] as u16 & 1) << 8),
        sleep_time: p[9],
        report_rate: p[10],
        precision: p[11],
        sensitivity: p[12],
    }
}

/// Active profile 0..=3, as ReadMappingConfigVersionAllCommandNewXInput reads it.
pub fn parse_profile(p: &[u8]) -> u8 {
    match p[5] {
        v @ 0..=3 => v,
        v @ 4..=7 => v - 4,
        _ => 0,
    }
}

pub const REPORT_RATES: [(u8, &str); 4] = [(1, "1000 Hz"), (2, "500 Hz"), (3, "250 Hz"), (4, "125 Hz")];
pub const SLEEP_TIMES: [(u8, &str); 6] =
    [(1, "1 мин"), (5, "5 мин"), (15, "15 мин"), (60, "1 час"), (180, "3 часа"), (0, "Никогда")];
pub const PRECISIONS: [(u8, &str); 7] =
    [(1, "8 bit"), (4, "9 bit"), (2, "10 bit"), (5, "11 bit"), (3, "12 bit"), (6, "14 bit"), (7, "16 bit")];
pub const SENSITIVITIES: [(u8, &str); 7] = [
    (14, "Самая высокая"),
    (15, "Высокая"),
    (16, "Выше средней"),
    (17, "Средняя"),
    (18, "Ниже средней"),
    (19, "Низкая"),
    (20, "Самая низкая"),
];

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Input {
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
    pub buttons: u32,
    pub lt: u8,
    pub rt: u8,
    pub gyro: [i16; 3],
    pub accel: [i16; 3],
}

/// Button names in bit order of `Input::buttons` (bytes 11, 12, 13, 14 of the 0xef report).
pub const BUTTONS: [&str; 32] = [
    "↑", "↓", "←", "→", "A", "B", "Select", "X", //
    "Y", "Start", "LB", "RB", "", "", "L3", "R3", //
    "C", "Z", "M1", "M2", "M3", "M4", "LM", "RM", //
    "O", "", "", "Home", "", "", "", "",
];

pub fn parse_input(p: &[u8]) -> Option<Input> {
    if p.len() < 29 || !is_reply(p, cmd::EXT_INPUT) {
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
        s.split(' ').map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn builds_requests_like_the_official_app() {
        assert_eq!(request(cmd::HEARTBEAT, &[])[..5], [0x5a, 0xa5, 0x01, 0x02, 0x03]);
        assert_eq!(request(cmd::TEST_MODE, &[0xff, 1, 0xff, 0xff, 0xff])[..10], hex("5a a5 11 07 ff 01 ff ff ff 15")[..]);
        assert_eq!(Toggle::JoystickDebounce.packet(true)[..7], [0x5a, 0xa5, 0x13, 0x04, 0x05, 0x01, 0x1d]);
    }

    #[test]
    fn parses_real_vader5_replies() {
        let hb = hex("5a a5 01 01 00 82 01 00 00 00 00 13 45 01 00 71 50 00 00 35 18 00 00 00 00 00 00 10 26 1f 00 41");
        let info = parse_heartbeat(&hb);
        assert_eq!((info.device_id, info.battery, info.charging), (0x82, 3, true));
        assert_eq!((info.firmware.as_str(), info.nearlink.as_str()), ("7.1.5.0", "1.0.2.6"));

        let st = parse_hw_status(&hex("5a a5 03 01 00 fb 72 01 00 0f 00 02 11 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 94"));
        assert!(st.usable(Toggle::JoystickRebound) && st.enabled(Toggle::JoystickRebound));
        assert!(!st.usable(Toggle::MotionDebounce) && st.usable(Toggle::MappingSwitch) && st.usable(Toggle::Audio) && !st.enabled(Toggle::Audio));
        assert_eq!((st.sleep_time, st.precision, st.sensitivity), (15, 2, 17));

        let inp = parse_input(&hex("5a a5 ef 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 04 00 03 00 22 00 f1 ff 0a 10 00 00 23")).unwrap();
        assert_eq!(inp.accel, [0x22, -15, 4106]);
        assert_eq!(inp.gyro, [1, 4, 3]);
    }

    #[test]
    fn remaps_a_key_in_the_profile_blob() {
        let old = vec![map::IDENTITY; map::SIZE];
        let mut new = old.clone();
        assert_eq!(map::target(&new, 18), map::IDENTITY);
        map::set_target(&mut new, 18, 4);
        map::set_version(&mut new, 0x0840);
        // M1 sits at byte 13 + 18 * 3 = 67 (packet 3), the version at 225 (packet 11)
        assert_eq!((new[67], new[225], new[226]), (4, 0x40, 0x08));
        assert_eq!(map::changed_span(&old, &new), Some((3, 9)));
        assert_eq!(map::changed_span(&old, &old), None);
    }
}
