//! Every model the app knows. A new device of a known protocol family is a file here and a line in
//! [`ALL`]: its USB ids, the id it reports, its buttons and which profile parts it has.

mod vader5_pro;

use crate::station::{Features, Kind, Model};

pub use vader5_pro::MODEL as VADER_5_PRO;

/// Known models, searched in this order.
pub const ALL: &[&Model] = &[&VADER_5_PRO];

/// A device of a known family that reports an id no model claims: shown with its info only.
pub const UNKNOWN: Model = Model {
    name: "Unknown Flydigi",
    slug: "flydigi",
    kind: Kind::Gamepad,
    usb: &[],
    config_interface: 1,
    device_ids: &[],
    profile_slots: 0,
    features: Features {
        remap: false,
        macros: false,
        sticks: false,
        triggers: false,
        gyro: false,
        vibration: false,
        lighting: false,
    },
    keys: &[],
    targets: &[],
    buttons: &[],
    factory: None,
};

/// The model that reports `device_id`, [`UNKNOWN`] when none does.
pub fn by_device_id(device_id: u8) -> &'static Model {
    ALL.iter().copied().find(|m| m.device_ids.contains(&device_id)).unwrap_or(&UNKNOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vader_5_pro_id_finds_its_model() {
        for id in [130, 144, 145] {
            assert_eq!(by_device_id(id).slug, "vader5pro");
        }
    }

    #[test]
    fn an_unknown_id_finds_the_info_only_model() {
        assert_eq!(by_device_id(7).slug, UNKNOWN.slug);
        assert_eq!(by_device_id(7).profile_slots, 0);
    }

    #[test]
    fn models_do_not_share_device_ids() {
        let ids: Vec<u8> = ALL.iter().flat_map(|m| m.device_ids.iter().copied()).collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }
}
