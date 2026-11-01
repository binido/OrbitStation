//! Every model the app knows. A new device of a known protocol family is a file here and a line in
//! [`PADS`] or [`DOCKS`]: the ids it reports, its buttons and which profile parts it has.
//!
//! Older Flydigi controllers (Vader 3 and 4, Apex 3 and 4, the Direwolf line) are plain XInput
//! devices without Flydigi's vendor id, so nothing on Linux can configure them and they are not here.

mod apex5;
mod vader5_pro;

use crate::flydigi::profile::map::IDENTITY;
use crate::station::{Kind, Model};

pub use apex5::MODEL as APEX_5;
pub use vader5_pro::MODEL as VADER_5_PRO;

/// What a remapped button may send on Flydigi's XInput controllers: only buttons the host can see,
/// the back paddles and extra keys are sources, never targets.
const XINPUT_TARGETS: &[(u8, &str)] = &[
    (IDENTITY, "Default"),
    (4, "A"),
    (5, "B"),
    (7, "X"),
    (8, "Y"),
    (0, "↑"),
    (2, "↓"),
    (3, "←"),
    (1, "→"),
    (10, "LB"),
    (11, "RB"),
    (12, "LT"),
    (13, "RT"),
    (14, "L3"),
    (15, "R3"),
    (6, "Select"),
    (9, "Start"),
    (27, "Home"),
];

/// The Apex 6. It speaks a newer dialect for everything past its identity, so only its info is read.
pub const APEX_6: Model = Model { name: "Flydigi Apex 6", slug: "apex6", device_ids: &[149], ..Model::INFO_ONLY };

/// The Apex 6 Pro and its Phantom Blade Zero edition, info only like the Apex 6.
pub const APEX_6_PRO: Model =
    Model { name: "Flydigi Apex 6 Pro", slug: "apex6pro", device_ids: &[150, 152], ..Model::INFO_ONLY };

/// Controllers, found by the id they report.
pub const PADS: &[&Model] = &[&VADER_5_PRO, &APEX_5, &APEX_6, &APEX_6_PRO];

/// A controller that reports an id no model claims: shown with its info only.
pub const UNKNOWN: Model = Model { name: "Unknown Flydigi", slug: "flydigi", ..Model::INFO_ONLY };

/// The CD2 charging dock; charger types 0..4 are its editions, which differ in artwork only.
pub const CD2: Model = Model {
    name: "Flydigi CD2",
    slug: "cd2",
    kind: Kind::Dock,
    device_ids: &[0, 1, 2, 3, 4],
    settings: true,
    ..Model::INFO_ONLY
};

/// Docks, found by the charger type they report.
pub const DOCKS: &[&Model] = &[&CD2];

/// A dock that reports a type no model claims: Space Station would drive it as a CD2, this app only
/// shows its info.
pub const UNKNOWN_DOCK: Model =
    Model { name: "Unknown Flydigi dock", slug: "flydigi-dock", kind: Kind::Dock, ..Model::INFO_ONLY };

/// The controller that reports `device_id`, [`UNKNOWN`] when none does.
pub fn pad(device_id: u8) -> &'static Model {
    find(PADS, device_id).unwrap_or(&UNKNOWN)
}

/// The dock that reports `charger_type`, [`UNKNOWN_DOCK`] when none does.
pub fn dock(charger_type: u8) -> &'static Model {
    find(DOCKS, charger_type).unwrap_or(&UNKNOWN_DOCK)
}

fn find(list: &[&'static Model], id: u8) -> Option<&'static Model> {
    list.iter().copied().find(|m| m.device_ids.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vader_5_pro_id_finds_its_model() {
        for id in [130, 144, 145] {
            assert_eq!(pad(id).slug, "vader5pro");
        }
    }

    #[test]
    fn apex_editions_find_their_models() {
        assert_eq!(pad(129).slug, "apex5");
        assert_eq!(pad(149).slug, "apex6");
        assert_eq!(pad(152).slug, "apex6pro");
    }

    #[test]
    fn an_unknown_id_finds_the_info_only_model() {
        assert_eq!(pad(7).slug, UNKNOWN.slug);
        assert_eq!(pad(7).profile_slots, 0);
        assert_eq!(dock(9).slug, UNKNOWN_DOCK.slug);
    }

    #[test]
    fn every_cd2_edition_is_a_cd2() {
        assert!((0..=4).all(|t| dock(t).slug == "cd2"));
    }

    fn unique(list: &[&Model]) -> bool {
        let ids: Vec<u8> = list.iter().flat_map(|m| m.device_ids.iter().copied()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        ids.len() == sorted.len()
    }

    #[test]
    fn models_of_one_kind_do_not_share_ids() {
        assert!(unique(PADS) && unique(DOCKS));
    }
}
