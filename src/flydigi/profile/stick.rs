//! Stick response curves in the profile blob. The pad plays the 9-point table at 790
//! (value = output % + 50, at 0, 12.5 .. 100 % travel); the polyline at 109 is the source form
//! Space Station's editor reads back, so both are written together.

const OFF_CURVE: usize = 109;
const CURVE: usize = 7;
const OFF_EXTRA: usize = 790;
const EXTRA: usize = 12;

/// JoystickSensitivityType, with the points each preset stands for.
pub const PRESETS: [(u8, &str); 4] = [(0, "Linear"), (1, "Fast"), (2, "Smooth"), (3, "Custom")];
/// Stick names, translated where they are shown.
pub const SIDES: [&str; 2] = ["Left stick", "Right stick"];

/// One stick's response settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stick {
    /// Preset id from [`PRESETS`].
    pub kind: u8,
    /// Inner dead zone, % of travel.
    pub center: u8,
    /// Outer dead zone, % of travel.
    pub edge: u8,
    /// Curve breakpoints on the 0..127 scale.
    pub p1: (u8, u8),
    pub p2: (u8, u8),
    pub circular: bool,
}

/// Breakpoints of a preset, `None` for the custom curve.
pub fn preset(kind: u8) -> Option<((u8, u8), (u8, u8))> {
    match kind {
        0 => Some(((63, 63), (127, 127))),
        1 => Some(((64, 96), (127, 127))),
        2 => Some(((64, 32), (127, 127))),
        _ => None,
    }
}

/// Settings of stick `side` (0 left, 1 right).
pub fn get(blob: &[u8], side: usize) -> Stick {
    let c = &blob[OFF_CURVE + side * CURVE..][..CURVE];
    let e = &blob[OFF_EXTRA + side * EXTRA..][..EXTRA];
    // Values over 100 are the "not a stick" sentinel or the negative half Flydigi's own reader
    // and writer disagree on; neither is an editable dead zone.
    let pct = |b: u8| if b <= 100 { b } else { 0 };
    Stick { kind: c[0], center: pct(c[1]), edge: pct(e[11]), p1: (c[2], c[3]), p2: (c[4], c[5]), circular: e[10] == 1 }
}

/// The 9-point table the pad plays for stick `side`.
pub fn bank(blob: &[u8], side: usize) -> [u8; 9] {
    std::array::from_fn(|i| blob[OFF_EXTRA + side * EXTRA + 1 + i])
}

/// Stores `s` in both the editor's source form and the table the pad plays.
pub fn set(blob: &mut [u8], side: usize, s: Stick) {
    let c = &mut blob[OFF_CURVE + side * CURVE..][..CURVE];
    c[..6].copy_from_slice(&[s.kind, s.center, s.p1.0, s.p1.1, s.p2.0, s.p2.1]);
    let e = &mut blob[OFF_EXTRA + side * EXTRA..][..EXTRA];
    e[0] = s.kind;
    e[1..10].copy_from_slice(&compile(&s));
    e[10] = s.circular as u8;
    e[11] = s.edge;
}

/// Samples the polyline start, p1, p2, end. The breakpoints' x is squeezed into the span the
/// dead zones leave, as Space Station's CalculatePoint does. Truncation matches a factory table.
pub fn compile(s: &Stick) -> [u8; 9] {
    let (c, e) = (s.center as f64, s.edge.min(100 - s.center) as f64);
    let (start, end) = ((c, 0.0), (100.0 - e, 100.0));
    let span = end.0 - start.0;
    let inner = |p: (u8, u8)| (start.0 + span * p.0 as f64 / 127.0, p.1 as f64 * 100.0 / 127.0);
    let nodes = [start, inner(s.p1), inner(s.p2), end];
    let at = |x: f64| -> f64 {
        if span <= 0.0 {
            return if x < start.0 { 0.0 } else { 100.0 };
        }
        for w in 0..3 {
            let ((x0, y0), (x1, y1)) = (nodes[w], nodes[w + 1]);
            if x <= x1 || w == 2 {
                return if x1 == x0 { if x < x0 { y0 } else { y1 } } else { y0 + (y1 - y0) * (x - x0) / (x1 - x0) };
            }
        }
        100.0
    };
    std::array::from_fn(|i| (at(12.5 * i as f64).clamp(-50.0, 100.0) as i32 + 50) as u8)
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    #[test]
    fn curve_of_a_real_pad_reads_back() {
        let mut blob = vec![0u8; SIZE];
        // left stick of profile 1 as read from a Vader 5 Pro: 5 % dead zone, linear
        blob[109..116].copy_from_slice(&[0, 5, 63, 63, 127, 127, 127]);
        blob[790..802].copy_from_slice(&[0, 45, 58, 71, 84, 97, 110, 123, 137, 150, 0, 0]);
        let s = get(&blob, 0);
        assert_eq!((s.kind, s.center, s.edge, s.p1, s.circular), (0, 5, 0, (63, 63), false));
        assert_eq!(bank(&blob, 0), [45, 58, 71, 84, 97, 110, 123, 137, 150]);
    }

    #[test]
    fn compiled_tables_follow_the_dead_zones() {
        let linear = Stick { kind: 0, center: 0, edge: 0, p1: (63, 63), p2: (127, 127), circular: false };
        assert_eq!(compile(&linear), [50, 62, 75, 87, 100, 112, 125, 137, 150]);
        let b = compile(&Stick { center: 20, edge: 20, ..linear });
        assert!(b[1] < 50 && b[7] == 150 && b[8] == 150, "{b:?}");
    }

    #[test]
    fn set_writes_both_forms() {
        let mut blob = vec![0u8; SIZE];
        let linear = Stick { kind: 0, center: 0, edge: 10, p1: (63, 63), p2: (127, 127), circular: true };
        set(&mut blob, 1, linear);
        assert_eq!(blob[116..122], [0, 0, 63, 63, 127, 127]);
        assert_eq!(blob[812..814], [1, 10]);
        assert_eq!(get(&blob, 1).edge, 10);
    }
}
