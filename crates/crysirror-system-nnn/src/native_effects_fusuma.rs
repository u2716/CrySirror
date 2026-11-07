// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Native Fusuma84 parameter/count/two-picture policy. Complete pinned
//! effectFusuma.cpp/.h and original4267C0/4268E0/426930/426CA0/426CB0
//! (vtable550F14) were inspected. Shared composition owns all pixel work.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use crysirror_image::temporal_pattern::PatternRng;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusumaState {
    pub frames: i32,
    pub width: i32,
    pub picture: i32,
    pub easing: i32,
    pub subframe: i32,
    pub spare: [i32; 2],
    pub count: i32,
}
impl FusumaState {
    pub fn parse(values: &[i32], canvas: [u32; 2]) -> Result<Self> {
        let state = Self {
            frames: values.first().copied().unwrap_or(20),
            width: values
                .get(1)
                .copied()
                .unwrap_or(160)
                .min(canvas[0] as i32 / 2),
            picture: values.get(2).copied().unwrap_or(13),
            easing: values.get(3).copied().unwrap_or(0),
            subframe: values.get(4).copied().unwrap_or(0),
            //4267C0 copies seven words but initializes only five defaults.
            //The last two are never read by Print; initialize absent padding
            //instead of serializing original uninitialized stack memory.
            spare: [
                values.get(5).copied().unwrap_or(0),
                values.get(6).copied().unwrap_or(0),
            ],
            count: 0,
        };
        state.validate(canvas)?;
        Ok(state)
    }
    pub fn parameters(&self) -> [i32; 7] {
        [
            self.frames,
            self.width,
            self.picture,
            self.easing,
            self.subframe,
            self.spare[0],
            self.spare[1],
        ]
    }
    pub fn picture_slot(&self) -> i32 {
        self.picture.clamp(0, 15)
    }
    pub fn validate(&self, canvas: [u32; 2]) -> Result<()> {
        ensure!(
            canvas.into_iter().all(|n| n > 0 && n <= 16384)
                && u64::from(canvas[0]) * u64::from(canvas[1])
                    <= crysirror_image::effects::MAX_EFFECT_PIXELS,
            "NNN Fusuma shared canvas budget"
        );
        crate::native_curve::validate(self.frames, self.subframe, self.easing)
    }
    /// True requests a terminal repaint. Native CountIsMax returns true but
    /// never changes command84 or its count; this effect must remain installed.
    pub fn count_up(&mut self, steps: u32) -> bool {
        if self.count >= self.frames {
            return false;
        }
        self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        self.count == self.frames
    }
    pub(crate) fn opening_with_random(&self, random: &mut PatternRng) -> Result<i32> {
        crate::native_curve::value_with_random(
            self.count,
            self.frames,
            0,
            self.width,
            self.subframe,
            self.easing,
            random,
        )
    }
}
#[derive(Clone)]
pub struct FusumaFrame {
    pub state: FusumaState,
    pub canvas: [u32; 2],
    /// Native Blt(...FALSE) ignores masks, including at the zero-dot endpoint.
    pub first: Option<Arc<dyn ImageResource>>,
    pub second: Option<Arc<dyn ImageResource>>,
}
impl FusumaFrame {
    pub(crate) fn image_input_with_random(
        &self,
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<ImageInput> {
        self.state.validate(self.canvas)?;
        let (Some(first), Some(second)) = (&self.first, &self.second) else {
            return Ok(base);
        };
        //Original426930 checks BOTH pictures/buffers before sampling its curve.
        //A missing/small secondary picture consumes no process-global RNG,
        //even when dot would be zero and only the first image would be drawn.
        if [first.size(), second.size()]
            .into_iter()
            .any(|size| size[0] < self.canvas[0] || size[1] < self.canvas[1])
        {
            return Ok(base);
        }
        ensure!(
            first.opaque() && second.opaque(),
            "NNN Fusuma requires shared opaque picture preparation"
        );
        let dot = self.state.opening_with_random(random)?;
        let width = self.canvas[0] as i32;
        let height = self.canvas[1] as i32;
        let mut layers = Vec::with_capacity(5);
        let mut blit = |image: &Arc<dyn ImageResource>, dx: i32, sx: i32, extent: i32| {
            if extent > 0 {
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(image.clone())),
                    source: [sx, 0, extent, height],
                    destination: [dx, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Copy,
                });
            }
        };
        if dot == 0 {
            blit(first, 0, 0, width);
        } else {
            let outer = (width / 2).wrapping_sub(self.state.width);
            let inner = self.state.width.wrapping_sub(dot);
            blit(first, 0, 0, outer);
            blit(first, outer, outer.wrapping_add(dot), inner);
            blit(
                first,
                width.wrapping_sub(outer).wrapping_sub(inner),
                width / 2,
                inner,
            );
            blit(
                first,
                width.wrapping_sub(outer),
                width.wrapping_sub(outer),
                outer,
            );
            let middle = outer.wrapping_add(inner);
            blit(second, middle, middle, dot.wrapping_mul(2));
        }
        //Keep the original five blits and their integer halves. For an odd
        //canvas width the original leaves a one-column gap; do not stretch or
        //synthesize a pixel to hide it. Shared clipping preserves lower paint.
        Ok(ImageInput::Composite {
            base: Box::new(base),
            layers,
        })
    }
}
