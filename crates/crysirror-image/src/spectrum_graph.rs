// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Spectrum graph rasterization over an existing immutable-derived surface.
use crate::ImageSurface;
use anyhow::{Result, ensure};
use crysirror_core::service::{SpectrumGraphSpec, SpectrumGraphState, SpectrumGraphStyle};
pub fn draw(
    image: &mut ImageSurface,
    spec: &SpectrumGraphSpec,
    state: &SpectrumGraphState,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    image.validate()?;
    spec.validate([image.width, image.height])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let [left, top, w, h] = spec.rect;
    let pixel = |image: &mut ImageSurface, x: u32, y: u32, color: [u8; 4]| {
        let at = ((top + y) as usize * image.width as usize + (left + x) as usize) * 4;
        image.rgba[at..at + 4].copy_from_slice(&color);
    };
    match &spec.style {
        SpectrumGraphStyle::Fire => {
            const FIRE: [u32; 16] = [
                0xff20ff00, 0xff40ff00, 0xff60ff00, 0xff80ff00, 0xffa0ff00, 0xffc0ff00, 0xffe0ff00,
                0xffffff00, 0xffffe000, 0xffffc000, 0xffffa000, 0xffff8000, 0xffff6000, 0xffff4000,
                0xffff2000, 0xffffff00,
            ];
            ensure!(
                state.levels.len() == w as usize && state.peaks.len() == w as usize,
                "spectrum fire state extent"
            );
            for x in 0..w {
                check()?;
                let level = state.levels[x as usize];
                for y in 0..h {
                    let color = if (y as i32) < level {
                        let c = 15 - level + y as i32;
                        if c < 0 {
                            0xff00ff00
                        } else {
                            FIRE[c.min(15) as usize]
                        }
                    } else {
                        0
                    };
                    pixel(image, x, h - 1 - y, rgba(color));
                }
                // Source pointer stores can leave its rectangle after resizing;
                // portable rasterization clips them instead of corrupting memory.
                if let Ok(y) = u32::try_from(state.peaks[x as usize]) {
                    if y < h {
                        pixel(image, x, h - 1 - y, rgba(0xff808080));
                    }
                }
            }
        }
        SpectrumGraphStyle::Bars {
            divisions,
            thickness,
            on,
            off,
            background,
            peak,
        } => {
            let count = *divisions as usize;
            ensure!(
                state.levels.len() == count && state.peaks.len() == count,
                "spectrum bar state extent"
            );
            let bw = w / divisions;
            let rows = h / thickness;
            let step = 1000 / rows as i32;
            for i in 0..count {
                check()?;
                let mut peakdrawn = false;
                for row in 0..rows {
                    let y = row * thickness;
                    let value = row as i32 * step;
                    for x in 0..bw {
                        pixel(image, i as u32 * bw + x, h - 1 - y, *background);
                    }
                    let color = if !peakdrawn && state.peaks[i] <= value && peak != off {
                        peakdrawn = true;
                        *peak
                    } else if state.levels[i] >= value {
                        *on
                    } else {
                        *off
                    };
                    for dy in 1..*thickness {
                        pixel(image, i as u32 * bw, h - 1 - y - dy, *background);
                        for x in 1..bw {
                            pixel(image, i as u32 * bw + x, h - 1 - y - dy, color);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
fn rgba(argb: u32) -> [u8; 4] {
    [
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        (argb >> 24) as u8,
    ]
}
