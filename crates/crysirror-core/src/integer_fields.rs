// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Packed integer views over caller-owned32-bit words stored in engine i64
//! values. Pure CPU operations: no allocation, VM, device or ambient state.
//! Narrow fields are unsigned; whole-word reads/writes sign-extend i32.
//! Arithmetic reference: siglus_rs5f358ef runtime/forms/int_list.rs; provenance
//! is retained in docs/shared-native-capability-sources.json.
use core::convert::TryFrom;

pub fn logical_size(storage_words: usize, width: u32) -> Option<usize> {
    if !matches!(width, 1 | 2 | 4 | 8 | 16 | 32) {
        return None;
    }
    storage_words.checked_mul(32 / width as usize)
}
fn location(length: usize, width: u32, index: i64) -> Option<(usize, u32)> {
    logical_size(0, width)?;
    let bit = u64::try_from(index).ok()?.checked_mul(u64::from(width))?;
    let word = usize::try_from(bit / 32).ok()?;
    (word < length).then_some((word, (bit % 32) as u32))
}
/// None reports an invalid width/index; it never synthesizes a stored zero.
pub fn get(words: &[i64], width: u32, index: i64) -> Option<i64> {
    let (word, shift) = location(words.len(), width, index)?;
    if width == 32 {
        return Some(i64::from(words[word] as i32));
    }
    Some(i64::from(
        ((words[word] as u32) >> shift) & ((1u32 << width) - 1),
    ))
}
/// Returns false without mutation for an invalid width/index. An exclusive
/// slice borrow defines the ownership boundary; no worker/global data is kept.
pub fn set(words: &mut [i64], width: u32, index: i64, value: i64) -> bool {
    let Some((word, shift)) = location(words.len(), width, index) else {
        return false;
    };
    if width == 32 {
        words[word] = i64::from(value as i32);
        return true;
    }
    let mask = (1u32 << width) - 1;
    let next = ((words[word] as u32) & !(mask << shift)) | (((value as u32) & mask) << shift);
    words[word] = i64::from(next as i32);
    true
}
/// Fill the intersection of an inclusive signed range and the available view.
/// Invalid indices are ignored; reversed/disjoint ranges change nothing.
/// Work is bounded by the borrowed storage, even for extreme signed endpoints.
pub fn fill_clipped(
    words: &mut [i64],
    width: u32,
    first: i64,
    last: i64,
    value: i64,
) -> Option<usize> {
    let size = i64::try_from(logical_size(words.len(), width)?).ok()?;
    let first = first.max(0);
    let last = last.min(size - 1);
    if first > last {
        return Some(0);
    }
    for index in first..=last {
        set(words, width, index, value);
    }
    usize::try_from(last - first + 1).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fields_cross_word_boundaries_without_touching_their_neighbors() {
        for width in [1, 2, 4, 8, 16, 32] {
            let mut words = [-1i64; 3];
            let count = logical_size(words.len(), width).unwrap();
            for index in 0..count {
                let old = words;
                assert!(set(&mut words, width, index as i64, 0));
                assert_eq!(get(&words, width, index as i64), Some(0));
                for other in 0..count {
                    if other != index {
                        assert_eq!(
                            get(&words, width, other as i64),
                            get(&old, width, other as i64)
                        );
                    }
                }
            }
            assert_eq!(words, [0; 3]);
        }
    }
    #[test]
    fn signed_words_unsigned_subfields_and_invalid_requests_are_explicit() {
        let mut words = [0i64; 2];
        assert!(set(&mut words, 32, 0, i64::from(i32::MIN)));
        assert_eq!(get(&words, 32, 0), Some(i64::from(i32::MIN)));
        assert_eq!(get(&words, 16, 1), Some(32768));
        assert!(set(&mut words, 8, 3, -1));
        assert_eq!(get(&words, 8, 3), Some(255));
        let before = words;
        for (width, index) in [(0, 0), (3, 0), (64, 0), (8, -1), (8, 8), (32, i64::MAX)] {
            assert_eq!(get(&words, width, index), None);
            assert!(!set(&mut words, width, index, 1));
            assert_eq!(words, before);
        }
        assert_eq!(logical_size(usize::MAX, 1), None);
    }
    #[test]
    fn clipped_ranges_are_bounded_and_preserve_fields_outside_the_intersection() {
        let mut words = [-1i64; 2];
        assert_eq!(fill_clipped(&mut words, 8, 3, 4, 0x12), Some(2));
        assert_eq!(words, [0x12ff_ffff, i64::from(0xffff_ff12u32 as i32)]);
        let before = words;
        assert_eq!(fill_clipped(&mut words, 8, 5, 3, 1), Some(0));
        assert_eq!(fill_clipped(&mut words, 3, 0, 1, 1), None);
        assert_eq!(words, before);
        assert_eq!(fill_clipped(&mut words, 1, i64::MIN, i64::MAX, 0), Some(64));
        assert_eq!(words, [0, 0]);
    }
}
