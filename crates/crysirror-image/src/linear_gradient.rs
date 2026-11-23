//! Straight-byte linear channel ramps with explicit sample and quantization
//! rules. Unselected channels retain the immutable input's stored bytes.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinearGradientSample {
    /// start + trunc((end-start)*position/length).
    StartTruncate,
    /// Sample at the texel centre, then round the final nonnegative byte.
    CenterRound,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearGradientByte {
    pub vertical: bool,
    pub start: [u8; 4],
    pub end: [u8; 4],
    pub write_mask: [bool; 4],
    pub sample: LinearGradientSample,
}
impl LinearGradientByte {
    pub fn validate(&self) -> Result<()> {
        Ok(())
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        input.validate()?;
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "linear gradient pixel budget"
        );
        check()?;
        let mut output = input.clone();
        let length = i64::from(if self.vertical {
            input.height
        } else {
            input.width
        });
        for y in 0..input.height {
            check()?;
            for x in 0..input.width {
                let position = i64::from(if self.vertical { y } else { x });
                let at = ((y * input.width + x) * 4) as usize;
                for channel in 0..4 {
                    if !self.write_mask[channel] {
                        continue;
                    }
                    let start = i64::from(self.start[channel]);
                    let delta = i64::from(self.end[channel]) - start;
                    let value = match self.sample {
                        LinearGradientSample::StartTruncate => start + delta * position / length,
                        LinearGradientSample::CenterRound => {
                            (start * length * 2 + delta * (position * 2 + 1) + length)
                                / (length * 2)
                        }
                    };
                    output.rgba[at + channel] = value.clamp(0, 255) as u8;
                }
            }
        }
        check()?;
        Ok(output)
    }
}
