// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy SHAKE18: VT5672C4, Set4B8AD0, Count4B8A80, Calcu4B8A20,
//! Print4B8BC0 and table5808B8. Legacy HAIKEIYURE100: VT567334, Set4B7D00,
//! CountIsMax4B7CC0, Calcu4B7C00, Print4B80E0 and MoveScreen4B7E90.
//! This module owns only native phase/rect policy; the shared immutable
//! Composite graph performs the original overlapping screen copy.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShakeState {
    pub frames: i32,
    pub axis: i32,
    pub loop_flag: i32,
    pub count: i32,
    pub kind: ShakeKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShakeKind {
    /// SHAKE18 fixed four-phase table.
    Table,
    /// HAIKEIYURE100 native para[0..14] and Calcu-owned para14/15.
    Yure { para: [i32; 14], phase: [i32; 2] },
}
impl ShakeState {
    pub fn parse(values: &[i32]) -> Self {
        // Native copies at most three arguments, ignoring later arguments.
        Self {
            frames: values.first().copied().unwrap_or(20),
            axis: values.get(1).copied().unwrap_or(0),
            loop_flag: values.get(2).copied().unwrap_or(0),
            count: 0,
            kind: ShakeKind::Table,
        }
    }
    /// 4B7D00 copies at most14 arguments with native defaults. Count and
    /// para14/15 start from para6; drawing is selected separately at Print.
    pub fn parse_yure(values: &[i32]) -> Result<Self> {
        let defaults = [20, 20, 0, 25, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut para = defaults;
        for (slot, value) in para.iter_mut().zip(values.iter().take(14)) {
            *slot = *value;
        }
        ensure!(
            matches!(para[5], 0 | 1),
            "NNN HAIKEIYURE invalid layer selector"
        );
        if para[11] != 0 {
            crate::native_curve::validate(para[0], para[13], para[12])?;
        }
        Ok(Self {
            frames: para[0],
            axis: 0,
            loop_flag: 0,
            count: 0,
            kind: ShakeKind::Yure {
                para,
                phase: [para[6], para[6]],
            },
        })
    }
    /// 4B80E0: signed x86 arithmetic, after the optional native speed curve.
    fn yure_offset(para: &[i32; 14], count: i32, scale: i32) -> [i32; 2] {
        let wave = |speed: i32| {
            let mut phase = (para[6]
                .wrapping_add(speed.wrapping_mul(count))
                .wrapping_add(75))
                % 100;
            phase = (phase - 25 + 100) % 100;
            if phase > 50 {
                phase = 100 - phase;
            }
            phase
        };
        let (lr, ud) = (
            para[1].wrapping_mul(scale) / 100,
            para[2].wrapping_mul(scale) / 100,
        );
        [
            wave(para[3])
                .wrapping_mul(2)
                .wrapping_mul(lr)
                .wrapping_div(50)
                .wrapping_sub(lr),
            wave(para[4])
                .wrapping_mul(2)
                .wrapping_mul(ud)
                .wrapping_div(50)
                .wrapping_sub(ud),
        ]
    }
    /// Native para words as stored in an original effect record.
    pub fn parameters(&self) -> Vec<i32> {
        match &self.kind {
            ShakeKind::Table => vec![self.frames, self.axis, self.loop_flag],
            ShakeKind::Yure { para, phase } => para.iter().chain(phase).copied().collect(),
        }
    }
    pub fn command(&self) -> i32 {
        match self.kind {
            ShakeKind::Table => 18,
            ShakeKind::Yure { .. } => 100,
        }
    }
    pub fn calcu(&mut self) {
        if let ShakeKind::Yure { para, phase } = &mut self.kind {
            // 4B7C00: phase += speed, clamped at zero, modulo 100.
            for (value, speed) in phase.iter_mut().zip([para[3], para[4]]) {
                *value = value.wrapping_add(speed).max(0) % 100;
            }
            return;
        }
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
    pub fn needs_picture_underlay(&self) -> bool {
        !matches!(&self.kind, ShakeKind::Yure { para, .. } if para[5] == 0)
    }
    pub fn image_input(&self, base: ImageInput, canvas: [u32; 2]) -> Result<ImageInput> {
        ensure!(
            !matches!(&self.kind,ShakeKind::Yure{para,..} if para[11]!=0 && para[12]/200!=0),
            "NNN random shake requires the shared global RNG transaction"
        );
        self.render(
            base,
            canvas,
            None,
            &mut crysirror_image::temporal_pattern::PatternRng::msvc(0),
        )
    }
    fn render(
        &self,
        base: ImageInput,
        canvas: [u32; 2],
        picture: Option<ImageInput>,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<ImageInput> {
        if self.count >= self.frames {
            return Ok(base);
        }
        if let ShakeKind::Yure { para, .. } = &self.kind {
            let scale = if para[11] != 0 {
                crate::native_curve::value_with_random(
                    self.count,
                    self.frames,
                    0,
                    100,
                    para[13],
                    para[12],
                    random,
                )?
            } else {
                100
            };
            let [dx, dy] = Self::yure_offset(para, self.count, scale);
            let [width, height] = [i32::try_from(canvas[0])?, i32::try_from(canvas[1])?];
            if para[5] != 1 {
                let Some(picture) = picture else {
                    return Ok(base);
                };
                let mut layers = vec![ImageCompositeLayer {
                    input: Box::new(picture.clone()),
                    source: [0, 0, width, height],
                    destination: [dx, dy],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                }];
                // Original picture copy followed by the horizontal exposed band,
                // then the vertical band excluding that first band's rows.
                // All clipping and pixel arithmetic belongs to shared Composite.
                let mut band = |r: [i32; 4]| {
                    if r[2] <= 0 || r[3] <= 0 {
                        return;
                    }
                    let input = if para[7] == 0 {
                        picture.clone()
                    } else {
                        ImageInput::Solid {
                            size: canvas,
                            color: [para[8] as u8, para[9] as u8, para[10] as u8, 255],
                        }
                    };
                    layers.push(ImageCompositeLayer {
                        input: Box::new(input),
                        source: r,
                        destination: [r[0], r[1]],
                        clip: None,
                        opacity: 255,
                        math: ImageCompositeMath::CopyRgb,
                    });
                };
                if dy != 0 {
                    band([
                        0,
                        if dy < 0 { height.saturating_add(dy) } else { 0 },
                        width,
                        dy.saturating_abs(),
                    ]);
                }
                if dx != 0 {
                    band([
                        if dx < 0 { width.saturating_add(dx) } else { 0 },
                        dy.max(0),
                        dx.saturating_abs(),
                        height.saturating_sub(dy.saturating_abs()),
                    ]);
                }
                return Ok(ImageInput::Composite {
                    base: Box::new(base),
                    layers,
                });
            }
            // 4B7E90 returns without copying for a zero or full-surface move.
            if dx == 0 && dy == 0
                || dx.unsigned_abs() >= width as u32
                || dy.unsigned_abs() >= height as u32
            {
                return Ok(base);
            }
            let source = [
                (-dx).max(0),
                (-dy).max(0),
                width - dx.abs(),
                height - dy.abs(),
            ];
            return Ok(ImageInput::Composite {
                base: Box::new(base.clone()),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(base),
                    source,
                    destination: [dx.max(0), dy.max(0)],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Copy,
                }],
            });
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
    pub picture: Option<std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>>,
}
impl ShakeFrame {
    pub fn image_input(
        &self,
        base: ImageInput,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<ImageInput> {
        self.state.render(
            base,
            self.canvas,
            self.picture.clone().map(ImageInput::Resource),
            random,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yure_matches_native_print_phase_and_admits_picture_and_speed_curves() {
        // MUGEN2 prologue: SETEFFECT layer13 effect100.
        let state =
            ShakeState::parse_yure(&[15, 6, 10, 25, 55, 1, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        let ShakeKind::Yure { para, .. } = &state.kind else {
            panic!()
        };
        // count0: phase ((0+0+75)%100-25+100)%100=50 -> dx=6*2*50/50-6.
        assert_eq!(ShakeState::yure_offset(para, 0, 100), [6, 10]);
        // count1: lr (25+75)%100=0 ->75 ->25 -> 0; ud (55+75)%100=30 ->5 -> -8.
        assert_eq!(ShakeState::yure_offset(para, 1, 100), [0, -8]);
        assert!(ShakeState::parse_yure(&[15, 6, 10, 25, 55, 0]).is_ok());
        assert!(ShakeState::parse_yure(&[15, 6, 10, 25, 55, 1, 0, 0, 0, 0, 0, 1]).is_ok());
    }
}
