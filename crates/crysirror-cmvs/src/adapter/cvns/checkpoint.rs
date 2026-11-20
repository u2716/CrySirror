// SPDX-License-Identifier: MPL-2.0
//! Little-endian fields inside fixed-size, length-checked CVNS scene records.
pub(super) fn put_short(dst: &mut [u8], offset: usize, value: i16) {
    dst[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
pub(super) fn put_word(dst: &mut [u8], offset: usize, value: u32) {
    dst[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
pub(super) fn short(src: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(src[offset..offset + 2].try_into().unwrap())
}
pub(super) fn word(src: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(src[offset..offset + 4].try_into().unwrap())
}
