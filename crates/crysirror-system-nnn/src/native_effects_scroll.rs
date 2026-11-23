// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Native Scroll39 policy, not a raster implementation. Complete pinned
//! effectScroll.cpp/.h and original SetParam426400/CountIsMax4264E0/
//! Calcu426540/Print426590 (vtable550EDC) were statically compared.
//! Four clipped opaque picture blits are shared ImageInput operations.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollState {
    pub frames: i32,
    pub delta: [i32; 2],
    /// Preserve the native integer, not just a normalized boolean, in saves.
    pub loop_flag: i32,
    pub count: i32,
}
impl ScrollState {
    pub fn parse(values: &[i32], canvas: [u32; 2]) -> Result<Self> {
        //426400 copies only the first four arguments; extra arguments are
        //explicitly ignored by the native method, not an unknown-op fallback.
        let state = Self {
            frames: values.first().copied().unwrap_or(20),
            delta: [
                values.get(1).copied().unwrap_or(32),
                values.get(2).copied().unwrap_or(0),
            ],
            loop_flag: values.get(3).copied().unwrap_or(0),
            count: 0,
        };
        state.validate(canvas)?;
        Ok(state)
    }
    pub fn parameters(&self) -> [i32; 4] {
        [self.frames, self.delta[0], self.delta[1], self.loop_flag]
    }
    pub fn validate(&self, canvas: [u32; 2]) -> Result<()> {
        ensure!(
            canvas.into_iter().all(|n| n > 0 && n <= 16384)
                && u64::from(canvas[0]) * u64::from(canvas[1])
                    <= crysirror_image::effects::MAX_EFFECT_PIXELS,
            "NNN Scroll shared canvas budget"
        );
        Ok(())
    }
    /// Called once in the native Calcu phase, before picture/effect Print.
    /// The original subtracts once, not a modulo reduction or while loop.
    pub fn calcu(&mut self) {
        if self.loop_flag != 0 && self.count >= self.frames {
            self.count = self.count.wrapping_sub(self.frames);
        }
    }
    /// Return true when the native command becomes NOP. A looping Scroll
    /// instead resets only count, preserving parameters and previousEffect.
    pub fn count_is_max(&mut self) -> bool {
        if self.loop_flag == 0 {
            true
        } else {
            self.count = 0;
            false
        }
    }
    pub fn count_up(&mut self, steps: u32) -> bool {
        //CAllEffect::CountUp does not invoke CountIsMax for an already-ended
        //or nonpositive-duration effect; it only invokes it on a new arrival.
        if self.count >= self.frames {
            return false;
        }
        self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        self.count == self.frames && self.count_is_max()
    }
    pub fn displacement(&self, canvas: [u32; 2]) -> Result<[i32; 2]> {
        self.validate(canvas)?;
        let count = if self.frames < 1 { 1 } else { self.count };
        Ok(std::array::from_fn(|axis| {
            let extent = canvas[axis] as i32;
            let value = self.delta[axis].wrapping_mul(count);
            //Original426590 uses signed imul/neg/idiv. In particular negating
            //INT_MIN wraps before the remainder; rem_euclid would change that
            //defined machine result. Extents are positive and bounded above.
            let value = if value <= 0 {
                extent.wrapping_sub(value.wrapping_neg() % extent)
            } else {
                value
            };
            value % extent
        }))
    }
}
#[derive(Clone)]
pub struct ScrollFrame {
    pub state: ScrollState,
    pub canvas: [u32; 2],
    /// The assigned native picture with its alpha/mask already ignored through
    /// shared image preparation. Never substitute a captured screen here.
    pub source: Option<Arc<dyn ImageResource>>,
}
impl ScrollFrame {
    pub fn image_input(&self, base: ImageInput) -> Result<ImageInput> {
        let offset = self.state.displacement(self.canvas)?;
        let Some(source) = self.source.as_ref() else {
            return Ok(base);
        };
        ensure!(
            source.opaque(),
            "NNN Scroll requires shared opaque picture preparation"
        );
        let size = source.size();
        ensure!(
            size.into_iter().all(|n| n > 0),
            "NNN Scroll source dimensions"
        );
        let extent = [
            size[0].min(self.canvas[0]) as i32,
            size[1].min(self.canvas[1]) as i32,
        ];
        let mut layers = Vec::with_capacity(4);
        for y in -1..=0 {
            for x in -1..=0 {
                layers.push(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(source.clone())),
                    source: [0, 0, extent[0], extent[1]],
                    destination: [
                        offset[0] + x * self.canvas[0] as i32,
                        offset[1] + y * self.canvas[1] as i32,
                    ],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Copy,
                });
            }
        }
        //Native CPicture::Blt clips a small picture, retaining the canvas-sized
        //tile spacing. Unwritten gaps retain lower painter operations in base.
        Ok(ImageInput::Composite {
            base: Box::new(base),
            layers,
        })
    }
}
