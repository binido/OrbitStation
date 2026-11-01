//! Flydigi Vader 5 Pro, wired or through its receiver.

use super::XINPUT_TARGETS;
use crate::station::{Factory, Features, Kind, Model};

/// Remappable buttons, ids from Space Station's device_config_f5.
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
    (16, "C"),
    (17, "Z"),
    (14, "L3"),
    (15, "R3"),
    (6, "Select"),
    (9, "Start"),
    (27, "Home"),
    (18, "M1"),
    (19, "M2"),
    (20, "M3"),
    (21, "M4"),
    (22, "LM"),
    (23, "RM"),
];

/// Button names in bit order of the 0xef report (bytes 11, 12, 13, 14).
pub(super) const BUTTONS: &[&str] = &[
    "↑", "↓", "←", "→", "A", "B", "Select", "X", //
    "Y", "Start", "LB", "RB", "", "", "L3", "R3", //
    "C", "Z", "M1", "M2", "M3", "M4", "LM", "RM", //
    "O", "", "", "Home", "", "", "", "",
];

/// The Vader 5 Pro.
pub const MODEL: Model = Model {
    name: "Flydigi Vader 5 Pro",
    slug: "vader5pro",
    kind: Kind::Gamepad,
    device_ids: &[130, 144, 145],
    profile_slots: 4,
    macro_store: true,
    settings: true,
    live_input: true,
    features: Features {
        remap: true,
        macros: true,
        sticks: true,
        triggers: true,
        gyro: true,
        vibration: true,
        trigger_motors: true,
        lighting: true,
    },
    keys: KEYS,
    targets: XINPUT_TARGETS,
    buttons: BUTTONS,
    // every slot of an untouched pad, read off a Vader 5 Pro; only the title's digit differs
    factory: Some(Factory {
        map: include_bytes!("../../factory/vader5pro.map.bin"),
        macros: include_bytes!("../../factory/vader5pro.macros.bin"),
        rgb: include_bytes!("../../factory/vader5pro.rgb.bin"),
        title: "配置",
    }),
};
