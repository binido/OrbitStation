//! Macro store of protocol 3.2 (MacroConfigParserV10), one per profile. Vader 5 Pro answers 83 packets,
//! Space Station writes only the first 81 and the pad keeps the tail itself (the last 4 bytes stay 0).
//! Header: version u16, count u16, ten u16 body offsets in 4-byte words after the header.
//! Body: trigger key, step count u16, kind, repeat interval u16 (ms), 6 x ff, 20-byte name,
//! then 4 bytes per step: time since the macro started u16 (ms), key, event.

use super::ProfileError;

const HEADER: usize = 24;
const SLOTS: usize = 10;
const BODY: usize = 32;
/// GetPackageCount (81) x 20 bytes, what Space Station owns of the store.
const OWNED: usize = 81 * 20;

/// MacroEnableType.
pub const KINDS: [(u8, &str); 3] = [(1, "Once"), (2, "While held"), (3, "Toggle on press")];
/// MacroActionEvent. Space Station never writes 5 (hold): its "hold" is a press and a release.
pub const EVENTS: [(u8, &str); 4] = [(1, "Press"), (0, "Release"), (2, "Left stick"), (3, "Right stick")];
/// Stick steps carry a direction instead of a key, ControllerKey_Joystick_*; Center releases the stick.
pub const DIRECTIONS: [(u8, &str); 9] =
    [(161, "↑"), (162, "↗"), (163, "→"), (164, "↘"), (165, "↓"), (166, "↙"), (167, "←"), (168, "↖"), (160, "Center")];

/// Whether a step of this event moves a stick instead of pressing a key.
pub fn is_stick(event: u8) -> bool {
    event == 2 || event == 3
}

/// Most macros a profile holds.
pub const MAX: usize = SLOTS;

/// One step: wait `delay_ms` after the previous one, then send `event` for `key`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Step {
    pub delay_ms: u16,
    pub key: u8,
    pub event: u8,
}

/// A macro bound to the button `key`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Macro {
    pub key: u8,
    pub kind: u8,
    pub interval_ms: u16,
    pub steps: Vec<Step>,
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

/// The macros of a store, an unreadable store reads as none.
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
///
/// # Errors
/// [`ProfileError`] when there are too many macros, they do not fit or one is longer than 65 s.
pub fn encode(old: &[u8], list: &[Macro]) -> Result<Vec<u8>, ProfileError> {
    if list.len() > SLOTS {
        return Err(ProfileError::TooManyMacros(SLOTS));
    }
    let mut out = old.to_vec();
    let owned = OWNED.min(out.len());
    out[2..owned].fill(0xff);
    out[2..4].copy_from_slice(&(list.len() as u16).to_le_bytes());
    let mut p = HEADER;
    for (i, m) in list.iter().enumerate() {
        if p + BODY + 4 * m.steps.len() > owned {
            return Err(ProfileError::MacrosTooLarge);
        }
        out[4 + 2 * i..][..2].copy_from_slice(&(((p - HEADER) / 4) as u16).to_le_bytes());
        out[p] = m.key;
        out[p + 1..p + 3].copy_from_slice(&(m.steps.len() as u16).to_le_bytes());
        out[p + 3] = m.kind;
        out[p + 4..p + 6].copy_from_slice(&m.interval_ms.to_le_bytes());
        let mut t = 0u16;
        for (j, s) in m.steps.iter().enumerate() {
            t = t.checked_add(s.delay_ms).ok_or(ProfileError::MacroTooLong)?;
            let at = p + BODY + 4 * j;
            out[at..at + 2].copy_from_slice(&t.to_le_bytes());
            out[at + 2] = s.key;
            out[at + 3] = s.event;
        }
        p += BODY + 4 * m.steps.len();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_store() -> Vec<u8> {
        // as read from a Vader 5 Pro: version 0x0100, no macros, 83 packets
        let mut empty = vec![0xff; 83 * 20];
        empty[..24].fill(0);
        empty[1] = 1;
        empty[1656..].fill(0);
        empty
    }

    #[test]
    fn empty_store_has_no_macros() {
        assert!(parse(&empty_store()).is_empty());
    }

    #[test]
    fn encoded_macros_parse_back_and_keep_the_tail() {
        let empty = empty_store();
        let tap = vec![Step { delay_ms: 0, key: 4, event: 1 }, Step { delay_ms: 50, key: 4, event: 0 }];
        let m1 = Macro { key: 18, kind: 1, interval_ms: 0, steps: tap };
        let m2 = Macro { key: 19, kind: 2, interval_ms: 100, steps: vec![Step { delay_ms: 10, key: 5, event: 1 }] };
        let blob = encode(&empty, &[m1.clone(), m2.clone()]).expect("two small macros fit");
        assert_eq!(blob[..8], [0, 1, 2, 0, 0, 0, 10, 0]); // second body after 8 + 2 words
        assert_eq!(blob[8..10], [0xff, 0xff]);
        assert_eq!(blob[24..30], [18, 2, 0, 1, 0, 0]);
        assert_eq!(blob[56..64], [0, 0, 4, 1, 50, 0, 4, 0]);
        assert_eq!(blob[1620..], empty[1620..]);
        assert_eq!(parse(&blob), vec![m1, m2]);
    }

    #[test]
    fn eleven_macros_are_refused() {
        let one = Macro { key: 18, kind: 1, interval_ms: 0, steps: vec![] };
        assert!(matches!(encode(&empty_store(), &vec![one; 11]), Err(ProfileError::TooManyMacros(10))));
    }
}
