// SPDX-License-Identifier: MPL-2.0
//! CVNS screen shake record (0x422e10), stepped before sprite animations.
use anyhow::{Context, Result, ensure};

pub(super) struct Shake {
    flags: u32,
    amplitude: i32,
    period: u32,
    limit: u32,
    elapsed: u32,
    steps: u32,
    pub paused: bool,
    offset: [i32; 2],
}

impl Shake {
    pub fn new([flags, amplitude, period, limit]: [u32; 4]) -> Self {
        Self {
            flags,
            amplitude: amplitude as i32,
            period,
            limit,
            elapsed: 0,
            steps: 0,
            paused: false,
            offset: [0; 2],
        }
    }

    pub(super) fn record(&self, out:&mut Vec<u8>) -> Result<()> {
        let args=[self.flags,self.amplitude as u32,self.period,self.limit];
        let mut bytes=Vec::with_capacity(56);
        for word in args.into_iter().chain(args).chain([if self.flags & 0xf00 != 0 { self.steps & 1 } else { 0 },
            u32::from(self.paused),self.elapsed,self.steps,self.offset[0] as u32,self.offset[1] as u32]) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        crate::vm::save_slot::csp1::append(out,0x600,&[],&bytes)
    }

    pub(super) fn from_record(bytes:&[u8]) -> Result<Self> {
        ensure!(bytes.len()==56,"invalid CSP1 screen shake length");
        let (raw,_)=bytes.as_chunks::<4>();
        let word=|i:usize|u32::from_le_bytes(raw[i]);
        let mut state=Self::new([word(4),word(5),word(6),word(7)]);
        state.paused=word(9)!=0;
        state.elapsed=word(10);
        state.steps=word(11);
        state.offset=[word(12) as i32,word(13) as i32];
        Ok(state)
    }

    pub fn remaining(&self) -> u32 {
        if self.limit == u32::MAX {
            self.limit
        } else {
            self.limit.wrapping_sub(self.steps)
        }
    }

    /// 0x423150 distinguishes message/ADV parts (types 10 and 30).
    pub fn offset(&self, message_or_part: bool) -> [i32; 2] {
        if self.paused || self.flags & 15 == if message_or_part { 1 } else { 2 } {
            [0; 2]
        } else {
            self.offset
        }
    }

    /// 0x422f40 tests the limit against the step counter *before* incrementing.
    pub fn advance(&mut self, delta: u32, random: &mut u32) -> Result<bool> {
        if self.paused {
            return Ok(false);
        }
        ensure!(self.period != 0, "CVNS shake has a zero period");
        self.elapsed = self.elapsed.wrapping_add(delta);
        let steps = self.elapsed / self.period;
        self.elapsed %= self.period;
        let ended = steps != 0
            && self.limit != u32::MAX
            && (self.steps > self.limit || steps - 1 > self.limit - self.steps);
        self.steps = self.steps.wrapping_add(steps);
        let axis = self.flags & 0xf0;
        if self.flags & 0xf00 != 0 {
            if axis <= 0x10 {
                let forward = self
                    .amplitude
                    .wrapping_mul(2)
                    .wrapping_mul(self.elapsed as i32)
                    .checked_div(self.period as i32)
                    .context("CVNS shake phase overflow")?;
                let value = if forward >= self.amplitude {
                    self.amplitude.wrapping_mul(2).wrapping_sub(forward)
                } else {
                    forward
                };
                self.offset = [0; 2];
                self.offset[usize::from(axis == 0x10)] = if self.steps & 1 == 0 {
                    value.wrapping_neg()
                } else {
                    value
                };
            }
        } else if steps != 0 {
            match axis {
                0 | 0x10 => {
                    let value = &mut self.offset[usize::from(axis == 0x10)];
                    *value = if *value == 0 {
                        if steps & 1 != 0 {
                            self.amplitude
                        } else {
                            self.amplitude.wrapping_neg()
                        }
                    } else if steps & 1 != 0 {
                        value.wrapping_neg()
                    } else {
                        *value
                    };
                }
                0x20 => {
                    let amplitude = self.amplitude as u32;
                    ensure!(amplitude != 0, "CVNS random shake has a zero amplitude");
                    for _ in 0..steps {
                        self.offset = std::array::from_fn(|_| {
                            let sample = super::msvc_rand(random) as u32 % amplitude;
                            sample.wrapping_mul(2).wrapping_sub(amplitude) as i32
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(ended)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_target_phase_and_retirement_follow_the_native_counter() -> Result<()> {
        let mut motion = Shake::new([0x112, 10, 20, 1]);
        let mut random = 1;
        assert!(!motion.advance(10, &mut random)?);
        assert_eq!(motion.offset(true), [0, -10]);
        assert_eq!(motion.offset(false), [0, 0]);
        motion.paused = true;
        assert!(!motion.advance(90, &mut random)?);
        assert_eq!(motion.offset(true), [0, 0]);
        motion.paused = false;
        assert!(!motion.advance(20, &mut random)?);
        assert_eq!(motion.offset(true), [0, 10]);
        assert_eq!(motion.remaining(), 0);
        assert!(!motion.advance(20, &mut random)?);
        assert!(motion.advance(20, &mut random)?);
        Ok(())
    }
}
