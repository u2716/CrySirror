//! Native ABMP key records. Field names/strides are checked against
//! TABMPLayer10.LoadFromStream/ExportStringList; interpolation uses the engine
//! ScalarTween. Native records are never rendered to a private framebuffer.
use super::*;
mod compact;
use compact::compact_key;
#[derive(Clone)]
pub(super) struct Animation {
    pub size: [u32; 2],
    pub motions: Vec<Motion>,
    pub capture_both: bool,
}
#[derive(Clone)]
pub(super) struct Motion {
    pub name: String,
    pub duration_ms: u64,
    pub layers: Vec<Track>,
}
#[derive(Clone)]
pub(super) struct Track {
    /// Serialized TABMPLayer+45 gates sound callbacks, not drawing. The
    /// independent native draw-state byte+44 is not stored in this header.
    pub sound_enabled: bool,
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
        ensure!(version == 1 || version == 2, "ABMP data version {version}");
        let size = [r.int()?, r.int()?];
        ensure!(size.iter().all(|n| *n <= 8192), "ABMP viewport size");
        r.int()?;
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
                r.text()?;
                let sound_enabled = r.byte()? != 0;
                let count = if matches!(
                    tag.as_str(),
                    "ablayer18" | "ablayer20" | "ablayer21" | "ablayer22" | "ablayer23"
                ) {
                    r.byte()? as usize
                } else {
                    r.int()? as usize
                };
                let channel = r.byte()?;
                let stride = match tag.as_str() {
                    "ablayer18" => 162,
                    "ablayer20" | "ablayer21" | "ablayer22" | "ablayer23" => 440,
                    "ablayer24" | "ablayer25" => r.int()? as usize,
                    _ => bail!("unsupported ABMP layer layout {tag}"),
                };
                ensure!(
                    stride == 162 || stride == 440 || stride == 460,
                    "ABMP key stride {stride}"
                );
                ensure!(count <= 65536 && channel <= 3, "ABMP key/channel bounds");
                total += count;
                ensure!(total <= 65536, "ABMP animation key budget");
                let mut frames = Vec::with_capacity(count);
                for _ in 0..count {
                    let raw = r.take(stride)?;
                    let mut values = if stride == 162 {
                        compact_key(raw)?
                    } else {
                        let mut values = [0; 115];
                        for (i, word) in raw.chunks_exact(4).enumerate() {
                            values[i] = i32::from_le_bytes(word.try_into()?);
                        }
                        values
                    };
                    if stride < 460 {
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
                layers.push(Track {
                    sound_enabled,
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
            motions,
            capture_both,
        })
    }
}
pub(super) fn easing(mode: i32) -> Result<crysirror_scene::layer::Easing> {
    use crysirror_scene::layer::Easing as E;
    Ok(match mode {
        0 | 1 | 3 => E::Linear,
        4 => E::EaseOutQuart,
        7 => E::EaseInQuart,
        8 => E::EaseOutInQuart,
        12 => E::EaseInOutQuart,
        _ => bail!("unsupported QLIE interpolation mode {mode}"),
    })
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
    pub mask: i32,
    pub mask_progress: f32,
    pub mask_power: f32,
    pub add: [f32; 3],
    pub seek: u64,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub anchor: [f32; 2],
    pub tint: [f32; 4],
    pub rotation: f32,
    pub order: i32,
    pub blend: &'static str,
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
pub(super) fn sample(track: &Track, time: u64, values: &Values) -> Result<Option<Sample>> {
    use crysirror_scene::animation::ScalarTween;
    let index = track.frames.partition_point(|f| f[0] as u64 <= time);
    if index == 0 {
        return Ok(None);
    }
    let a = &track.frames[index - 1];
    let b = track.frames.get(index).unwrap_or(a);
    let number = |n: i32| values.number(n, time, a[0] as u64, b[0] as u64);
    let scalar = |field: usize, mode: usize| -> Result<f32> {
        if a[mode] == 0 || a[0] == b[0] {
            return number(a[field]);
        }
        Ok(ScalarTween::new(
            number(a[field])?,
            number(b[field])?,
            a[0] as u64,
            (b[0] - a[0]) as u64,
            easing(a[mode])?,
        )?
        .sample(time))
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
            for (i, v) in color.iter_mut().enumerate() {
                *v = ScalarTween::new(
                    *v,
                    rgb(number(b[field])?.round() as i32)[i],
                    a[0] as u64,
                    (b[0] - a[0]) as u64,
                    easing(a[45])?,
                )?
                .sample(time);
            }
        }
        Ok(color)
    };
    let color = sample_rgb(48)?;
    let alpha = scalar(49, 45)?;
    let blend = match a[2] {
        0 | 1 => "normal",
        2 => "screen",
        3 => "multiply",
        7 => "add",
        8 => "subtract",
        _ => bail!("unsupported QLIE ABMP blend {}", a[2]),
    };
    let tint = [color[0], color[1], color[2], alpha];
    Ok(Some(Sample {
        kind: a[1] as u8,
        image: number(a[9])?.round() as i32,
        nested_motion: usize::try_from(number(a[97])?.round() as i32)
            .context("QLIE nested motion index")?,
        mask: number(a[10])?.round() as i32,
        mask_progress: scalar(50, 45)?,
        mask_power: number(a[51])?,
        add: sample_rgb(47)?.map(|v| v / 255.),
        seek: scalar(13, 11)?.max(0.) as u64,
        position: [scalar(33, 31)?, scalar(34, 31)?],
        scale: [scalar(36, 31)?, scalar(37, 31)?],
        anchor: [number(a[28])?, number(a[29])?],
        tint,
        rotation: scalar(41, 31)?,
        order: a[42],
        blend,
    }))
}
