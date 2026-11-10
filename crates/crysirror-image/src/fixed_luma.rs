// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Checked grouped fixed-point luminance and colour mixing. Quantization groups
//! model SIMD pair-wise arithmetic as well as ordinary sum-before-shift luma;
//! no VM or platform code is required to use this image operation.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedLumaSpec {
    /// Coefficients in engine RGB order, with sum no greater than 2^bits.
    pub coefficients: [u32; 3],
    /// Equal group IDs are summed before a shift; groups are shifted separately.
    pub groups: [u8; 3],
    pub fractional_bits: u8,
    /// Luminance colour mix weight, in explicit /256 units.
    pub amount256: u16,
    /// Optional per-channel tint of the luminance target: `(luma*tint)>>8`
    /// before mixing (sepia-style toning). `None` mixes toward grey.
    #[serde(default)]
    pub tint: Option<[u8; 3]>,
}
impl FixedLumaSpec {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.fractional_bits <= 16,
            "fixed luminance fractional bits"
        );
        let scale = 1u32 << self.fractional_bits;
        ensure!(
            self.coefficients.iter().all(|v| *v <= scale)
                && self.coefficients.iter().map(|v| u64::from(*v)).sum::<u64>() <= u64::from(scale),
            "fixed luminance coefficients"
        );
        ensure!(
            self.groups.iter().all(|v| *v < 3),
            "fixed luminance quantization groups"
        );
        ensure!(self.amount256 <= 256, "fixed luminance blend amount");
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
            "fixed luminance pixel budget"
        );
        check()?;
        let mut output = input.clone();
        for row in output.rgba.chunks_exact_mut(input.width as usize * 4) {
            check()?;
            for pixel in row.chunks_exact_mut(4) {
                let mut sums = [0u32; 3];
                for channel in 0..3 {
                    sums[self.groups[channel] as usize] +=
                        u32::from(pixel[channel]) * self.coefficients[channel];
                }
                let luma = sums.iter().map(|v| *v >> self.fractional_bits).sum::<u32>();
                for (index, channel) in pixel[..3].iter_mut().enumerate() {
                    let target = match self.tint {
                        Some(tint) => (luma * u32::from(tint[index])) >> 8,
                        None => luma,
                    };
                    *channel = ((u32::from(*channel) * u32::from(256 - self.amount256)
                        + target * u32::from(self.amount256))
                        >> 8) as u8;
                }
            }
        }
        check()?;
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image() -> ImageSurface {
        ImageSurface {
            width: 4,
            height: 1,
            rgba: vec![
                255, 255, 255, 7, 255, 0, 0, 13, 0, 255, 0, 19, 0, 0, 255, 23,
            ],
        }
    }
    #[test]
    fn grouped_rounding_is_explicit_and_retains_alpha() {
        let spec = FixedLumaSpec {
            coefficients: [76, 150, 29],
            groups: [1, 0, 0],
            fractional_bits: 8,
            amount256: 256,
            tint: None,
        };
        let out = spec.apply(&image(), &|| Ok(())).unwrap();
        assert_eq!(
            out.rgba,
            [
                253, 253, 253, 7, 75, 75, 75, 13, 149, 149, 149, 19, 28, 28, 28, 23
            ]
        );
        let ordinary = FixedLumaSpec {
            groups: [0, 0, 0],
            ..spec
        }
        .apply(&image(), &|| Ok(()))
        .unwrap();
        assert_eq!(&ordinary.rgba[..4], &[254, 254, 254, 7]);
        let effect = crate::effects::ImageEffect::FixedLuma { spec };
        let encoded = serde_json::to_vec(&effect).unwrap();
        let decoded = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(image().apply_effect(&decoded).unwrap().rgba, out.rgba);
        let unchanged = FixedLumaSpec {
            amount256: 0,
            ..spec
        }
        .apply(&image(), &|| Ok(()))
        .unwrap();
        assert_eq!(unchanged.rgba, image().rgba);
        let mixed = FixedLumaSpec {
            amount256: 128,
            ..spec
        }
        .apply(&image(), &|| Ok(()))
        .unwrap();
        assert_eq!(&mixed.rgba[4..8], &[165, 37, 37, 13]);
    }
    #[test]
    fn tinted_target_matches_grouped_mmx_sepia() {
        // MMX LayerSepia: y=((G*150+B*29)>>8)+((R*76)>>8); t=(y*c)>>8;
        // out=(t*ps+src*(256-ps))>>8.
        let spec = FixedLumaSpec {
            coefficients: [76, 150, 29],
            groups: [1, 0, 0],
            fractional_bits: 8,
            amount256: 256,
            tint: Some([107, 74, 43]),
        };
        let out = spec.apply(&image(), &|| Ok(())).unwrap();
        // White: y=253 -> (253*107)>>8=105, (253*74)>>8=73, (253*43)>>8=42.
        assert_eq!(&out.rgba[..4], &[105, 73, 42, 7]);
        let half = FixedLumaSpec { amount256: 128, ..spec }.apply(&image(), &|| Ok(())).unwrap();
        // Red: y=75 -> t=[31,21,12]; ((31*128+255*128)>>8)=143, (21*128)>>8=10, 6.
        assert_eq!(&half.rgba[4..8], &[143, 10, 6, 13]);
        // Older encoded specs without `tint` still mean grey.
        let legacy: FixedLumaSpec = serde_json::from_str(
            r#"{"coefficients":[76,150,29],"groups":[1,0,0],"fractional_bits":8,"amount256":256}"#,
        ).unwrap();
        assert_eq!(legacy.tint, None);
    }
    #[test]
    fn rejects_invalid_profiles_and_cancels_without_changing_input() {
        let base = FixedLumaSpec {
            coefficients: [76, 150, 29],
            groups: [1, 0, 0],
            fractional_bits: 8,
            amount256: 256,
            tint: None,
        };
        assert!(
            FixedLumaSpec {
                amount256: 257,
                ..base
            }
            .validate()
            .is_err()
        );
        assert!(
            FixedLumaSpec {
                groups: [3, 0, 0],
                ..base
            }
            .validate()
            .is_err()
        );
        assert!(
            FixedLumaSpec {
                coefficients: [255, 255, 255],
                ..base
            }
            .validate()
            .is_err()
        );
        let image = image();
        let before = image.rgba.clone();
        assert!(base.apply(&image, &|| anyhow::bail!("cancelled")).is_err());
        assert_eq!(image.rgba, before);
    }
}
