// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy CHARAGREY124 (4B39B0/4B3B10/GreyBlt2), TRANS8 (4B2D70/4B2F10) and COLORADDSUB99 (4B3CD0/4B3E70).
//! Native parameters, curves and save counters only. Immutable Image graphs
//! perform clipping, saturated byte arithmetic and mask-weighted composition.
use anyhow::{ensure, Result};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use crysirror_image::{
    effects::ImageEffect, integer_color::ColorMatrixFixed, temporal_pattern::PatternRng,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PictureEffectKind {
    Trans {
        dummy: i32,
        percent: i32,
    },
    Grey {
        easing: i32,
        subframe: i32,
        end: i32,
        start: i32,
    },
    Flash {
        color: [i32; 3],
        normal: i32,
        colored: i32,
    },
    ColorAdd {
        end: [i32; 3],
        start: [i32; 3],
        transparent: i32,
        easing: i32,
        subframe: i32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PictureEffectState {
    pub kind: PictureEffectKind,
    pub frames: i32,
    pub count: i32,
    /// Original para0 is independent of the authoritative saved countMax.
    pub parameter_frames: i32,
}
impl PictureEffectState {
    pub fn parse(command: i32, p: &[i32]) -> Result<Self> {
        let get = |i: usize, default| p.get(i).copied().unwrap_or(default);
        let (kind, frames) = match command {
            // The first argument is saved but never controls countMax.
            8 => (
                PictureEffectKind::Trans {
                    dummy: get(0, 20),
                    percent: get(1, 50),
                },
                1,
            ),
            124 => {
                let grey =
                    crate::native_effects::LayerGreyState::parse(p.get(..p.len().min(5)).unwrap())?;
                (
                    PictureEffectKind::Grey {
                        easing: grey.easing,
                        subframe: grey.subframe,
                        end: grey.end_percent,
                        start: grey.start_percent,
                    },
                    grey.frames,
                )
            }
            90 => {
                let color = [
                    get(1, 255),
                    get(2, get(1, 255)),
                    get(3, get(2, get(1, 255))),
                ];
                (
                    PictureEffectKind::Flash {
                        color,
                        normal: get(4, 1).max(1),
                        colored: get(5, 1).max(1),
                    },
                    get(0, 20),
                )
            }
            99 => {
                let end = [get(1, 0), get(2, get(1, 0)), get(3, get(2, get(1, 0)))];
                (
                    PictureEffectKind::ColorAdd {
                        end,
                        start: [get(4, end[0]), get(5, end[1]), get(6, end[2])],
                        transparent: get(7, 1),
                        easing: get(8, 0),
                        subframe: get(9, 0),
                    },
                    get(0, 20),
                )
            }
            _ => anyhow::bail!("NNN unknown picture effect {command}"),
        };
        let state = Self {
            kind,
            frames,
            count: 0,
            parameter_frames: get(0, 20),
        };
        state.validate()?;
        Ok(state)
    }
    pub fn command(&self) -> i32 {
        match self.kind {
            PictureEffectKind::Trans { .. } => 8,
            PictureEffectKind::Grey { .. } => 124,
            PictureEffectKind::Flash { .. } => 90,
            PictureEffectKind::ColorAdd { .. } => 99,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (-864000..=1000000).contains(&self.frames),
            "NNN picture-effect frame budget"
        );
        if let PictureEffectKind::ColorAdd {
            easing, subframe, ..
        }
        | PictureEffectKind::Grey {
            easing, subframe, ..
        } = self.kind
        {
            crate::native_curve::validate(self.frames, subframe, easing)?;
        }
        Ok(())
    }
    pub fn parameters(&self) -> Vec<i32> {
        match self.kind {
            PictureEffectKind::Trans { dummy, percent } => vec![dummy, percent],
            PictureEffectKind::Grey {
                easing,
                subframe,
                end,
                start,
            } => vec![self.parameter_frames, easing, subframe, end, start],
            PictureEffectKind::Flash {
                color,
                normal,
                colored,
            } => vec![
                self.parameter_frames,
                color[0],
                color[1],
                color[2],
                normal,
                colored,
            ],
            PictureEffectKind::ColorAdd {
                end,
                start,
                transparent,
                easing,
                subframe,
            } => vec![
                self.parameter_frames,
                end[0],
                end[1],
                end[2],
                start[0],
                start[1],
                start[2],
                transparent,
                easing,
                subframe,
            ],
        }
    }
    pub fn count_up(&mut self, steps: u32) {
        // These CountIsMax methods never change command/flag (90 returns FALSE).
        if self.count < self.frames {
            self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        }
    }
    pub fn uses_mask(&self, background: bool) -> bool {
        match self.kind {
            PictureEffectKind::Trans { .. } => !background,
            PictureEffectKind::Flash { .. } | PictureEffectKind::Grey { .. } => true,
            PictureEffectKind::ColorAdd { transparent, .. } => transparent != 0,
        }
    }
}

#[derive(Clone)]
pub struct PictureEffectFrame {
    pub slot: i32,
    pub state: PictureEffectState,
    pub source: Option<Arc<dyn ImageResource>>,
    pub rectangles: crate::native_geometry::NativeRects,
    pub canvas: [u32; 2],
    pub has_mask: bool,
}
impl PictureEffectFrame {
    pub fn geometry(&self) -> crate::native_move::NativeMoveState {
        crate::native_move::NativeMoveState {
            count: self.state.count,
            frames: self.state.frames,
            ended: false,
            rectangles: self.rectangles.clone(),
        }
    }
    pub fn calcu(&mut self) {
        let mut geometry = self.geometry();
        geometry.calcu(self.source.is_some());
        self.rectangles = geometry.rectangles;
    }
    pub fn image_input(
        &self,
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<ImageInput>> {
        //124 evaluates its curve before the missing-picture return, unlike99.
        let grey_amount = if let PictureEffectKind::Grey {
            easing,
            subframe,
            end,
            start,
        } = self.state.kind
        {
            Some(
                (crate::native_curve::value_with_random(
                    self.state.count,
                    self.state.frames,
                    start.wrapping_mul(256),
                    end.wrapping_mul(256),
                    subframe,
                    easing,
                    random,
                )? / 100)
                    .clamp(0, 256) as u16,
            )
        } else {
            None
        };
        let Some(source) = &self.source else {
            return Ok(None);
        };
        // Native helpers return before curve evaluation if no picture exists.
        let (effects, weight) = match self.state.kind {
            PictureEffectKind::Grey { .. } => (
                vec![ImageEffect::FixedLuma {
                    spec: crysirror_image::fixed_luma::FixedLumaSpec {
                        coefficients: [76, 150, 29],
                        groups: [1, 0, 0],
                        fractional_bits: 8,
                        amount256: grey_amount.unwrap(),
                    },
                }],
                256,
            ),
            PictureEffectKind::Flash {
                color,
                normal,
                colored,
            } => {
                let period = normal.wrapping_add(colored).max(1);
                let flashing =
                    self.state.count < self.state.frames && self.state.count % period >= normal;
                if !flashing {
                    let [x, y, w, h] = self.rectangles.destination;
                    if w <= 0 || h <= 0 {
                        return Ok(None);
                    }
                    return Ok(Some(ImageInput::Composite {
                        base: Box::new(base),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(source.clone())),
                            source: [self.rectangles.source[0], self.rectangles.source[1], w, h],
                            destination: [x, y],
                            clip: None,
                            opacity: 255,
                            math: ImageCompositeMath::MaskSeparate256,
                        }],
                    }));
                }
                // ShapeBlt4D6C40 explicitly returns when the picture has no
                // mask. Its key is the first mask byte AFTER both source and
                // destination clipping, not alpha0 or the whole image corner.
                if !self.has_mask {
                    return Ok(None);
                }
                let [x, y, w, h] = self.rectangles.destination;
                let Some((region, _)) = crysirror_core::region_map::clip_region_copy(
                    Some(source.size()),
                    self.canvas,
                    [
                        i64::from(self.rectangles.source[0]),
                        i64::from(self.rectangles.source[1]),
                        i64::from(w),
                        i64::from(h),
                    ],
                    [i64::from(x), i64::from(y)],
                    None,
                )?
                else {
                    return Ok(None);
                };
                let key = ImageEffect::CoverageReferenceKey {
                    spec: crysirror_image::coverage_key::CoverageReferenceKey {
                        reference: [region[0].try_into()?, region[1].try_into()?],
                        channel: 3,
                        equal: 0,
                        different: 255,
                    },
                };
                let solid = ImageEffect::IntegerColorMatrix {
                    spec: ColorMatrixFixed {
                        matrix: [[0; 3]; 3],
                        offset: color.map(|n| i32::from(n as u8)),
                        divisor: 1,
                    },
                };
                (vec![key, solid], 256)
            }
            PictureEffectKind::Trans { percent, .. } => {
                (Vec::new(), (percent.clamp(0, 100) * 256 / 100) as u16)
            }
            PictureEffectKind::ColorAdd {
                end,
                start,
                easing,
                subframe,
                ..
            } => {
                let mut delta = [0; 3];
                for channel in 0..3 {
                    delta[channel] = crate::native_curve::value_with_random(
                        self.state.count,
                        self.state.frames,
                        start[channel],
                        end[channel],
                        subframe,
                        easing,
                        random,
                    )?;
                }
                // Original 4CBE70 packs signed shifted ints before PADDUSB /
                // PSUBUSB. Preserve spill between bytes and INT_MIN's wrapping
                // NEG instead of clamping each supplied native parameter.
                let pack = |v: [i32; 3]| -> [i32; 3] {
                    let word = (v[0] as u32).wrapping_shl(16)
                        | (v[1] as u32).wrapping_shl(8)
                        | v[2] as u32;
                    [
                        ((word >> 16) & 255) as i32,
                        ((word >> 8) & 255) as i32,
                        (word & 255) as i32,
                    ]
                };
                let add = pack(delta.map(|n| n.max(0)));
                let sub = pack(delta.map(|n| n.min(0).wrapping_neg()));
                let matrix = |offset| ImageEffect::IntegerColorMatrix {
                    spec: ColorMatrixFixed {
                        matrix: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
                        offset,
                        divisor: 1,
                    },
                };
                (vec![matrix(add), matrix(sub.map(|n| -n))], 256)
            }
        };
        let [x, y, w, h] = self.rectangles.destination;
        if w <= 0 || h <= 0 {
            return Ok(None);
        }
        let input = ImageInput::Resource(source.clone());
        let input = if effects.is_empty() {
            input
        } else {
            ImageInput::Process {
                input: Box::new(input),
                effects: serde_json::to_vec(&effects)?.into(),
            }
        };
        Ok(Some(ImageInput::Composite {
            base: Box::new(base),
            layers: vec![ImageCompositeLayer {
                input: Box::new(input),
                source: [self.rectangles.source[0], self.rectangles.source[1], w, h],
                destination: [x, y],
                clip: None,
                opacity: 255,
                math: if self.state.command() == 124 {
                    //GreyBlt2's partial-mask MMX increments source coverage before
                    //multiplication (4D6E10); ColorAddBlt deliberately does not.
                    ImageCompositeMath::MaskIncrement256
                } else {
                    ImageCompositeMath::MaskPiecewise256 { weight }
                },
            }],
        }))
    }
}
