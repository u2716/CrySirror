// SPDX-License-Identifier: MPL-2.0
//! Marsaglia xorshift128 integer sequence (2003). One owner keeps the state;
//! the serialized words are the complete stream position.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Xorshift128 {
    /// x, y, z, w in Marsaglia's notation.
    pub words: [u32; 4],
}
impl Default for Xorshift128 {
    /// The published seeds 123456789, 362436069, 521288629, 88675123.
    fn default() -> Self {
        Self { words: [123456789, 362436069, 521288629, 88675123] }
    }
}
impl Xorshift128 {
    /// Advance once and return the new `w`.
    pub fn next_u32(&mut self) -> u32 {
        let [x, y, z, w] = self.words;
        let t = x ^ (x << 11);
        let next = w ^ (w >> 19) ^ t ^ (t >> 8);
        self.words = [y, z, w, next];
        next
    }
    /// The 31-bit output of `next_u32() & 0x7fffffff`.
    pub fn next_i31(&mut self) -> i32 {
        (self.next_u32() & 0x7fff_ffff) as i32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_sequence_start() {
        let mut r = Xorshift128::default();
        // First outputs of the reference xor128() with its default seeds.
        assert_eq!(r.next_u32(), 3701687786);
        assert_eq!(r.next_u32(), 458299110);
    }
}
