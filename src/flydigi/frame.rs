//! Framing of the Space Station protocol on the vendor HID interface.
//! Request: `5a a5 CMD LEN PAYLOAD.. SUM`, LEN = 2 + payload length, SUM = low byte of CMD + LEN + payload.
//! Reply: `5a a5 CMD TOTAL INDEX DATA..`, 32 bytes.

/// Bytes in every request and reply.
pub const PKT: usize = 32;

/// Command ids, named after Space Station's command classes.
pub mod cmd {
    pub const HEARTBEAT: u8 = 0x01;
    pub const NICKNAME: u8 = 0x02;
    pub const HW_STATUS: u8 = 0x03;
    pub const TEST_MODE: u8 = 0x11;
    pub const VIBRATION: u8 = 0x12;
    pub const TOGGLE: u8 = 0x13;
    pub const REPORT_RATE: u8 = 0x14;
    pub const PRECISION: u8 = 0x15;
    pub const SENSITIVITY: u8 = 0x16;
    pub const SLEEP_TIME: u8 = 0x17;
    pub const SET_NICKNAME: u8 = 0x18;
    pub const PROFILE_INFO: u8 = 0xa1;
    pub const APPLY_PROFILE: u8 = 0xa2;
    pub const MAP_READ: u8 = 0xa3;
    pub const MAP_WRITE_START: u8 = 0xa4;
    pub const MAP_WRITE_PACK: u8 = 0xa5;
    pub const MAP_SAVE: u8 = 0xa6;
    pub const MAP_RESET: u8 = 0xaf;
    pub const MACRO_READ: u8 = 0xac;
    pub const MACRO_WRITE_START: u8 = 0xad;
    pub const MACRO_WRITE_PACK: u8 = 0xae;
    pub const RGB_READ: u8 = 0xa7;
    pub const RGB_WRITE_START: u8 = 0xa8;
    pub const RGB_WRITE_PACK: u8 = 0xa9;
    pub const EXT_INPUT: u8 = 0xef;
}

/// A request for `cmd` with its payload and checksum.
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

/// A request with a payload longer than a 32-byte packet holds, for devices with bigger reports.
pub fn request_long(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut p = vec![0x5a, 0xa5, cmd, 2 + payload.len() as u8];
    p.extend_from_slice(payload);
    p.push(p[2..].iter().fold(0u8, |s, b| s.wrapping_add(*b)));
    p
}

/// Shortest reply: a numbered input report holds 31 bytes after its id, an unnumbered one 32.
pub const MIN_REPLY: usize = 31;

/// Whether `p` is a full reply to `cmd`.
pub fn is_reply(p: &[u8], cmd: u8) -> bool {
    p.len() >= MIN_REPLY && p[..3] == [0x5a, 0xa5, cmd]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_match_the_official_app() {
        assert_eq!(request(cmd::HEARTBEAT, &[])[..5], [0x5a, 0xa5, 0x01, 0x02, 0x03]);
        let test_mode = [0x5a, 0xa5, 0x11, 0x07, 0xff, 0x01, 0xff, 0xff, 0xff, 0x15];
        assert_eq!(request(cmd::TEST_MODE, &[0xff, 1, 0xff, 0xff, 0xff])[..10], test_mode);
    }
}
