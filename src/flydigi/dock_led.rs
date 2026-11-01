//! The CD2 dock's lighting. The dock has no effect generator: it plays frames the host computes,
//! 162 LEDs in a wedge of 16 rows, and the mode byte only records which effect made them. The
//! effects are Space Station's `useLedEffectRenderer`, as openflydigi transcribed and checked them
//! against the dock side by side, rounding quirks included.

use std::f64::consts::PI;

/// Colour of one LED.
pub type Rgb = [u8; 3];

pub const LED_COUNT: usize = 162;
/// LEDs per row, top to bottom.
pub const ROWS: [usize; 16] = [14, 15, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3];
/// The frame count is one byte of the header.
pub const MAX_FRAMES: usize = 255;
/// One period unit, measured: a period of 1 shows each frame for 20 ms.
pub const PERIOD_MS: u32 = 20;

/// Effects the dock is given, `ChargerLedType` ids. Default (2) plays a file Space Station installs
/// and is not computed here; Custom (3) plays the frames of a picture.
pub mod mode {
    pub const OFF: u8 = 0;
    pub const SOLID: u8 = 1;
    pub const PICTURE: u8 = 3;
    pub const DIAGONAL_FLOW: u8 = 4;
    pub const BREATH: u8 = 5;
    pub const GRADIENT: u8 = 6;
    pub const WAVE_GRADIENT: u8 = 7;
    pub const RAINBOW: u8 = 8;
    pub const PULSE: u8 = 9;
}

/// Effects as (id, untranslated name), in Space Station's order.
pub const EFFECTS: [(u8, &str); 9] = [
    (mode::OFF, "Lights off"),
    (mode::SOLID, "Single color"),
    (mode::BREATH, "Breathing"),
    (mode::PULSE, "Pulse"),
    (mode::GRADIENT, "Color cycle"),
    (mode::RAINBOW, "Rainbow"),
    (mode::WAVE_GRADIENT, "Wave"),
    (mode::DIAGONAL_FLOW, "Swirl"),
    (mode::PICTURE, "Picture or GIF"),
];

/// Directions the rainbow and the wave move in, `ChargerLedDirection` ids.
pub const DIRECTIONS: [(u8, &str); 4] = [(1, "Right"), (2, "Left"), (3, "Down"), (4, "Up")];
const RIGHT: u8 = 1;
const LEFT: u8 = 2;
const DOWN: u8 = 3;
const UP: u8 = 4;

const BLUE: Rgb = [0, 116, 255];
const VIOLET: Rgb = [195, 32, 230];
const CYAN: Rgb = [22, 230, 249];
const RED: Rgb = [255, 0, 0];
const DARK_BLUE: Rgb = [0, 0, 255];
/// Space Station's shade for a solid effect given no colour.
const FALLBACK: Rgb = [0x21, 0x22, 0x25];

/// How many colours an effect takes.
pub fn colors_for(effect: u8) -> usize {
    match effect {
        mode::SOLID | mode::BREATH | mode::PULSE => 1,
        mode::WAVE_GRADIENT | mode::DIAGONAL_FLOW => 2,
        _ => 0,
    }
}

/// Whether an effect moves in a direction.
pub fn has_direction(effect: u8) -> bool {
    matches!(effect, mode::RAINBOW | mode::WAVE_GRADIENT)
}

/// The period slider's range per effect, as Space Station's interface allows it.
pub fn period_range(effect: u8) -> (u8, u8) {
    match effect {
        mode::GRADIENT => (1, 10),
        mode::DIAGONAL_FLOW | mode::BREATH | mode::WAVE_GRADIENT | mode::RAINBOW | mode::PULSE => (1, 5),
        _ => (1, 1),
    }
}

/// The lighting as the dock stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedConfig {
    pub mode: u8,
    /// 1..100.
    pub brightness: u8,
    /// Frame time in [`PERIOD_MS`] units, bigger is slower.
    pub period: u8,
    pub direction: u8,
    pub colors: Vec<Rgb>,
    /// The frames of a picture or GIF, used by [`mode::PICTURE`] only.
    pub picture: Vec<[Rgb; LED_COUNT]>,
}

impl LedConfig {
    /// Space Station's defaults for an effect, at `brightness`.
    pub fn defaults(effect: u8, brightness: u8) -> LedConfig {
        let (period, colors, direction): (u8, &[Rgb], u8) = match effect {
            mode::DIAGONAL_FLOW => (2, &[VIOLET, CYAN], 0),
            mode::BREATH => (3, &[BLUE], 0),
            mode::GRADIENT => (5, &[], 0),
            mode::WAVE_GRADIENT => (2, &[VIOLET, CYAN], RIGHT),
            mode::RAINBOW => (2, &[], RIGHT),
            mode::PULSE => (2, &[BLUE], 0),
            mode::SOLID => (1, &[BLUE], 0),
            _ => (3, &[], 0),
        };
        LedConfig { mode: effect, brightness, period, direction, colors: colors.to_vec(), picture: vec![] }
    }

    /// The header the dock answers command 20 with: mode, brightness, period, direction, colour
    /// count and the palette, in that order (not the write's).
    pub fn from_header(p: &[u8]) -> Option<LedConfig> {
        let b = p.get(..9)?;
        let count = usize::from(b[8]);
        let colors = if (1..10).contains(&count) {
            (0..count).filter_map(|i| p.get(9 + i * 3..12 + i * 3).map(|c| [c[0], c[1], c[2]])).collect()
        } else {
            vec![]
        };
        Some(LedConfig { mode: b[4], brightness: b[5], period: b[6], direction: b[7], colors, picture: vec![] })
    }

    fn color(&self, i: usize, fallback: Rgb) -> Rgb {
        self.colors.get(i).copied().unwrap_or(fallback)
    }

    /// The frames of the effect.
    pub fn frames(&self) -> Vec<[Rgb; LED_COUNT]> {
        match self.mode {
            mode::SOLID => vec![[self.color(0, FALLBACK); LED_COUNT]],
            mode::BREATH => breath(self.color(0, BLUE)),
            mode::GRADIENT => (0..50).map(|s| [hsl(f64::from(s) / 50.0); LED_COUNT]).collect(),
            mode::RAINBOW => rainbow(self.direction),
            mode::WAVE_GRADIENT => wave(self.color(0, RED), self.color(1, DARK_BLUE), self.direction),
            mode::DIAGONAL_FLOW => swirl(self.color(0, RED), self.color(1, DARK_BLUE)),
            mode::PULSE => pulse(self.color(0, RED)),
            mode::PICTURE if !self.picture.is_empty() => self.picture.clone(),
            _ => vec![[[0; 3]; LED_COUNT]],
        }
    }

    /// The write's byte layout (`ParseFrameLedConfigToArray`): frame count, period, brightness,
    /// mode, direction, colour count, the palette, then every frame.
    pub fn serialise(&self) -> Vec<u8> {
        let frames = self.frames();
        let count = frames.len().min(MAX_FRAMES);
        let mut out =
            vec![count as u8, self.period, self.brightness, self.mode, self.direction, self.colors.len() as u8];
        out.extend(self.colors.iter().flatten());
        out.extend(frames[..count].iter().flatten().flatten());
        out
    }
}

/// JavaScript's `Math.round`: halves go up.
fn js_round(v: f64) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

/// Space Station's HSL at saturation 1 and lightness 0.5, the only values it calls with.
fn hsl(hue: f64) -> Rgb {
    let channel = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        let v = if t < 1.0 / 6.0 {
            6.0 * t
        } else if t < 0.5 {
            1.0
        } else if t < 2.0 / 3.0 {
            (2.0 / 3.0 - t) * 6.0
        } else {
            0.0
        };
        js_round(v * 255.0)
    };
    [channel(hue + 1.0 / 3.0), channel(hue), channel(hue - 1.0 / 3.0)]
}

/// Fades each step by `1 - step / 50` of what is left, then plays back without either end twice.
fn breath(color: Rgb) -> Vec<[Rgb; LED_COUNT]> {
    let fade =
        |c: u8, step: u32| (f64::from(c) - f64::from(c) * f64::from(step) / 50.0).clamp(0.0, 255.0).floor() as u8;
    let mut current = color;
    let mut frames = vec![[current; LED_COUNT]];
    let mut step = 1;
    while current != [0; 3] {
        current = current.map(|c| fade(c, step));
        frames.push([current; LED_COUNT]);
        step += 1;
    }
    let back: Vec<_> = frames.iter().rev().skip(1).take(frames.len().saturating_sub(2)).copied().collect();
    frames.extend(back);
    frames
}

/// Row and column of every LED, in wire order.
fn positions() -> impl Iterator<Item = (usize, usize, usize)> {
    ROWS.iter().enumerate().flat_map(|(row, &len)| (0..len).map(move |col| (row, col, len)))
}

fn rainbow(direction: u8) -> Vec<[Rgb; LED_COUNT]> {
    let rows = ROWS.len() as f64;
    (0..50)
        .map(|step| {
            let offset = f64::from(step) / 50.0;
            let mut frame = [[0; 3]; LED_COUNT];
            for (i, (row, col, len)) in positions().enumerate() {
                let along = match direction {
                    DOWN => 1.0 - row as f64 / rows,
                    UP => row as f64 / rows,
                    RIGHT => 1.0 - col as f64 / len as f64,
                    _ => col as f64 / len as f64,
                };
                frame[i] = hsl((offset + along) % 1.0);
            }
            frame
        })
        .collect()
}

fn wave(first: Rgb, second: Rgb, direction: u8) -> Vec<[Rgb; LED_COUNT]> {
    let rows = ROWS.len();
    (0..50)
        .map(|step| {
            let phase = f64::from(step) / 50.0 * PI * 2.0;
            let mut frame = [[0; 3]; LED_COUNT];
            for (i, (row, col, len)) in positions().enumerate() {
                let (ripple, mut across) = if matches!(direction, DOWN | UP) {
                    let ripple = if direction == UP { row } else { rows - 1 - row };
                    (ripple, row as f64 / (rows - 1) as f64)
                } else {
                    let ripple = if direction == LEFT { col } else { len - 1 - col };
                    (ripple, col as f64 / (len - 1) as f64)
                };
                if matches!(direction, UP | LEFT) {
                    across = 1.0 - across;
                }
                let level = ((phase + ripple as f64 * 0.3).sin() + 1.0) / 2.0;
                frame[i] = std::array::from_fn(|c| {
                    let (a, b) = (f64::from(first[c]), f64::from(second[c]));
                    js_round((a + (b - a) * across) * level)
                });
            }
            frame
        })
        .collect()
}

/// The synthetic grid the swirl and the pulse were written against: centred on the preview circle
/// nearest the middle of a 450x420 box, pitch width/20 across and that times √3/2 down.
fn lattice() -> Vec<(f64, f64)> {
    let (w, h) = (450.0, 420.0);
    let (cx, cy) = (47.56 * w / 100.0, 50.48 * h / 100.0);
    let px = w / 20.0;
    let py = px * 3f64.sqrt() / 2.0;
    positions()
        .map(|(row, col, len)| {
            let left = cx - (len - 1) as f64 * px / 2.0;
            (left + col as f64 * px - cx, cy + (row as f64 - ROWS.len() as f64 / 2.0) * py - cy)
        })
        .collect()
}

fn swirl(first: Rgb, second: Rgb) -> Vec<[Rgb; LED_COUNT]> {
    let grid = lattice();
    (0..50)
        .map(|step| {
            let phase = f64::from(step) / 50.0 * PI * 2.0;
            let mut frame = [[0; 3]; LED_COUNT];
            for (i, (x, y)) in grid.iter().enumerate() {
                let turn = ((y.atan2(*x) + phase + PI) / (PI * 2.0)).rem_euclid(1.0);
                let across = if turn < 0.5 { turn * 2.0 } else { 2.0 - turn * 2.0 };
                frame[i] = std::array::from_fn(|c| {
                    let (a, b) = (f64::from(first[c]), f64::from(second[c]));
                    js_round(a + (b - a) * across)
                });
            }
            frame
        })
        .collect()
}

fn pulse(color: Rgb) -> Vec<[Rgb; LED_COUNT]> {
    let grid = lattice();
    let furthest = grid.iter().map(|(x, y)| x.hypot(*y)).fold(0.0, f64::max).max(1.0);
    (0..50)
        .map(|step| {
            let phase = f64::from(step) / 50.0 * PI * 2.0;
            let mut frame = [[0; 3]; LED_COUNT];
            for (i, (x, y)) in grid.iter().enumerate() {
                let level = ((phase - x.hypot(*y) / furthest * 10.0).sin() + 1.0) / 2.0;
                frame[i] = color.map(|c| js_round(f64::from(c) * level));
            }
            frame
        })
        .collect()
}

/// Size of the canvas a picture is cropped to before its LEDs are read off it.
pub const CANVAS: (usize, usize) = (334, 304);
/// Most frames Space Station's own GIF page sends; the wire allows 255.
pub const PICTURE_FRAMES: usize = 200;

/// The pixel of the canvas each LED shows, row by row. Space Station's own table, transcribed as
/// openflydigi did: it is not a lattice (the pitch slips by two pixels here and there), and a
/// formula would move part of the picture by a pixel.
const LED_PIXELS: [(usize, usize); LED_COUNT] = [
    // row 0
    (48, 32),
    (66, 32),
    (84, 32),
    (104, 32),
    (122, 32),
    (140, 32),
    (158, 32),
    (176, 32),
    (196, 32),
    (214, 32),
    (232, 32),
    (250, 32),
    (268, 32),
    (288, 32),
    // row 1
    (38, 46),
    (58, 46),
    (76, 46),
    (94, 46),
    (112, 46),
    (130, 46),
    (150, 46),
    (168, 46),
    (186, 46),
    (204, 46),
    (222, 46),
    (242, 46),
    (260, 46),
    (278, 46),
    (296, 46),
    // row 2
    (30, 62),
    (48, 62),
    (66, 62),
    (84, 62),
    (104, 62),
    (122, 62),
    (140, 62),
    (158, 62),
    (176, 62),
    (196, 62),
    (214, 62),
    (232, 62),
    (250, 62),
    (268, 62),
    (288, 62),
    (306, 62),
    // row 3
    (38, 78),
    (58, 78),
    (76, 78),
    (94, 78),
    (112, 78),
    (130, 78),
    (150, 78),
    (168, 78),
    (186, 78),
    (204, 78),
    (222, 78),
    (242, 78),
    (260, 78),
    (278, 78),
    (296, 78),
    // row 4
    (48, 94),
    (66, 94),
    (84, 94),
    (104, 94),
    (122, 94),
    (140, 94),
    (158, 94),
    (176, 94),
    (196, 94),
    (214, 94),
    (232, 94),
    (250, 94),
    (268, 94),
    (288, 94),
    // row 5
    (58, 110),
    (76, 110),
    (94, 110),
    (112, 110),
    (130, 110),
    (150, 110),
    (168, 110),
    (186, 110),
    (204, 110),
    (222, 110),
    (242, 110),
    (260, 110),
    (278, 110),
    // row 6
    (66, 126),
    (84, 126),
    (104, 126),
    (122, 126),
    (140, 126),
    (158, 126),
    (176, 126),
    (196, 126),
    (214, 126),
    (232, 126),
    (250, 126),
    (268, 126),
    // row 7
    (76, 142),
    (94, 142),
    (112, 142),
    (130, 142),
    (150, 142),
    (168, 142),
    (186, 142),
    (204, 142),
    (222, 142),
    (242, 142),
    (260, 142),
    // row 8
    (84, 158),
    (104, 158),
    (122, 158),
    (140, 158),
    (158, 158),
    (176, 158),
    (196, 158),
    (214, 158),
    (232, 158),
    (250, 158),
    // row 9
    (94, 174),
    (112, 174),
    (130, 174),
    (150, 174),
    (168, 174),
    (186, 174),
    (204, 174),
    (222, 174),
    (242, 174),
    // row 10
    (104, 190),
    (122, 190),
    (140, 190),
    (158, 190),
    (176, 190),
    (196, 190),
    (214, 190),
    (232, 190),
    // row 11
    (112, 206),
    (130, 206),
    (150, 206),
    (168, 206),
    (186, 206),
    (204, 206),
    (222, 206),
    // row 12
    (122, 222),
    (140, 222),
    (158, 222),
    (176, 222),
    (196, 222),
    (214, 222),
    // row 13
    (130, 238),
    (150, 238),
    (168, 238),
    (186, 238),
    (204, 238),
    // row 14
    (140, 254),
    (158, 254),
    (176, 254),
    (196, 254),
    // row 15
    (150, 270),
    (168, 270),
    (186, 270),
];

/// One frame from a canvas of [`CANVAS`] size, RGB, three bytes a pixel, rows unpadded: the nearest
/// pixel per LED, as Space Station samples it.
pub fn sample(rgb: &[u8]) -> Option<[Rgb; LED_COUNT]> {
    if rgb.len() != CANVAS.0 * CANVAS.1 * 3 {
        return None;
    }
    Some(LED_PIXELS.map(|(x, y)| {
        let i = (y * CANVAS.0 + x) * 3;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }))
}

/// A picture or GIF as the dock takes it: full brightness, frame time from the GIF's mean delay
/// in the dock's 20 ms units (Space Station divides by 10, the dock plays it at 20).
pub fn picture(frames: Vec<[Rgb; LED_COUNT]>, frame_ms: u32) -> LedConfig {
    let period = (frame_ms + PERIOD_MS / 2) / PERIOD_MS;
    LedConfig {
        mode: mode::PICTURE,
        brightness: 100,
        period: period.clamp(1, 255) as u8,
        direction: 0,
        colors: vec![],
        picture: frames.into_iter().take(PICTURE_FRAMES).collect(),
    }
}

/// Centre of each LED in Space Station's 450x420 preview, for drawing the wedge.
pub fn preview_positions() -> Vec<(f32, f32)> {
    positions()
        .map(|(row, col, len)| (225.0 + 22.0 * (col as f32 - (len - 1) as f32 / 2.0), 60.0 + 19.0 * row as f32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wedge_has_162_leds() {
        assert_eq!(ROWS.iter().sum::<usize>(), LED_COUNT);
        assert_eq!(positions().count(), LED_COUNT);
    }

    #[test]
    fn computed_effects_have_their_frame_counts() {
        for (effect, frames) in [(mode::GRADIENT, 50), (mode::RAINBOW, 50), (mode::PULSE, 50), (mode::OFF, 1)] {
            assert_eq!(LedConfig::defaults(effect, 50).frames().len(), frames, "effect {effect}");
        }
    }

    #[test]
    fn breath_fades_to_black_and_back_without_doubled_ends() {
        let frames = LedConfig::defaults(mode::BREATH, 50).frames();
        let n = frames.len();
        assert_eq!(frames[0][0], BLUE);
        assert!(frames.iter().any(|f| f[0] == [0; 3]));
        assert_eq!(frames[1], frames[n - 1]);
        assert_eq!(n % 2, 0);
    }

    #[test]
    fn hsl_hits_the_primaries() {
        assert_eq!(hsl(0.0), [255, 0, 0]);
        assert_eq!(hsl(1.0 / 3.0), [0, 255, 0]);
        assert_eq!(hsl(2.0 / 3.0), [0, 0, 255]);
    }

    #[test]
    fn js_round_takes_halves_up() {
        assert_eq!((js_round(4.5), js_round(5.5), js_round(4.49)), (5, 6, 4));
    }

    #[test]
    fn serialised_header_is_in_write_order() {
        let c = LedConfig {
            mode: mode::SOLID,
            brightness: 80,
            period: 1,
            direction: 0,
            colors: vec![[1, 2, 3]],
            picture: vec![],
        };
        let b = c.serialise();
        assert_eq!(b[..9], [1, 1, 80, mode::SOLID, 0, 1, 1, 2, 3]);
        assert_eq!(b.len(), 6 + 3 + LED_COUNT * 3);
    }

    #[test]
    fn header_read_is_in_read_order() {
        let p = [0x5a, 0xa5, 20, 0, 9, 50, 2, 1, 1, 22, 230, 249];
        let c = LedConfig::from_header(&p).expect("a header");
        assert_eq!((c.mode, c.brightness, c.period, c.direction, c.colors), (9, 50, 2, 1, vec![CYAN]));
    }

    #[test]
    fn the_sampler_reads_one_pixel_per_led() {
        let mut canvas = vec![0u8; CANVAS.0 * CANVAS.1 * 3];
        // the top-left LED reads (48, 32), the bottom-right (186, 270)
        canvas[(32 * CANVAS.0 + 48) * 3] = 200;
        canvas[(270 * CANVAS.0 + 186) * 3 + 2] = 100;
        let frame = sample(&canvas).expect("a full canvas");
        assert_eq!((frame[0], frame[LED_COUNT - 1]), ([200, 0, 0], [0, 0, 100]));
        assert_eq!(sample(&canvas[1..]), None);
    }

    #[test]
    fn a_gif_keeps_its_pace_in_20_ms_units() {
        let still = [[[1; 3]; LED_COUNT]];
        assert_eq!(picture(still.to_vec(), 0).period, 1);
        assert_eq!(picture(vec![still[0]; 300], 100).period, 5);
        assert_eq!(picture(vec![still[0]; 300], 100).picture.len(), PICTURE_FRAMES);
        assert_eq!(picture(still.to_vec(), 100).frames().len(), 1);
    }
}
