//! Three-stop spatial RGBA gradients blended into stored RGB. The profile
//! separates percentage positions from its two10-bit colour ramps.
use crate::{
    ImageSurface,
    effects::MAX_EFFECT_PIXELS,
    luma_lookup::{LookupIndex, LumaLookup},
    rgb_lut::ConstantByteBlend,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradientOverlayByte {
    pub vertical: bool,
    pub positions_percent: [i32; 3],
    pub colors: [[u8; 4]; 3],
    pub blend: ConstantByteBlend,
    pub opacity: u8,
}
impl GradientOverlayByte {
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
            "gradient overlay pixel budget"
        );
        check()?;
        let mut stops =
            std::array::from_fn::<_, 3, _>(|i| (self.positions_percent[i], self.colors[i]));
        stops.sort_by_key(|v| v.0);
        let length = if self.vertical {
            input.height
        } else {
            input.width
        } as usize;
        let position = std::array::from_fn::<_, 3, _>(|i| {
            length * stops[i].0.clamp(if i == 0 { 0 } else { 1 }, 100) as usize / 100
        });
        let [a, b, c] = position;
        let mut ramp = vec![0u8; length];
        if b > a {
            for (i, v) in ramp[a..b].iter_mut().enumerate() {
                *v = (127 * i / (b - a)) as u8;
            }
        }
        if c > b {
            for (i, v) in ramp[b..c].iter_mut().enumerate() {
                *v = (128 + 127 * i / (c - b)) as u8;
            }
        }
        if length - 1 > c {
            //55F870 starts its white tail one sample before the third stop.
            //For a short axis, clamp that start to0 rather than reproduce
            //the original negative write into a zero-length prefix.
            ramp[c.saturating_sub(1)..].fill(255);
        }
        let table = LumaLookup::gradient_byte1024(
            &[
                (0, stops[0].1),
                (127, stops[1].1),
                (128, stops[1].1),
                (255, stops[2].1),
            ],
            LookupIndex::Alpha,
            false,
        )?
        .table;
        let mut output = input.clone();
        for y in 0..input.height {
            check()?;
            for x in 0..input.width {
                let value = ramp[if self.vertical { y } else { x } as usize];
                let at = ((y * input.width + x) * 4) as usize;
                let old = output.rgba[at..at + 4].try_into().unwrap();
                let pixel = self
                    .blend
                    .pixel(old, table[usize::from(value)], self.opacity);
                output.rgba[at..at + 4].copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(output)
    }
}
