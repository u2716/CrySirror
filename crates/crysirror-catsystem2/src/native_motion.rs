// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS MotionSet natives 299/301/302: the 36-word pose block crosses the VM as eight
//! pointers plus two values, in the layout 788D60 and 788DF0 copy.
use crate::{kcs::Machine, motion::PoseWords};
use anyhow::Result;

/// (argument index, word offset, word count), with value arguments marked count 0.
const LAYOUT: [(usize, usize, usize); 7] = [(1, 0, 3), (2, 3, 3), (3, 6, 3), (4, 9, 16), (5, 25, 3), (7, 29, 3), (8, 32, 3)];
pub(crate) fn read_pose(machine: &Machine, a: &[u32]) -> Result<PoseWords> {
    let mut words = [0u32; 36];
    for (argument, offset, count) in LAYOUT {
        for i in 0..count {
            words[offset + i] = machine.read_word(a[argument] + i as u32 * 4)?;
        }
    }
    words[28] = a[6];
    words[35] = a[9];
    Ok(PoseWords(words))
}
pub(crate) fn pose_writes(a: &[u32], pose: PoseWords) -> Vec<(u32, Vec<u8>)> {
    let bytes = |range: &[u32]| range.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
    let mut writes: Vec<(u32, Vec<u8>)> = LAYOUT.iter().map(|(argument, offset, count)| (a[*argument], bytes(&pose.0[*offset..*offset + *count]))).collect();
    writes.push((a[6], pose.0[28].to_le_bytes().to_vec()));
    if a[9] != 0 {
        writes.push((a[9], pose.0[35].to_le_bytes().to_vec()));
    }
    writes
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pose_layout_covers_every_word_once() {
        let mut seen = [false; 36];
        for (_, offset, count) in LAYOUT {
            for flag in &mut seen[offset..offset + count] {
                assert!(!*flag);
                *flag = true;
            }
        }
        seen[28] = true;
        seen[35] = true;
        assert!(seen.iter().all(|flag| *flag));
    }
    #[test]
    fn writes_skip_the_optional_last_word() {
        let a = [0, 100, 200, 300, 400, 500, 600, 700, 800, 0];
        assert_eq!(pose_writes(&a, PoseWords([0; 36])).len(), 8);
    }
}
