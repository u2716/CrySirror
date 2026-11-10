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

#[cfg(test)]
mod quantized_tests {
    use super::*;
    #[test]
    fn explicit_axis_phase_and_weighted_quad_use_stored_byte_samples(){
        use crate::box_reduce::{BoxReduce,BoxAnchors,BoxArithmetic,BoxAlpha};
        let source=ImageSurface{width:2,height:2,rgba:[10,30,50,210].into_iter().flat_map(|v|[v,v,v,255]).collect()};
        let reduction=BoxReduce{size:[1,1],source:None,window:[2,2],anchors:BoxAnchors::Grid{stride:[2,2]},arithmetic:BoxArithmetic::WeightedQuad{weights:[1,1,2,0],divisor:4},alpha:BoxAlpha::Opaque};
        assert_eq!(reduction.apply(&source,&||Ok(())).unwrap().rgba,[35,35,35,255]);
        let source=ImageSurface{width:1,height:8,rgba:(0..8).flat_map(|v|[v,v,v,255]).collect()};
        let spec=RectMap{regions:vec![RectMapRegion{source:[0,0,1,8],destination:[0,0,1,6]}],output_size:Some([1,6]),clear:[0;4],sample:RectMapSample::FloorQuantized{fractional_bits:[None,Some(1)]},copy_alpha:true};
        // Quantising8/6 to one fractional bit gives1.0 per output pixel;
        // exact rational phases would sample0,1,2,4,5,6 instead.
        assert_eq!(spec.apply(&source,&||Ok(())).unwrap().rgba,[0,0,0,255,1,1,1,255,2,2,2,255,3,3,3,255,4,4,4,255,5,5,5,255]);
        let source=ImageSurface{width:1,height:1,rgba:vec![100,50,10,128]};
        let destination=ImageSurface{width:1,height:1,rgba:vec![240,20,80,255]};
        let mut blend=crate::masked_add::MaskedAddSubtract256{source:[0,0,1,1],destination:[0,0],scale:256,subtract:false};
        assert_eq!(blend.apply(&destination,&source,&||Ok(())).unwrap().rgba,[247,45,85,255]);
        blend.subtract=true;
        assert_eq!(blend.apply(&destination,&source,&||Ok(())).unwrap().rgba,[190,10,75,255]);
        let mut invalid=spec;invalid.sample=RectMapSample::FloorQuantized{fractional_bits:[Some(25),None]};
        assert!(invalid.validate().is_err());
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RectMapSample {
    Floor,
    /// Nearest sampling with independently quantized source increments.
    /// None uses the exact rational phase; Some(n) floors the increment to
    /// n fractional bits before multiplying by the destination coordinate.
    FloorQuantized { fractional_bits: [Option<u8>;2] },
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
        if let RectMapSample::FloorQuantized{fractional_bits}=self.sample{
            ensure!(fractional_bits.into_iter().flatten().all(|bits|bits<=24),"rectangle map sample fractional bits");
        }
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
            let sample = |at: i64, extent: i32, destination: i32,axis:usize| -> i64 {
                let numerator = at * i64::from(extent);
                let n = match self.sample {
                    RectMapSample::Floor => numerator / i64::from(destination),
                    RectMapSample::FloorQuantized{fractional_bits}=>{
                        if let Some(bits)=fractional_bits[axis]{
                            (at*((i64::from(extent)<<bits)/i64::from(destination)))>>bits
                        }else{numerator/i64::from(destination)}
                    }
                    RectMapSample::NearestRound | RectMapSample::NearestRoundLinear => {
                        (numerator * 2 + i64::from(destination)) / (i64::from(destination) * 2)
                    }
                };
                if matches!(self.sample, RectMapSample::NearestRoundLinear) { n }
                else { n.clamp(0, i64::from(extent) - 1) }
            };
            for y in top..bottom {
                check()?;
                let source_y = i64::from(sy) + sample(y - i64::from(dy), sh, dh,1);
                for x in left..right {
                    let source_x = i64::from(sx) + sample(x - i64::from(dx), sw, dw,0);
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
