//! A reusable blur/colour/coverage/composition chain over immutable input.
//! All work remains in the image job; native adapters only translate policy.
use crate::{
    ImageSurface,
    box_blur_clamped::BoxBlurClamped,
    effects::MAX_EFFECT_PIXELS,
    rgb_lut::{ConstantByteBlend, RgbLookup},
    shape_mask::ShapeMask,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskedBlur {
    pub blur: BoxBlurClamped,
    pub color: [u8; 4],
    pub color_mode: ConstantByteBlend,
    pub mask: ShapeMask,
    pub blend: ConstantByteBlend,
    pub opacity: u8,
}
impl MaskedBlur {
    pub fn validate(&self) -> Result<()> {
        self.blur.validate()?;
        self.mask.validate()?;
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
            "masked blur pixel budget"
        );
        check()?;
        let blurred = self.blur.apply(input, check)?;
        let colored =
            RgbLookup::constant_byte256(self.color, self.color_mode).apply(&blurred, check)?;
        drop(blurred);
        let masked = self.mask.apply(&colored, check)?;
        drop(colored);
        let mut output = input.clone();
        for (target, source) in output
            .rgba
            .chunks_exact_mut(input.width as usize * 4)
            .zip(masked.rgba.chunks_exact(input.width as usize * 4))
        {
            check()?;
            for (d, s) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
                let old = d[..].try_into().unwrap();
                let pixel = self.blend.pixel(old, s.try_into().unwrap(), self.opacity);
                d.copy_from_slice(&pixel);
            }
        }
        check()?;
        Ok(output)
    }
}
