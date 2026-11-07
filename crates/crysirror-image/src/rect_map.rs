// SPDX-License-Identifier: MPL-2.0
//! Bounded ordered nearest-neighbour rectangle maps over immutable input.
//! This is useful for integer scanline/stripe deformation without constructing
//! hundreds of duplicate image-graph nodes or exposing raster loops to a VM.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectMapRegion {
    /// Source XYWH. Sampling is relative to this original rectangle, including
    /// when the destination is clipped by the output image.
    pub source: [i32; 4],
    /// Destination XYWH; zero extents perform no writes.
    pub destination: [i32; 4],
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RectMapSample {
    Floor,
    /// round(i*source_extent/destination_extent), clamped inside the rectangle.
    /// The clamp repairs the otherwise out-of-rectangle final upscale sample.
    NearestRound,
    ///Round the rational table without clipping to the source subrectangle.
    ///Coordinates address the complete immutable image in linear row order;
    ///a sample at X==image.width carries into the following row. Only an
    ///address outside the complete allocation clamps to its nearest endpoint.
    ///This bounded profile preserves initialized border samples of native
    ///table kernels without reproducing reads of uninitialized memory.
    NearestRoundLinear,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectMap {
    pub regions: Vec<RectMapRegion>,
    ///None retains source extent. An explicit extent changes only the output
    ///canvas, leaving all source coordinates and linear row addressing intact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_size: Option<[u32; 2]>,
    /// A defined background for every pixel not written by a mapping.
    pub clear: [u8; 4],
    pub sample: RectMapSample,
    /// False copies only RGB and retains the clear/staged destination alpha.
    pub copy_alpha: bool,
}
impl RectMap {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.regions.len() <= 2048, "rectangle map region budget");
        if let Some([width, height]) = self.output_size {
            ensure!((1..=16384).contains(&width) && (1..=16384).contains(&height)
                && u64::from(width) * u64::from(height) <= MAX_EFFECT_PIXELS,
                "rectangle map output size/pixel budget");
        }
        let mut work = 0u64;
        for r in &self.regions {
            ensure!(
                r.source
                    .iter()
                    .chain(&r.destination)
                    .all(|n| n.unsigned_abs() <= 1_000_000)
                    && r.source[2..]
                        .iter()
                        .chain(&r.destination[2..])
                        .all(|n| *n >= 0),
                "rectangle map coordinates"
            );
            work = work
                .checked_add(r.destination[2] as u64 * r.destination[3] as u64)
                .ok_or_else(|| anyhow::anyhow!("rectangle map work overflow"))?;
            ensure!(work <= MAX_EFFECT_PIXELS * 4, "rectangle map write budget");
        }
        Ok(())
    }
    pub fn apply(
        &self,
        source: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        source.validate()?;
        ensure!(
            u64::from(source.width) * u64::from(source.height) <= MAX_EFFECT_PIXELS,
            "rectangle map pixel budget"
        );
        //Reject all malformed source rectangles before allocating or exposing
        //an output. Empty regions are native no-write operations.
        for r in &self.regions {
            if r.source[2] == 0
                || r.source[3] == 0
                || r.destination[2] == 0
                || r.destination[3] == 0
            {
                continue;
            }
            ensure!(
                r.source[0] >= 0
                    && r.source[1] >= 0
                    && i64::from(r.source[0]) + i64::from(r.source[2]) <= i64::from(source.width)
                    && i64::from(r.source[1]) + i64::from(r.source[3]) <= i64::from(source.height),
                "rectangle map source extent"
            );
        }
        check()?;
        let [width, height] = self.output_size.unwrap_or([source.width, source.height]);
        let mut out = ImageSurface {
            width,
            height,
            rgba: vec![0; crate::rgba_len(width, height)?],
        };
        for row in out.rgba.chunks_mut(width as usize * 4) {
            check()?;
            for pixel in row.chunks_exact_mut(4) {
                pixel.copy_from_slice(&self.clear);
            }
        }
        for r in &self.regions {
            let [sx, sy, sw, sh] = r.source;
            let [dx, dy, dw, dh] = r.destination;
            if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
                continue;
            }
            let left = i64::from(dx).max(0);
            let top = i64::from(dy).max(0);
            let right = (i64::from(dx) + i64::from(dw)).min(i64::from(out.width));
            let bottom = (i64::from(dy) + i64::from(dh)).min(i64::from(out.height));
            let sample = |at: i64, extent: i32, destination: i32| -> i64 {
                let numerator = at * i64::from(extent);
                let n = match self.sample {
                    RectMapSample::Floor => numerator / i64::from(destination),
                    RectMapSample::NearestRound | RectMapSample::NearestRoundLinear => {
                        (numerator * 2 + i64::from(destination)) / (i64::from(destination) * 2)
                    }
                };
                if matches!(self.sample, RectMapSample::NearestRoundLinear) { n }
                else { n.clamp(0, i64::from(extent) - 1) }
            };
            for y in top..bottom {
                check()?;
                let source_y = i64::from(sy) + sample(y - i64::from(dy), sh, dh);
                for x in left..right {
                    let source_x = i64::from(sx) + sample(x - i64::from(dx), sw, dw);
                    let pixel = source_y * i64::from(source.width) + source_x;
                    let pixel = if matches!(self.sample, RectMapSample::NearestRoundLinear) {
                        pixel.clamp(0, i64::from(source.width) * i64::from(source.height) - 1)
                    } else { pixel };
                    let si = (pixel * 4) as usize;
                    let di = ((y * i64::from(out.width) + x) * 4) as usize;
                    let channels = if self.copy_alpha { 4 } else { 3 };
                    out.rgba[di..di + channels].copy_from_slice(&source.rgba[si..si + channels]);
                }
            }
        }
        check()?;
        Ok(out)
    }
}
