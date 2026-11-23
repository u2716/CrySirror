//! Stored-byte 3x3 feedback convolution, scanned top-to-bottom, left-to-right.
//! Each write participates in later samples, with one truncated division.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackBoxBlur {
    pub passes: u8,
    /// Include edge pixels with the actual in-bounds neighbour count. False
    /// retains the existing interior scan and native border-repair profile.
    #[serde(default)]
    pub clipped_edges: bool,
}
impl FeedbackBoxBlur {
    pub fn validate(&self) -> Result<()> {
        ensure!((1..=32).contains(&self.passes), "feedback blur pass limit");
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
            input.width >= 2 && input.height >= 2,
            "feedback blur needs two rows and columns"
        );
        ensure!(
            u64::from(input.width) * u64::from(input.height) <= MAX_EFFECT_PIXELS,
            "feedback blur pixel budget"
        );
        check()?;
        let mut output = input.clone();
        let (width, height) = (input.width as usize, input.height as usize);
        if self.clipped_edges {
            for _ in 0..self.passes {
                for y in 0..height {
                    check()?;
                    for x in 0..width {
                        let mut sum=[0u16;4];let mut count=0u16;
                        for row in y.saturating_sub(1)..=(y+1).min(height-1) {
                            for column in x.saturating_sub(1)..=(x+1).min(width-1) {
                                let at=(row*width+column)*4;
                                for c in 0..4 {sum[c]+=u16::from(output.rgba[at+c]);}
                                count+=1;
                            }
                        }
                        let at=(y*width+x)*4;
                        for c in 0..4 {output.rgba[at+c]=(sum[c]/count) as u8;}
                    }
                }
            }
            check()?;return Ok(output);
        }
        for _ in 0..self.passes {
            for y in 1..height - 1 {
                check()?;
                for x in 1..width - 1 {
                    let mut sum = [0u16; 4];
                    for row in y - 1..=y + 1 {
                        for column in x - 1..=x + 1 {
                            let at = (row * width + column) * 4;
                            for c in 0..4 {
                                sum[c] += u16::from(output.rgba[at + c]);
                            }
                        }
                    }
                    let at = (y * width + x) * 4;
                    for c in 0..4 {
                        output.rgba[at + c] = (sum[c] / 9) as u8;
                    }
                }
            }
        }
        // Repair borders once, after all passes. The right edge is retained.
        // Original N2 trial 46FBC0 writes the rightmost pixel to a guard cell
        // *after* each row, rather than to its final visible column. Those
        // guard writes cannot affect subsequent samples and are omitted here.
        let stride = width * 4;
        output.rgba.copy_within(stride..2 * stride, 0);
        output.rgba.copy_within(
            (height - 2) * stride..(height - 1) * stride,
            (height - 1) * stride,
        );
        for y in 0..height {
            check()?;
            let at = y * stride;
            output.rgba.copy_within(at + 4..at + 8, at);
        }
        check()?;
        Ok(output)
    }
}
