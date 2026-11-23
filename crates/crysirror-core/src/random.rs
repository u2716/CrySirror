//! Device-independent reproducible MT19937 state for native runtimes and
//! engine simulations. Session seed admission belongs to the host service.

#[derive(Clone, Debug)]
pub struct Mt19937 {
    words: [u32; 624],
    cursor: usize,
}
impl Mt19937 {
    pub fn new(seed: u32) -> Self {
        let mut words = [0; 624];
        words[0] = seed;
        for index in 1..624 {
            words[index] = 1812433253u32
                .wrapping_mul(words[index - 1] ^ (words[index - 1] >> 30))
                .wrapping_add(index as u32);
        }
        Self { words, cursor: 624 }
    }
    pub fn next_u32(&mut self) -> u32 {
        if self.cursor == 624 {
            for index in 0..624 {
                let joined =
                    (self.words[index] & 0x80000000) | (self.words[(index + 1) % 624] & 0x7fffffff);
                self.words[index] = self.words[(index + 397) % 624]
                    ^ (joined >> 1)
                    ^ if joined & 1 == 0 { 0 } else { 0x9908b0df };
            }
            self.cursor = 0;
        }
        let mut value = self.words[self.cursor];
        self.cursor += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c5680;
        value ^= (value << 15) & 0xefc60000;
        value ^= value >> 18;
        value
    }
    /// Native signed remainder convention: positive ranges return[0,n),
    /// negative ranges return(-|n|,0], and a zero range consumes no state.
    pub fn signed_remainder(&mut self, range: i32) -> i32 {
        if range == 0 {
            return 0;
        }
        let value = self.next_u32() % range.unsigned_abs();
        if range < 0 {
            -(value as i32)
        } else {
            value as i32
        }
    }
    pub fn state(&self) -> (&[u32; 624], usize) {
        (&self.words, self.cursor)
    }
    pub fn from_state(words: [u32; 624], cursor: usize) -> crate::Result<Self> {
        if cursor > 624 || words.iter().all(|v| *v == 0) {
            return Err(crate::Error::Invalid("MT19937 state"));
        }
        Ok(Self { words, cursor })
    }
}
