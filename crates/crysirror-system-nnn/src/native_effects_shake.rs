// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy SHAKE18: VT5672C4, Set4B8AD0, Count4B8A80, Calcu4B8A20,
//! Print4B8BC0 and table5808B8. This module owns only native phase/rect policy;
//! the shared immutable Composite graph performs the original overlapping copy.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShakeState {
    pub frames: i32,
    pub axis: i32,
    pub loop_flag: i32,
    pub count: i32,
}
impl ShakeState {
    pub fn parse(values: &[i32]) -> Self {
        // Native copies at most three arguments, ignoring later arguments.
        Self {
            frames: values.first().copied().unwrap_or(20),
            axis: values.get(1).copied().unwrap_or(0),
            loop_flag: values.get(2).copied().unwrap_or(0),
            count: 0,
        }
    }
    pub fn parameters(&self) -> [i32; 3] {
        [self.frames, self.axis, self.loop_flag]
    }
    pub fn calcu(&mut self) {
        // This surprising native comparison is loop_flag==0, not !=0.
        // Subtraction is once per Calcu and is x86 wrapping, not modulo.
        if self.loop_flag == 0 && self.count >= self.frames {
            self.count = self.count.wrapping_sub(self.frames);
        }
    }
    /// True only when a new endpoint converts this native command to NOP.
    pub fn count_up(&mut self, steps: u32) -> bool {
        if self.count >= self.frames {
            return false;
        }
        self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        if self.count != self.frames {
            return false;
        }
        if self.loop_flag == 0 {
            true
        } else {
            self.count = 0;
            false
        }
    }
    pub fn image_input(&self, base: ImageInput, canvas: [u32; 2]) -> Result<ImageInput> {
        if self.count >= self.frames {
            return Ok(base);
        }
        let phase = self.frames.wrapping_sub(self.count) % 4;
        ensure!(
            (0..4).contains(&phase),
            "NNN SHAKE phase outside native table"
        );
        let delta: i32 = [-16, -12, 16, 12][phase as usize];
        let axis = usize::from(self.axis != 0);
        ensure!(
            canvas[axis] > delta.unsigned_abs(),
            "NNN SHAKE copy exceeds shared surface"
        );
        let mut source = [0, 0, i32::try_from(canvas[0])?, i32::try_from(canvas[1])?];
        let mut destination = [0, 0];
        source[axis + 2] -= delta.abs();
        if delta > 0 {
            source[axis] = delta;
        } else {
            destination[axis] = -delta;
        }
        // The original memmove leaves the exposed edge unchanged. Copying the
        // immutable pre-effect input over itself reproduces that behavior in
        // both directions without an adapter-owned pixel loop.
        Ok(ImageInput::Composite {
            base: Box::new(base.clone()),
            layers: vec![ImageCompositeLayer {
                input: Box::new(base),
                source,
                destination,
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::Copy,
            }],
        })
    }
}
#[derive(Clone)]
pub struct ShakeFrame {
    pub slot: i32,
    pub state: ShakeState,
    pub canvas: [u32; 2],
    pub visible: bool,
}
