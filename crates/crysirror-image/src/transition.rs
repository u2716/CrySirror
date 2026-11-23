// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Shared two-image crossfade, rule reveal and blur-crossfade resource operations.
use crate::{ImageSurface, effects::ImageEffect};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
/// A resource transition at explicit progress. A mask reveals bright pixels
/// first; softness is the normalized reveal ramp. Endpoints preserve exact input
/// bytes. Blur grows toward the midpoint; this is an engine profile, not an
/// alias for every legacy transition provider's arithmetic.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageTransition {
    pub progress: f32,
    pub softness: f32,
    pub blur: f32,
    /// Exact integer-weighted profile. With this profile mask/blur/softness
    /// must be absent/zero; floating progress is retained only for old schemas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_weighted: Option<crate::fixed_mix::FixedMixSpec>,
    /// Exact per-pixel integer weights looked up from one mask channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_table: Option<RuleTable>,
}
/// A 256-entry weight table (0..=256) indexed by a stored mask byte. Without a
/// mask every pixel takes entry 0. Blur, softness and progress are unused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleTable {
    pub weights: Vec<u16>,
    /// Mask byte in RGBA order, 0..=3.
    pub channel: u8,
    pub operation: RuleOperation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOperation {
    /// All four bytes (other*w + self*(256-w)) >> 8. A fully transparent self
    /// pixel first takes the other image's RGB, so fading edges keep its colour.
    Mix,
    /// RGB kept; alpha becomes (alpha*w) >> 8.
    AlphaScale,
    /// RGB kept; alpha becomes 255 - ((alpha*w) >> 8).
    AlphaScaleInverse,
}
impl RuleTable {
    fn validate(&self) -> Result<()> {
        ensure!(self.weights.len() == 256 && self.weights.iter().all(|w| *w <= 256), "rule table weights");
        ensure!(self.channel <= 3, "rule table mask channel");
        Ok(())
    }
    fn apply(&self, from: &ImageSurface, to: &ImageSurface, mask: Option<&ImageSurface>, check: &dyn Fn() -> Result<()>) -> Result<ImageSurface> {
        let mut out = from.clone();
        let channel = usize::from(self.channel);
        for (index, pixel) in out.rgba.chunks_exact_mut(4).enumerate() {
            if index % 65536 == 0 {
                check()?;
            }
            let at = index * 4;
            let weight = u32::from(self.weights[mask.map_or(0, |m| usize::from(m.rgba[at + channel]))]);
            match self.operation {
                RuleOperation::Mix => {
                    if pixel[3] == 0 {
                        pixel[..3].copy_from_slice(&to.rgba[at..at + 3]);
                    }
                    for c in 0..4 {
                        pixel[c] = ((u32::from(to.rgba[at + c]) * weight + u32::from(pixel[c]) * (256 - weight)) >> 8) as u8;
                    }
                }
                RuleOperation::AlphaScale => pixel[3] = ((u32::from(pixel[3]) * weight) >> 8) as u8,
                RuleOperation::AlphaScaleInverse => pixel[3] = 255 - ((u32::from(pixel[3]) * weight) >> 8) as u8,
            }
        }
        Ok(out)
    }
}
impl ImageTransition {
    pub fn validate(&self) -> Result<()> {
        if let Some(table) = &self.rule_table {
            table.validate()?;
            ensure!(self.fixed_weighted.is_none() && self.blur == 0. && self.softness == 0., "rule table transition cannot also blur, soften or use fixed weights");
        }
        if let Some(spec) = self.fixed_weighted {
            spec.validate()?;
            ensure!(
                self.blur == 0. && self.softness == 0.,
                "fixed weighted transition cannot blur or soften"
            );
        }
        ensure!(
            self.progress.is_finite() && (0.0..=1.).contains(&self.progress),
            "transition progress"
        );
        ensure!(
            self.softness.is_finite() && (0.0..=1.).contains(&self.softness),
            "transition softness"
        );
        ensure!(
            self.blur.is_finite() && (0.0..=16.).contains(&self.blur),
            "transition blur"
        );
        Ok(())
    }
}
impl ImageSurface {
    pub fn transition_to(
        &self,
        other: &Self,
        mask: Option<&Self>,
        spec: &ImageTransition,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        self.validate()?;
        other.validate()?;
        spec.validate()?;
        ensure!(
            [self.width, self.height] == [other.width, other.height],
            "transition image dimensions differ"
        );
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= crate::effects::MAX_EFFECT_PIXELS,
            "transition pixel budget"
        );
        if let Some(m) = mask {
            m.validate()?;
            ensure!(
                [m.width, m.height] == [self.width, self.height],
                "transition mask dimensions differ"
            );
        }
        check()?;
        if let Some(table) = &spec.rule_table {
            return table.apply(self, other, mask, check);
        }
        if let Some(fixed) = spec.fixed_weighted {
            ensure!(mask.is_none(), "fixed weighted transition cannot use mask");
            return fixed.apply(self, other, check);
        }
        if spec.progress == 0. {
            return Ok(self.clone());
        }
        if spec.progress == 1. {
            return Ok(other.clone());
        }
        let sigma = spec.blur * 2. * spec.progress.min(1. - spec.progress);
        let a = self.apply_effect_cancellable(&ImageEffect::GaussianBlur { sigma }, check)?;
        let b = other.apply_effect_cancellable(&ImageEffect::GaussianBlur { sigma }, check)?;
        let mut out = self.clone();
        for y in 0..self.height as usize {
            check()?;
            for x in 0..self.width as usize {
                let at = (y * self.width as usize + x) * 4;
                let t = if let Some(mask) = mask {
                    let p = &mask.rgba[at..at + 4];
                    let lum =
                        (f64::from(p[0]) * 54. + f64::from(p[1]) * 183. + f64::from(p[2]) * 19.)
                            / (256. * 255.);
                    let threshold = 1. - lum;
                    let p = f64::from(spec.progress);
                    if spec.softness == 0. {
                        if p >= threshold { 1. } else { 0. }
                    } else {
                        ((p * (1. + f64::from(spec.softness)) - threshold)
                            / f64::from(spec.softness))
                        .clamp(0., 1.)
                    }
                } else {
                    f64::from(spec.progress)
                };
                let aa = f64::from(a.rgba[at + 3]);
                let ba = f64::from(b.rgba[at + 3]);
                let alpha = aa * (1. - t) + ba * t;
                out.rgba[at + 3] = alpha.round().clamp(0., 255.) as u8;
                for c in 0..3 {
                    out.rgba[at + c] = if alpha > 0. {
                        ((f64::from(a.rgba[at + c]) * aa * (1. - t)
                            + f64::from(b.rgba[at + c]) * ba * t)
                            / alpha)
                            .round()
                            .clamp(0., 255.) as u8
                    } else {
                        0
                    };
                }
            }
        }
        Ok(out)
    }
}
