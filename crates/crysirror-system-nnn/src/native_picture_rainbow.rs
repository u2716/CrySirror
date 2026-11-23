// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! RAINBOW96 software32 policy. Palette/offset belong to the single
//! CEffectRainbow object per AllEffect container, not to each effect layer.
//! Calcu runs in layer order; all subsequent Prints use the resulting table.
use anyhow::{Result, ensure};
use crysirror_image::{row_tint_blend::RowTintBlend, temporal_pattern::PatternRng};
use serde::{Deserialize, Serialize};

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    let mut p = [0; 16];
    for i in 0..16 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 100,
            2 => p[1],
            6 | 7 => 1,
            _ => 0,
        });
    }
    // Omitted para8 was copied from an uninitialised stack word. It and
    // unused trailing words have zero defaults; supplied/save words survive.
    p
}
pub(crate) fn validate(p: &[i32; 16], frames: i32) -> Result<()> {
    crate::native_curve::validate(frames, p[4], p[3])
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RainbowPalette {
    pub offset: i32,
    pub colors: Vec<[u8; 3]>,
}
impl RainbowPalette {
    pub(crate) fn new(height: u32) -> Result<Self> {
        ensure!((1..=16384).contains(&height), "NNN RAINBOW palette height");
        // Original new int[] was uninitialised until first Calcu. Defined
        // black replaces that memory without inventing an extra Calcu/RNG step.
        Ok(Self {
            offset: 0,
            colors: vec![[0; 3]; height as usize],
        })
    }
    pub(crate) fn init(&mut self) {
        self.offset = 0;
    }
    pub(crate) fn on_set(&mut self, random: &mut PatternRng) -> Result<()> {
        // SetParam and InitParameter consume three unused random RGB draws.
        // They reset the shared offset but retain the previous colour table.
        for _ in 0..3 {
            let _ = random.next_draw()? % 256;
        }
        self.init();
        Ok(())
    }
    pub(crate) fn calcu(&mut self, p: &[i32; 16]) -> Result<()> {
        ensure!(
            (1..=16384).contains(&self.colors.len()),
            "NNN RAINBOW palette state"
        );
        let height = self.colors.len() as i32;
        self.offset = self.offset.wrapping_add(p[6]);
        if self.offset < 0 {
            self.offset = (self.offset.wrapping_neg() % height).wrapping_neg();
        } else {
            self.offset %= height;
        }
        let table = [
            [255, 0, 0],
            [255, 255, 0],
            [0, 255, 0],
            [0, 255, 255],
            [0, 0, 255],
            [255, 0, 255],
            [255, 0, 0],
        ];
        let span = (height + 5) / 6;
        for segment in 0..6 {
            for i in 0..span {
                let at = segment as i32 * span + i;
                if at >= height {
                    continue;
                }
                let row = (at + self.offset + height) % height;
                self.colors[row as usize] = std::array::from_fn(|c| {
                    let from = table[segment][c];
                    (from + (table[segment + 1][c] - from) * i / span) as u8
                });
            }
        }
        // para5/para7 are stored, but unused by the actual Calcu/Print code.
        Ok(())
    }
}

pub(crate) fn clipped_geometry(
    canvas: [u32; 2],
    picture: [u32; 2],
    clip: [i32; 4],
    point: [i32; 2],
) -> Result<Option<([i32; 4], [i32; 2])>> {
    ensure!(
        canvas
            .iter()
            .chain(&picture)
            .all(|n| (1..=16384).contains(n)),
        "NNN masked picture dimensions"
    );
    ensure!(
        clip.iter()
            .chain(&point)
            .all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN masked picture coordinates"
    );
    let [left, top, right, bottom] = clip.map(i64::from);
    if left >= right || top >= bottom {
        return Ok(None);
    }
    ensure!(
        left >= 0 && top >= 0 && right <= i64::from(picture[0]) && bottom <= i64::from(picture[1]),
        "NNN masked picture trim"
    );
    let [px, py] = point.map(i64::from);
    let dx = px + left;
    let dy = py + top;
    let x0 = 0.max(-dx);
    let y0 = 0.max(-dy);
    let x1 = (right - left).min(i64::from(canvas[0]) - dx);
    let y1 = (bottom - top).min(i64::from(canvas[1]) - dy);
    if x0 >= x1 || y0 >= y1 {
        return Ok(None);
    }
    Ok(Some((
        [
            left as i32 + x0 as i32,
            top as i32 + y0 as i32,
            (x1 - x0) as i32,
            (y1 - y0) as i32,
        ],
        [(dx + x0) as i32, (dy + y0) as i32],
    )))
}

pub(crate) fn draw_plan(
    p: &[i32; 16],
    count: i32,
    frames: i32,
    canvas: [u32; 2],
    picture: [u32; 2],
    clip: [i32; 4],
    point: [i32; 2],
    assigned: bool,
    has_mask: bool,
    palette: &RainbowPalette,
    random: &mut PatternRng,
) -> Result<Option<RowTintBlend>> {
    if !assigned || !has_mask {
        return Ok(None);
    }
    let Some((source, destination)) = clipped_geometry(canvas, picture, clip, point)? else {
        return Ok(None);
    };
    ensure!(
        palette.colors.len() == canvas[1] as usize,
        "NNN RAINBOW palette/canvas mismatch"
    );
    // Curve RNG is consumed only after picture/mask/trim checks succeed.
    let weight = (crate::native_curve::value_with_random(
        count,
        frames.max(1),
        p[2].wrapping_mul(256),
        p[1].wrapping_mul(256),
        p[4],
        p[3],
        random,
    )? / 100)
        .clamp(0, 256) as u16;
    let y = destination[1] as usize;
    let spec = RowTintBlend {
        source,
        destination,
        colors: palette.colors[y..y + source[3] as usize].to_vec(),
        weight,
        // Original LOOP2 compares [edx] (the packed row colour) with256,
        // not transPercent. Retain the actual branch rather than "fixing"
        // it to test weight and changing supported high-resolution palettes.
        source_bypass_color: Some([0, 1, 0]),
    };
    spec.validate()?;
    Ok(Some(spec))
}
