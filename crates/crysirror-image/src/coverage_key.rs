// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Binary coverage from equality with a checked reference texel. Selection is
//! over immutable stored channels; RGB is preserved, including hidden colour.
use crate::{effects::MAX_EFFECT_PIXELS, ImageSurface};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageReferenceKey {
    pub reference: [u32; 2],
    /// Stored RGBA channel (0..=3) compared with the reference byte.
    pub channel: u8,
    pub equal: u8,
    pub different: u8,
}
impl CoverageReferenceKey {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.channel < 4, "coverage reference channel");
        ensure!(
            self.reference.iter().all(|v| *v < 16384),
            "coverage reference coordinates"
        );
        Ok(())
    }
}
impl ImageSurface {
    pub fn coverage_reference_key(
        &self,
        spec: &CoverageReferenceKey,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        self.validate()?;
        spec.validate()?;
        ensure!(
            u64::from(self.width) * u64::from(self.height) <= MAX_EFFECT_PIXELS,
            "coverage reference pixel budget"
        );
        ensure!(
            spec.reference[0] < self.width && spec.reference[1] < self.height,
            "coverage reference outside source"
        );
        check()?;
        let index = (spec.reference[1] as usize * self.width as usize + spec.reference[0] as usize)
            * 4
            + spec.channel as usize;
        let key = self.rgba[index];
        let mut out = self.clone();
        for row in out.rgba.chunks_exact_mut(self.width as usize * 4) {
            check()?;
            for p in row.chunks_exact_mut(4) {
                p[3] = if p[spec.channel as usize] == key {
                    spec.equal
                } else {
                    spec.different
                };
            }
        }
        Ok(out)
    }
}
