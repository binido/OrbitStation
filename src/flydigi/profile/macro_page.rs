//! Macros of protocol 3.1 (Apex 5), kept inside the mapping blob rather than in a store of their
//! own. Layout from openflydigi's notes, measured on an Apex 5:
//!
//! - page at 230, 538 bytes (`m_fdg_macro_page_struct_t`): count (1..5, anything else is none),
//!   five body offsets in 4-byte words after the 6-byte header, then the bodies;
//! - body: trigger key, step count u16, kind, then 4 bytes per step: time since the macro started
//!   u16 in 10 ms ticks, key, event;
//! - one repeat interval per slot at 820, in 10 ms, ff for a slot never written.
//!
//! The page plays only after the profile is applied, which every write here ends with.

use super::ProfileError;
use super::macros::{Macro, Step};

const PAGE: usize = 230;
const PAGE_LEN: usize = 538;
const HEADER: usize = 6;
/// Bytes a body spends on its own header, and each step.
const WORD: usize = 4;
const INTERVALS: usize = 820;
const UNSET: u8 = 0xff;
/// Step times and repeat intervals are stored in this unit.
const TICK_MS: u16 = 10;

/// Most macros a page holds.
pub const MAX: usize = 5;

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

/// The macros of a mapping blob; an empty or unreadable page reads as none.
pub fn parse(map: &[u8]) -> Vec<Macro> {
    let Some(page) = map.get(PAGE..PAGE + PAGE_LEN) else { return vec![] };
    let count = usize::from(page[0]);
    if !(1..=MAX).contains(&count) {
        return vec![];
    }
    let mut out = vec![];
    for i in 0..count {
        let at = HEADER + usize::from(page[1 + i]) * WORD;
        let Some(head) = page.get(at..at + WORD) else { break };
        let n = usize::from(u16_at(head, 1));
        let Some(body) = page.get(at + WORD..at + WORD + n * WORD) else { break };
        let mut prev = 0;
        let steps = body
            .chunks(WORD)
            .map(|s| {
                let t = u16_at(s, 0).saturating_mul(TICK_MS);
                let step = Step { delay_ms: t.saturating_sub(prev), key: s[2], event: s[3] };
                prev = t;
                step
            })
            .collect();
        let interval = map.get(INTERVALS + i).copied().unwrap_or(UNSET);
        let interval_ms = if interval == UNSET { 0 } else { u16::from(interval) * TICK_MS };
        out.push(Macro { key: head[0], kind: head[3], interval_ms, steps });
    }
    out
}

/// Writes `list` into the page of `map`, step times rounded to 10 ms. Interval slots past the list
/// are cleared, so a removed macro's interval does not pass to the one that takes its slot.
///
/// # Errors
/// [`ProfileError`] when there are more than five macros, they do not fit the page, or one runs
/// longer than the 16-bit tick count allows.
pub fn encode(map: &mut [u8], list: &[Macro]) -> Result<(), ProfileError> {
    if list.len() > MAX {
        return Err(ProfileError::TooManyMacros(MAX));
    }
    let mut page = vec![UNSET; PAGE_LEN];
    page[0] = list.len() as u8;
    let mut word = 0usize;
    for (i, m) in list.iter().enumerate() {
        let at = HEADER + word * WORD;
        if at + WORD * (1 + m.steps.len()) > PAGE_LEN {
            return Err(ProfileError::MacrosTooLarge);
        }
        page[1 + i] = u8::try_from(word).map_err(|_| ProfileError::MacrosTooLarge)?;
        let n = u16::try_from(m.steps.len()).map_err(|_| ProfileError::MacrosTooLarge)?;
        page[at] = m.key;
        page[at + 1..at + 3].copy_from_slice(&n.to_le_bytes());
        page[at + 3] = m.kind;
        let mut ticks = 0u16;
        for (j, s) in m.steps.iter().enumerate() {
            ticks = ticks.checked_add(s.delay_ms.div_ceil(TICK_MS)).ok_or(ProfileError::MacroTooLong)?;
            let p = at + WORD * (1 + j);
            page[p..p + 2].copy_from_slice(&ticks.to_le_bytes());
            page[p + 2] = s.key;
            page[p + 3] = s.event;
        }
        word += 1 + m.steps.len();
    }
    map[PAGE..PAGE + PAGE_LEN].copy_from_slice(&page);
    for slot in 0..MAX {
        map[INTERVALS + slot] = list.get(slot).map_or(UNSET, |m| (m.interval_ms / TICK_MS).min(254) as u8);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::map::SIZE;
    use super::*;

    fn tap(key: u8, gap: u16) -> Vec<Step> {
        vec![Step { delay_ms: 0, key, event: 1 }, Step { delay_ms: gap, key, event: 0 }]
    }

    fn factory() -> Vec<u8> {
        let mut map = vec![0xff; SIZE];
        // an untouched Apex 5: no macros, every interval 30 ms
        map[INTERVALS..INTERVALS + 5].fill(3);
        map
    }

    #[test]
    fn an_untouched_page_has_no_macros() {
        assert!(parse(&factory()).is_empty());
    }

    #[test]
    fn macros_land_where_the_firmware_reads_them() {
        let mut map = factory();
        let m1 = Macro { key: 18, kind: 1, interval_ms: 0, steps: tap(4, 40) };
        let m2 = Macro { key: 19, kind: 2, interval_ms: 300, steps: tap(7, 60) };
        encode(&mut map, &[m1.clone(), m2.clone()]).expect("two taps fit");
        // count, offsets 0 and 3 words, then M1's body: key, 2 steps, once, press A at 0, release at 4 ticks
        assert_eq!(map[230..236], [2, 0, 3, 0xff, 0xff, 0xff]);
        assert_eq!(map[236..248], [18, 2, 0, 1, 0, 0, 4, 1, 4, 0, 4, 0]);
        assert_eq!(map[820..825], [0, 30, 0xff, 0xff, 0xff]);
        assert_eq!(parse(&map), vec![m1, m2]);
    }

    #[test]
    fn step_times_round_up_to_ticks() {
        let mut map = factory();
        let m = Macro { key: 18, kind: 1, interval_ms: 0, steps: tap(4, 45) };
        encode(&mut map, std::slice::from_ref(&m)).expect("a tap fits");
        assert_eq!(parse(&map)[0].steps[1].delay_ms, 50);
    }

    #[test]
    fn six_macros_are_refused() {
        let one = Macro { key: 18, kind: 1, interval_ms: 0, steps: vec![] };
        assert_eq!(encode(&mut factory(), &vec![one; 6]), Err(ProfileError::TooManyMacros(5)));
    }

    #[test]
    fn a_page_holds_133_words() {
        let long =
            |n| Macro { key: 18, kind: 1, interval_ms: 0, steps: vec![Step { delay_ms: 0, key: 4, event: 1 }; n] };
        assert!(encode(&mut factory(), &[long(132)]).is_ok());
        assert_eq!(encode(&mut factory(), &[long(133)]), Err(ProfileError::MacrosTooLarge));
    }
}
