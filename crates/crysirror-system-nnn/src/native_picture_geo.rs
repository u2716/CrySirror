// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! GEOBOX/GEOCIRCLE native parameter and curve policy. The source's GEOBOXADD
//! unexpectedly calls TransCircleFill, not an additive box; keep that semantic.
use anyhow::Result;
use crysirror_image::{
    elliptic_paint::{EllipticCutout, EllipticPaint, EllipticPaintMath},
    temporal_pattern::PatternRng,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoParameters {
    command: i32,
    words: Vec<i32>,
}
impl GeoParameters {
    pub(crate) fn from_saved(command: i32, values: &[i32]) -> Result<Self> {
        let length = match command {
            114 | 118 => 24,
            87 | 115 => 25,
            89 | 116 => 33,
            88 | 117 => 3,
            _ => anyhow::bail!("NNN unknown saved geometric command {command}"),
        };
        anyhow::ensure!(values.len() >= length, "NNN saved geometric parameter span");
        // All eight inherit an empty InitParameter. Raw words are preserved;
        // neither constructor defaults nor SetParam restart their counters.
        Ok(Self {
            command,
            words: values[..length].to_vec(),
        })
    }
    pub(crate) fn parse(command: i32, values: &[i32], canvas: [u32; 2]) -> Result<Self> {
        let words = match command {
            114 | 118 => box_parameters(values, canvas).to_vec(),
            87 | 115 => circle_parameters(values, canvas).to_vec(),
            89 | 116 => torus_parameters(values, canvas).to_vec(),
            88 | 117 => polygon_parameters(values).to_vec(),
            _ => anyhow::bail!("NNN unknown geometric native command {command}"),
        };
        Ok(Self { command, words })
    }
    pub(crate) fn command(&self) -> i32 {
        self.command
    }
    pub(crate) fn parameters(&self) -> Vec<i32> {
        self.words.clone()
    }
    pub(crate) fn frames(&self) -> i32 {
        if matches!(self.command, 88 | 117) {
            9999
        } else {
            self.words[0]
        }
    }
    pub(crate) fn parameter0(&self) -> i32 {
        self.words[0]
    }
    pub(crate) fn validate(&self, frames: i32) -> Result<()> {
        match self.command {
            114 | 118 => anyhow::ensure!(self.words.len() == 24, "NNN box parameter words"),
            87 | 115 => circle_validate(self.words.as_slice().try_into()?, frames)?,
            89 | 116 => torus_validate(self.words.as_slice().try_into()?, frames)?,
            88 | 117 => anyhow::ensure!(self.words.len() == 3, "NNN polygon parameter words"),
            _ => anyhow::bail!("NNN unknown geometric command"),
        }
        Ok(())
    }
    /// Common CheckNeedPicPrint is TRUE for all eight classes. The caller must
    /// run the original normal picture pass before invoking this custom Print.
    pub(crate) fn image_input(
        &self,
        frame: &crate::native_picture_effect::PictureEffectFrame,
        base: crysirror_core::image_processing::ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<crysirror_core::image_processing::ImageInput>> {
        use crysirror_core::image_processing::{
            ImageCompositeLayer, ImageCompositeMath, ImageInput,
        };
        use crysirror_image::effects::ImageEffect;
        let ellipse = match self.command {
            87 | 115 => Some(circle_phase(
                self.words.as_slice().try_into()?,
                frame.state.count,
                frame.state.frames,
                self.command == 115,
                random,
            )?),
            89 | 116 => Some(torus_phase(
                self.words.as_slice().try_into()?,
                frame.state.count,
                frame.state.frames,
                frame.canvas,
                self.command == 116,
                random,
            )?),
            114 | 118 => {
                let Some((rect, color)) = box_phase(
                    self.words.as_slice().try_into()?,
                    frame.state.count,
                    frame.state.frames,
                ) else {
                    return Ok(None);
                };
                if self.command == 118 {
                    Some(EllipticPaint {
                        center: [rect[0], rect[1]],
                        radius: [rect[2], rect[3]],
                        cutout: None,
                        rows: None,
                        math: EllipticPaintMath::Mix {
                            color,
                            weights: [256, 0],
                        },
                    })
                } else {
                    if rect[2] <= 0 || rect[3] <= 0 {
                        return Ok(None);
                    }
                    let Some((region, at)) = crysirror_core::region_map::clip_region_copy(
                        None,
                        frame.canvas,
                        [0, 0, rect[2], rect[3]].map(i64::from),
                        [rect[0], rect[1]].map(i64::from),
                        None,
                    )?
                    else {
                        return Ok(None);
                    };
                    // The ordinary box helper's percent is exactly100; its
                    // stored-byte result is a rectangular RGB copy.
                    return Ok(Some(ImageInput::Composite {
                        base: Box::new(base),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(ImageInput::Solid {
                                size: frame.canvas,
                                color: [color[0], color[1], color[2], 255],
                            }),
                            source: [0, 0, region[2] as i32, region[3] as i32],
                            destination: [at[0] as i32, at[1] as i32],
                            clip: None,
                            opacity: 255,
                            math: ImageCompositeMath::CopyRgb,
                        }],
                    }));
                }
            }
            88 | 117 => {
                let color = polygon_color(self.words.as_slice().try_into()?);
                return Ok(Some(ImageInput::Composite {
                    base: Box::new(base),
                    layers: vec![ImageCompositeLayer {
                        input: Box::new(ImageInput::Solid {
                            size: frame.canvas,
                            color: [color[0], color[1], color[2], 255],
                        }),
                        source: [0, 0, frame.canvas[0] as i32, frame.canvas[1] as i32],
                        destination: [0, 0],
                        clip: None,
                        opacity: 255,
                        math: ImageCompositeMath::CopyRgb,
                    }],
                }));
            }
            _ => anyhow::bail!("NNN unsupported geometric command"),
        };
        let Some(spec) = ellipse else {
            return Ok(None);
        };
        spec.validate()?;
        Ok(Some(ImageInput::Process {
            input: Box::new(base),
            effects: serde_json::to_vec(&[ImageEffect::EllipticPaint { spec }])?.into(),
        }))
    }
}
pub(crate) fn box_parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 24] {
    let mut p = [0; 24];
    for i in 0..24 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            3 | 4 => 100,
            5 | 6 => p[i - 2],
            7..=9 => 128,
            10..=12 => p[i - 3],
            13 => canvas[0] as i32 / 2,
            14 => canvas[1] as i32 / 2,
            15 | 16 => p[i - 2],
            _ => 0,
        });
    }
    // Native17..23 were uninitialised when omitted and are never read by
    // these painters. Define zero defaults; supplied/save words still persist.
    p
}
pub(crate) fn circle_parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 25] {
    let mut p = [0; 25];
    for i in 0..25 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => canvas[0] as i32 / 2,
            2 => canvas[1] as i32 / 2,
            3 | 4 | 8 => 100,
            5..=7 => 128,
            9 | 10 => p[i - 8],
            13 | 14 => p[i - 10],
            17..=19 => p[i - 12],
            22 => p[8],
            _ => 0,
        });
    }
    p
}
fn packed(values: [i32; 3]) -> [u8; 3] {
    let word = values[0].wrapping_shl(16) | values[1].wrapping_shl(8) | values[2];
    [(word >> 16) as u8, (word >> 8) as u8, word as u8]
}
fn paint_math(color: [i32; 3], percent: i32, additive: bool) -> EllipticPaintMath {
    if additive {
        let delta = color.map(|n| n.wrapping_mul(percent) / 100);
        EllipticPaintMath::Offset {
            add: packed(delta.map(|n| n.max(0))),
            subtract: packed(delta.map(|n| n.wrapping_neg().max(0))),
        }
    } else {
        EllipticPaintMath::Mix {
            color: packed(color),
            weights: [
                (percent.wrapping_mul(256) / 100) as u16,
                (100i32.wrapping_sub(percent).wrapping_mul(256) / 100) as u16,
            ],
        }
    }
}
pub(crate) fn polygon_parameters(values: &[i32]) -> [i32; 3] {
    let r = values.first().copied().unwrap_or(0);
    let g = values.get(1).copied().unwrap_or(r);
    [r, g, values.get(2).copied().unwrap_or(g)]
}
pub(crate) fn polygon_color(p: [i32; 3]) -> [u8; 3] {
    // GEOPOLYGON88 and GEOPOLYGONADD117 are identical whole-screen rep-stosd
    // fillers. Neither contains polygon geometry or additive pixel arithmetic.
    p.map(|n| n as u8)
}
pub(crate) fn torus_parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 33] {
    let mut p = [0; 33];
    for i in 0..33 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 | 5 => canvas[0] as i32 / 2,
            2 | 6 => canvas[1] as i32 / 2,
            3 | 4 | 12 => 100,
            7 | 8 => 50,
            9..=11 => 128,
            13 | 14 => p[i - 12],
            15 | 16 => p[i - 10],
            19 | 20 => p[i - 16],
            21 | 22 => p[i - 14],
            25..=27 => p[i - 16],
            30 => p[12],
            _ => 0,
        });
    }
    p
}
pub(crate) fn torus_validate(p: &[i32; 33], frames: i32) -> Result<()> {
    for (sub, kind) in [(18, 17), (24, 23), (29, 28), (32, 31)] {
        crate::native_curve::validate(frames, p[sub], p[kind])?;
    }
    Ok(())
}
pub(crate) fn torus_phase(
    p: &[i32; 33],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    additive: bool,
    random: &mut PatternRng,
) -> Result<EllipticPaint> {
    let sample = |start: usize, end: usize, sub: usize, kind: usize, random: &mut PatternRng| {
        crate::native_curve::value_with_random(
            count, frames, p[start], p[end], p[sub], p[kind], random,
        )
    };
    let center = [
        sample(13, 1, 18, 17, random)?,
        sample(14, 2, 18, 17, random)?,
    ];
    let inner_center = [
        sample(15, 5, 18, 17, random)?,
        sample(16, 6, 18, 17, random)?,
    ];
    let radius = [
        sample(19, 3, 24, 23, random)?,
        sample(20, 4, 24, 23, random)?,
    ];
    let inner_radius = [
        sample(21, 7, 24, 23, random)?,
        sample(22, 8, 24, 23, random)?,
    ];
    let color = [
        sample(25, 9, 29, 28, random)?,
        sample(26, 10, 29, 28, random)?,
        sample(27, 11, 29, 28, random)?,
    ];
    let percent = sample(30, 12, 32, 31, random)?;
    // Both software helpers assign yEnd instead of yEnd2 when the inner
    // ellipse reaches below the screen. This defined outer-row extension is
    // preserved, including its one-pixel endpoint below the outer ellipse.
    let rows = (i64::from(inner_center[1]) + i64::from(inner_radius[1]) >= i64::from(canvas[1]))
        .then(|| {
            [
                center[1].wrapping_sub(radius[1]),
                (canvas[1] as i32).wrapping_sub(1),
            ]
        });
    let spec = EllipticPaint {
        center,
        radius,
        cutout: Some(EllipticCutout {
            center: inner_center,
            radius: inner_radius,
            padding: 1,
            intersect_outer: false,
        }),
        rows,
        math: paint_math(color, percent, additive),
    };
    spec.validate()?;
    Ok(spec)
}
pub(crate) fn box_phase(p: &[i32; 24], count: i32, frames: i32) -> Option<([i32; 4], [u8; 3])> {
    if count < p[1] || count >= frames.wrapping_sub(p[2]) {
        return None;
    }
    let duration = frames.wrapping_sub(p[1]).wrapping_sub(p[2]);
    if duration < 1 {
        return None;
    }
    let elapsed = count.wrapping_sub(p[1]);
    let interpolate = |start: usize, end: usize| {
        p[start].wrapping_add(p[end].wrapping_sub(p[start]).wrapping_mul(elapsed) / duration)
    };
    let color = packed([interpolate(7, 10), interpolate(8, 11), interpolate(9, 12)]);
    Some((
        [
            interpolate(13, 15),
            interpolate(14, 16),
            interpolate(3, 5),
            interpolate(4, 6),
        ],
        color,
    ))
}
pub(crate) fn circle_validate(p: &[i32; 25], frames: i32) -> Result<()> {
    for (sub, kind) in [(12, 11), (16, 15), (21, 20), (24, 23)] {
        crate::native_curve::validate(frames, p[sub], p[kind])?;
    }
    Ok(())
}
pub(crate) fn circle_phase(
    p: &[i32; 25],
    count: i32,
    frames: i32,
    additive: bool,
    random: &mut PatternRng,
) -> Result<EllipticPaint> {
    let sample = |start: usize, end: usize, sub: usize, kind: usize, random: &mut PatternRng| {
        crate::native_curve::value_with_random(
            count, frames, p[start], p[end], p[sub], p[kind], random,
        )
    };
    // Original Print always consumes all eight calls in this order, even
    // equal endpoints and radii for which the drawing helper will return.
    let center = [
        sample(9, 1, 12, 11, random)?,
        sample(10, 2, 12, 11, random)?,
    ];
    let radius = [
        sample(13, 3, 16, 15, random)?,
        sample(14, 4, 16, 15, random)?,
    ];
    let color = [
        sample(17, 5, 21, 20, random)?,
        sample(18, 6, 21, 20, random)?,
        sample(19, 7, 21, 20, random)?,
    ];
    let percent = sample(22, 8, 24, 23, random)?;
    let math = if additive {
        let delta = color.map(|n| n.wrapping_mul(percent) / 100);
        EllipticPaintMath::Offset {
            add: packed(delta.map(|n| n.max(0))),
            subtract: packed(delta.map(|n| n.wrapping_neg().max(0))),
        }
    } else {
        EllipticPaintMath::Mix {
            color: packed(color),
            weights: [
                (percent.wrapping_mul(256) / 100) as u16,
                (100i32.wrapping_sub(percent).wrapping_mul(256) / 100) as u16,
            ],
        }
    };
    let spec = EllipticPaint {
        center,
        radius,
        cutout: None,
        rows: None,
        math,
    };
    spec.validate()?;
    Ok(spec)
}
