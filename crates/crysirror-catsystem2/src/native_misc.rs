// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Small KCS natives with no engine state: calendar and rectangle arithmetic, text
//! trimming, and calls that act only on a handle the player never produced.
use crate::kcs::Machine;
use anyhow::Result;

type Outcome = (Option<u32>, Vec<(u32, Vec<u8>)>);
fn word(address: u32, value: u32) -> (u32, Vec<u8>) {
    (address, value.to_le_bytes().to_vec())
}
/// 6A7450: day of the week from year, month and day.
pub(crate) fn day_of_week(year: i32, month: i32, day: i32) -> i32 {
    let (y, m) = if month > 2 { (year, month + 1) } else { (year - 1, month + 13) };
    let v = y / 400 + (f64::from(y) * 365.25) as i32 - (f64::from(m) * -30.6) as i32 - y / 100 + day - 429;
    (1 - 7 * (v / 7) + v) % 7
}
/// Civil UTC date from a UNIX time; the original uses GetLocalTime.
pub(crate) fn civil(seconds: u64) -> [u32; 7] {
    let days = (seconds / 86400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as u32;
    let week = (days + 4).rem_euclid(7) as u32;
    let rest = (seconds % 86400) as u32;
    [year, month, week, day, rest / 3600, rest / 60 % 60, rest % 60]
}
/// 6A5280: bytes of one character, UTF-8 by its lead bits.
fn utf8_len(bytes: &[u8]) -> usize {
    let lead = bytes.first().copied().unwrap_or(0);
    let count = (0..6).take_while(|bit| lead >> (7 - bit) & 1 != 0).count();
    if count <= 1 { 1 } else { count.min(bytes.len()).max(1) }
}
/// Natives that test their first argument and do nothing for null. The creators of
/// these objects are not available yet, so scripts only ever hold null here.
pub(crate) fn null_handle(id: u32, args: &[u32]) -> Option<Option<u32>> {
    if args.first().copied().unwrap_or(0) != 0 {
        return None;
    }
    match id {
        269 | 266 | 998 | 635 | 636 | 639 | 640 | 608 | 980 | 1033 | 872 | 873 | 877 | 685 => Some(Some(0)),
        267 | 270 | 417 | 788 | 684 | 697 | 700 | 785 | 876 | 977 | 870 | 878 | 634 | 723 | 726 | 727 | 730 | 737 | 713 | 711 => Some(None),
        _ => None,
    }
}
pub(crate) const IDS: &[u32] = &[11, 218, 786, 875, 1045, 10, 1039, 445, 629, 932, 933, 845, 847, 852, 566];
pub(crate) fn apply(id: u32, args: &[u32], machine: &Machine) -> Result<Outcome> {
    let arg = |at: usize| args.get(at).copied().unwrap_or(0);
    Ok(match id {
        11 => {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            (None, civil(now).iter().enumerate().map(|(at, value)| word(arg(at), *value)).collect())
        }
        218 => {
            // IntersectRect of the rectangle at the pointer with the argument rectangle.
            let bytes = machine.read_bytes(arg(0), 16)?;
            let read = |at: usize| i32::from_le_bytes(bytes[at * 4..at * 4 + 4].try_into().unwrap());
            let (l, t, r, b) = (read(0).max(arg(1) as i32), read(1).max(arg(2) as i32), read(2).min(arg(3) as i32), read(3).min(arg(4) as i32));
            let rect = if l < r && t < b { [l, t, r, b] } else { [0; 4] };
            (None, vec![(arg(0), rect.iter().flat_map(|v| v.to_le_bytes()).collect())])
        }
        786 => (Some(day_of_week(arg(0) as i32, arg(1) as i32, arg(2) as i32) as u32), vec![]),
        875 => (None, vec![word(arg(1), arg(0) as u16 as i16 as i32 as u32), word(arg(2), (arg(0) >> 16) as u16 as i16 as i32 as u32)]),
        1045 => {
            let text = machine.read_cstring(arg(0), 65534)?;
            let mut at = 0;
            while at < text.len() {
                let next = at + utf8_len(&text[at..]);
                if next >= arg(1) as usize {
                    break;
                }
                at = next;
            }
            (Some(at as u32), vec![(arg(0) + at as u32, vec![0, 0])])
        }
        // No host clipboard, dialog or process launch is offered to game scripts, and
        // the debug and window-state hooks have no state here.
        10 | 1039 | 445 | 629 | 932 | 933 | 845 | 847 => (None, vec![]),
        // Render-manager and startup configuration words that read as clear.
        852 | 566 => (Some(0), vec![]),
        _ => anyhow::bail!("CatSystem2 misc native {id} is not in this table"),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_matches_known_dates() {
        // 2026-11-03 is a Tuesday.
        assert_eq!(day_of_week(2026, 11, 3), 2);
        assert_eq!(day_of_week(2000, 1, 1), 6);
        assert_eq!(civil(0), [1970, 1, 4, 1, 0, 0, 0]);
        assert_eq!(civil(1_793_664_000)[..4], [2026, 11, 2, 3]);
    }
    #[test]
    fn null_handles_are_quiet_but_live_ones_are_not() {
        assert_eq!(null_handle(269, &[0]), Some(Some(0)));
        assert_eq!(null_handle(270, &[0]), Some(None));
        assert_eq!(null_handle(270, &[5]), None);
        assert_eq!(null_handle(1, &[0]), None);
    }
}
