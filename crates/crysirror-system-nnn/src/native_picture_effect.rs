// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native picture-effect parameters, save words and clocks. Motion, blur and
//! tone policies live in the adjacent modules; all raster work uses shared jobs.
//! Legacy GREY124 is 4B39B0/4B3B10/GreyBlt2, TRANS8 is 4B2D70/4B2F10,
//! and COLORADDSUB99 is 4B3CD0/4B3E70. CHARASEPIA127 follows CSepiaBlt.
//! Native parameters, curves and save counters only. Immutable Image graphs
//! perform clipping, saturated byte arithmetic and mask-weighted composition.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use crysirror_image::{
    effects::ImageEffect, integer_color::ColorMatrixFixed, temporal_pattern::PatternRng,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PictureEffectKind {
    Rainbow{parameters:[i32;16]},
    Meisai{parameters:[i32;16]},
    Relief {parameters:[i32;16]},
    OldFilm {parameters:[i32;56]},
    CharaBlur {parameters:[i32;16]},
    BackgroundBlur {parameters:[i32;16]},
    DualMask {parameters:[i32;12]},
    MaskAppear {parameters:[i32;8]},
    Sentence {parameters:[i32;16]},
    StretchBlit {parameters:[i32;25]},
    ClipZoom {parameters:[i32;33]},
    Koma { parameters: [i32;16] },
    Ghost { parameters: [i32;16] },
    Noise { parameters: [i32; 144] },
    Target { parameters: [i32; 16] },
    CharaRaster { parameters: [i32; 16] },
    Saike {
        parameters: [i32; 20],
    },
    Lupin {
        parameters: [i32; 4],
    },
    MeltDown {
        parameters: [i32; 3],
    },
    Curtain {
        parameters: [i32; 16],
    },
    Raster {
        parameters: [i32; 5],
    },
    ScrollLoop {
        parameters: [i32; 16],
    },
    Quadratic {
        parameters: [i32; 9],
    },
    Bezier {
        parameters: [i32; 11],
    },
    BoxBlur {
        parameters: [i32; 16],
    },
    Jitter {
        parameters: [i32; 26],
    },
    Zoom {
        parameters: [i32; 9],
    },
    ScreenGradient {
        parameters: [i32; 18],
    },
    ScreenLight {
        parameters: [i32; 6],
    },
    AddSubtract {
        parameters: [i32; 5],
    },
    Stretch {
        parameters: [i32; 7],
    },
    /// Original GET/PUT/BUFFER/SIMPLEWIPE records. Fixed-size storage retains
    /// the native save words without exposing allocator/pointer state.
    Buffered {
        command: i32,
        parameters: [i32; 16],
    },
    Trans {
        dummy: i32,
        percent: i32,
    },
    Grey {
        easing: i32,
        subframe: i32,
        end: i32,
        start: i32,
        tint: Option<[i32; 3]>,
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
        balance: Option<([u8; 3], bool)>,
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
    pub(crate) fn parse_for_canvas(command: i32, p: &[i32], canvas: [u32; 2]) -> Result<Self> {
        if command==55 {
            let parameters=crate::native_picture_old_film::parameters(p,canvas)?;
            let state=Self{frames:9999,count:0,parameter_frames:parameters[0],kind:PictureEffectKind::OldFilm{parameters}};
            state.validate()?;return Ok(state);
        }
        if command==122 {
            let parameters=crate::native_picture_clip_zoom::parameters(p,canvas);
            let state=Self{frames:parameters[0],count:0,parameter_frames:parameters[0],kind:PictureEffectKind::ClipZoom{parameters}};
            state.validate()?;return Ok(state);
        }
        if command == 103 {
            let parameters = crate::native_picture_target::parameters(p, canvas);
            let state=Self {frames:parameters[0],count:0,parameter_frames:parameters[0],
                kind:PictureEffectKind::Target{parameters}};
            state.validate()?;return Ok(state);
        }
        if command == 23 {
            let parameters = crate::native_picture_motion::raster_parameters(p, canvas);
            let state = Self {
                frames: parameters[0],
                count: 0,
                parameter_frames: parameters[0],
                kind: PictureEffectKind::Raster { parameters },
            };
            state.validate()?;
            return Ok(state);
        }
        if command == 37 {
            let parameters = crate::native_picture_motion::zoom_parameters(p, canvas)?;
            let state = Self {
                frames: parameters[0],
                count: 0,
                parameter_frames: parameters[0],
                kind: PictureEffectKind::Zoom { parameters },
            };
            state.validate()?;
            return Ok(state);
        }
        if command == 50 {
            ensure!(
                canvas.iter().all(|v| (1..=16384).contains(v)),
                "NNN GRADATION canvas"
            );
            let get = |i: usize, d| p.get(i).copied().unwrap_or(d);
            // C++ copies unused para17 from uninitialised stack storage when
            // omitted. Define it as zero; supplied/saved words still round-trip.
            let mut parameters = std::array::from_fn(|i| {
                get(
                    i,
                    match i {
                        0 => 20,
                        4..=6 => 255,
                        16 => canvas[1] as i32,
                        _ => 0,
                    },
                )
            });
            parameters[15] = parameters[15].clamp(0, canvas[1] as i32 - 1);
            parameters[16] = parameters[16].min(canvas[1] as i32 - parameters[15]);
            let state = Self {
                frames: parameters[0],
                count: 0,
                parameter_frames: parameters[0],
                kind: PictureEffectKind::ScreenGradient { parameters },
            };
            state.validate()?;
            return Ok(state);
        }
        if command != 36 {
            return Self::parse(command, p);
        }
        let parameters = crate::native_stretch::parameters(p, canvas)?;
        let state = Self {
            kind: PictureEffectKind::Stretch { parameters },
            frames: parameters[0],
            count: 0,
            parameter_frames: parameters[0],
        };
        state.validate()?;
        Ok(state)
    }
    pub fn parse(command: i32, p: &[i32]) -> Result<Self> {
        let get = |i: usize, default| p.get(i).copied().unwrap_or(default);
        let (kind, frames) = match command {
            72=>{let parameters=crate::native_picture_meisai::parameters(p);(PictureEffectKind::Meisai{parameters},parameters[0])}
            96=>{let parameters=crate::native_picture_rainbow::parameters(p);(PictureEffectKind::Rainbow{parameters},parameters[0])}
            33=>{let parameters=crate::native_picture_releaf::parameters(p);(PictureEffectKind::Relief{parameters},parameters[0])}
            55=>{let parameters=crate::native_picture_old_film::parameters(p,[800,600])?;(PictureEffectKind::OldFilm{parameters},9999)}
            125=>{let parameters=crate::native_picture_chara_blur::parameters(p);(PictureEffectKind::CharaBlur{parameters},parameters[0])}
            126=>{let parameters=crate::native_picture_background_blur::parameters(p);(PictureEffectKind::BackgroundBlur{parameters},parameters[0])}
            65=>{let parameters=crate::native_picture_stretch_blt::parameters(p);(PictureEffectKind::StretchBlit{parameters},parameters[0])}
            121=>{let parameters=crate::native_picture_dual_mask::parameters(p);(PictureEffectKind::DualMask{parameters},parameters[0])}
            122=>{let parameters=crate::native_picture_clip_zoom::parameters(p,[800,600]);(PictureEffectKind::ClipZoom{parameters},parameters[0])}
            123=>{let parameters=crate::native_picture_mask_appear::parameters(p).ok_or_else(||anyhow::anyhow!("NNN CHARAmaskAppear requires two parameters"))?;(PictureEffectKind::MaskAppear{parameters},parameters[0])}
            133=>{let parameters=crate::native_picture_sentence::parameters(p);(PictureEffectKind::Sentence{parameters},parameters[0])}
            63 => {
                let parameters=crate::native_picture_koma::parameters(p);
                (PictureEffectKind::Koma{parameters},parameters[0])
            }
            110 => {
                let parameters=crate::native_picture_ghost::parameters(p);
                (PictureEffectKind::Ghost{parameters},parameters[0])
            }
            106 => {
                let parameters=crate::native_picture_noise::parameters(p);
                (PictureEffectKind::Noise{parameters},parameters[0])
            }
            107 => {
                let parameters = crate::native_picture_raster::parameters(p);
                (PictureEffectKind::CharaRaster { parameters }, parameters[0])
            }
            130 => {
                let parameters = crate::native_picture_tone::parameters(p);
                (PictureEffectKind::Saike { parameters }, parameters[0])
            }
            113 => {
                let r = get(1, 0);
                let g = get(2, r);
                let parameters = [get(0, 20), r, g, get(3, g)];
                (PictureEffectKind::Lupin { parameters }, 9999)
            }
            32 => {
                let parameters = [get(0, 20), get(1, 0), get(2, 0)];
                (PictureEffectKind::MeltDown { parameters }, parameters[0])
            }
            68 => {
                let parameters = crate::native_picture_motion::curtain_parameters(p);
                (PictureEffectKind::Curtain { parameters }, parameters[0])
            }
            94 => {
                let parameters = crate::native_picture_motion::scroll_parameters(p);
                (PictureEffectKind::ScrollLoop { parameters }, 9999)
            }
            119 => {
                let parameters = crate::native_picture_motion::quadratic_parameters(p);
                (PictureEffectKind::Quadratic { parameters }, parameters[0])
            }
            62 => {
                let parameters = crate::native_picture_motion::bezier_parameters(p);
                (PictureEffectKind::Bezier { parameters }, parameters[0])
            }
            77 => {
                let parameters = crate::native_picture_blur::parameters(p);
                (PictureEffectKind::BoxBlur { parameters }, parameters[0])
            }
            132 => {
                let parameters = crate::native_picture_motion::jitter_parameters(p);
                (PictureEffectKind::Jitter { parameters }, parameters[0])
            }
            20 => {
                let r = get(1, 64);
                let g = get(2, r);
                let b = get(3, g);
                let p = [get(0, 20), r, g, b, get(4, 0), get(5, 0)];
                (PictureEffectKind::ScreenLight { parameters: p }, p[0])
            }
            131 => {
                let end = get(1, 100).clamp(-9999, 9999);
                let p = [
                    get(0, 20),
                    end,
                    get(2, end).clamp(-9999, 9999),
                    get(3, 0),
                    get(4, 0),
                ];
                (PictureEffectKind::AddSubtract { parameters: p }, p[0])
            }
            2 | 59 | 66 | 73 | 75 => {
                let parameters = super::native_capture_effect::parameters(command, p)?;
                (
                    PictureEffectKind::Buffered {
                        command,
                        parameters,
                    },
                    parameters[0],
                )
            }
            // The first argument is saved but never controls countMax.
            8 => (
                PictureEffectKind::Trans {
                    dummy: get(0, 20),
                    percent: get(1, 50),
                },
                1,
            ),
            124 | 127 => {
                let grey =
                    crate::native_effects::LayerGreyState::parse(p.get(..p.len().min(5)).unwrap())?;
                (
                    PictureEffectKind::Grey {
                        easing: grey.easing,
                        subframe: grey.subframe,
                        end: grey.end_percent,
                        start: grey.start_percent,
                        tint: (command == 127).then(|| [get(5, 107), get(6, 74), get(7, 43)]),
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
            99 | 120 => {
                let end = [get(1, 0), get(2, get(1, 0)), get(3, get(2, get(1, 0)))];
                (
                    PictureEffectKind::ColorAdd {
                        end,
                        start: [get(4, end[0]), get(5, end[1]), get(6, end[2])],
                        transparent: get(7, 1),
                        easing: get(8, 0),
                        subframe: get(9, 0),
                        balance: if command == 120 {
                            let bands = [get(10, 0), get(11, 0), get(12, 0)];
                            ensure!(
                                bands.iter().all(|v| (0..10).contains(v)),
                                "NNN color balance band index"
                            );
                            Some((bands.map(|v| v as u8), get(13, 0) != 0))
                        } else {
                            None
                        },
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
            PictureEffectKind::Rainbow{..}=>96,
            PictureEffectKind::Meisai{..}=>72,
            PictureEffectKind::Relief{..}=>33,
            PictureEffectKind::OldFilm{..}=>55,
            PictureEffectKind::CharaBlur{..}=>125,
            PictureEffectKind::BackgroundBlur{..}=>126,
            PictureEffectKind::DualMask{..}=>121,
            PictureEffectKind::MaskAppear{..}=>123,
            PictureEffectKind::Sentence{..}=>133,
            PictureEffectKind::StretchBlit{..}=>65,
            PictureEffectKind::ClipZoom{..}=>122,
            PictureEffectKind::Koma { .. } => 63,
            PictureEffectKind::Ghost { .. } => 110,
            PictureEffectKind::Noise { .. } => 106,
            PictureEffectKind::Target { .. } => 103,
            PictureEffectKind::CharaRaster { .. } => 107,
            PictureEffectKind::Saike { .. } => 130,
            PictureEffectKind::Lupin { .. } => 113,
            PictureEffectKind::MeltDown { .. } => 32,
            PictureEffectKind::Curtain { .. } => 68,
            PictureEffectKind::Raster { .. } => 23,
            PictureEffectKind::ScrollLoop { .. } => 94,
            PictureEffectKind::Quadratic { .. } => 119,
            PictureEffectKind::Bezier { .. } => 62,
            PictureEffectKind::BoxBlur { .. } => 77,
            PictureEffectKind::Jitter { .. } => 132,
            PictureEffectKind::Zoom { .. } => 37,
            PictureEffectKind::ScreenGradient { .. } => 50,
            PictureEffectKind::ScreenLight { .. } => 20,
            PictureEffectKind::AddSubtract { .. } => 131,
            PictureEffectKind::Stretch { .. } => 36,
            PictureEffectKind::Buffered { command, .. } => command,
            PictureEffectKind::Trans { .. } => 8,
            PictureEffectKind::Grey { tint, .. } => {
                if tint.is_some() {
                    127
                } else {
                    124
                }
            }
            PictureEffectKind::Flash { .. } => 90,
            PictureEffectKind::ColorAdd { balance, .. } => {
                if balance.is_some() {
                    120
                } else {
                    99
                }
            }
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self.kind {
            PictureEffectKind::Rainbow{parameters}=>crate::native_picture_rainbow::validate(&parameters,self.frames)?,
            PictureEffectKind::Meisai{parameters}=>crate::native_picture_meisai::validate(&parameters,self.frames)?,
            PictureEffectKind::Relief{parameters}=>crate::native_picture_releaf::validate(&parameters,self.frames)?,
            PictureEffectKind::OldFilm{parameters}=>crate::native_picture_old_film::validate(&parameters)?,
            PictureEffectKind::CharaBlur{parameters}=>crate::native_picture_chara_blur::validate(&parameters,self.frames)?,
            PictureEffectKind::BackgroundBlur{parameters}=>crate::native_picture_background_blur::validate(&parameters,self.frames)?,
            PictureEffectKind::DualMask{parameters}=>crate::native_picture_dual_mask::validate(&parameters,self.frames)?,
            PictureEffectKind::MaskAppear{parameters}=>crate::native_picture_mask_appear::validate(&parameters,self.frames)?,
            PictureEffectKind::Sentence{parameters}=>crate::native_picture_sentence::validate(&parameters,self.frames)?,
            PictureEffectKind::StretchBlit{parameters}=>crate::native_picture_stretch_blt::validate(&parameters,self.frames)?,
            PictureEffectKind::ClipZoom{parameters}=>crate::native_picture_clip_zoom::validate(&parameters,self.frames)?,
            _=>(),
        }
        if let PictureEffectKind::Ghost{parameters:p}=self.kind {
            crate::native_picture_ghost::validate(&p,self.frames)?;
        }
        if let PictureEffectKind::Noise{parameters:p}=self.kind {
            crate::native_picture_noise::validate(&p)?;
            if p[9]!=100||p[10]!=100 {crate::native_curve::validate(self.frames,p[12],p[11])?;}
        }
        if let PictureEffectKind::Target { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames,p[11],p[10])?;
        }
        if let PictureEffectKind::CharaRaster { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[5], p[4])?;
            if p[6] != 100 || p[7] != 100 {
                crate::native_curve::validate(self.frames, p[9], p[8])?;
            }
        }
        if let PictureEffectKind::Saike { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[2], p[1])?;
            crate::native_curve::validate(self.frames, p[16], p[15])?;
        }
        if let PictureEffectKind::MeltDown { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[2], p[1])?;
        }
        if let PictureEffectKind::Curtain { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[7], p[6])?;
            crate::native_curve::validate(self.frames, p[12], p[11])?;
        }
        if let PictureEffectKind::Raster { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[4], p[3])?;
        }
        if let PictureEffectKind::Quadratic { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[8], p[7])?;
        }
        if let PictureEffectKind::Bezier { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[10], p[9])?;
        }
        if let PictureEffectKind::BoxBlur { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[6], p[5])?;
        }
        if let PictureEffectKind::Jitter { parameters: p } = self.kind {
            crate::native_curve::validate(p[21], p[7], p[6])?;
        }
        if let PictureEffectKind::Zoom { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[6], p[5])?;
        }
        if let PictureEffectKind::ScreenGradient { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[8], p[7])?;
        }
        if let PictureEffectKind::ScreenLight { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[5], p[4])?;
        }
        ensure!(
            (-864000..=1000000).contains(&self.frames),
            "NNN picture-effect frame budget"
        );
        if let PictureEffectKind::Buffered {
            command: 59,
            parameters: p,
        } = self.kind
        {
            ensure!(
                crate::native_simple_wipe::supported(p[1]),
                "NNN SIMPLEWIPE selector {}",
                p[1]
            );
            crate::native_curve::validate(self.frames, p[4], p[3])?;
        }
        if let PictureEffectKind::Stretch { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[6], p[5])?;
        }
        if let PictureEffectKind::AddSubtract { parameters: p } = self.kind {
            crate::native_curve::validate(self.frames, p[4], p[3])?;
        }
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
            PictureEffectKind::Rainbow{parameters}|PictureEffectKind::Meisai{parameters}=>parameters.to_vec(),
            PictureEffectKind::Relief{parameters}|PictureEffectKind::CharaBlur{parameters}|PictureEffectKind::BackgroundBlur{parameters}=>parameters.to_vec(),
            PictureEffectKind::OldFilm{parameters}=>parameters.to_vec(),
            PictureEffectKind::DualMask{parameters}=>parameters.to_vec(),
            PictureEffectKind::MaskAppear{parameters}=>parameters.to_vec(),
            PictureEffectKind::Sentence{parameters}=>parameters.to_vec(),
            PictureEffectKind::StretchBlit{parameters}=>parameters.to_vec(),
            PictureEffectKind::ClipZoom{parameters}=>parameters.to_vec(),
            PictureEffectKind::Koma { parameters } | PictureEffectKind::Ghost { parameters } => parameters.to_vec(),
            PictureEffectKind::Noise { parameters } => parameters.to_vec(),
            PictureEffectKind::Target { parameters } => parameters.to_vec(),
            PictureEffectKind::CharaRaster { parameters } => parameters.to_vec(),
            PictureEffectKind::Saike { parameters } => parameters.to_vec(),
            PictureEffectKind::Lupin { parameters } => parameters.to_vec(),
            PictureEffectKind::MeltDown { parameters } => parameters.to_vec(),
            PictureEffectKind::Curtain { parameters } => parameters.to_vec(),
            PictureEffectKind::Raster { parameters } => parameters.to_vec(),
            PictureEffectKind::ScrollLoop { parameters } => parameters.to_vec(),
            PictureEffectKind::Quadratic { parameters } => parameters.to_vec(),
            PictureEffectKind::Bezier { parameters } => parameters.to_vec(),
            PictureEffectKind::BoxBlur { parameters } => parameters.to_vec(),
            PictureEffectKind::Jitter { parameters } => parameters.to_vec(),
            PictureEffectKind::Zoom { parameters } => parameters.to_vec(),
            PictureEffectKind::ScreenGradient { parameters } => parameters.to_vec(),
            PictureEffectKind::ScreenLight { parameters } => parameters.to_vec(),
            PictureEffectKind::AddSubtract { parameters } => parameters.to_vec(),
            PictureEffectKind::Stretch { parameters } => parameters.to_vec(),
            PictureEffectKind::Buffered {
                command,
                parameters,
            } => parameters[..match command {
                59 => 9,
                66 => 3,
                _ => 16,
            }]
                .to_vec(),
            PictureEffectKind::Trans { dummy, percent } => vec![dummy, percent],
            PictureEffectKind::Grey {
                easing,
                subframe,
                end,
                start,
                tint,
            } => {
                let mut parameters = vec![self.parameter_frames, easing, subframe, end, start];
                if let Some(tint) = tint {
                    parameters.extend(tint);
                }
                parameters
            }
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
                balance,
            } => {
                let mut parameters = vec![
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
                ];
                if let Some((bands, light)) = balance {
                    parameters.extend(bands.map(i32::from));
                    parameters.push(i32::from(light));
                }
                parameters
            }
        }
    }
    pub fn count_up(&mut self, steps: u32) {
        // These CountIsMax methods never change command/flag (90 returns FALSE).
        if self.count < self.frames {
            self.count = self.count.wrapping_add(steps as i32).min(self.frames);
        }
    }
    /// Native GetPicture clamps its bank index. The legacy profile uses the
    /// configured picture-bank bound; invalid GetDstPoint indices formerly
    /// accessed outside native EFFECT storage, so use the same bounded bank.
    pub(crate) fn companion_slot(&self,layers:u16)->Option<(i32,bool)> {
        match self.kind {
            PictureEffectKind::Buffered{command:59,parameters}=>Some((parameters[2],false)),
            PictureEffectKind::DualMask{parameters}=>Some((parameters[1].clamp(0,i32::from(layers)-1),true)),
            PictureEffectKind::MaskAppear{parameters} if parameters[1]!=-1=>Some((parameters[1].clamp(0,i32::from(layers)-1),false)),
            _=>None,
        }
    }
    pub fn uses_mask(&self, background: bool) -> bool {
        match self.kind {
            PictureEffectKind::Rainbow{..}|PictureEffectKind::Meisai{..}=>true,
            PictureEffectKind::Relief{..}|PictureEffectKind::OldFilm{..}|PictureEffectKind::BackgroundBlur{..}=>false,
            PictureEffectKind::CharaBlur{..}=>true,
            PictureEffectKind::DualMask{..}|PictureEffectKind::MaskAppear{..}|PictureEffectKind::Sentence{..}=>true,
            PictureEffectKind::StretchBlit{parameters}=>parameters[9]!=0 && parameters[9]!=1,
            PictureEffectKind::ClipZoom{..}=>false,
            PictureEffectKind::Koma { parameters } => parameters[8]!=0,
            PictureEffectKind::Ghost { .. } => true,
            PictureEffectKind::Noise { .. } => true,
            PictureEffectKind::Target { .. } => false,
            PictureEffectKind::CharaRaster { .. } => true,
            PictureEffectKind::Saike { .. } => true,
            PictureEffectKind::Lupin { .. } => false,
            PictureEffectKind::MeltDown { .. } => false,
            PictureEffectKind::Curtain { .. } => false,
            PictureEffectKind::Raster { .. } | PictureEffectKind::ScrollLoop { .. } => false,
            PictureEffectKind::Quadratic { .. } => !background,
            PictureEffectKind::Bezier { .. } => !background,
            PictureEffectKind::BoxBlur { .. } => false,
            PictureEffectKind::Jitter { .. } => !background,
            PictureEffectKind::Zoom { .. } => false,
            PictureEffectKind::ScreenGradient { .. } => false,
            PictureEffectKind::ScreenLight { .. } => false,
            PictureEffectKind::AddSubtract { .. } => true,
            PictureEffectKind::Stretch { .. } => false,
            PictureEffectKind::Buffered { command: 75, .. } => !background,
            PictureEffectKind::Buffered { .. } => false,
            PictureEffectKind::Trans { .. } => !background,
            PictureEffectKind::Flash { .. } | PictureEffectKind::Grey { .. } => true,
            PictureEffectKind::ColorAdd {
                transparent,
                balance,
                ..
            } => balance.is_some() || transparent != 0,
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
    pub(crate) source_clip: Option<[i32;4]>,
    pub(crate) rainbow_palette:Option<Arc<crate::native_picture_rainbow::RainbowPalette>>,
    pub(crate) second_clip: Option<[i32;4]>,
    pub(crate) motion_background: bool,
    pub(crate) visible: bool,
    pub(crate) geometry_assigned: bool,
    pub(crate) first_slot: i32,
    pub(crate) second_slot: i32,
    pub(crate) first_destination: [i32; 2],
    pub(crate) second: Option<Arc<dyn ImageResource>>,
    pub(crate) second_destination: [i32; 2],
    pub(crate) normal: Option<crate::native_presentation::MoveFrame>,
}
impl PictureEffectFrame {
    fn masked_special_input(&self,base:ImageInput,random:&mut PatternRng)->Result<Option<ImageInput>> {
        use crysirror_image::byte_raster::ByteRaster;
        let Some(source)=&self.source else{return Ok(None);};
        let size=source.size();
        let clip=self.source_clip.unwrap_or([0,0,size[0] as i32,size[1] as i32]);
        let point=[self.rectangles.destination[0],self.rectangles.destination[1]];
        let spec=match self.state.kind {
            PictureEffectKind::Meisai{parameters}=>crate::native_picture_meisai::draw_plan(&parameters,self.state.count,self.state.frames,self.canvas,size,clip,point,self.geometry_assigned,self.has_mask,random)?.map(|spec|ByteRaster::MaskedBackdropShift{spec}),
            PictureEffectKind::Rainbow{parameters}=>{
                let palette=self.rainbow_palette.as_ref().ok_or_else(||anyhow::anyhow!("NNN RAINBOW shared palette absent"))?;
                crate::native_picture_rainbow::draw_plan(&parameters,self.state.count,self.state.frames,self.canvas,size,clip,point,self.geometry_assigned,self.has_mask,palette,random)?.map(|spec|ByteRaster::RowTintBlend{spec})
            },_=>unreachable!(),
        };
        let Some(spec)=spec else{return Ok(None);};
        Ok(Some(ImageInput::Process{input:Box::new(ImageInput::Raster{destination:Box::new(base),source:Box::new(ImageInput::Resource(source.clone()))}),effects:serde_json::to_vec(&[ImageEffect::ByteRaster{spec}])?.into()}))
    }

    fn clipped_copy(&self)->Result<Option<([i32;4],[i32;2])>> {
        let [x,y,w,h]=self.rectangles.destination;
        let source=[self.rectangles.source[0],self.rectangles.source[1],w,h];
        if let Some(bounds)=self.source_clip {
            crysirror_image::coverage_bounds::intersect_source_window(source,[x,y],bounds)
        }else{Ok((w>0&&h>0).then_some((source,[x,y])))}
    }

    fn blur_input(&self,base:ImageInput,random:&mut PatternRng)->Result<Option<ImageInput>> {
        use crate::native_picture_chara_blur::BlurDraw;
        use crysirror_image::byte_raster::{ByteRaster,ByteRasterMath};
        let picture=self.source.as_ref().map(|p|p.size()).unwrap_or([1,1]);
        let clip=self.source_clip.unwrap_or([0,0,picture[0] as i32,picture[1] as i32]);
        let point=[self.rectangles.destination[0],self.rectangles.destination[1]];
        let size=[self.rectangles.destination[2],self.rectangles.destination[3]];
        let draw=match self.state.kind {
            PictureEffectKind::CharaBlur{parameters}=>crate::native_picture_chara_blur::draw_plan(&parameters,self.state.count,self.state.frames,self.canvas,picture,clip,point,size,self.has_mask,random)?,
            PictureEffectKind::BackgroundBlur{parameters}=>crate::native_picture_background_blur::draw_plan(&parameters,self.state.count,self.state.frames,self.canvas,picture,clip,point,size,random)?,
            _=>unreachable!(),
        };
        // Both source painters sample their window curve before checking pic.
        if !self.geometry_assigned{return Ok(None);}
        let (Some(source),Some(draw))=(&self.source,draw) else{return Ok(None);};
        Ok(Some(match draw {
            BlurDraw::Copy{source:rect,destination,masked}=>crate::native_picture_sampling::copy_input(base,source.clone(),rect,destination,
                if masked{ByteRasterMath::AlphaWeightedSeparateRgb256{weight:256}}else{ByteRasterMath::CopyRgb},self.canvas,None)?,
            BlurDraw::Blur(spec)=>ImageInput::Process{input:Box::new(ImageInput::Raster{destination:Box::new(base),source:Box::new(ImageInput::Resource(source.clone()))}),effects:serde_json::to_vec(&[ImageEffect::ByteRaster{spec:ByteRaster::NativeBoxBlur{spec}}])?.into()},
        }))
    }

    pub fn geometry(&self) -> crate::native_move::NativeMoveState {
        crate::native_move::NativeMoveState {
            count: self.state.count,
            frames: self.state.frames,
            ended: false,
            rectangles: self.rectangles.clone(),
        }
    }
    pub fn calcu(&mut self, random: &mut PatternRng) -> Result<()> {
        if let PictureEffectKind::Koma {parameters}= &self.state.kind {
            crate::native_picture_koma::calcu(parameters,&mut self.state.count);
        }
        if let PictureEffectKind::Noise{parameters}= &mut self.state.kind {
            let height=self.source.as_ref().map(|s|s.size()[1] as i32).unwrap_or(1);
            crate::native_picture_noise::calcu(parameters,height,random)?;
        }
        if let PictureEffectKind::ScrollLoop { parameters: p } = &mut self.state.kind {
            let mut delta = self.state.count.wrapping_sub(p[12]);
            if self.state.count >= self.state.frames {
                if p[3] == 0 {
                    self.state.count = self.state.frames;
                    delta = 0;
                } else {
                    self.state.count %= self.state.frames.max(1);
                }
            }
            p[12] = self.state.count;
            p[13] = delta;
            // Signed remainder preserves the native negative displacement;
            // direct remainder avoids source NEG(INT_MIN) overflow.
            p[14] = p[14].wrapping_add(p[1].wrapping_mul(delta)) % (self.canvas[0] as i32);
            p[15] = p[15].wrapping_add(p[2].wrapping_mul(delta)) % (self.canvas[1] as i32);
        }
        if let PictureEffectKind::Jitter { parameters: p } = &mut self.state.kind {
            p[20] = p[20].wrapping_add(1);
            if p[20] > p[21] {
                crate::native_picture_motion::jitter_resample(p, random)?;
            }
        }
        let mut geometry = self.geometry();
        geometry.calcu(self.geometry_assigned);
        self.rectangles = geometry.rectangles;
        if let Some(normal) = self.normal.as_mut() {
            normal.state.rectangles = self.rectangles.clone();
        }
        Ok(())
    }
    pub(crate) fn resolve_points(
        &mut self,
        rectangles: &std::collections::BTreeMap<i32, crate::native_move::NativeMoveState>,
    ) {
        if let Some(state) = rectangles.get(&self.first_slot) {
            self.first_destination = [
                state.rectangles.destination[0],
                state.rectangles.destination[1],
            ];
        }
        if let Some(state) = rectangles.get(&self.second_slot) {
            self.second_destination = [
                state.rectangles.destination[0],
                state.rectangles.destination[1],
            ];
        }
    }
    pub fn image_input(
        &self,
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<ImageInput>> {
        match self.state.kind {
            PictureEffectKind::Rainbow{..}|PictureEffectKind::Meisai{..}=>return self.masked_special_input(base,random),
            PictureEffectKind::Relief{parameters}=>{
                let spec=crate::native_picture_releaf::spec(&parameters,self.state.count,self.state.frames,self.canvas,random)?;
                return Ok(Some(ImageInput::Process{input:Box::new(base),effects:serde_json::to_vec(&[ImageEffect::NeighborRelief{spec}])?.into()}));
            },
            PictureEffectKind::OldFilm{..}=>anyhow::bail!("NNN OLDFILM requires committed native Print state"),
            PictureEffectKind::CharaBlur{..}|PictureEffectKind::BackgroundBlur{..}=>return self.blur_input(base,random),
            PictureEffectKind::DualMask{parameters}=>return crate::native_picture_dual_mask::image_input(self,parameters,base,random),
            PictureEffectKind::MaskAppear{parameters}=>return crate::native_picture_mask_appear::image_input(self,parameters,base,random),
            PictureEffectKind::Sentence{parameters}=>return crate::native_picture_sentence::image_input(self,parameters,base,random),
            PictureEffectKind::StretchBlit{parameters}=>return crate::native_picture_stretch_blt::image_input(self,parameters,base,random),
            PictureEffectKind::ClipZoom{parameters}=>return crate::native_picture_clip_zoom::image_input(self,parameters,base,random),
            _=>(),
        }
        if let PictureEffectKind::Koma{parameters}=self.state.kind {
            return crate::native_picture_koma::image_input(self,parameters,base);
        }
        if let PictureEffectKind::Ghost{parameters}=self.state.kind {
            return crate::native_picture_ghost::image_input(self,parameters,base,random);
        }
        if let PictureEffectKind::Noise{parameters}=self.state.kind {
            return crate::native_picture_noise::image_input(self,parameters,base,random);
        }
        if let PictureEffectKind::Target { parameters } = self.state.kind {
            return crate::native_picture_target::image_input(self, parameters, base, random);
        }
        if let PictureEffectKind::CharaRaster { parameters } = self.state.kind {
            return crate::native_picture_raster::image_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Saike { parameters } = self.state.kind {
            return crate::native_picture_tone::image_input(self, parameters, base, random);
        }
        // effectLupin.cpp fills the entire screen, independent of pic/geometry.
        // para0 is saved but countMax is always 9999; colour words wrap to bytes.
        if let PictureEffectKind::Lupin { parameters: p } = self.state.kind {
            return Ok(Some(ImageInput::Solid {
                size: self.canvas,
                color: [p[1] as u8, p[2] as u8, p[3] as u8, 255],
            }));
        }
        if let PictureEffectKind::MeltDown { parameters } = self.state.kind {
            return crate::native_picture_motion::meltdown_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Curtain { parameters } = self.state.kind {
            return crate::native_picture_motion::curtain_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Raster { parameters } = self.state.kind {
            return crate::native_picture_motion::raster_input(self, parameters, base, random);
        }
        if let PictureEffectKind::ScrollLoop { parameters } = self.state.kind {
            return crate::native_picture_motion::scroll_input(self, parameters, base);
        }
        if let PictureEffectKind::Quadratic { parameters } = self.state.kind {
            return crate::native_picture_motion::quadratic_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Bezier { parameters } = self.state.kind {
            return crate::native_picture_motion::bezier_input(self, parameters, base, random);
        }
        if let PictureEffectKind::BoxBlur { parameters } = self.state.kind {
            return crate::native_picture_blur::image_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Jitter { parameters } = self.state.kind {
            return crate::native_picture_motion::jitter_input(self, parameters, base, random);
        }
        if let PictureEffectKind::Zoom { parameters } = self.state.kind {
            return crate::native_picture_motion::zoom_input(self, parameters, random);
        }
        if let PictureEffectKind::ScreenGradient { parameters: p } = self.state.kind {
            let mut top = [0; 3];
            let mut bottom = [0; 3];
            for c in 0..3 {
                top[c] = crate::native_curve::value_with_random(
                    self.state.count,
                    self.state.frames,
                    p[c + 9],
                    p[c + 1],
                    p[8],
                    p[7],
                    random,
                )?;
            }
            for c in 0..3 {
                bottom[c] = crate::native_curve::value_with_random(
                    self.state.count,
                    self.state.frames,
                    p[c + 12],
                    p[c + 4],
                    p[8],
                    p[7],
                    random,
                )?;
            }
            // The original negative/zero loop counters can run beyond their
            // allocation. Define an empty region as no writes after its six
            // native curve evaluations, preserving the well-defined RNG order.
            if p[16] <= 0 {
                return Ok(None);
            }
            let spec = crysirror_image::row_color::RowWordAdd::gradient(
                p[15] as u32,
                self.canvas[0] / 4 * 4,
                p[16] as u32,
                top,
                bottom,
            )?;
            return Ok(Some(ImageInput::Process {
                input: Box::new(base),
                effects: serde_json::to_vec(&[ImageEffect::RowWordAdd { spec }])?.into(),
            }));
        }
        if let PictureEffectKind::ScreenLight { parameters: p } = self.state.kind {
            let mut color = [0; 3];
            for c in 0..3 {
                color[c] = crate::native_curve::value_with_random(
                    self.state.count,
                    self.state.frames,
                    0,
                    p[c + 1],
                    p[5],
                    p[4],
                    random,
                )?;
            }
            let mut lookup = crysirror_image::rgb_lut::RgbLookup::packed_word_add(color);
            let pixels = self.canvas[0]
                .checked_mul(self.canvas[1])
                .ok_or_else(|| anyhow::anyhow!("NNN LIGHT canvas pixel overflow"))?;
            lookup.profile = crysirror_image::rgb_lut::RgbLookupProfile::StraightPrefix {
                pixels: pixels / 4 * 4,
            };
            return Ok(Some(ImageInput::Process {
                input: Box::new(base),
                effects: serde_json::to_vec(&[ImageEffect::RgbLookup { spec: lookup }])?.into(),
            }));
        }
        if let PictureEffectKind::AddSubtract { parameters: p } = self.state.kind {
            let Some(source) = &self.source else {
                return Ok(None);
            };
            let percent = crate::native_curve::value_with_random(
                self.state.count,
                self.state.frames,
                p[2],
                p[1],
                p[4],
                p[3],
                random,
            )?;
            if percent == 0 {
                return Ok(None);
            }
            if self.rectangles.destination[2] <= 0 || self.rectangles.destination[3] <= 0 {
                return Ok(None);
            }
            let scale = percent.wrapping_mul(256) / 100;
            ensure!(
                scale.unsigned_abs() <= 32767,
                "NNN ADDSUB signed-word multiplier"
            );
            return Ok(Some(ImageInput::Process {
                input: Box::new(ImageInput::Raster {
                    destination: Box::new(base),
                    source: Box::new(ImageInput::Resource(source.clone())),
                }),
                effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                    spec: crysirror_image::byte_raster::ByteRaster::MaskedAddSubtract256 {
                        spec: crysirror_image::masked_add::MaskedAddSubtract256 {
                            source: [
                                self.rectangles.source[0],
                                self.rectangles.source[1],
                                self.rectangles.destination[2],
                                self.rectangles.destination[3],
                            ],
                            destination: [
                                self.rectangles.destination[0],
                                self.rectangles.destination[1],
                            ],
                            scale: scale.unsigned_abs() as u16,
                            subtract: scale < 0,
                        },
                    },
                }])?
                .into(),
            }));
        }
        if let PictureEffectKind::Stretch { parameters } = self.state.kind {
            return crate::native_stretch::image_input(self, parameters, base, random);
        }
        //124 evaluates its curve before the missing-picture return, unlike99.
        let grey_amount = if let PictureEffectKind::Grey {
            easing,
            subframe,
            end,
            start,
            ..
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
            PictureEffectKind::Rainbow{..}|PictureEffectKind::Meisai{..}|PictureEffectKind::Relief{..}|PictureEffectKind::OldFilm{..}|PictureEffectKind::CharaBlur{..}|PictureEffectKind::BackgroundBlur{..}=>unreachable!(),
            PictureEffectKind::DualMask{..}|PictureEffectKind::MaskAppear{..}|PictureEffectKind::Sentence{..}|PictureEffectKind::StretchBlit{..}|PictureEffectKind::ClipZoom{..}=>unreachable!(),
            PictureEffectKind::Koma { .. } | PictureEffectKind::Ghost { .. } => unreachable!(),
            PictureEffectKind::Noise { .. } => unreachable!(),
            PictureEffectKind::Target { .. } => unreachable!(),
            PictureEffectKind::CharaRaster { .. } => unreachable!(),
            PictureEffectKind::Saike { .. } => unreachable!(),
            PictureEffectKind::Lupin { .. } => unreachable!(),
            PictureEffectKind::MeltDown { .. } => unreachable!(),
            PictureEffectKind::Curtain { .. } => unreachable!(),
            PictureEffectKind::Raster { .. } | PictureEffectKind::ScrollLoop { .. } => {
                unreachable!()
            }
            PictureEffectKind::Quadratic { .. } => unreachable!(),
            PictureEffectKind::Bezier { .. } => unreachable!(),
            PictureEffectKind::BoxBlur { .. } => unreachable!(),
            PictureEffectKind::Jitter { .. } => unreachable!(),
            PictureEffectKind::Zoom { .. } => unreachable!(),
            PictureEffectKind::ScreenGradient { .. } => unreachable!(),
            PictureEffectKind::ScreenLight { .. } => unreachable!(),
            PictureEffectKind::AddSubtract { .. } => unreachable!(),
            PictureEffectKind::Stretch { .. } => unreachable!(),
            PictureEffectKind::Buffered { .. } => {
                anyhow::bail!("NNN buffered effect requires its ordered frame transaction")
            }
            PictureEffectKind::Grey { tint, .. } => (
                vec![ImageEffect::FixedLuma {
                    spec: crysirror_image::fixed_luma::FixedLumaSpec {
                        coefficients: [76, 150, 29],
                        groups: [1, 0, 0],
                        fractional_bits: 8,
                        amount256: grey_amount.unwrap(),
                        tint: tint.map(|[r, g, b]| {
                            let packed = r.wrapping_shl(16) | g.wrapping_shl(8) | b;
                            [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8]
                        }),
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
                    let Some((region,at))=self.clipped_copy()? else{return Ok(None);};
                    return Ok(Some(ImageInput::Composite {
                        base: Box::new(base),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(source.clone())),
                            source: region,
                            destination: at,
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
                let Some((native_region,native_at))=self.clipped_copy()? else{return Ok(None);};
                let Some((region, _)) = crysirror_core::region_map::clip_region_copy(
                    Some(source.size()),
                    self.canvas,
                    native_region.map(i64::from),
                    native_at.map(i64::from),
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
                balance,
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
                if let Some((bands, preserve_luminance)) = balance {
                    // ColorBalance's software path requires an actual mask;
                    // para7 is saved but never gates this mask-only painter.
                    if !self.has_mask {
                        return Ok(None);
                    }
                    use crysirror_image::tonal_balance::{TonalBalance, TonalBand};
                    let band = |v| match v {
                        0 => TonalBand::All,
                        1 => TonalBand::Shadows,
                        2 => TonalBand::Midtones,
                        3 => TonalBand::Highlights,
                        4 => TonalBand::ShadowsNarrow,
                        5 => TonalBand::MidtonesNarrow,
                        6 => TonalBand::HighlightsNarrow,
                        7 => TonalBand::ShadowsWide,
                        8 => TonalBand::MidtonesWide,
                        _ => TonalBand::HighlightsWide,
                    };
                    (
                        vec![ImageEffect::LumaOffsetLookup {
                            spec: TonalBalance {
                                offsets: delta,
                                bands: bands.map(band),
                                preserve_luminance,
                            }
                            .byte_lookup(),
                        }],
                        256,
                    )
                } else {
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
            }
        };
        let Some((region,at))=self.clipped_copy()? else{return Ok(None);};
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
                source: region,
                destination: at,
                clip: None,
                opacity: 255,
                math: if self.state.command() == 120 {
                    // Original software MMX uses a=mask and256-a even when
                    // mask is255, with one final shift; no opaque shortcut.
                    ImageCompositeMath::MaskHoldAlphaScalar256
                } else if matches!(self.state.command(), 124 | 127) {
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
