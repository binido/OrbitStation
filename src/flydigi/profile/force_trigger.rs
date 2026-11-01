//! Force-trigger effects stored in the mapping blob (Apex line), 20 bytes per trigger at 185:
//! mode, bind type (2 for the rumble effect, else 0), bind filter, bind scale, five bind params, a
//! mixed border byte, then ten effect params. Layout and slot use from openflydigi, which took them
//! from Space Station's `SaveTriggerAdapterConfig` and felt each effect on an Apex 5.
//!
//! A stored effect does nothing until a live command carries it to the trigger:
//! [`live_requests`] builds those, one per trigger (an "both triggers" command is ignored).

use super::super::frame::{PKT, request};

const OFF: usize = 185;
const ENTRY: usize = 20;
const PARAMS: usize = 10;
/// Live commands: an effect, and the game's rumble routed into the trigger.
const SET_FORCE_TRIGGER: u8 = 81;
const SYNC_WITH_GRIP: u8 = 82;
/// The bind type Flydigi always sends with the rumble effect.
const BIND_GRIP: u8 = 2;

/// Effect ids, `AdapterTriggerType`. 2 and 3 are labelled the other way round in Space Station
/// (2 rattles like a machine gun, 3 resists and breaks through) and so are they here.
pub mod mode {
    pub const NORMAL: u8 = 0;
    pub const RACE: u8 = 1;
    pub const MACHINE_GUN: u8 = 2;
    pub const SNIPER: u8 = 3;
    pub const LOCK: u8 = 4;
    pub const RUMBLE: u8 = 5;
}

/// Effects as (id, untranslated name), in Space Station's order.
pub const EFFECTS: [(u8, &str); 6] = [
    (mode::NORMAL, "No resistance"),
    (mode::RACE, "Racing"),
    (mode::MACHINE_GUN, "Recoil"),
    (mode::SNIPER, "Sniper"),
    (mode::LOCK, "Trigger lock"),
    (mode::RUMBLE, "Game rumble in the trigger"),
];

/// Trigger names.
pub const SIDES: [&str; 2] = ["LT", "RT"];

/// One setting of an effect: untranslated name, Space Station's slider bounds and a gentle default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Knob {
    pub name: &'static str,
    pub min: u8,
    pub max: u8,
    pub default: u8,
    /// An on/off setting: 0 or 1.
    pub switch: bool,
}

const fn knob(name: &'static str, min: u8, max: u8, default: u8) -> Knob {
    Knob { name, min, max, default, switch: false }
}

/// With it on the trigger reports nothing until the start position, then its full range.
const MATCH: Knob = Knob { name: "Match input to the start position", min: 0, max: 1, default: 1, switch: true };

/// The settings of an effect, in the order [`get`] and [`set`] use.
pub fn knobs(effect: u8) -> &'static [Knob] {
    const RACE: [Knob; 2] = [knob("Damping start position", 0, 192, 50), knob("Damping strength", 1, 255, 40)];
    const GUN: [Knob; 5] = [
        knob("Vibration start position", 0, 192, 50),
        knob("Start pressure", 1, 255, 25),
        knob("Vibration strength", 1, 255, 20),
        knob("Vibration frequency", 1, 255, 20),
        MATCH,
    ];
    const SNIPER: [Knob; 4] = [
        knob("Breakthrough start position", 0, 192, 50),
        knob("Breakthrough travel", 1, 255, 30),
        knob("Breakthrough resistance", 1, 255, 40),
        MATCH,
    ];
    const LOCK: [Knob; 1] = [knob("Lock position", 20, 200, 60)];
    const RUMBLE: [Knob; 4] = [
        knob("Intensity coefficient", 0, 200, 50),
        knob("Vibration threshold", 1, 255, 10),
        knob("Travel range", 1, 200, 50),
        knob("Vibration frequency", 1, 255, 20),
    ];
    match effect {
        mode::RACE => &RACE,
        mode::MACHINE_GUN => &GUN,
        mode::SNIPER => &SNIPER,
        mode::LOCK => &LOCK,
        mode::RUMBLE => &RUMBLE,
        _ => &[],
    }
}

/// Param slot of each knob, `None` for the rumble effect, whose knobs live in the bind half.
fn slots(effect: u8) -> &'static [usize] {
    match effect {
        mode::RACE => &[0, 1],
        mode::MACHINE_GUN => &[0, 1, 2, 3, 4],
        mode::SNIPER => &[0, 1, 2, 4],
        mode::LOCK => &[0],
        _ => &[],
    }
}

/// Constants Space Station writes into slots an effect has no control for.
fn fixed(effect: u8) -> &'static [(usize, u8)] {
    match effect {
        mode::LOCK => &[(1, 255), (2, 1)],
        mode::RUMBLE => &[(2, 1), (3, 90)],
        _ => &[],
    }
}

fn block(map: &[u8], side: usize) -> &[u8] {
    &map[OFF + side * ENTRY..][..ENTRY]
}

/// One trigger's effect and its knob values. A byte out of a knob's range was left by another
/// effect (every effect shares the slots) and reads as the knob's default.
pub fn get(map: &[u8], side: usize) -> (u8, Vec<u8>) {
    let b = block(map, side);
    let effect = if EFFECTS.iter().any(|e| e.0 == b[0]) { b[0] } else { mode::NORMAL };
    let raw: Vec<u8> = if effect == mode::RUMBLE {
        // scale, filter, bind stroke, bind frequency
        vec![b[3], b[2], b[4], b[7]]
    } else {
        slots(effect).iter().map(|s| b[10 + s]).collect()
    };
    let values =
        knobs(effect).iter().zip(raw).map(|(k, v)| if (k.min..=k.max).contains(&v) { v } else { k.default }).collect();
    (effect, values)
}

/// Stores one trigger's effect. "No resistance" keeps the params, so tuned numbers survive turning
/// the effect off and on again.
pub fn set(map: &mut [u8], side: usize, effect: u8, values: &[u8]) {
    let b = &mut map[OFF + side * ENTRY..][..ENTRY];
    b[0] = effect;
    b[1] = if effect == mode::RUMBLE { BIND_GRIP } else { 0 };
    if effect == mode::NORMAL {
        return;
    }
    let knobs = knobs(effect);
    let value = |i: usize| knobs.get(i).map_or(0, |k| values.get(i).copied().unwrap_or(k.default).clamp(k.min, k.max));
    let mut params = [0u8; PARAMS];
    for &(slot, v) in fixed(effect) {
        params[slot] = v;
    }
    if effect == mode::RUMBLE {
        let (scale, filter, stroke, frequency) = (value(0), value(1), value(2), value(3));
        (params[0], params[1]) = (stroke, frequency);
        (b[2], b[3]) = (filter, scale);
        b[4..9].copy_from_slice(&[stroke, 1, 1, frequency, 0]);
    } else {
        for (i, &slot) in slots(effect).iter().enumerate() {
            params[slot] = value(i);
        }
    }
    b[10..20].copy_from_slice(&params);
}

/// The live commands that start the stored effects, one per trigger (side ids 1 and 2), as Space
/// Station sends them after a profile is applied.
pub fn live_requests(map: &[u8]) -> Vec<[u8; PKT]> {
    (0..2)
        .map(|side| {
            let b = block(map, side);
            let id = side as u8 + 1;
            let p = &b[10..20];
            let at_least_one = |v: u8| v.max(1);
            let matching = u8::from(p[4] == 1);
            match b[0] {
                mode::RACE => request(SET_FORCE_TRIGGER, &[1, id, mode::RACE, p[0], at_least_one(p[1]), matching]),
                mode::MACHINE_GUN => request(
                    SET_FORCE_TRIGGER,
                    &[
                        1,
                        id,
                        mode::MACHINE_GUN,
                        p[0],
                        at_least_one(p[1]),
                        at_least_one(p[2]),
                        at_least_one(p[3]),
                        matching,
                    ],
                ),
                mode::SNIPER => {
                    request(SET_FORCE_TRIGGER, &[1, id, mode::SNIPER, p[0], p[1], at_least_one(p[2]), 0, matching])
                }
                mode::LOCK => request(SET_FORCE_TRIGGER, &[1, id, mode::LOCK, p[0], 255, 1]),
                // the rumble effect is the grip binding, from the bind half of the block
                mode::RUMBLE => request(SYNC_WITH_GRIP, &[id, BIND_GRIP, b[2], b[3], b[4], b[5], b[6], b[7], 0]),
                _ => request(SET_FORCE_TRIGGER, &[1, id, mode::NORMAL]),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    #[test]
    fn a_stored_race_reads_back_and_starts_per_trigger() {
        let mut map = vec![0u8; SIZE];
        set(&mut map, 1, mode::RACE, &[30, 120]);
        assert_eq!(map[205..207], [mode::RACE, 0]);
        assert_eq!(map[215..217], [30, 120]);
        assert_eq!(get(&map, 1), (mode::RACE, vec![30, 120]));
        let live = live_requests(&map);
        assert_eq!(live[0][..7], [0x5a, 0xa5, 81, 5, 1, 1, 0]);
        assert_eq!(live[1][..10], [0x5a, 0xa5, 81, 8, 1, 2, 1, 30, 120, 0]);
    }

    #[test]
    fn lock_writes_space_stations_constants() {
        let mut map = vec![0u8; SIZE];
        set(&mut map, 0, mode::LOCK, &[60]);
        assert_eq!(map[195..198], [60, 255, 1]);
        assert_eq!(live_requests(&map)[0][4..10], [1, 1, 4, 60, 255, 1]);
    }

    #[test]
    fn game_rumble_lives_in_the_bind_half() {
        let mut map = vec![0u8; SIZE];
        set(&mut map, 0, mode::RUMBLE, &[50, 10, 40, 20]);
        assert_eq!(map[185..194], [5, 2, 10, 50, 40, 1, 1, 20, 0]);
        assert_eq!(map[195..199], [40, 20, 1, 90]);
        assert_eq!(get(&map, 0), (mode::RUMBLE, vec![50, 10, 40, 20]));
        assert_eq!(live_requests(&map)[0][2..13], [82, 11, 1, 2, 10, 50, 40, 1, 1, 20, 0]);
    }

    #[test]
    fn leftovers_out_of_range_read_as_defaults() {
        let mut map = vec![0u8; SIZE];
        map[185] = mode::LOCK;
        // slot 0 holds 5, below the lock's 20..200
        map[195] = 5;
        assert_eq!(get(&map, 0), (mode::LOCK, vec![60]));
    }

    #[test]
    fn turning_the_effect_off_keeps_the_tuned_numbers() {
        let mut map = vec![0u8; SIZE];
        set(&mut map, 0, mode::MACHINE_GUN, &[40, 25, 20, 30, 1]);
        set(&mut map, 0, mode::NORMAL, &[]);
        assert_eq!(map[185], mode::NORMAL);
        assert_eq!(map[195..200], [40, 25, 20, 30, 1]);
    }
}
