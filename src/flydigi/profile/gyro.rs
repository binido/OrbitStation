//! Gyro mapped to a stick, 8 bytes at 137 (m_fdg_macro_motion_mapping_struct_t): target, enable key,
//! enable type, dead zone, sensitivity x, sensitivity y, use mode, second enable key. The pad does the
//! mapping itself and only while an enable key turns it on.

const OFF: usize = 137;
/// MotionMapType. 3 (mouse) is done on the host by Space Station, so it is not offered.
pub const TARGETS: [(u8, &str); 3] = [(0, "Disabled"), (2, "Right stick (aim)"), (1, "Left stick (steering)")];
/// MotionEnableType.
pub const ENABLE: [(u8, &str); 2] = [(1, "While held"), (0, "Toggle on press")];
/// ControllerKey.None.
pub const NO_KEY: u8 = 255;

/// Gyro-to-stick settings of a profile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gyro {
    pub target: u8,
    pub key: u8,
    pub key2: u8,
    pub enable: u8,
    pub dead_zone: u8,
    pub sensitivity: u8,
}

/// The gyro settings of a mapping blob.
pub fn get(blob: &[u8]) -> Gyro {
    let b = &blob[OFF..][..8];
    Gyro { target: b[0], key: b[1], key2: b[7], enable: b[2], dead_zone: b[3], sensitivity: b[4].max(b[5]) }
}

/// Use mode follows the target as in Space Station's save path: left stick is Racer (1), else FPS (0).
pub fn set(blob: &mut [u8], g: Gyro) {
    let mode = (g.target == 1) as u8;
    blob[OFF..][..8].copy_from_slice(&[
        g.target,
        g.key,
        g.enable,
        g.dead_zone,
        g.sensitivity,
        g.sensitivity,
        mode,
        g.key2,
    ]);
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    #[test]
    fn block_round_trips_with_mode_from_target() {
        let mut blob = vec![0u8; SIZE];
        // factory value read from a Vader 5 Pro: off, key LT, toggle, dead zone 4, sensitivity 25/20
        blob[137..145].copy_from_slice(&[0, 12, 0, 4, 25, 20, 0, 0]);
        let g = get(&blob);
        assert_eq!((g.target, g.key, g.enable, g.dead_zone, g.sensitivity), (0, 12, 0, 4, 25));
        set(&mut blob, Gyro { target: 1, key: 10, enable: 1, sensitivity: 60, ..g });
        assert_eq!(blob[137..145], [1, 10, 1, 4, 60, 60, 1, 0]);
    }
}
