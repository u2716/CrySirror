// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CLOUD82 software row tables and shared-class Print clock. Full1319-line
//! source/header read; D3D projects quads and moves in the opposite direction,
//! so it is not substituted for this packed-word software profile.
use anyhow::{Result, ensure};
#[derive(Clone, Default)]
pub(crate) struct CloudClass {
    xy: [i32; 2],
}
pub(crate) fn parameters(values: &[i32], canvas: [u32; 2], retained_word14: i32) -> [i32; 16] {
    let mut p = [0; 16];
    let defaults = [9999, 200, 0, 50, 200, 100, 500, 100, 0, 0, 0, 0, 0, 0];
    for i in 0..14 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            11 | 12 => p[i - 1],
            _ => defaults[i],
        });
    }
    p[1] = p[1].clamp(1, canvas[1] as i32);
    p[13] = p[13].clamp(0, canvas[1] as i32 - 1);
    p[5] = p[5].max(1);
    // Widen the source's overflowing near+1 edge rather than creating a
    // nonpositive divisor; the finite admitted projection domain is explicit.
    p[6] = p[6].max(p[5].saturating_add(1));
    p[7] = p[7].max(1);
    p[13] = p[13].min(p[1] - 1).max(0);
    // SetParam does not WRITE EFFECT.para14 at all. Retain the old slot
    // word even if the incoming parameter vector includes a fourteenth tail.
    p[14] = retained_word14;
    p
}
pub(crate) fn validate(p: &[i32; 16]) -> Result<()> {
    ensure!(
        p[1] > 0 && p[1] <= 16384 && p[5] > 0 && p[6] > p[5] && p[7] > 0,
        "NNN cloud perspective divisor/domain"
    );
    Ok(())
}
#[derive(Clone, Copy)]
pub(crate) struct Row {
    pub y: i32,
    pub x_q16: u32,
    pub step_q16: u32,
    pub add_rgb: [u8; 3],
    pub subtract_rgb: [u8; 3],
}
impl CloudClass {
    pub(crate) fn reset(&mut self) {
        self.xy = [0, 0];
    }
    pub(crate) fn print_clock(
        &mut self,
        p: &[i32; 16],
        count: &mut i32,
        picture_size: Option<[u32; 2]>,
    ) {
        if *count <= 0 {
            return;
        }
        if let Some(size) = picture_size.filter(|s| s.iter().all(|n| *n > 0 && *n <= 16384)) {
            for axis in 0..2 {
                let modulus = size[axis] as i32 * 100;
                self.xy[axis] = self.xy[axis].wrapping_add(p[2 + axis]).rem_euclid(modulus);
            }
        }
        // This reset also occurs if the picture/null-buffer guard failed.
        *count = 0;
    }
    pub(crate) fn rows(&self, p: &[i32; 16], size: [u32; 2], canvas: [u32; 2]) -> Result<Vec<Row>> {
        validate(p)?;
        ensure!(
            size.iter()
                .chain(canvas.iter())
                .all(|n| (1..=16384).contains(n)),
            "NNN cloud source/canvas extent"
        );
        let height = p[1].min(canvas[1] as i32);
        let h = f64::from(p[4]);
        let near = f64::from(p[5]);
        let far = f64::from(p[6]);
        let h2 = h * (far - near) / far;
        let initial = 100.0 / f64::from(p[7]);
        let mut rows = Vec::with_capacity(height as usize);
        for i in 0..height {
            let mut hk = f64::from(i + 1);
            hk *= h2;
            hk /= f64::from(height);
            let nk = h * near / (h - hk);
            let step = initial * nk / near;
            let sy = f64::from(self.xy[1]) * 0.01 + (nk - near) * initial;
            ensure!(
                step.is_finite()
                    && sy.is_finite()
                    && step >= 0.0
                    && step <= f64::from(i32::MAX)
                    && sy.abs() <= f64::from(i32::MAX),
                "NNN cloud non-finite original projection"
            );
            let y = (sy as i32) % size[1] as i32;
            let high = (step as i32) % size[0] as i32;
            let low = (((step - f64::from(step as i32)) * 65536.0 + 0.5) as i32).min(65535);
            let mut dx_high = canvas[0] as i32 / 2 + self.xy[0] / 100;
            let mut dx_low = ((self.xy[0] % 100) * 65536) / 100;
            dx_high = (dx_high + ((dx_low >> 16) & 0xffff)) % size[0] as i32;
            dx_low &= 0xffff;
            let mut ddx_high = high.wrapping_mul(canvas[0] as i32 / 2);
            let mut ddx_low = low.wrapping_mul(canvas[0] as i32 / 2);
            ddx_high = (ddx_high + ((ddx_low >> 16) & 0xffff)) % size[0] as i32;
            ddx_low &= 0xffff;
            ddx_high = (size[0] as i32 - ddx_high) % size[0] as i32;
            if ddx_low > 0 {
                ddx_high = (ddx_high - 1 + size[0] as i32) % size[0] as i32;
                ddx_low = 65536 - ddx_low;
            }
            dx_high += ddx_high;
            dx_low += ddx_low;
            dx_high = (dx_high + ((dx_low >> 16) & 0xffff)) % size[0] as i32;
            dx_low &= 0xffff;
            let mut add = 0i32;
            let mut sub = 0i32;
            if p[9] != 0 && i >= p[13] {
                let dv = (height - p[13]).max(1);
                let cc = (i - p[13]).min(dv);
                let scaled = |v: i32| v.max(0).wrapping_mul(cc).wrapping_add(dv / 2) / dv;
                let r = std::array::from_fn::<_, 3, _>(|c| scaled(p[10 + c]));
                let s = std::array::from_fn::<_, 3, _>(|c| scaled(p[10 + c].wrapping_neg()));
                add = r[0].wrapping_shl(16) | r[1].wrapping_shl(8) | r[2];
                sub = s[0].wrapping_shl(16) | s[1].wrapping_shl(8) | s[2];
            }
            rows.push(Row {
                y,
                x_q16: (dx_high as u32) << 16 | dx_low as u32,
                step_q16: (high as u32) << 16 | low as u32,
                add_rgb: [(add >> 16) as u8, (add >> 8) as u8, add as u8],
                subtract_rgb: [(sub >> 16) as u8, (sub >> 8) as u8, sub as u8],
            });
        }
        Ok(rows)
    }
}
// Shared software sampler must reproduce packed32-bit X addition followed by
// SIGNED16 high-word compare and high-word-only subtraction, not ordinary
// modulo. Unmasked add/sub writes pairs only, leaving an odd last pixel alone.
// Masked sums truncate once and copy mask255. The add/sub masked branch has a
// further native quirk: mm7 is also scratch during partial coverage and used
// by the following mask255 subtraction; tmpmm7 is only used by partial pixels.
// Retain that row-local register state in shared code, rather than replacing
// it with a stateless colour filter. No CharaClip/assigned-picture guard.
// Raw InitParameter inherited empty: retain classXY, all16 words and counters.
// Fresh SetParam writes0..13/15, resetsXY; CountIsMaxTRUE; NeedPicPrintFALSE.

impl CloudClass {
    pub(crate) fn raster_spec(
        &self,
        p: &[i32; 16],
        source_size: [u32; 2],
        canvas: [u32; 2],
        mask_exists: bool,
    ) -> Result<crysirror_image::packed_repeat_rows::PackedRepeatRows> {
        use crysirror_image::packed_repeat_rows::{PackedRepeatRow, PackedRepeatRows};
        let spec = PackedRepeatRows {
            rows: self
                .rows(p, source_size, canvas)?
                .into_iter()
                .map(|row| PackedRepeatRow {
                    source_y: row.y,
                    source_x_q16: row.x_q16,
                    step_q16: row.step_q16,
                    add_rgb: row.add_rgb,
                    subtract_rgb: row.subtract_rgb,
                })
                .collect(),
            masked: p[8] != 0 && mask_exists,
            offset_colors: p[9] != 0,
            unmasked_offset_pairs: true,
            opaque_subtract_prior_blend: true,
        };
        spec.validate()?;
        Ok(spec)
    }
}
