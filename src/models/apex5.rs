//! Flydigi Apex 5 and its editions. Facts from openflydigi, measured there on a wired Apex 5
//! (`37d7:2501`, report ids 3 out and 4 in); not yet run against this app on hardware.

use super::XINPUT_TARGETS;
use crate::station::{Features, Kind, Model};

/// Remappable buttons, ids from Space Station's key table; no C and Z, M5 and M6 instead.
const KEYS: &[(u8, &str)] = &[
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
    (18, "M1"),
    (19, "M2"),
    (20, "M3"),
    (21, "M4"),
    (22, "M5"),
    (23, "M6"),
];

/// Bit order of the 0xef report, taken to be the Vader's with M5 and M6 on the bits of LM and RM.
const BUTTONS: &[&str] = &[
    "↑", "↓", "←", "→", "A", "B", "Select", "X", //
    "Y", "Start", "LB", "RB", "", "", "L3", "R3", //
    "", "", "M1", "M2", "M3", "M4", "M5", "M6", //
    "", "", "", "Home", "", "", "", "",
];

/// The Apex 5. Its macros live inside the mapping blob (protocol 3.1), up to five, timed in 10 ms
/// steps. It has force triggers rather than trigger motors.
pub const MODEL: Model = Model {
    name: "Flydigi Apex 5",
    slug: "apex5",
    kind: Kind::Gamepad,
    device_ids: &[128, 129, 133, 134, 135, 136],
    profile_slots: 4,
    macro_store: false,
    settings: true,
    live_input: true,
    features: Features {
        remap: true,
        macros: true,
        sticks: true,
        triggers: true,
        gyro: true,
        vibration: true,
        trigger_motors: false,
        lighting: true,
    },
    keys: KEYS,
    targets: XINPUT_TARGETS,
    buttons: BUTTONS,
    factory: None,
};
