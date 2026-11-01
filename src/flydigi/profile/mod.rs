//! Space Station profiles: three blobs per slot (mapping, macros, lighting), their parts and the
//! `.osp` file the app saves them to.

pub mod gyro;
pub mod macros;
pub mod map;
pub mod rgb;
pub mod stick;
pub mod trigger;
pub mod vibration;

use crate::station::Model;

/// Why a profile could not be built, read from a file or applied to the editor.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("{}", t!("no more than {} macros", .0))]
    TooManyMacros(usize),
    #[error("{}", t!("the macros do not fit in the gamepad memory"))]
    MacrosTooLarge,
    #[error("{}", t!("a macro is longer than 65 seconds"))]
    MacroTooLong,
    #[error("{}", t!("button {} has more than one macro", .0))]
    DuplicateMacro(&'static str),
    #[error("{}", t!("the file is from another gamepad or damaged"))]
    OtherDevice,
    #[error("{}", t!("could not read the file"))]
    BadFile,
}

/// One profile slot as the device stores it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Profile {
    pub map: Vec<u8>,
    pub macros: Vec<u8>,
    pub rgb: Vec<u8>,
}

/// Profile file and backup format: `OSP1`, then mapping, macros and lighting, each with a u16 LE length.
const FILE_MAGIC: &[u8; 4] = b"OSP1";

impl Profile {
    /// The profile `slot` holds out of the box, `None` for a model without a factory copy.
    pub fn factory(model: &Model, slot: u8) -> Option<Profile> {
        let f = model.factory.as_ref()?;
        let mut map = f.map.to_vec();
        map::set_title(&mut map, &format!("{}{}", f.title, slot + 1));
        Some(Profile { map, macros: f.macros.to_vec(), rgb: f.rgb.to_vec() })
    }

    /// Whether `other` has the blob sizes of this one, so it is from the same kind of device.
    pub fn fits(&self, other: &Profile) -> bool {
        (self.map.len(), self.macros.len(), self.rgb.len()) == (other.map.len(), other.macros.len(), other.rgb.len())
    }

    /// The `.osp` file contents.
    pub fn to_file(&self) -> Vec<u8> {
        let mut out = FILE_MAGIC.to_vec();
        for b in [&self.map, &self.macros, &self.rgb] {
            out.extend_from_slice(&(b.len() as u16).to_le_bytes());
            out.extend_from_slice(b);
        }
        out
    }

    /// Reads an `.osp` file.
    ///
    /// # Errors
    /// [`ProfileError::BadFile`] when the magic or a length does not match the data.
    pub fn from_file(data: &[u8]) -> Result<Profile, ProfileError> {
        let mut rest = data.strip_prefix(FILE_MAGIC).ok_or(ProfileError::BadFile)?;
        let mut next = || {
            let n = u16::from_le_bytes([*rest.first()?, *rest.get(1)?]) as usize;
            let blob = rest.get(2..2 + n)?.to_vec();
            rest = &rest[2 + n..];
            Some(blob)
        };
        let mut blob = || next().ok_or(ProfileError::BadFile);
        Ok(Profile { map: blob()?, macros: blob()?, rgb: blob()? })
    }
}

/// Points every key that has a macro at the macro store and resets keys that lost theirs, otherwise
/// the pad would go on playing an orphaned macro under the new binding.
///
/// # Errors
/// [`ProfileError::DuplicateMacro`] when two macros share a key.
pub fn bind_macros(map: &mut [u8], keys: &[(u8, &'static str)], list: &[macros::Macro]) -> Result<(), ProfileError> {
    for &(id, name) in keys {
        let count = list.iter().filter(|m| m.key == id).count();
        if count > 1 {
            return Err(ProfileError::DuplicateMacro(name));
        }
        if count == 1 {
            map::set_key(map, id, map::Key { target: map::MACRO, turbo: 0, hz: 0 });
        } else if map::key(map, id).target == map::MACRO {
            map::set_key(map, id, map::Key { target: map::IDENTITY, turbo: 0, hz: 0 });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::VADER_5_PRO;

    fn factory() -> Profile {
        Profile::factory(&VADER_5_PRO, 1).expect("the Vader 5 Pro has a factory profile")
    }

    #[test]
    fn factory_profile_is_titled_by_its_slot() {
        let p = factory();
        assert_eq!((p.map.len(), p.macros.len(), p.rgb.len()), (map::SIZE, 1660, 320));
        assert_eq!(map::title(&p.map), "配置2");
    }

    #[test]
    fn file_round_trips() {
        let p = factory();
        assert_eq!(Profile::from_file(&p.to_file()), Ok(p));
    }

    #[test]
    fn short_files_are_refused() {
        assert_eq!(Profile::from_file(b"OSP1\x01"), Err(ProfileError::BadFile));
        assert_eq!(Profile::from_file(&factory().to_file()[..100]), Err(ProfileError::BadFile));
    }

    #[test]
    fn keys_follow_their_macros() {
        let mut map = factory().map;
        let m = macros::Macro { key: 18, kind: 1, interval_ms: 0, steps: vec![] };
        bind_macros(&mut map, VADER_5_PRO.keys, std::slice::from_ref(&m)).expect("one macro per key");
        assert_eq!(map::key(&map, 18).target, map::MACRO);
        bind_macros(&mut map, VADER_5_PRO.keys, &[]).expect("no macros at all");
        assert_eq!(map::key(&map, 18).target, map::IDENTITY);
        assert_eq!(bind_macros(&mut map, VADER_5_PRO.keys, &[m.clone(), m]), Err(ProfileError::DuplicateMacro("M1")));
    }
}
