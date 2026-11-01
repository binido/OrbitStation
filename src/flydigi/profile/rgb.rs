//! Lighting blob, one per profile. Vader 5 Pro: 320 bytes, a 20-byte header and 10 frames of 10 LEDs.
//! Header: version u16, click feedback, loop start, loop end, cycle time (bigger is slower),
//! brightness, LED count, effect id, grip sync. The pad has no effect generator, it plays the frames,
//! so an effect is written as frames. Byte 2 freezes the shown frame and is never touched here.

const LOOP_START: usize = 3;
const LOOP_END: usize = 4;
/// Header byte of the cycle time, bigger is slower.
pub const PERIOD: usize = 5;
/// Header byte of the brightness, 0..100.
pub const BRIGHTNESS: usize = 6;
const LEDS: usize = 7;
/// Header byte of the effect id.
pub const MODE: usize = 8;
/// Header byte that makes the lights follow the rumble.
pub const GRIP_SYNC: usize = 9;
const FRAMES: usize = 20;

/// Space Station's effect ids; the id only records which generator produced the frames.
pub const EFFECTS: [(u8, &str); 8] = [
    (0, "Lights off"),
    (4, "Single color"),
    (5, "Several colors"),
    (3, "Breathing"),
    (1, "Flow"),
    (6, "Rainbow"),
    (7, "Wave"),
    (8, "Flashes"),
];
/// Most colours an effect is generated from.
pub const MAX_COLORS: usize = 5;
/// One colour, red, green, blue.
pub type Rgb = [u8; 3];

/// LEDs per frame.
pub fn leds(blob: &[u8]) -> usize {
    (blob[LEDS] as usize).clamp(1, 32)
}

/// Frames in the blob.
pub fn frames(blob: &[u8]) -> usize {
    blob.len().saturating_sub(FRAMES) / (3 * leds(blob))
}

/// Colour of one LED in one frame.
pub fn led(blob: &[u8], frame: usize, led: usize) -> Rgb {
    let i = FRAMES + (frame * leds(blob) + led) * 3;
    [blob[i], blob[i + 1], blob[i + 2]]
}

fn blend(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

/// Colour at `pos` 0..1 around the cyclic list, blended between stops.
fn sample(colors: &[Rgb], pos: f32) -> Rgb {
    let x = pos.rem_euclid(1.0) * colors.len() as f32;
    let i = x as usize % colors.len();
    blend(colors[i], colors[(i + 1) % colors.len()], x.fract())
}

fn scale(c: Rgb, level: f32) -> Rgb {
    c.map(|v| (v as f32 * level) as u8)
}

fn hue(pos: f32) -> Rgb {
    let sector = pos * 6.0;
    let rise = (255.0 * sector.fract()) as u8;
    let fall = 255 - rise;
    [[255, rise, 0], [fall, 255, 0], [0, 255, rise], [0, fall, 255], [rise, 0, 255], [255, 0, fall]]
        [sector as usize % 6]
}

/// Rewrites every frame for `effect` and loops over all of them.
pub fn apply(blob: &mut [u8], effect: u8, colors: &[Rgb]) {
    let (n_leds, n_frames) = (leds(blob), frames(blob));
    let rainbow: Vec<Rgb> = (0..6).map(|i| hue(i as f32 / 6.0)).collect();
    let colors = match (effect, colors.is_empty()) {
        (6, _) => &rainbow[..],
        (_, true) => &[[0, 116, 255]][..],
        _ => colors,
    };
    for f in 0..n_frames {
        let ft = f as f32 / n_frames as f32;
        for l in 0..n_leds {
            let lt = l as f32 / n_leds as f32;
            let c = match effect {
                0 => [0, 0, 0],
                4 => colors[0],
                5 => sample(colors, lt),
                // a triangle ramp so the loop joins back to itself
                3 => scale(colors[(ft * colors.len() as f32) as usize % colors.len()], 1.0 - (2.0 * ft - 1.0).abs()),
                1 | 6 => sample(colors, lt + ft),
                7 => scale(sample(colors, lt), 1.0 - (2.0 * (lt + ft).fract() - 1.0).abs()),
                _ => {
                    if f % 2 == 0 {
                        colors[f / 2 % colors.len()]
                    } else {
                        [0, 0, 0]
                    }
                }
            };
            let i = FRAMES + (f * n_leds + l) * 3;
            blob[i..i + 3].copy_from_slice(&c);
        }
    }
    blob[MODE] = effect;
    blob[LOOP_START] = 0;
    blob[LOOP_END] = n_frames.saturating_sub(1) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Vec<u8> {
        // read from a Vader 5 Pro: loop 0..9, period 4, brightness 20, 10 LEDs, wave
        let mut blob = vec![0u8; 320];
        blob[..10].copy_from_slice(&[0, 3, 0, 0, 9, 4, 20, 10, 7, 0]);
        blob
    }

    #[test]
    fn single_color_fills_every_frame_and_keeps_the_header() {
        let mut blob = header();
        assert_eq!((leds(&blob), frames(&blob)), (10, 10));
        apply(&mut blob, 4, &[[255, 0, 0]]);
        assert!((0..10).all(|f| (0..10).all(|l| led(&blob, f, l) == [255, 0, 0])));
        assert_eq!(blob[..10], [0, 3, 0, 0, 9, 4, 20, 10, 4, 0]);
    }

    #[test]
    fn flow_moves_the_pattern_by_frame() {
        let mut blob = header();
        apply(&mut blob, 1, &[[255, 0, 0], [0, 0, 255]]);
        assert_eq!(led(&blob, 0, 0), [255, 0, 0]);
        assert_eq!(led(&blob, 0, 5), [0, 0, 255]);
        assert_eq!(led(&blob, 5, 0), [0, 0, 255]);
    }

    #[test]
    fn lights_off_clears_every_frame() {
        let mut blob = header();
        apply(&mut blob, 0, &[]);
        assert!(blob[20..].iter().all(|b| *b == 0) && blob[8] == 0 && blob[2] == 0);
    }
}
