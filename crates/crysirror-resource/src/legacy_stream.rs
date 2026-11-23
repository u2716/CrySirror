// Copyright (c) 2026 CrySirror contributors.
// MPL-2.0; legacy MT profile follows the published 1998 MT19937 algorithm.
//! Deterministic legacy resource transforms. These are format compatibility
//! kernels, not cryptographic security or game authentication primitives.
/// Stateful published 1998 MT19937 profile. Initialization is the 69069
/// recurrence, distinct from MT2002 and split-high-word INT variants. Cloning
/// retains the exact cursor across a twist boundary; no process/global state.
#[derive(Clone, Debug)]
pub struct Mt19937_1998 {
    state: [u32; 624],
    cursor: usize,
}
impl Mt19937_1998 {
    /// Retains the caller's complete seed, including zero. A native API that
    /// requires odd/nonzero seeds must apply that policy before construction.
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            state[i] = state[i - 1].wrapping_mul(69069);
        }
        Self { state, cursor: 624 }
    }
    pub fn next_u32(&mut self) -> u32 {
        if self.cursor == 624 {
            for i in 0..624 {
                self.state[i] = transition(
                    self.state[i],
                    self.state[(i + 1) % 624],
                    self.state[(i + 397) % 624],
                );
            }
            self.cursor = 0;
        }
        let value = self.state[self.cursor];
        self.cursor += 1;
        temper(value)
    }
    /// One complete generator output per byte, consuming only its low byte.
    pub fn xor_low_bytes(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte ^= self.next_u32() as u8;
        }
    }
}
/// Classic split-high-word MT initialization. Each state word joins the high
/// halves of two consecutive wrapping69069+1 values. This is distinct from
/// the1998 direct recurrence and modern MT2002 initialization. It shares the
/// exact same twist/temper kernel, retains seed0, and has no global state.
#[derive(Clone, Debug)]
pub struct Mt19937Split {
    generator: Mt19937_1998,
}
impl Mt19937Split {
    pub fn new(mut seed: u32) -> Self {
        let mut state = [0; 624];
        for word in &mut state {
            *word = split_seed_word(&mut seed);
        }
        Self {
            generator: Mt19937_1998 { state, cursor: 624 },
        }
    }
    /// Produce one full tempered word; cloning retains its exact cursor.
    pub fn next_u32(&mut self) -> u32 {
        self.generator.next_u32()
    }
    /// Stateless first word without constructing/twisting all624 words. Used
    /// by archive record keys; it is exactly new(seed).next_u32().
    pub fn first_u32(mut seed: u32) -> u32 {
        let first = split_seed_word(&mut seed);
        let second = split_seed_word(&mut seed);
        let mut middle = 0;
        for _ in 2..=397 {
            middle = split_seed_word(&mut seed);
        }
        temper(transition(first, second, middle))
    }
}
fn split_seed_word(seed: &mut u32) -> u32 {
    let mut word = *seed & 0xffff0000;
    *seed = seed.wrapping_mul(69069).wrapping_add(1);
    word |= *seed >> 16;
    *seed = seed.wrapping_mul(69069).wrapping_add(1);
    word
}
fn transition(first: u32, second: u32, middle: u32) -> u32 {
    let pair = (first & 0x80000000) | (second & 0x7fffffff);
    middle ^ (pair >> 1) ^ if pair & 1 != 0 { 0x9908b0df } else { 0 }
}
fn temper(mut value: u32) -> u32 {
    value ^= value >> 11;
    value ^= (value << 7) & 0x9d2c5680;
    value ^= (value << 15) & 0xefc60000;
    value ^= value >> 18;
    value
}
/// Stateless envelope convenience over the same shared stateful kernel.
pub fn mt19937_1998_xor(bytes: &mut [u8], seed: u32) {
    Mt19937_1998::new(seed).xor_low_bytes(bytes);
}
#[cfg(test)]
mod tests {
    #[test]
    fn published_1998_profile_matches_reference_c_oracle() {
        // Unmodified libsys4 mt19937int.c, b14839f, seed 0x5d3e3;
        // one genrand low byte per byte. No game-derived fixture.
        let expected = [
            0x28, 0xb0, 0x50, 0xe9, 0xd6, 0xcf, 0x93, 0x5b, 0xe4, 0xed, 0x7e, 0xa1, 0xfe, 0x72,
            0xc1, 0xae, 0x25, 0x65, 0xda, 0xde, 0x68, 0x5e, 0x1b, 0x6d, 0xbb, 0x95, 0x88, 0x11,
            0xf1, 0xbe, 0x72, 0xa4,
        ];
        let mut bytes = [0; 32];
        super::mt19937_1998_xor(&mut bytes, 0x5d3e3);
        assert_eq!(bytes, expected);
        super::mt19937_1998_xor(&mut bytes, 0x5d3e3);
        assert_eq!(bytes, [0; 32]);
    }
    #[test]
    fn stateful_profile_matches_original_word_oracle_across_twists_and_clone() {
        // Unmodified pinned libsys4 mt19937int.c compiled locally; full words,
        // not a vector produced by this Rust implementation. No game fixture.
        let expected = [
            (0, 3510405877),
            (1, 4290933890),
            (2, 2191955339),
            (3, 564929546),
            (4, 152112058),
            (5, 4262624192),
            (6, 2687398418),
            (7, 268830360),
            (623, 730882493),
            (624, 2222118351),
            (1247, 3705639611),
            (1248, 1244107692),
        ];
        let mut mt = super::Mt19937_1998::new(4357);
        let mut checkpoint = None;
        for i in 0..1249 {
            let word = mt.next_u32();
            if let Some((_, expected)) = expected.iter().find(|(at, _)| *at == i) {
                assert_eq!(word, *expected, "native output {i}");
            }
            if i == 622 {
                checkpoint = Some(mt.clone());
            }
        }
        let mut checkpoint = checkpoint.unwrap();
        assert_eq!(checkpoint.next_u32(), 730882493);
        assert_eq!(checkpoint.next_u32(), 2222118351);
        let mut full = [0; 700];
        super::mt19937_1998_xor(&mut full, 1);
        let mut split = [0; 700];
        let mut mt = super::Mt19937_1998::new(1);
        mt.xor_low_bytes(&mut split[..317]);
        mt.xor_low_bytes(&mut split[317..]);
        assert_eq!(split, full);
    }
    #[test]
    fn split_high_profile_matches_independent_c_oracle_at_twist_boundaries() {
        //Unmodified libsys4 mt19937_genrand, pinnedb14839f23af0963abbdb17f7cac57a36fd8e9bd2,
        //linked to an independent split-seeding driver. No game bytes or seed
        //values derived from secrets; these are public numeric test vectors.
        let cases = [
            (
                0,
                [
                    477480905, 3667282373, 2229421029, 899856095, 1355199074, 4217746537, 451286093,
                ],
            ),
            (
                1,
                [
                    3556162021, 4012392791, 221657543, 2524562135, 2383753642, 232418228,
                    1157195907,
                ],
            ),
            (
                4357,
                [
                    2867219139, 1585203162, 3113124129, 589403970, 1044672925, 330565676,
                    4125320456,
                ],
            ),
            (
                0xdeadbeef,
                [
                    1446745592, 353357745, 905235918, 1868098800, 4162753390, 2545225832,
                    2871472611,
                ],
            ),
        ];
        let positions = [0, 1, 2, 623, 624, 1247, 1248];
        for (seed, expected) in cases {
            assert_eq!(super::Mt19937Split::first_u32(seed), expected[0]);
            let mut mt = super::Mt19937Split::new(seed);
            let mut checkpoint = None;
            for index in 0..=1248 {
                let word = mt.next_u32();
                if let Some(at) = positions.iter().position(|p| *p == index) {
                    assert_eq!(word, expected[at], "seed{seed} index{index}");
                }
                if index == 622 {
                    checkpoint = Some(mt.clone());
                }
            }
            let mut checkpoint = checkpoint.unwrap();
            assert_eq!(checkpoint.next_u32(), expected[3]);
            assert_eq!(checkpoint.next_u32(), expected[4]);
        }
    }
}
/// Decode complete little-endian words with independently advancing XOR and
/// additive masks. Arithmetic wraps exactly at 32 bits. Used by native script
/// and resource profiles; the caller supplies all policy and seed values.
pub fn rolling_word_xor_add(
    bytes: &mut [u8],
    mut xor: u32,
    xor_increment: u32,
    add_increment: u32,
) -> anyhow::Result<()> {
    anyhow::ensure!(bytes.len() % 4 == 0, "rolling word transform alignment");
    let mut add = 0u32;
    for word in bytes.chunks_exact_mut(4) {
        let value = u32::from_le_bytes(word.try_into().unwrap());
        word.copy_from_slice(&(value ^ xor).wrapping_add(add).to_le_bytes());
        xor = xor.wrapping_add(xor_increment);
        add = add.wrapping_add(add_increment);
    }
    Ok(())
}
