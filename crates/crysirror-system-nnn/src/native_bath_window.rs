// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! BATHWINDOW97 parameters and per-Calcu stamp geometry/RNG. Constructor
//! templates and alpha mutation belong to shared pattern/image workers.
//! Full active class/header read; disabled old erase/stamp blocks excluded.
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 9] {
    let defaults = [20, 20, 1, 5, 0, 0, canvas[0] as i32, canvas[1] as i32, 0];
    let mut p = std::array::from_fn(|i| values.get(i).copied().unwrap_or(defaults[i]));
    p[1] = p[1].max(8);
    // Native >64 maps to128, rather than clamping to64 or128 normally.
    if p[1] > 64 {
        p[1] = 128;
    }
    p[2] = 1;
    p[3] = p[3].clamp(0, 32);
    for a in 0..2 {
        p[4 + a] = p[4 + a].clamp(0, canvas[a] as i32 - 1);
        p[6 + a] = p[6 + a].clamp(1, canvas[a] as i32);
    }
    p
}
pub(crate) fn validate(p: &[i32; 9]) -> Result<()> {
    ensure!(
        p[1] > 0 && p[1] <= 128 && (0..=32).contains(&p[3]),
        "NNN bath stamp parameter domain"
    );
    Ok(())
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stamp {
    pub destination: [i32; 4],
    pub source: [i32; 2],
    pub template: u32,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Erase {
    pub destination: [i32; 4],
    pub source: [i32; 2],
    pub divisor: [i32; 2],
    pub template: u32,
}
#[derive(Clone)]
pub(crate) struct Update {
    pub flush_filename: bool,
    pub steam: Vec<Stamp>,
    pub erase: Option<Erase>,
}
#[derive(Clone)]
pub(crate) struct BathWindowClass {
    last_mouse: [i32; 2],
}
impl Default for BathWindowClass {
    fn default() -> Self {
        Self {
            last_mouse: [-1, -1],
        }
    }
}
impl BathWindowClass {
    pub(crate) fn calcu(
        &mut self,
        p: &[i32; 9],
        size: [u32; 2],
        canvas: [u32; 2],
        mouse: [i32; 2],
        mask_exists: bool,
        rng: &mut PatternRng,
    ) -> Result<Option<Update>> {
        validate(p)?;
        // Caller resets EFFECT.count even when the picture/mask guard fails.
        if !mask_exists {
            return Ok(None);
        }
        ensure!(
            size.iter().all(|s| (1..=16384).contains(s)),
            "NNN bath picture extent"
        );
        let block_x = (size[0] as i32 / 8).max(1);
        let height = size[1] as i32;
        let mut steam = Vec::with_capacity(p[3] as usize);
        for _ in 0..p[3] {
            let mut x = (rng.next_draw()? % block_x as u32) as i32 - 2;
            let mut y = (rng.next_draw()? % size[1]) as i32 - 16;
            let mut w = 4;
            let mut h = 32;
            let mut sx = 0;
            let mut sy = 0;
            if x < 0 {
                w -= -x;
                sx += -x;
                x = 0;
                w = w.max(0);
            }
            if x + w > block_x {
                w -= x + w - block_x;
                x = block_x - w;
            }
            w = w.max(0);
            if y < 0 {
                h -= -y;
                sy += -y;
                y = 0;
                h = h.max(0);
            }
            if y + h > height {
                h -= y + h - height;
                y = height - h;
            }
            h = h.max(0);
            if w == 0 || h == 0 {
                w = 0;
                h = 0;
            }
            steam.push(Stamp {
                destination: [x * 8, y, w * 8, h],
                source: [sx * 8, sy],
                template: 0,
            });
        }
        // This draw happens even with zero stamps. Empty stamps still advance
        // the four-template index, just like the MMX outer loop.
        let first = rng.next_draw()? % 4;
        for (i, stamp) in steam.iter_mut().enumerate() {
            stamp.template = (first + i as u32) & 3;
        }
        let dont_erase = mouse == self.last_mouse
            || mouse[0] < 0
            || mouse[1] < 0
            || mouse[0] >= canvas[0] as i32
            || mouse[1] >= canvas[1] as i32;
        self.last_mouse = mouse;
        let mut x = i64::from(mouse[0]) - i64::from(p[1] / 2);
        let mut y = i64::from(mouse[1]) - i64::from(p[1] / 2);
        let divisor = [2 * (p[1] / 2); 2];
        let mut w = i64::from(divisor[0]);
        let mut h = i64::from(divisor[1]);
        let mut sx = 0;
        let mut sy = 0;
        if x < 0 {
            w -= -x;
            sx += -x;
            x = 0;
            w = w.max(0);
        }
        if x + w > i64::from(size[0]) {
            w -= x + w - i64::from(size[0]);
        }
        if y < 0 {
            h -= -y;
            sy += -y;
            y = 0;
            h = h.max(0);
        }
        if y + h > i64::from(size[1]) {
            h -= y + h - i64::from(size[1]);
        }
        let erase = if !dont_erase && w > 0 && h > 0 {
            Some(Erase {
                destination: [x as i32, y as i32, w as i32, h as i32],
                source: [sx as i32, sy as i32],
                divisor,
                template: rng.next_draw()? % 2,
            })
        } else {
            None
        };
        Ok(Some(Update {
            flush_filename: p[8] != 0,
            steam,
            erase,
        }))
    }
}
// Software32 Calcu saturating-adds the selected32x32 steam templates into
// retained mask bytes, then saturating-subtracts the scaled128x128 erase plane.
// Constructor template order is4*32*32 draws then2*128*128 draws; the former
// sqrt radius affects additive offset, the latter scales the random sample.
// These per-pixel templates MUST be generated by a shared worker with RNG
// returned transactionally, not reproduced by a private VM raster loop.
// Print uses Blt(dst=para4/5,src=para4/5,size=para6/7,TRUE), including CharaClip;
// CountIsMaxFALSE, countMax9999, NeedPicPrintFALSE, CheckAllPrintTRUE, eternal1.
// Source has no InitParameter override: raw restore must NOT rerun SetParam.

impl Update {
    /// The atlas is constructor-owned shared image data. This method produces
    /// only ordered native geometry; alpha reads/writes execute in the engine.
    pub(crate) fn brushes(
        &self,
        picture_width: u32,
    ) -> Result<crysirror_image::alpha_brushes::AlphaBrushes> {
        use crysirror_image::alpha_brushes::{AlphaBrush, AlphaBrushMath, AlphaBrushes};
        let mut brushes = Vec::with_capacity(self.steam.len() + usize::from(self.erase.is_some()));
        for stamp in &self.steam {
            brushes.push(AlphaBrush {
                destination: stamp.destination,
                source_origin: [0, (stamp.template * 32) as i32],
                source_phase: stamp.source,
                numerator: [1, 1],
                divisor: [1, 1],
                math: AlphaBrushMath::SaturatingAdd,
            });
        }
        if let Some(erase) = self.erase {
            brushes.push(AlphaBrush {
                destination: erase.destination,
                source_origin: [0, 128 + (erase.template * 128) as i32],
                source_phase: erase.source,
                numerator: [128, 128],
                divisor: erase.divisor.map(|n| n as u32),
                math: AlphaBrushMath::SaturatingSubtract,
            });
        }
        let spec = AlphaBrushes {
            destination_pitch: picture_width,
            brushes,
        };
        spec.validate()?;
        Ok(spec)
    }
}
