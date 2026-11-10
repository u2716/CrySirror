// Copyright (c)2026 CrySirror contributors. MPL-2.0.
//! Explicit directional glyph coverage stamps; no native VM policy lives here.
use crate::raster::GlyphMask;
use anyhow::{Context, Result, ensure};

pub(crate) fn stamp(
    source: &GlyphMask,
    offsets: &[[i16; 2]],
    check: &dyn Fn() -> Result<()>,
) -> Result<GlyphMask> {
    ensure!(
        !offsets.is_empty() && offsets.len() <= 256,
        "glyph stamp count"
    );
    ensure!(
        offsets.iter().flatten().all(|v| v.unsigned_abs() <= 64),
        "glyph stamp offset"
    );
    let min_x = offsets.iter().map(|p| i32::from(p[0])).min().unwrap();
    let min_y = offsets.iter().map(|p| i32::from(p[1])).min().unwrap();
    let max_x = offsets.iter().map(|p| i32::from(p[0])).max().unwrap();
    let max_y = offsets.iter().map(|p| i32::from(p[1])).max().unwrap();
    let width = source
        .width
        .checked_add((max_x - min_x) as usize)
        .context("glyph stamp width")?;
    let height = source
        .height
        .checked_add((max_y - min_y) as usize)
        .context("glyph stamp height")?;
    let pixels = width.checked_mul(height).context("glyph stamp surface")?;
    let work = source
        .width
        .checked_mul(source.height)
        .and_then(|n| n.checked_mul(offsets.len()))
        .context("glyph stamp work")?;
    ensure!(
        pixels <= 8 * 1024 * 1024 && work <= 64 * 1024 * 1024,
        "glyph stamp budget"
    );
    let mut output = GlyphMask {
        x: source.x.checked_add(min_x).context("glyph stamp x")?,
        y: source.y.checked_add(min_y).context("glyph stamp y")?,
        width,
        height,
        alpha: vec![0; pixels],
    };
    for offset in offsets {
        let dx = (i32::from(offset[0]) - min_x) as usize;
        let dy = (i32::from(offset[1]) - min_y) as usize;
        for y in 0..source.height {
            check()?;
            let src = &source.alpha[y * source.width..(y + 1) * source.width];
            let start = (y + dy) * width + dx;
            for (target, value) in output.alpha[start..start + source.width]
                .iter_mut()
                .zip(src)
            {
                *target = (*target).max(*value);
            }
        }
    }
    Ok(output)
}
