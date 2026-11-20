// SPDX-License-Identifier: MPL-2.0

/// Integer arithmetic used by the PCM16 fader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pcm16GainScale {
    /// Unity is 256; attenuate signed samples by arithmetic right shift.
    #[default]
    Q8,
    /// Unity is 100; divide signed samples toward zero.
    Percent,
}

/// Optional PCM16 fader. Products wrap before division and the final
/// channel gain is capped at unity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioPcm16Gain {
    /// Coefficient scale and sample rounding rule.
    pub scale: Pcm16GainScale,
    /// Secondary gain word.
    pub multiplier: u32,
    /// Left and right attenuation words.
    pub stereo: [u32; 2],
}

impl Default for AudioPcm16Gain {
    fn default() -> Self {
        Self {
            scale: Pcm16GainScale::Q8,
            multiplier: 256,
            stereo: [256; 2],
        }
    }
}

impl AudioPcm16Gain {
    /// Unity coefficients for a percentage-based fader.
    pub fn percent() -> Self {
        Self { scale: Pcm16GainScale::Percent, multiplier: 100, stereo: [100; 2] }
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
