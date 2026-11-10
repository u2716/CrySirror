// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Public software32 CCalcuSpeed profile, distinct from legacy482070.
//! The source's process-static custom exponent becomes scoped native context.
//! Time mapping, powers and other curve arithmetic remain shared Image APIs.
use crate::config::Names;
use anyhow::{Result, ensure};
use crysirror_image::{
    progress_curve::{ProgressDirection, ProgressShape},
    temporal_pattern::PatternRng,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceCurveContext {
    pub custom_parameter: i32,
}
impl SourceCurveContext {
    pub(crate) fn read(game: &Names) -> Result<Self> {
        Ok(Self {
            custom_parameter: game.integer(b"SpeedCustomParam")?.unwrap_or(100),
        })
    }
    fn shaped_progress(&self, count: i32, max: i32, subframe: i32, kind: i32) -> Result<f64> {
        // For selector45..59 the legacy routine is exactly the same time map
        // with a linear shape. Apply the source's extra power to that shared
        // result before random modulation; other selectors already coincide.
        let progress = super::progress(count, max, subframe, kind, 1.)?;
        if kind % 100 / 15 == 3 {
            let shape = ProgressShape::Power {
                exponent: f64::from(self.custom_parameter.clamp(1, 1000)) * 0.01,
                direction: ProgressDirection::In,
            };
            shape.validate()?;
            Ok(shape.apply(progress))
        } else {
            Ok(progress)
        }
    }
    pub(crate) fn validate(&self, max: i32, subframe: i32, kind: i32) -> Result<()> {
        self.shaped_progress(0, max, subframe, kind)?;
        Ok(())
    }
    pub(crate) fn value(
        &self,
        count: i32,
        max: i32,
        start: i32,
        end: i32,
        subframe: i32,
        kind: i32,
    ) -> Result<i32> {
        ensure!(
            kind / 200 == 0,
            "NNN source32 random curve requires its original RNG transaction"
        );
        super::quantize(
            start,
            end,
            kind,
            self.shaped_progress(count, max, subframe, kind)?,
        )
    }
    pub(crate) fn value_with_random(
        &self,
        count: i32,
        max: i32,
        start: i32,
        end: i32,
        subframe: i32,
        kind: i32,
        random: &mut PatternRng,
    ) -> Result<i32> {
        let mut progress = self.shaped_progress(count, max, subframe, kind)?;
        if kind / 200 != 0 {
            progress *= f64::from(random.next_draw()? % 10000) * 0.0001;
        }
        // The extra C++ kasokuType argument is always overwritten from kind.
        // Preserve signed32 endpoint subtraction and half-biased truncation.
        super::quantize(start, end, kind, progress)
    }
}
