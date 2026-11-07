//! Native ABMP key records. Field names/strides are checked against
//! TABMPLayer10.LoadFromStream/ExportStringList; interpolation uses the engine
//! ScalarTween. Native records are never rendered to a private framebuffer.
use super::*;
mod compact;
mod legacy;
use compact::compact_key;
#[derive(Clone)]
pub(super) struct Animation {
    pub size: [u32; 2],
    pub depth: u32,
    pub motions: Vec<Motion>,
    pub capture_both: bool,
    /// Header memo (TABMPData+3C), assigned to a list by `mem`.
    pub memo: String,
}
#[derive(Clone)]
pub(super) struct Motion {
    pub name: String,
    pub duration_ms: u64,
    pub layers: Vec<Track>,
}
#[derive(Clone)]
pub(super) struct Track {
    pub name: String,
    /// Native track+45 initializes its renderer enable slot (59DEC0) and
    /// independently gates its sound events (59EE58).
    pub enabled: bool,
    pub channel: u8,
    pub frames: Vec<[i32; 115]>,
}
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
    wide: bool,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.at.checked_add(n).context("ABMP cursor overflow")?;
        ensure!(end <= self.b.len(), "truncated ABMP animation");
        let b = &self.b[self.at..end];
        self.at = end;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    fn int(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn text(&mut self) -> Result<String> {
        let n = self.word()? as usize;
        ensure!(n <= 32768, "ABMP string size");
        if !self.wide {
            let bytes = self.take(n)?;
            return encoding_rs::SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(bytes)
                .map(|s| s.into_owned())
                .context("ABMP CP932 string");
        }
        let b = self.take(n * 2)?;
        String::from_utf16(
            &b.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
        .context("ABMP string encoding")
    }
    fn tag(&mut self) -> Result<String> {
        Ok(std::str::from_utf8(self.take(16)?)?
            .trim_end_matches('\0')
            .into())
    }
}
impl Animation {
    pub fn parse(bytes: &[u8], wide: bool) -> Result<Self> {
        let mut r = Reader {
            b: bytes,
            at: 0,
            wide,
        };
        r.text()?;
        let version = r.int()?;
        ensure!(version <= 2, "ABMP data version {version}");
        let size = [r.int()?, r.int()?];
        ensure!(size.iter().all(|n| *n <= 8192), "ABMP viewport size");
        let depth = r.int()?;
        ensure!(depth <= 8192, "ABMP viewport depth");
        if version == 1 {
            r.int()?;
            r.int()?;
        }
        let metadata = r.text()?;
        let capture_both = metadata
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("*CaptureFadein=1"));
        let count = r.byte()?;
        let mut motions = Vec::new();
        let mut total = 0;
        for _ in 0..count {
            let tag = r.tag()?;
            ensure!(
                tag == "abmotion10" || tag == "abmotion11" || tag == "abmotion12",
                "unsupported ABMP motion {tag}"
            );
            if tag != "abmotion10" {
                ensure!(r.int()? == 1, "ABMP motion revision");
            }
            let name = r.text()?;
            let mut duration_ms = None;
            if tag != "abmotion10" {
                r.int()?;
                duration_ms = Some(u64::from(r.int()?));
            }
            if tag == "abmotion12" {
                r.int()?;
            }
            let n = r.byte()?;
            let mut layers = Vec::new();
            for _ in 0..n {
                let tag = r.tag()?;
                let name = r.text()?;
                let enabled = r.byte()? != 0;
                let count = if matches!(
                    tag.as_str(),
                    "ablayer15"
                        | "ablayer16"
                        | "ablayer17"
                        | "ablayer18"
                        | "ablayer20"
                        | "ablayer21"
                        | "ablayer22"
                        | "ablayer23"
                        | "ablayer24"
                ) {
                    r.byte()? as usize
                } else {
                    r.int()? as usize
                };
                let channel = r.byte()?;
                let stride = match tag.as_str() {
                    "ablayer15" => 92,
                    "ablayer16" => 145,
                    "ablayer17" | "ablayer18" => 162,
                    "ablayer20" | "ablayer21" | "ablayer22" | "ablayer23" | "ablayer24" => 440,
                    "ablayer25" => r.int()? as usize,
                    _ => bail!("unsupported ABMP layer layout {tag}"),
                };
                ensure!(
                    matches!(stride, 162 | 440 | 460)
                        || matches!(
                            (tag.as_str(), stride),
                            ("ablayer15", 92) | ("ablayer16", 145)
                        ),
                    "ABMP key stride {stride}"
                );
                ensure!(count <= 65536 && channel <= 3, "ABMP key/channel bounds");
                total += count;
                ensure!(total <= 65536, "ABMP animation key budget");
                let mut frames = Vec::with_capacity(count);
                for _ in 0..count {
                    let raw = r.take(stride)?;
                    let mut values =
                        if matches!(tag.as_str(), "ablayer15" | "ablayer16" | "ablayer17") {
                            legacy::upgrade(&tag, raw)?
                        } else if stride == 162 {
                            compact_key(raw)?
                        } else {
                            let mut values = [0; 115];
                            for (i, word) in raw.chunks_exact(4).enumerate() {
                                values[i] = i32::from_le_bytes(word.try_into()?);
                            }
                            values
                        };
                    if stride < 460
                        && !matches!(
                            tag.as_str(),
                            "ablayer15" | "ablayer16" | "ablayer17" | "ablayer24"
                        )
                    {
                        // Native 5B43E0 / 5B441C upgrades. V20 predates the
                        // flags field; V23 has already remapped it.
                        if tag == "ablayer20" {
                            values[94] = 6;
                            values[95] = 62;
                            values[96] = 3;
                        }
                        if tag != "ablayer23" && matches!(values[1] as u8, 3 | 4 | 7 | 13 | 18 | 19)
                        {
                            values[97] = values[10];
                            values[10] = 0;
                        }
                        if tag != "ablayer23" {
                            let old = values[94];
                            values[94] = if old & 1 != 0 { 3 } else { 0 }
                                | if old & 2 != 0 { 4 } else { 0 }
                                | if old & 4 != 0 { 8 } else { 0 };
                        }
                        if matches!(values[1] as u8, 20 | 21) {
                            values[103] = 50;
                        }
                    }
                    ensure!(values[0] >= 0, "ABMP negative key time");
                    ensure!(
                        frames.last().is_none_or(|p: &[i32; 115]| p[0] <= values[0]),
                        "ABMP unsorted key times"
                    );
                    frames.push(values);
                }
                if stride < 460 {
                    legacy::finish(&tag, channel, &mut frames);
                }
                layers.push(Track {
                    name,
                    enabled,
                    channel,
                    frames,
                });
            }
            let duration_ms = duration_ms.unwrap_or_else(|| {
                layers
                    .iter()
                    .filter_map(|t| t.frames.last())
                    .map(|k| k[0] as u64)
                    .max()
                    .unwrap_or(0)
            });
            ensure!(duration_ms <= 86_400_000, "QLIE motion duration limit");
            motions.push(Motion {
                name,
                layers,
                duration_ms,
            });
        }
        ensure!(r.at == bytes.len(), "ABMP animation trailing data");
        Ok(Self {
            size,
            depth,
            motions,
            capture_both,
            memo: metadata,
        })
    }
}
pub(super) fn easing(mode: i32) -> Result<crysirror_scene::layer::Easing> {
    easing_with_strength(mode, 100)
}
/// Native 5CFB7C dispatch; strength is scaled by 0.01 before the curve.
/// Names in old script constant tables differ between player versions, so
/// dispatch follows the serialized numeric mode rather than those names.
pub(super) fn easing_with_strength(
    mode: i32,
    strength: i32,
) -> Result<crysirror_scene::layer::Easing> {
    use crysirror_image::progress_curve::{ProgressDirection as D, ProgressShape as S};
    use crysirror_scene::layer::Easing as E;
    let strength = f64::from(strength) * 0.01;
    let exponent = 1. + 1.4 * strength;
    let odd = ((strength * 100. + 0.5).trunc() as i64 | 1).max(2);
    let shape = match mode {
        0 => S::Steps { segments: 1 },
        1 => return Ok(E::Linear),
        2 => S::BSpline { strength },
        6 => bail!("QLIE noisy interpolation requires a retained procedural motion producer"),
        3 => S::Power {
            exponent,
            direction: D::Out,
        },
        4 => S::Power {
            exponent,
            direction: D::In,
        },
        7 => S::Power {
            exponent,
            direction: D::OutIn,
        },
        8 => S::Power {
            exponent,
            direction: D::InOut,
        },
        12 => S::Square {
            segments: ((strength * 100. + 0.5).trunc() as i64 & 0x0ffffffe)
                .max(2)
                .try_into()?,
        },
        13 => S::Triangle {
            segments: odd.try_into()?,
        },
        14 => S::Cosine {
            radians: 3.14159265 * odd as f64,
        },
        15..=20 => S::DampedCosine {
            exponent,
            radians: 3.14159265 * if matches!(mode, 17 | 18) { 1. } else { 4. },
            absolute: matches!(mode, 19 | 20),
            reverse: matches!(mode, 16 | 18 | 20),
        },
        21 => S::Steps {
            segments: ((strength * 100.).floor() as i64)
                .max(1)
                .checked_add(1)
                .context("QLIE step curve count")?
                .try_into()?,
        },
        // Script5CFB7C has no handlers for2-neighbour-free5/9/10/11;
        //its explicit native fallback immediately selects the destination.
        _ => S::End,
    };
    shape.validate()?;
    Ok(E::Progress { shape })
}
/// Native loop values below -1 preserve the previous loop policy. All other
/// negatives mean an unbounded restart, positives count additional passes.
pub(super) fn repeat(object: &Object, lane: &str, requested: i32) -> i32 {
    if requested >= -1 {
        requested
    } else {
        object
            .properties
            .get(lane)
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0)
    }
}
#[derive(Clone)]
pub(super) struct Sample {
    pub kind: u8,
    pub image: i32,
    pub nested_motion: usize,
    pub model_object: i32,
    pub model_morph_object: i32,
    pub model_morph_weight: f32,
    pub native_blend: u8,
    pub native_sampling: u8,
    pub absolute_dimensions: bool,
    pub snap_anchor: bool,
    pub native_depth: u8,
    pub mask: i32,
    pub mask_progress: f32,
    pub mask_power: f32,
    pub primary_rect: [f32; 4],
    pub secondary_rect: [f32; 4],
    pub particle_legacy: [f32; 8],
    pub particle_extra: [f32; 8],
    pub seek_end: u64,
    pub z: f32,
    pub scale_z: f32,
    pub anchor_z: f32,
    pub rotation_xy: [f32; 2],
    pub add: [f32; 3],
    pub seek: u64,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub anchor: [f32; 2],
    pub tint: [f32; 4],
    pub rotation: f32,
    pub order: f32,
    pub blend: crysirror_scene::layer::Blend,
}
/// Native value banks (59D50C). Decode references before interpolation;
/// treating their tag as a coordinate produces a 268-million-pixel rectangle.
pub(super) struct Values {
    pub external: [f32; 32],
    pub elapsed: u64,
    pub size: [u32; 2],
    pub canvas: [u32; 2],
}
impl Values {
    pub(super) fn number(&self, n: i32, time: u64, key: u64, next: u64) -> Result<f32> {
        let values = self;
        let mut internal = [0.; 32];
        internal[0] = values.elapsed.min(0xffffff) as f32;
        internal[1] = values.elapsed.min(0xffffff) as f32;
        internal[2] = time.min(0xffffff) as f32;
        internal[3] = time.saturating_sub(key).min(0xffffff) as f32;
        internal[4] = if next > key {
            (time.saturating_sub(key) as f32 / (next - key) as f32).clamp(0., 1.) * 255.
        } else {
            0.
        };
        internal[7] = values.size[0] as f32;
        internal[8] = values.size[1] as f32;
        internal[9] = values.canvas[0] as f32;
        internal[10] = values.canvas[1] as f32;
        internal[11] = internal[7] * 0.5;
        internal[12] = internal[8] * 0.5;
        internal[13] = internal[9] * 0.5;
        internal[14] = internal[10] * 0.5;
        Ok(match n {
            0x10000000..=0x1000001f => values.external[(n - 0x10000000) as usize],
            // Native reset (59DDBC) uses unit scale AND unit offset in
            // this internal bank. The external bank starts at scale1/offset0.
            0x10000100..=0x1000011f => internal[(n - 0x10000100) as usize] + 1.,
            0x10000200..=0x1000021f => {
                bail!("QLIE animation generator value bank is not installed: {n:#x}")
            }
            n if n >= 0x10000000 => bail!("QLIE unknown animation value selector {n:#x}"),
            n => n as f32,
        }
        .clamp(-16777215., 16777215.))
    }
}
pub(super) fn sample(
    track: &Track,
    time: u64,
    values: &Values,
    random: &mut animation_clock::RandomTable,
) -> Result<Option<Sample>> {
    use crysirror_scene::animation::ScalarTween;
    let index = track.frames.partition_point(|f| f[0] as u64 <= time);
    if index == 0 {
        return Ok(None);
    }
    let a = &track.frames[index - 1];
    let b = track.frames.get(index).unwrap_or(a);
    let number = |n: i32| values.number(n, time, a[0] as u64, b[0] as u64);
    let random = std::cell::RefCell::new(random);
    let interpolate = |points: [f32; 4], mode: i32, strength: i32| -> Result<f32> {
        use crysirror_image::progress_curve::{
            sample_four_point, sample_scalar_jitter, FourPointCurve as F, ProgressDirection,
            ProgressShape,
        };
        if mode == 0
            || mode == 11
            || !(0..=21).contains(&mode)
            || a[0] == b[0]
            || points.windows(2).all(|pair| pair[0] == pair[1])
        {
            //5AA568 suppresses interpolation (including RNG consumption)
            //when all four scalar controls are identical.
            return Ok(points[1]);
        }
        let raw = (time.saturating_sub(a[0] as u64) as f64 / (b[0] - a[0]) as f64).clamp(0., 1.);
        let strength = f64::from(strength) * 0.01;
        if let Some(curve) = match mode {
            2 => Some(F::BSpline),
            5 => Some(F::Hermite),
            9 => Some(F::TangentBezier),
            10 => Some(F::AlternatingBezier {
                odd: (index - 1) & 1 != 0,
            }),
            _ => None,
        } {
            return Ok(sample_four_point(curve, points.map(f64::from), raw, strength)? as f32);
        }
        if mode == 6 {
            let noise = random.borrow_mut().sample(0xffffff) as f64 / 16777215.;
            return Ok(sample_scalar_jitter(
                ProgressShape::Power {
                    exponent: 1. + 1.4 * strength,
                    direction: ProgressDirection::Out,
                },
                f64::from(points[1]),
                f64::from(points[2]),
                raw,
                20. * strength,
                noise,
            )? as f32);
        }
        Ok(ScalarTween::new(
            points[1],
            points[2],
            a[0] as u64,
            (b[0] - a[0]) as u64,
            easing_with_strength(mode, (strength * 100.).round() as i32)?,
        )?
        .sample(time))
    };
    let scalar = |field: usize, mode: usize| -> Result<f32> {
        let previous = track.frames.get(index.saturating_sub(2)).unwrap_or(a);
        let following = track.frames.get(index + 1).unwrap_or(b);
        interpolate(
            [
                number(previous[field])?,
                number(a[field])?,
                number(b[field])?,
                number(following[field])?,
            ],
            a[mode],
            number(a[mode + 1])?.round() as i32,
        )
    };
    let rgb = |color: i32| {
        [
            ((color >> 16) & 255) as f32,
            ((color >> 8) & 255) as f32,
            (color & 255) as f32,
        ]
    };
    let sample_rgb = |field: usize| -> Result<[f32; 3]> {
        let mut color = rgb(number(a[field])?.round() as i32);
        if a[45] != 0 && a[0] != b[0] {
            let previous = track.frames.get(index.saturating_sub(2)).unwrap_or(a);
            let following = track.frames.get(index + 1).unwrap_or(b);
            for (i, v) in color.iter_mut().enumerate() {
                *v = interpolate(
                    [
                        rgb(number(previous[field])?.round() as i32)[i],
                        *v,
                        rgb(number(b[field])?.round() as i32)[i],
                        rgb(number(following[field])?.round() as i32)[i],
                    ],
                    a[45],
                    number(a[46])?.round() as i32,
                )?;
            }
        }
        Ok(color)
    };
    let color = sample_rgb(48)?;
    let alpha = scalar(49, 45)?;
    let blend = blend(a[2]as u8);
    let tint = [color[0], color[1], color[2], alpha];
    let source_rect = [
        scalar(20, 18)?,
        scalar(21, 18)?,
        scalar(22, 18)?,
        scalar(23, 18)?,
        scalar(24, 18)?,
        scalar(25, 18)?,
        scalar(26, 18)?,
        scalar(27, 18)?,
    ];
    let selected_value=scalar(97,7)?;
    //5AC9B4 uses TRUNC then subtracts1 for every negative value.
    let selected=f64::from(selected_value.trunc())-if selected_value<0.{1.}else{0.};
    ensure!(selected.is_finite()&&selected>=i32::MIN as f64&&selected<=i32::MAX as f64,"QLIE object selector overflow");
    let selected_object=selected as i32;
    Ok(Some(Sample {
        kind: a[1] as u8,
        image: number(a[9])?.round() as i32,
        nested_motion: usize::try_from(selected_object).unwrap_or(usize::MAX),
        model_object: selected_object,
        model_morph_object: number(b[97])?.floor().clamp(0.,255.)as i32,
        model_morph_weight: if b[0]>a[0] {((time.saturating_sub(a[0]as u64)as f32)/(b[0]-a[0])as f32).clamp(0.,1.)}else{0.},
        native_blend: a[2] as u8,
        native_sampling: a[3] as u8,
        absolute_dimensions: a[102]as u8==1,
        snap_anchor: a[30]as u8==1,
        native_depth: a[4]as u8,
        mask: number(a[10])?.round() as i32,
        mask_progress: scalar(50, 45)?,
        mask_power: number(a[51])?,
        particle_legacy: source_rect,
        particle_extra: [
            scalar(107, 104)?,
            scalar(108, 104)?,
            scalar(109, 104)?,
            scalar(110, 104)?,
            scalar(111, 104)?,
            scalar(112, 104)?,
            scalar(113, 104)?,
            scalar(114, 104)?,
        ],
        seek_end: scalar(14, 11)?.max(0.) as u64,
        z: scalar(35, 31)?,
        scale_z: scalar(38, 31)?,
        anchor_z: number(a[103])?,
        rotation_xy: [scalar(39, 31)?, scalar(40, 31)?],
        primary_rect: source_rect[..4].try_into().unwrap(),
        secondary_rect: source_rect[4..].try_into().unwrap(),
        add: sample_rgb(47)?.map(|v| v / 255.),
        seek: scalar(13, 11)?.max(0.) as u64,
        position: [scalar(33, 31)?, scalar(34, 31)?],
        scale: [scalar(36, 31)?, scalar(37, 31)?],
        anchor: [number(a[28])?, number(a[29])?],
        tint,
        rotation: scalar(41, 31)?,
        order: number(a[42])?,
        blend,
    }))
}
///59D470 ->55E8EC ->538138. All ordinary native blend selectors
///produce odd raster IDs>=3: source alpha is retained and output alpha is
///overwritten with255 (5430A0's two packed OR masks). Unknown byte selectors
///take the original explicit Normal fallback after masking their high bit.
pub(super) fn blend(native:u8)->crysirror_scene::layer::Blend {
    use crysirror_image::rgb_lut::ConstantByteBlend as B;
    let mode=match native&0x7f {
        2=>B::Screen,3=>B::Multiply,4=>B::Difference,5=>B::Lighten,6=>B::Darken,
        7=>B::Add,8=>B::Subtract,9=>B::LinearBurn,10=>B::Exclusion,
        11=>B::Overlay,12=>B::HardLight,13=>B::LinearLight,14=>B::PinLight,
        _=>B::Normal,
    };
    crysirror_scene::layer::Blend::StoredByte{mode,force_source_opaque:false,force_output_opaque:true}
}
