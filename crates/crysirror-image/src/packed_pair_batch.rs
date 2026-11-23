// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered two-source sprite composition with a single destination allocation.
//! Both source leases stay immutable; destination residuals see prior sprites.
use crate::{ImageSurface, effects::MAX_EFFECT_PIXELS, packed_pair_blend::PackedPairBlend};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackedPairBatch {
    pub operations: Vec<PackedPairBlend>,
}
impl PackedPairBatch {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.operations.len() <= 4096,
            "packed pair batch operation count"
        );
        let mut work = 0u64;
        for op in &self.operations {
            op.validate()?;
            work += op.sources[0][2] as u64 * op.sources[0][3] as u64;
        }
        ensure!(
            work <= MAX_EFFECT_PIXELS * 4,
            "packed pair batch sample budget"
        );
        Ok(())
    }
    pub fn apply(
        &self,
        destination: &ImageSurface,
        first: &ImageSurface,
        second: &ImageSurface,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ImageSurface> {
        self.validate()?;
        for image in [destination, first, second] {
            image.validate()?;
            ensure!(
                u64::from(image.width) * u64::from(image.height) <= MAX_EFFECT_PIXELS,
                "packed pair batch image budget"
            );
        }
        check()?;
        let mut out = destination.clone();
        for operation in &self.operations {
            check()?;
            // Shares the standalone operation's exact byte executor. The
            // source is never this mutable output, even for aliased handles.
            operation.apply_to(&mut out, first, second, check)?;
        }
        check()?;
        Ok(out)
    }
}
