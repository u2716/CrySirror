// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! OLDFILM55, effectoldfilm.cpp: retained line timers and global RNG order.
//! Constructor sets the continuous-effect flag, countMax9999, CountIsMax FALSE.
//! Line state changes in Print, not Calcu; hidden/unprinted frames do not move
//! lines or consume their random draws. End/Init/BeginPrint/Calcu own no state.
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
    temporal_pattern::PatternRng,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FilmLine {
    pub rectangle: [i32; 4],
    pub color: [u8; 4],
    pub weight: u16,
}

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> Result<[i32; 56]> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN OLDFILM dimensions"
    );
    let mut p = [0i32; 56];
    for i in 0..15 {
        // para0 has no default in the source and never controls its9999
        // lifetime. Explicit zero replaces that unused uninitialised word.
        p[i] = values.get(i).copied().unwrap_or(match i {
            3 => canvas[0] as i32,
            4 => canvas[1] as i32,
            5 => 3,
            6 => 1,
            7..=9 => 128,
            10 => 200,
            11 => 80,
            12 => 30,
            13 | 14 => 100,
            _ => 0,
        });
    }
    p[5] = p[5].min(10);
    p[6] = p[6].clamp(1, 4);
    for axis in 0..2 {
        let limit = canvas[axis] as i32;
        p[1 + axis] = p[1 + axis].max(0);
        if p[1 + axis] >= limit - p[6] + 1 {
            p[1 + axis] = limit - p[6];
        }
        if p[1 + axis].wrapping_add(p[3 + axis]) > limit {
            p[3 + axis] = limit - p[1 + axis];
        }
    }
    // para15 and inactive line scratch words are uninitialised in C++.
    // Deterministic zeroes are used until InitLine; original saves hydrate
    // all56 live words independently of this fresh-parameter constructor.
    Ok(p)
}

fn draw(random: &mut PatternRng, divisor: i32) -> Result<i32> {
    (random.next_draw()? as i32)
        .checked_rem(divisor)
        .context("NNN OLDFILM native zero random modulus")
}

fn init_line(p: &mut [i32; 56], n: usize, random: &mut PatternRng) -> Result<()> {
    let k = 16 + n * 4;
    p[k + 2] = draw(random, 20)? + 10;
    let duration = p[10];
    let spread = (p[11].max(1).wrapping_mul(duration) / 100).max(1);
    let mut delta = draw(random, spread)?;
    if draw(random, 100)? < 50 {
        delta = delta.wrapping_neg();
    }
    p[k + 3] = duration.wrapping_add(delta).max(1);
    let span = p[3].wrapping_sub(p[6] - 1).max(1);
    let x = draw(random, span)?;
    p[k] = x.wrapping_add(p[1]).wrapping_mul(256);
    p[k + 1] = draw(random, 1024)? + 128;
    if draw(random, 100)? < 50 {
        p[k + 1] = p[k + 1].wrapping_neg();
    }
    Ok(())
}

pub(crate) fn validate(p: &[i32; 56]) -> Result<()> {
    ensure!(
        p[5] <= 10 && (1..=4).contains(&p[6]),
        "NNN OLDFILM line count/width"
    );
    Ok(())
}

pub(crate) fn initialize(p: &mut [i32; 56], random: &mut PatternRng) -> Result<()> {
    validate(p)?;
    for n in 0..p[5].max(0) as usize {
        init_line(p, n, random)?;
    }
    Ok(())
}

pub(crate) fn print_plan(p: &mut [i32; 56], random: &mut PatternRng) -> Result<Vec<FilmLine>> {
    validate(p)?;
    let minimum = p[1].wrapping_mul(256);
    let maximum = p[1]
        .wrapping_add(p[3])
        .wrapping_sub(p[6] - 1)
        .wrapping_mul(256);
    // CAllGeo packs the raw integer components before extracting bytes. A
    // negative/oversized component can affect adjacent bytes; do not replace
    // this with independently clamped RGB components.
    let packed = (p[7] as u32).wrapping_shl(16) | (p[8] as u32).wrapping_shl(8) | p[9] as u32;
    let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255];
    let mut lines = Vec::with_capacity(p[5].max(0) as usize);
    for n in 0..p[5].max(0) as usize {
        let k = 16 + n * 4;
        p[k + 3] = p[k + 3].wrapping_sub(1);
        if p[k + 3] <= 0 {
            init_line(p, n, random)?;
        }
        p[k + 2] = p[k + 2].wrapping_sub(1);
        if p[k + 2] <= 0 {
            p[k + 2] = draw(random, 20)? + 10;
            p[k + 1] = draw(random, 1024)? + 128;
            if draw(random, 100)? < 50 {
                p[k + 1] = p[k + 1].wrapping_neg();
            }
        }
        let mut x = p[k].wrapping_add(p[k + 1]);
        if x >= maximum {
            x = maximum.wrapping_sub(1);
            p[k + 1] = p[k + 1].wrapping_neg();
        }
        if x < minimum {
            x = minimum;
            p[k + 1] = p[k + 1].wrapping_neg();
        }
        p[k] = x;
        if draw(random, 100)? < p[12] {
            let (low, high) = (p[13].min(p[14]), p[13].max(p[14]));
            // Even equal endpoints consume a rand()%1 in the native Print.
            let percent = low.wrapping_add(draw(random, high.wrapping_sub(low).wrapping_add(1))?);
            let weight = (percent.clamp(0, 100) * 256 / 100) as u16;
            lines.push(FilmLine {
                rectangle: [x / 256, p[2], p[6], p[4]],
                color,
                weight,
            });
        }
    }
    Ok(lines)
}

pub(crate) fn image_input(
    mut base: ImageInput,
    canvas: [u32; 2],
    lines: &[FilmLine],
) -> Result<ImageInput> {
    ensure!(
        canvas.iter().all(|n| (1..=16384).contains(n)),
        "NNN OLDFILM canvas"
    );
    for line in lines {
        let [x, y, w, h] = line.rectangle.map(i64::from);
        let (left, top) = (x.max(0), y.max(0));
        let (right, bottom) = (
            (x + w).min(i64::from(canvas[0])),
            (y + h).min(i64::from(canvas[1])),
        );
        if left >= right || top >= bottom {
            continue;
        }
        let size = [(right - left) as u32, (bottom - top) as u32];
        let spec = RowBlit {
            source: [0, 0, size[0] as i32, size[1] as i32],
            destination: [left as i32, top as i32],
            row_offsets: vec![0; size[1] as usize],
            // Solid alpha255 selects exactly(sum of two products)>>8,
            // matching TransBoxFill, while weight256 is opaque BoxFill.
            math: ByteRasterMath::MaskPiecewiseWrapping256 {
                weight: line.weight,
            },
            row_math: vec![],
        };
        spec.validate()?;
        base = ImageInput::Process {
            input: Box::new(ImageInput::Raster {
                destination: Box::new(base),
                source: Box::new(ImageInput::Solid {
                    size,
                    color: line.color,
                }),
            }),
            effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                spec: ByteRaster::RowBlit { spec },
            }])?
            .into(),
        };
    }
    Ok(base)
}
