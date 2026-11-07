//! Stored-channel box reduction with explicit integer arithmetic. It produces
//! one output sample per checked window; it is independent of any VM/save ABI.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoxAnchors {
    /// The first window starts at the crop origin. Subsequent windows advance
    /// by an independent integer stride on each axis.
    Grid { stride: [u32; 2] },
    /// Window starts are floor(crop_extent * output_index / output_extent).
    Proportional,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoxArithmetic {
    /// Unsigned full-precision sum divided by the complete window area.
    Mean,
    /// Wrap each channel sum to16 bits, interpret sum and multiplier as signed
    /// words, take the signed product's high word, then saturate to a byte.
    /// This is an explicit packed-integer contract, not a float approximation.
    SignedWordHigh { multiplier: u16 },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoxAlpha {
    Aggregate,
    Opaque,
    Zero,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxReduce {
    pub size: [u32; 2],
    /// Source x/y/width/height. Windows must be wholly inside this crop; there
    /// is no implicit edge clamp or unchecked overread.
    pub source: Option<[u32; 4]>,
    pub window: [u16; 2],
    pub anchors: BoxAnchors,
    pub arithmetic: BoxArithmetic,
    pub alpha: BoxAlpha,
}
impl BoxReduce {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size
                .iter()
                .all(|v| (1..=crate::MAX_IMAGE_DIMENSION).contains(v)),
            "box reduction output dimensions"
        );
        ensure!(
            self.window.iter().all(|v| (1..=1024).contains(v)),
            "box reduction window dimensions"
        );
        let pixels = u64::from(self.size[0]) * u64::from(self.size[1]);
        let taps = pixels * u64::from(self.window[0]) * u64::from(self.window[1]);
        ensure!(
            pixels <= MAX_EFFECT_PIXELS && taps <= MAX_EFFECT_PIXELS,
            "box reduction pixel/sample budget"
        );
        if let BoxAnchors::Grid { stride } = self.anchors {
            ensure!(
                stride
                    .iter()
                    .all(|v| (1..=crate::MAX_IMAGE_DIMENSION).contains(v)),
                "box reduction grid stride"
            );
        }
        if let Some([x, y, w, h]) = self.source {
            ensure!(
                w > 0
                    && h > 0
                    && [x, y, w, h]
                        .iter()
                        .all(|v| *v <= crate::MAX_IMAGE_DIMENSION),
                "box reduction source rectangle"
            );
        }
        Ok(())
    }
    pub fn output_size(&self, input: [u32; 2]) -> Result<[u32; 2]> {
        self.bounds(input)?;
        Ok(self.size)
    }
    fn anchor(&self, axis: usize, index: u32, extent: u32) -> u64 {
        match self.anchors {
            BoxAnchors::Grid { stride } => u64::from(index) * u64::from(stride[axis]),
            BoxAnchors::Proportional => {
                u64::from(extent) * u64::from(index) / u64::from(self.size[axis])
            }
        }
    }
    fn bounds(&self, input: [u32; 2]) -> Result<[u32; 4]> {
        self.validate()?;
        ensure!(
            input
                .iter()
                .all(|v| (1..=crate::MAX_IMAGE_DIMENSION).contains(v))
                && u64::from(input[0]) * u64::from(input[1]) <= MAX_EFFECT_PIXELS,
            "box reduction input budget"
        );
        let crop = self.source.unwrap_or([0, 0, input[0], input[1]]);
        for axis in 0..2 {
            ensure!(
                u64::from(crop[axis]) + u64::from(crop[axis + 2]) <= u64::from(input[axis]),
                "box reduction crop outside source"
            );
            let last = self.anchor(axis, self.size[axis] - 1, crop[axis + 2]);
            ensure!(
                last + u64::from(self.window[axis]) <= u64::from(crop[axis + 2]),
                "box reduction window outside crop"
            );
        }
        crate::rgba_len(self.size[0], self.size[1])?;
        Ok(crop)
    }
    pub fn apply(
        &self,
        input: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        input.validate()?;
        let [cx, cy, cw, ch] = self.bounds([input.width, input.height])?;
        check()?;
        let mut out = ImageSurface {
            width: self.size[0],
            height: self.size[1],
            rgba: vec![0; crate::rgba_len(self.size[0], self.size[1])?],
        };
        let divisor = u64::from(self.window[0]) * u64::from(self.window[1]);
        for y in 0..self.size[1] {
            check()?;
            let sy = cy + self.anchor(1, y, ch) as u32;
            for x in 0..self.size[0] {
                let sx = cx + self.anchor(0, x, cw) as u32;
                let mut sums = [0u64; 4];
                for row in 0..u32::from(self.window[1]) {
                    check()?;
                    let at = (((sy + row) * input.width + sx) * 4) as usize;
                    let line = &input.rgba[at..at + usize::from(self.window[0]) * 4];
                    for pixel in line.chunks_exact(4) {
                        for channel in 0..4 {
                            sums[channel] += u64::from(pixel[channel]);
                        }
                    }
                }
                let mut pixel = sums.map(|sum| match self.arithmetic {
                    BoxArithmetic::Mean => (sum / divisor) as u8,
                    BoxArithmetic::SignedWordHigh { multiplier } => {
                        let product = i32::from(sum as u16 as i16) * i32::from(multiplier as i16);
                        (product >> 16).clamp(0, 255) as u8
                    }
                });
                match self.alpha {
                    BoxAlpha::Aggregate => {}
                    BoxAlpha::Opaque => pixel[3] = 255,
                    BoxAlpha::Zero => pixel[3] = 0,
                }
                let at = ((y * self.size[0] + x) * 4) as usize;
                out.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        Ok(out)
    }
}
