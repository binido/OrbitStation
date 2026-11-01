//! Trigger travel window at 123, 7 bytes per trigger on a 0..255 scale:
//! type, start, p1 x, p1 y, p2 x, p2 y, end. The game still gets the full 0..255 range, squeezed
//! into the part of the travel between start and end. Space Station mirrors the points onto the window.

const OFF: usize = 123;
const ENTRY: usize = 7;
/// Trigger names.
pub const SIDES: [&str; 2] = ["LT", "RT"];

/// (start, end) of the travel, 0..255.
pub fn get(blob: &[u8], side: usize) -> (u8, u8) {
    let e = &blob[OFF + side * ENTRY..][..ENTRY];
    (e[1], e[6])
}

/// Stores the travel window, the ends may come in any order.
pub fn set(blob: &mut [u8], side: usize, start: u8, end: u8) {
    let (lo, hi) = (start.min(end), start.max(end));
    blob[OFF + side * ENTRY + 1..][..6].copy_from_slice(&[lo, lo, lo, hi, hi, hi]);
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    #[test]
    fn window_mirrors_its_points() {
        let mut blob = vec![0u8; SIZE];
        // factory value read from a Vader 5 Pro
        blob[123..137].copy_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255]);
        assert_eq!((get(&blob, 0), get(&blob, 1)), ((0, 255), (0, 255)));
        set(&mut blob, 1, 200, 20);
        assert_eq!(blob[130..137], [0, 20, 20, 20, 200, 200, 200]);
        assert_eq!(get(&blob, 1), (20, 200));
    }
}
