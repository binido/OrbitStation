//! Rumble settings in the profile blob, switches are inverted (0 on, ff off).
//! Grips at 145: master, then per side switch, min, max, strength %.
//! Trigger motors at 154: shared switch, then per side two 7-byte gears
//! {type, min, max, threshold, min_start, strength %, min_time}; Space Station edits only the first gear
//! and keeps strength and threshold equal on both triggers.

const GRIP: usize = 145;
const TRIG: usize = 154;

/// Rumble settings of a profile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vibration {
    pub grip_on: bool,
    pub grip_strength: u8,
    pub trig_on: bool,
    pub trig_strength: u8,
    pub trig_threshold: u8,
    /// Amplitude window per trigger, bytes 0..255.
    pub trig_window: [(u8, u8); 2],
}

fn gear(side: usize) -> usize {
    TRIG + 1 + side * 14
}

/// The rumble settings of a mapping blob.
pub fn get(blob: &[u8]) -> Vibration {
    let g = |side: usize| gear(side);
    Vibration {
        grip_on: blob[GRIP] == 0,
        grip_strength: blob[GRIP + 4],
        trig_on: blob[TRIG] == 0,
        trig_strength: blob[g(0) + 5],
        trig_threshold: blob[g(0) + 3],
        trig_window: [0, 1].map(|s| (blob[g(s) + 1], blob[g(s) + 2])),
    }
}

/// Stores `v`, keeping the gear bytes Space Station does not edit.
pub fn set(blob: &mut [u8], v: Vibration) {
    let sw = |on: bool| if on { 0 } else { 0xff };
    blob[GRIP] = sw(v.grip_on);
    blob[GRIP + 4] = v.grip_strength;
    blob[GRIP + 8] = v.grip_strength;
    blob[TRIG] = sw(v.trig_on);
    for (side, (lo, hi)) in v.trig_window.into_iter().enumerate() {
        let g = gear(side);
        blob[g + 1] = lo.min(hi);
        blob[g + 2] = lo.max(hi);
        blob[g + 3] = v.trig_threshold;
        blob[g + 5] = v.trig_strength;
    }
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    fn factory() -> Vec<u8> {
        let mut blob = vec![0u8; SIZE];
        // factory values read from a Vader 5 Pro
        blob[145..154].copy_from_slice(&[0, 0, 60, 255, 50, 0, 80, 255, 50]);
        blob[154..183].copy_from_slice(&[
            0, 1, 51, 114, 5, 1, 50, 0, 255, 40, 120, 5, 0, 50, 0, 1, 51, 114, 5, 1, 50, 0, 255, 40, 120, 5, 0, 50, 0,
        ]);
        blob
    }

    #[test]
    fn factory_blocks_read_and_write_back_unchanged() {
        let mut blob = factory();
        let v = get(&blob);
        assert!(v.grip_on && v.trig_on);
        assert_eq!((v.grip_strength, v.trig_strength, v.trig_threshold, v.trig_window), (50, 50, 5, [(51, 114); 2]));
        set(&mut blob, v);
        assert_eq!(blob, factory());
    }

    #[test]
    fn changes_touch_only_the_edited_bytes() {
        let mut blob = factory();
        let v = get(&blob);
        set(&mut blob, Vibration { trig_on: false, grip_strength: 80, trig_window: [(0, 255), (10, 20)], ..v });
        assert_eq!((blob[145], blob[149], blob[153], blob[154]), (0, 80, 80, 0xff));
        assert_eq!((blob[156], blob[157], blob[170], blob[171]), (0, 255, 10, 20));
        assert_eq!(blob[162..169], factory()[162..169]);
    }
}
