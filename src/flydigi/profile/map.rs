//! Profile mapping blob: 42 packets of 20 bytes, layout from MappingConfigParserV30.

/// Bytes per transfer packet of every profile blob.
pub const PKG: usize = 20;
/// Size of the mapping blob.
pub const SIZE: usize = 840;
const OFF_KEYS: usize = 13;
const OFF_VERSION: usize = 225;
const OFF_TITLE: usize = 770;
/// Longest profile title, in UTF-16 units.
pub const TITLE_CHARS: usize = 10;
/// Key table target meaning "the button does what it says on the shell".
pub const IDENTITY: u8 = 255;
/// Key table target meaning "the button plays its macro from the macro store".
pub const MACRO: u8 = 32;

/// Turbo modes, KeyMapTypeContinuousEnableType.
pub const TURBO: [(u8, &str); 3] = [(0, "Off"), (1, "While held"), (2, "Toggle on press")];
pub const TURBO_MAX_HZ: u8 = 30;

/// One key table entry: what the key sends, turbo mode and turbo presses per second.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Key {
    pub target: u8,
    pub turbo: u8,
    pub hz: u8,
}

/// The key table entry of key `id`.
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

/// Stores a new change tag, see [`version`].
pub fn set_version(blob: &mut [u8], v: u16) {
    blob[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&v.to_le_bytes());
}

/// Profile name, UTF-16LE in 20 bytes; Space Station trims both 0 and ffff padding.
pub fn title(blob: &[u8]) -> String {
    let units: Vec<u16> = blob[OFF_TITLE..OFF_TITLE + 20].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units).trim_end_matches(['\0', '\u{ffff}']).to_string()
}

/// Stores `name`, cut to [`TITLE_CHARS`].
pub fn set_title(blob: &mut [u8], name: &str) {
    let field = &mut blob[OFF_TITLE..OFF_TITLE + 20];
    field.fill(0);
    for (i, u) in name.encode_utf16().take(TITLE_CHARS).enumerate() {
        field[2 * i..2 * i + 2].copy_from_slice(&u.to_le_bytes());
    }
}

/// First changed packet and how many follow it, the official app also resends only the changed span.
pub fn changed_span(old: &[u8], new: &[u8]) -> Option<(usize, usize)> {
    let differs = |i: usize| old[i * PKG..(i + 1) * PKG] != new[i * PKG..(i + 1) * PKG];
    let first = (0..new.len() / PKG).find(|&i| differs(i))?;
    let last = (0..new.len() / PKG).rev().find(|&i| differs(i))?;
    Some((first, last - first + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remapped_key_lands_in_its_packet() {
        let old = vec![IDENTITY; SIZE];
        let mut new = old.clone();
        assert_eq!(key(&new, 18).target, IDENTITY);
        set_key(&mut new, 18, Key { target: 4, turbo: 0, hz: 0 });
        set_version(&mut new, 0x0840);
        // M1 sits at byte 13 + 18 * 3 = 67 (packet 3), the version at 225 (packet 11)
        assert_eq!((new[67], new[225], new[226]), (4, 0x40, 0x08));
        assert_eq!(changed_span(&old, &new), Some((3, 9)));
        assert_eq!(changed_span(&old, &old), None);
    }

    #[test]
    fn turbo_on_an_unchanged_key_names_the_key_itself() {
        let mut blob = vec![IDENTITY; SIZE];
        set_key(&mut blob, 4, Key { target: IDENTITY, turbo: 1, hz: 15 });
        assert_eq!(blob[13 + 12..13 + 15], [4, 1, 15]);
        assert_eq!(key(&blob, 4), Key { target: IDENTITY, turbo: 1, hz: 15 });
        set_key(&mut blob, 4, Key { target: IDENTITY, turbo: 0, hz: 15 });
        assert_eq!(blob[13 + 12..13 + 15], [255, 0, 0]);
    }

    #[test]
    fn title_reads_utf16_and_cuts_to_ten_units() {
        let mut blob = vec![0u8; SIZE];
        // title of profile 1 read from a Vader 5 Pro
        blob[770..790]
            .copy_from_slice(&[0x4d, 0x91, 0x6e, 0x7f, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(title(&blob), "配置1");
        set_title(&mut blob, "Шутеры и гонки");
        assert_eq!(title(&blob), "Шутеры и г");
    }
}
