//! Stored-byte two-texture operations. Both inputs are immutable worker-owned
//! images; blend equations operate on straight RGBA before tint/composition.
use crate::{
    ImageSurface,
    byte_sampling::{ByteSampleEdge, bilinear128_edge_validated},
    effects::MAX_EFFECT_PIXELS,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DualTextureMode {
    /// Copy the mapped secondary straight RGBA into primary-sized output.
    Replace,
    /// Interpolate all four straight channels with a truncated7-bit weight.
    Crossfade,
    /// Modulate alpha by secondary blue. RGB uses the stored-byte255/256
    /// coefficient, including its one-byte truncation for nonzero samples.
    BlueAlpha,
    /// Interpolate RGB using secondary coverage; retain primary alpha.
    AlphaWeighted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualTextureByte128 {
    pub mode: DualTextureMode,
    pub amount: u8,
    /// Primary crop whose normalized coordinates drive secondary addressing.
    ///None selects the full primary image. Pixels outside it are immaterial
    ///when the caller presents this same crop through a retained Layer UV.
    #[serde(default)]
    pub primary_rect: Option<[f32; 4]>,
    /// Secondary source rectangle in pixels; None maps the whole secondary
    /// image to the primary extent. Endpoints clamp before interpolation.
    #[serde(default)]
    pub secondary_rect: Option<[f32; 4]>,
    /// Bilinear sampling uses independent7-bit truncated passes. The default
    /// is nearest-neighbour stored-byte sampling.
    #[serde(default)]
    pub linear: bool,
}
impl DualTextureByte128 {
    pub fn validate(&self) -> Result<()> {
        if let Some(rect) = self.primary_rect {
            ensure!(
                rect.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                    && rect[2] > 0.
                    && rect[3] > 0.,
                "dual texture primary crop"
            );
        }
        if let Some(rect) = self.secondary_rect {
            ensure!(
                rect.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                "dual texture source bounds"
            );
        }
        Ok(())
    }
    pub fn apply(
        &self,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        first.validate()?;
        second.validate()?;
        check()?;
        ensure!(
            u64::from(first.width) * u64::from(first.height) <= MAX_EFFECT_PIXELS,
            "dual texture pixel budget"
        );
        let rect =
            self.secondary_rect
                .unwrap_or([0., 0., second.width as f32, second.height as f32]);
        let origin = [
            rect[0].clamp(0., second.width as f32),
            rect[1].clamp(0., second.height as f32),
        ];
        let end = [
            (rect[0] + rect[2]).clamp(0., second.width as f32),
            (rect[1] + rect[3]).clamp(0., second.height as f32),
        ];
        let primary =
            self.primary_rect
                .unwrap_or([0., 0., first.width as f32, first.height as f32]);
        let mut out = first.clone();
        for y in 0..first.height {
            check()?;
            for x in 0..first.width {
                let p = [
                    origin[0] + (end[0] - origin[0]) * (x as f32 - primary[0]) / primary[2],
                    origin[1] + (end[1] - origin[1]) * (y as f32 - primary[1]) / primary[3],
                ];
                let b = if self.linear {
                    bilinear128_edge_validated(second, p, ByteSampleEdge::Clamp)
                } else {
                    let sx = (p[0] as u32).min(second.width - 1);
                    let sy = (p[1] as u32).min(second.height - 1);
                    second.rgba[((sy * second.width + sx) * 4) as usize
                        ..((sy * second.width + sx) * 4 + 4) as usize]
                        .try_into()
                        .unwrap()
                };
                let i = ((y * first.width + x) * 4) as usize;
                let a: [u8; 4] = first.rgba[i..i + 4].try_into().unwrap();
                let mixed = match self.mode {
                    DualTextureMode::Replace => b,
                    DualTextureMode::Crossfade => {
                        std::array::from_fn(|c| mix(a[c], b[c], u16::from(self.amount >> 1)))
                    }
                    DualTextureMode::BlueAlpha => {
                        let masked = (u16::from(a[3]) * u16::from(b[2]) >> 8) as u8;
                        [
                            a[0].saturating_sub(1),
                            a[1].saturating_sub(1),
                            a[2].saturating_sub(1),
                            mix(a[3], masked, u16::from(self.amount >> 1)),
                        ]
                    }
                    DualTextureMode::AlphaWeighted => {
                        let weight = (u16::from(b[3]) * u16::from(self.amount) >> 8) >> 1;
                        [
                            mix(a[0], b[0], weight),
                            mix(a[1], b[1], weight),
                            mix(a[2], b[2], weight),
                            a[3],
                        ]
                    }
                };
                out.rgba[i..i + 4].copy_from_slice(&mixed);
            }
        }
        check()?;
        Ok(out)
    }
}
fn mix(a: u8, b: u8, weight: u16) -> u8 {
    (i32::from(a) + ((i32::from(b) - i32::from(a)) * i32::from(weight) >> 7)).clamp(0, 255) as u8
}
