// SPDX-License-Identifier: MPL-2.0

/// Optional legacy PCM16 fader with a wrapping Q8 secondary multiplier and
/// two raw Q8 attenuation coefficients. Unity is 256. The unity clamp
/// applies to the computed channel gain after all wrapping products.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioPcm16Gain {
    /// Secondary Q8 gain word, initially 256.
    pub multiplier: u32,
    /// Left and right Q8 gain words, initially both 256.
    pub stereo: [u32; 2],
}

impl Default for AudioPcm16Gain {
    fn default() -> Self {
        Self {
            multiplier: 256,
            stereo: [256; 2],
        }
    }
}

/// Update either coefficient without replacing the source or resetting a fade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPcm16GainUpdate {
    /// Replace only the secondary gain.
    Multiplier(u32),
    /// Replace both stereo words together.
    Stereo([u32; 2]),
}
