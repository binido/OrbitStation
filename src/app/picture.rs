//! Pictures and GIFs for the dock: decoded, filled into the dock's canvas the way Space Station's
//! first framing does (cover, then crop the middle), and read off one pixel per LED.

use std::io::Cursor;
use std::path::Path;

use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, ImageFormat};

use crate::flydigi::dock_led::{self, CANVAS, LED_COUNT, LedConfig, PICTURE_FRAMES, Rgb};

/// File types the picker offers.
pub const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];
/// Frame time of a GIF that does not say, the 10 frames a second browsers fall back to.
const DEFAULT_FRAME_MS: u32 = 100;

/// The dock lighting showing `path`, a still or the first [`PICTURE_FRAMES`] frames of a GIF.
///
/// # Errors
/// A readable message when the file cannot be read or decoded.
pub fn load(path: &Path) -> Result<LedConfig, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if image::guess_format(&bytes).is_ok_and(|f| f == ImageFormat::Gif) {
        let decoder = GifDecoder::new(Cursor::new(&bytes)).map_err(|e| e.to_string())?;
        let mut frames = vec![];
        let mut total_ms = 0u32;
        for frame in decoder.into_frames().take(PICTURE_FRAMES) {
            let frame = frame.map_err(|e| e.to_string())?;
            let (n, d) = frame.delay().numer_denom_ms();
            total_ms = total_ms.saturating_add(n.checked_div(d).unwrap_or(DEFAULT_FRAME_MS));
            frames.push(fill(&DynamicImage::ImageRgba8(frame.into_buffer())));
        }
        let mean = match u32::try_from(frames.len()) {
            Ok(n) if n > 0 && total_ms > 0 => total_ms / n,
            _ => DEFAULT_FRAME_MS,
        };
        Ok(dock_led::picture(frames, mean))
    } else {
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        Ok(dock_led::picture(vec![fill(&img)], 0))
    }
}

/// Scales `img` to cover the canvas, crops the middle and samples the LEDs.
fn fill(img: &DynamicImage) -> [Rgb; LED_COUNT] {
    let (w, h) = (CANVAS.0 as u32, CANVAS.1 as u32);
    let scale = (w as f32 / img.width().max(1) as f32).max(h as f32 / img.height().max(1) as f32);
    let (sw, sh) =
        (((img.width() as f32 * scale).ceil() as u32).max(w), ((img.height() as f32 * scale).ceil() as u32).max(h));
    let scaled = img.resize_exact(sw, sh, FilterType::Triangle);
    let canvas = scaled.crop_imm((sw - w) / 2, (sh - h) / 2, w, h).to_rgb8();
    // the canvas is exactly CANVAS in size, so sampling cannot fail
    dock_led::sample(canvas.as_raw()).unwrap_or([[0; 3]; LED_COUNT])
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb as Px, RgbImage};

    #[test]
    fn a_wide_picture_is_cropped_to_its_middle() {
        // left third red, middle third green, right third blue, twice as wide as tall
        let img = RgbImage::from_fn(600, 300, |x, _| match x / 200 {
            0 => Px([255, 0, 0]),
            1 => Px([0, 255, 0]),
            _ => Px([0, 0, 255]),
        });
        let frame = fill(&DynamicImage::ImageRgb8(img));
        // the bottom row of the wedge sits mid-canvas, inside the green third
        assert_eq!(frame[LED_COUNT - 2], [0, 255, 0]);
    }

    #[test]
    fn a_gif_loads_every_frame_at_its_pace() {
        use image::codecs::gif::GifEncoder;
        use image::{Delay, Frame, Rgba, RgbaImage};
        let path = std::env::temp_dir().join(format!("orbitstation-test-{}.gif", std::process::id()));
        {
            let file = std::fs::File::create(&path).expect("temp dir is writable");
            let mut gif = GifEncoder::new(file);
            for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
                let img = RgbaImage::from_pixel(40, 40, Rgba(color));
                let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(120, 1));
                gif.encode_frame(frame).expect("a frame encodes");
            }
        }
        let c = load(&path).expect("the gif decodes");
        let _ = std::fs::remove_file(&path);
        assert_eq!(c.picture.len(), 2);
        assert_eq!((c.picture[0][0], c.picture[1][0]), ([255, 0, 0], [0, 0, 255]));
        assert_eq!(c.period, 6);
    }
}
