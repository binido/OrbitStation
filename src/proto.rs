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
    pub const MACRO_READ: u8 = 0xac;
    pub const MACRO_WRITE_START: u8 = 0xad;
    pub const MACRO_WRITE_PACK: u8 = 0xae;
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
    /// Key table target meaning "the button plays its macro from the macro store".
    pub const MACRO: u8 = 32;

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

    /// Turbo modes, KeyMapTypeContinuousEnableType.
    pub const TURBO: [(u8, &str); 3] = [(0, "Выкл"), (1, "Пока зажата"), (2, "Вкл/выкл нажатием")];
    pub const TURBO_MAX_HZ: u8 = 30;

    /// One key table entry: what the key sends, turbo mode and turbo presses per second.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct Key {
        pub target: u8,
        pub turbo: u8,
        pub hz: u8,
    }

    pub fn key(blob: &[u8], id: u8) -> Key {
        let e = &blob[OFF_KEYS + id as usize * 3..][..3];
        // A turbo entry names the key itself, a plain one stores 255 for "unchanged".
        let target = if e[0] == id { IDENTITY } else { e[0] };
        if e[2] > 0 { Key { target, turbo: e[1], hz: e[2] } } else { Key { target, turbo: 0, hz: 0 } }
    }

    /// Turbo needs a real key id to repeat, so an unchanged key is written as itself.
    pub fn set_key(blob: &mut [u8], id: u8, k: Key) {
        let entry = if k.turbo > 0 && k.target != MACRO {
            let target = if k.target == IDENTITY { id } else { k.target };
            [target, k.turbo, k.hz.clamp(1, TURBO_MAX_HZ)]
        } else {
            [k.target, 0, 0]
        };
        blob[OFF_KEYS + id as usize * 3..][..3].copy_from_slice(&entry);
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
        let first = (0..new.len() / PKG).find(|&i| differs(i))?;
        let last = (0..new.len() / PKG).rev().find(|&i| differs(i))?;
        Some((first, last - first + 1))
    }
}

/// Stick response curves in the profile blob. The pad plays the 9-point table at 790
/// (value = output % + 50, at 0, 12.5 .. 100 % travel); the polyline at 109 is the source form
/// Space Station's editor reads back, so both are written together.
pub mod stick {
    const OFF_CURVE: usize = 109;
    const CURVE: usize = 7;
    const OFF_EXTRA: usize = 790;
    const EXTRA: usize = 12;

    /// JoystickSensitivityType, with the points each preset stands for.
    pub const PRESETS: [(u8, &str); 4] = [(0, "Линейная"), (1, "Быстрая"), (2, "Плавная"), (3, "Своя")];
    pub const SIDES: [&str; 2] = ["Левый стик", "Правый стик"];

    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct Stick {
        pub kind: u8,
        /// Inner dead zone, % of travel.
        pub center: u8,
        /// Outer dead zone, % of travel.
        pub edge: u8,
        /// Curve breakpoints on the 0..127 scale.
        pub p1: (u8, u8),
        pub p2: (u8, u8),
        pub circular: bool,
    }

    pub fn preset(kind: u8) -> Option<((u8, u8), (u8, u8))> {
        match kind {
            0 => Some(((63, 63), (127, 127))),
            1 => Some(((64, 96), (127, 127))),
            2 => Some(((64, 32), (127, 127))),
            _ => None,
        }
    }

    pub fn get(blob: &[u8], side: usize) -> Stick {
        let c = &blob[OFF_CURVE + side * CURVE..][..CURVE];
        let e = &blob[OFF_EXTRA + side * EXTRA..][..EXTRA];
        // Values over 100 are the "not a stick" sentinel or the negative half Flydigi's own reader
        // and writer disagree on; neither is an editable dead zone.
        let pct = |b: u8| if b <= 100 { b } else { 0 };
        Stick { kind: c[0], center: pct(c[1]), edge: pct(e[11]), p1: (c[2], c[3]), p2: (c[4], c[5]), circular: e[10] == 1 }
    }

    pub fn bank(blob: &[u8], side: usize) -> [u8; 9] {
        blob[OFF_EXTRA + side * EXTRA + 1..][..9].try_into().unwrap()
    }

    pub fn set(blob: &mut [u8], side: usize, s: Stick) {
        let c = &mut blob[OFF_CURVE + side * CURVE..][..CURVE];
        c[..6].copy_from_slice(&[s.kind, s.center, s.p1.0, s.p1.1, s.p2.0, s.p2.1]);
        let e = &mut blob[OFF_EXTRA + side * EXTRA..][..EXTRA];
        e[0] = s.kind;
        e[1..10].copy_from_slice(&compile(&s));
        e[10] = s.circular as u8;
        e[11] = s.edge;
    }

    /// Samples the polyline start, p1, p2, end. The breakpoints' x is squeezed into the span the
    /// dead zones leave, as Space Station's CalculatePoint does. Truncation matches a factory table.
    pub fn compile(s: &Stick) -> [u8; 9] {
        let (c, e) = (s.center as f64, s.edge.min(100 - s.center) as f64);
        let (start, end) = ((c, 0.0), (100.0 - e, 100.0));
        let span = end.0 - start.0;
        let inner = |p: (u8, u8)| (start.0 + span * p.0 as f64 / 127.0, p.1 as f64 * 100.0 / 127.0);
        let nodes = [start, inner(s.p1), inner(s.p2), end];
        let at = |x: f64| -> f64 {
            if span <= 0.0 {
                return if x < start.0 { 0.0 } else { 100.0 };
            }
            for w in 0..3 {
                let ((x0, y0), (x1, y1)) = (nodes[w], nodes[w + 1]);
                if x <= x1 || w == 2 {
                    return if x1 == x0 { if x < x0 { y0 } else { y1 } } else { y0 + (y1 - y0) * (x - x0) / (x1 - x0) };
                }
            }
            100.0
        };
        std::array::from_fn(|i| (at(12.5 * i as f64).clamp(-50.0, 100.0) as i32 + 50) as u8)
    }
}

/// Trigger travel window at 123, 7 bytes per trigger on a 0..255 scale:
/// type, start, p1 x, p1 y, p2 x, p2 y, end. The game still gets the full 0..255 range, squeezed
/// into the part of the travel between start and end. Space Station mirrors the points onto the window.
pub mod trigger {
    const OFF: usize = 123;
    const ENTRY: usize = 7;
    pub const SIDES: [&str; 2] = ["LT", "RT"];

    /// (start, end) of the travel, 0..255.
    pub fn get(blob: &[u8], side: usize) -> (u8, u8) {
        let e = &blob[OFF + side * ENTRY..][..ENTRY];
        (e[1], e[6])
    }

    pub fn set(blob: &mut [u8], side: usize, start: u8, end: u8) {
        let (lo, hi) = (start.min(end), start.max(end));
        blob[OFF + side * ENTRY + 1..][..6].copy_from_slice(&[lo, lo, lo, hi, hi, hi]);
    }
}

/// Macro store of protocol 3.2 (MacroConfigParserV10), one per profile. Vader 5 Pro answers 83 packets,
/// Space Station writes only the first 81 and the pad keeps the tail itself (the last 4 bytes stay 0).
/// Header: version u16, count u16, ten u16 body offsets in 4-byte words after the header.
/// Body: trigger key, step count u16, kind, repeat interval u16 (ms), 6 x ff, 20-byte name,
/// then 4 bytes per step: time since the macro started u16 (ms), key, event.
pub mod macros {
    const HEADER: usize = 24;
    const SLOTS: usize = 10;
    const BODY: usize = 32;
    /// GetPackageCount (81) x 20 bytes, what Space Station owns of the store.
    const OWNED: usize = 81 * 20;

    /// MacroEnableType.
    pub const KINDS: [(u8, &str); 3] = [(1, "Один раз"), (2, "Пока зажата"), (3, "Вкл/выкл нажатием")];
    /// MacroActionEvent values offered in the editor, 2/3 (sticks) and 5 (hold) are kept but not offered.
    pub const EVENTS: [(u8, &str); 2] = [(1, "Нажать"), (0, "Отпустить")];
    pub const MAX: usize = SLOTS;

    #[derive(Clone, PartialEq, Debug)]
    pub struct Step {
        pub delay_ms: u16,
        pub key: u8,
        pub event: u8,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct Macro {
        pub key: u8,
        pub kind: u8,
        pub interval_ms: u16,
        pub steps: Vec<Step>,
    }

    fn u16_at(b: &[u8], i: usize) -> u16 {
        u16::from_le_bytes([b[i], b[i + 1]])
    }

    pub fn parse(blob: &[u8]) -> Vec<Macro> {
        let count = u16_at(blob, 2) as usize;
        if count > SLOTS {
            return vec![];
        }
        let mut out = vec![];
        for i in 0..count {
            let p = HEADER + u16_at(blob, 4 + 2 * i) as usize * 4;
            let n = blob.get(p + 1..p + 3).map_or(0, |b| u16_at(b, 0) as usize);
            if p + BODY + 4 * n > blob.len() {
                break;
            }
            let mut prev = 0;
            let steps = (0..n)
                .map(|j| {
                    let s = &blob[p + BODY + 4 * j..][..4];
                    let t = u16_at(s, 0);
                    let step = Step { delay_ms: t.saturating_sub(prev), key: s[2], event: s[3] };
                    prev = t;
                    step
                })
                .collect();
            out.push(Macro { key: blob[p], kind: blob[p + 3], interval_ms: u16_at(blob, p + 4), steps });
        }
        out
    }

    /// Builds a store of the same size as `old`, keeping its version and the tail past 81 packets,
    /// unused bytes are ff like erased flash.
    pub fn encode(old: &[u8], list: &[Macro]) -> Result<Vec<u8>, String> {
        if list.len() > SLOTS {
            return Err(format!("не больше {SLOTS} макросов"));
        }
        let mut out = old.to_vec();
        let owned = OWNED.min(out.len());
        out[2..owned].fill(0xff);
        out[2..4].copy_from_slice(&(list.len() as u16).to_le_bytes());
        let mut p = HEADER;
        for (i, m) in list.iter().enumerate() {
            if p + BODY + 4 * m.steps.len() > owned {
                return Err("макросы не помещаются в память геймпада".into());
            }
            out[4 + 2 * i..][..2].copy_from_slice(&(((p - HEADER) / 4) as u16).to_le_bytes());
            out[p] = m.key;
            out[p + 1..p + 3].copy_from_slice(&(m.steps.len() as u16).to_le_bytes());
            out[p + 3] = m.kind;
            out[p + 4..p + 6].copy_from_slice(&m.interval_ms.to_le_bytes());
            let mut t = 0u16;
            for (j, s) in m.steps.iter().enumerate() {
                t = t.checked_add(s.delay_ms).ok_or("макрос длиннее 65 секунд")?;
                let at = p + BODY + 4 * j;
                out[at..at + 2].copy_from_slice(&t.to_le_bytes());
                out[at + 2] = s.key;
                out[at + 3] = s.event;
            }
            p += BODY + 4 * m.steps.len();
        }
        Ok(out)
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
        assert_eq!(map::key(&new, 18).target, map::IDENTITY);
        map::set_key(&mut new, 18, map::Key { target: 4, turbo: 0, hz: 0 });
        map::set_version(&mut new, 0x0840);
        // M1 sits at byte 13 + 18 * 3 = 67 (packet 3), the version at 225 (packet 11)
        assert_eq!((new[67], new[225], new[226]), (4, 0x40, 0x08));
        assert_eq!(map::changed_span(&old, &new), Some((3, 9)));
        assert_eq!(map::changed_span(&old, &old), None);
    }

    #[test]
    fn turbo_on_an_unchanged_key_names_the_key_itself() {
        let mut blob = vec![map::IDENTITY; map::SIZE];
        map::set_key(&mut blob, 4, map::Key { target: map::IDENTITY, turbo: 1, hz: 15 });
        assert_eq!(blob[13 + 12..13 + 15], [4, 1, 15]);
        assert_eq!(map::key(&blob, 4), map::Key { target: map::IDENTITY, turbo: 1, hz: 15 });
        map::set_key(&mut blob, 4, map::Key { target: map::IDENTITY, turbo: 0, hz: 15 });
        assert_eq!(blob[13 + 12..13 + 15], [255, 0, 0]);
    }

    #[test]
    fn reads_and_compiles_stick_curves() {
        let mut blob = vec![0u8; map::SIZE];
        // left stick of profile 1 as read from a Vader 5 Pro: 5 % dead zone, linear
        blob[109..116].copy_from_slice(&[0, 5, 63, 63, 127, 127, 127]);
        blob[790..802].copy_from_slice(&[0, 45, 58, 71, 84, 97, 110, 123, 137, 150, 0, 0]);
        let s = stick::get(&blob, 0);
        assert_eq!((s.kind, s.center, s.edge, s.p1, s.circular), (0, 5, 0, (63, 63), false));
        assert_eq!(stick::bank(&blob, 0), [45, 58, 71, 84, 97, 110, 123, 137, 150]);

        let linear = stick::Stick { center: 0, ..s };
        assert_eq!(stick::compile(&linear), [50, 62, 75, 87, 100, 112, 125, 137, 150]);
        let dead = stick::Stick { center: 20, edge: 20, ..s };
        let b = stick::compile(&dead);
        assert!(b[1] < 50 && b[7] == 150 && b[8] == 150, "{b:?}");

        stick::set(&mut blob, 1, stick::Stick { edge: 10, circular: true, ..linear });
        assert_eq!(blob[116..122], [0, 0, 63, 63, 127, 127]);
        assert_eq!(blob[812..814], [1, 10]);
        assert_eq!(stick::get(&blob, 1).edge, 10);
    }

    #[test]
    fn trigger_window_mirrors_points() {
        let mut blob = vec![0u8; map::SIZE];
        // factory value read from a Vader 5 Pro
        blob[123..137].copy_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255]);
        assert_eq!((trigger::get(&blob, 0), trigger::get(&blob, 1)), ((0, 255), (0, 255)));
        trigger::set(&mut blob, 1, 200, 20);
        assert_eq!(blob[130..137], [0, 20, 20, 20, 200, 200, 200]);
        assert_eq!(trigger::get(&blob, 1), (20, 200));
    }

    #[test]
    fn macro_store_roundtrip() {
        // empty store as read from a Vader 5 Pro: version 0x0100, no macros, 83 packets
        let mut empty = vec![0xff; 83 * 20];
        empty[..24].copy_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        empty[1656..].fill(0);
        assert!(macros::parse(&empty).is_empty());

        let m1 = macros::Macro {
            key: 18,
            kind: 1,
            interval_ms: 0,
            steps: vec![
                macros::Step { delay_ms: 0, key: 4, event: 1 },
                macros::Step { delay_ms: 50, key: 4, event: 0 },
            ],
        };
        let m2 = macros::Macro { key: 19, kind: 2, interval_ms: 100, steps: vec![macros::Step { delay_ms: 10, key: 5, event: 1 }] };
        let blob = macros::encode(&empty, &[m1.clone(), m2.clone()]).unwrap();
        assert_eq!(blob[..8], [0, 1, 2, 0, 0, 0, 10, 0]); // second body after 8 + 2 words
        assert_eq!(blob[8..10], [0xff, 0xff]);
        assert_eq!(blob[24..30], [18, 2, 0, 1, 0, 0]);
        assert_eq!(blob[56..64], [0, 0, 4, 1, 50, 0, 4, 0]);
        assert_eq!(blob[1620..], empty[1620..]);
        assert_eq!(macros::parse(&blob), vec![m1, m2]);
    }
}
