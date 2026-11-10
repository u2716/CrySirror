//! Stored-channel box reduction with explicit integer arithmetic. It produces
//! one output sample per checked window; it is independent of any VM/save ABI.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoxAnchors {
    /// Window origins span every valid source start: floor((extent-window+1)
    /// * index/output). Uses bounded column sums instead of repeated taps.
    WindowOrigins,
    /// The first window starts at the crop origin. Subsequent windows advance
    /// by an independent integer stride on each axis.
    Grid { stride: [u32; 2] },
    /// Window starts are floor(crop_extent * output_index / output_extent).
    Proportional,
    /// Exact area overlaps between rational source/output pixel boxes. The
    /// window is ignored; only Mean arithmetic is meaningful for this profile.
    Area,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoxArithmetic {
    /// Wrap to16 bits, logically halve, signed high multiply, double the word,
    /// then signed-saturate to a byte. Describes native packed blur arithmetic.
    HalfWordHighDouble { multiplier: u16 },
    /// Unsigned full-precision sum divided by the complete window area.
    Mean,
    /// Explicit row-major weights for a2x2 window. The unsigned sum is divided
    /// once and saturated to a byte; this also describes asymmetric filters.
    WeightedQuad { weights: [u16; 4], divisor: u16 },
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
            pixels <= MAX_EFFECT_PIXELS
                && (matches!(self.anchors, BoxAnchors::WindowOrigins) || taps <= MAX_EFFECT_PIXELS),
            "box reduction pixel/sample budget"
        );
        if matches!(self.anchors, BoxAnchors::Area) {
            ensure!(
                matches!(self.arithmetic, BoxArithmetic::Mean),
                "area reduction requires full-precision mean"
            );
        }
        if let BoxArithmetic::WeightedQuad { weights, divisor } = self.arithmetic {
            ensure!(
                self.window == [2, 2]
                    && divisor != 0
                    && weights.iter().map(|v| u32::from(*v)).sum::<u32>() <= 65535,
                "weighted quad window/weights/divisor"
            );
        }
        ensure!(
            !matches!(self.anchors, BoxAnchors::WindowOrigins)
                || !matches!(self.arithmetic, BoxArithmetic::WeightedQuad { .. }),
            "sliding reduction requires uniform window weights"
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
            BoxAnchors::WindowOrigins => {
                u64::from(extent.saturating_sub(u32::from(self.window[axis])) + 1)
                    * u64::from(index)
                    / u64::from(self.size[axis])
            }
            BoxAnchors::Grid { stride } => u64::from(index) * u64::from(stride[axis]),
            BoxAnchors::Proportional | BoxAnchors::Area => {
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
                matches!(self.anchors, BoxAnchors::Area)
                    || last + u64::from(self.window[axis]) <= u64::from(crop[axis + 2]),
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
        if matches!(self.anchors, BoxAnchors::Area) {
            return self.apply_area(input, [cx, cy, cw, ch], check);
        }
        if matches!(self.anchors, BoxAnchors::WindowOrigins) {
            return self.apply_sliding(input, [cx, cy, cw, ch], check);
        }
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
                    for (column, pixel) in line.chunks_exact(4).enumerate() {
                        let weight =
                            if let BoxArithmetic::WeightedQuad { weights, .. } = self.arithmetic {
                                u64::from(weights[row as usize * 2 + column])
                            } else {
                                1
                            };
                        for channel in 0..4 {
                            sums[channel] += u64::from(pixel[channel]) * weight;
                        }
                    }
                }
                let mut pixel = sums.map(|sum| match self.arithmetic {
                    BoxArithmetic::Mean => (sum / divisor) as u8,
                    BoxArithmetic::HalfWordHighDouble { multiplier } => {
                        let half = (sum as u16) >> 1;
                        let high = ((i32::from(half) * i32::from(multiplier as i16)) >> 16) as u16;
                        (high.wrapping_shl(1) as i16).clamp(0, 255) as u8
                    }
                    BoxArithmetic::WeightedQuad { divisor, .. } => {
                        (sum / u64::from(divisor)).min(255) as u8
                    }
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
    fn apply_sliding(
        &self,
        input: &ImageSurface,
        [cx, cy, cw, ch]: [u32; 4],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        let [kw, kh] = self.window.map(u32::from);
        ensure!(kw <= cw && kh <= ch, "sliding reduction window extent");
        let work = u64::from(cw) * (u64::from(kh) + u64::from(ch) * 2 + u64::from(self.size[1]))
            + u64::from(self.size[0]) * u64::from(self.size[1]);
        ensure!(
            work <= MAX_EFFECT_PIXELS * 16,
            "sliding reduction work budget"
        );
        let mut columns = vec![[0u64; 4]; cw as usize];
        let mut prefix = vec![[0u64; 4]; cw as usize + 1];
        let mut out = ImageSurface {
            width: self.size[0],
            height: self.size[1],
            rgba: vec![0; crate::rgba_len(self.size[0], self.size[1])?],
        };
        for y in 0..kh {
            check()?;
            let at = (((cy + y) * input.width + cx) * 4) as usize;
            for (x, pixel) in input.rgba[at..at + cw as usize * 4]
                .chunks_exact(4)
                .enumerate()
            {
                for c in 0..4 {
                    columns[x][c] += u64::from(pixel[c]);
                }
            }
        }
        let mut origin = 0u32;
        let divisor = u64::from(kw) * u64::from(kh);
        for y in 0..out.height {
            check()?;
            let next = self.anchor(1, y, ch) as u32;
            while origin < next {
                check()?;
                let remove = (((cy + origin) * input.width + cx) * 4) as usize;
                let add = (((cy + origin + kh) * input.width + cx) * 4) as usize;
                for x in 0..cw as usize {
                    for c in 0..4 {
                        columns[x][c] = columns[x][c] - u64::from(input.rgba[remove + x * 4 + c])
                            + u64::from(input.rgba[add + x * 4 + c]);
                    }
                }
                origin += 1;
            }
            prefix[0] = [0; 4];
            for x in 0..cw as usize {
                prefix[x + 1] = std::array::from_fn(|c| prefix[x][c] + columns[x][c]);
            }
            for x in 0..out.width {
                let sx = self.anchor(0, x, cw) as usize;
                let sums =
                    std::array::from_fn::<_, 4, _>(|c| prefix[sx + kw as usize][c] - prefix[sx][c]);
                let mut pixel = sums.map(|sum| match self.arithmetic {
                    BoxArithmetic::Mean => (sum / divisor) as u8,
                    BoxArithmetic::SignedWordHigh { multiplier } => {
                        ((i32::from(sum as u16 as i16) * i32::from(multiplier as i16)) >> 16)
                            .clamp(0, 255) as u8
                    }
                    BoxArithmetic::HalfWordHighDouble { multiplier } => {
                        let high = ((i32::from((sum as u16) >> 1) * i32::from(multiplier as i16))
                            >> 16) as u16;
                        (high.wrapping_shl(1) as i16).clamp(0, 255) as u8
                    }
                    BoxArithmetic::WeightedQuad { .. } => unreachable!(),
                });
                match self.alpha {
                    BoxAlpha::Aggregate => {}
                    BoxAlpha::Opaque => pixel[3] = 255,
                    BoxAlpha::Zero => pixel[3] = 0,
                }
                let at = ((y * out.width + x) * 4) as usize;
                out.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(out)
    }
    fn apply_area(
        &self,
        input: &ImageSurface,
        crop: [u32; 4],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        let [cx, cy, sw, sh] = crop;
        let [dw, dh] = self.size;
        // Each source/output grid crossing adds at most one interval per axis.
        ensure!(
            (u64::from(sw) + u64::from(dw)) * (u64::from(sh) + u64::from(dh))
                <= MAX_EFFECT_PIXELS * 16,
            "area overlap work budget"
        );
        let mut out = ImageSurface {
            width: dw,
            height: dh,
            rgba: vec![0; crate::rgba_len(dw, dh)?],
        };
        let denominator = u64::from(sw) * u64::from(sh);
        for y in 0..dh {
            check()?;
            let y0 = u64::from(y) * u64::from(sh);
            let y1 = u64::from(y + 1) * u64::from(sh);
            for x in 0..dw {
                let x0 = u64::from(x) * u64::from(sw);
                let x1 = u64::from(x + 1) * u64::from(sw);
                let mut sums = [0u64; 4];
                for sy in y0 / u64::from(dh)..y1.div_ceil(u64::from(dh)) {
                    let wy = y1.min((sy + 1) * u64::from(dh)) - y0.max(sy * u64::from(dh));
                    for sx in x0 / u64::from(dw)..x1.div_ceil(u64::from(dw)) {
                        let wx = x1.min((sx + 1) * u64::from(dw)) - x0.max(sx * u64::from(dw));
                        let at =
                            (((u64::from(cy) + sy) * u64::from(input.width) + u64::from(cx) + sx)
                                * 4) as usize;
                        for channel in 0..4 {
                            sums[channel] += u64::from(input.rgba[at + channel]) * wx * wy;
                        }
                    }
                }
                let at = ((u64::from(y) * u64::from(dw) + u64::from(x)) * 4) as usize;
                for channel in 0..4 {
                    out.rgba[at + channel] = (sums[channel] / denominator) as u8;
                }
                match self.alpha {
                    BoxAlpha::Aggregate => {}
                    BoxAlpha::Opaque => out.rgba[at + 3] = 255,
                    BoxAlpha::Zero => out.rgba[at + 3] = 0,
                }
            }
        }
        check()?;
        Ok(out)
    }
}

#[cfg(test)]
mod area_tests {
    use super::*;
    #[test]
    fn rational_area_weights_preserve_crop_hidden_channels_and_fractional_edges() {
        let image = ImageSurface {
            width: 3,
            height: 1,
            rgba: vec![0, 30, 60, 90, 90, 60, 30, 0, 240, 120, 0, 255],
        };
        let spec = BoxReduce {
            size: [2, 1],
            source: None,
            window: [1, 1],
            anchors: BoxAnchors::Area,
            arithmetic: BoxArithmetic::Mean,
            alpha: BoxAlpha::Aggregate,
        };
        // First box covers two thirds of pixel0 and one third of pixel1;
        // the second covers one third of pixel1 and two thirds of pixel2.
        assert_eq!(
            spec.apply(&image, &|| Ok(())).unwrap().rgba,
            [30, 40, 50, 60, 190, 100, 10, 170]
        );
        let spec = BoxReduce {
            size: [1, 1],
            source: Some([1, 0, 2, 1]),
            ..spec
        };
        assert_eq!(
            spec.apply(&image, &|| Ok(())).unwrap().rgba,
            [165, 90, 15, 127]
        );
        assert!(spec.apply(&image, &|| anyhow::bail!("cancelled")).is_err());
    }
}
