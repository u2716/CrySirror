// SPDX-License-Identifier: MPL-2.0
//! Square coverage cells cut from an 8-bit glyph sheet and resampled to another cell
//! size with 10-bit fixed-point bilinear weights. Engines with bitmap fonts (gaiji,
//! picture glyphs) draw these as coverage tinted by the text colour.

/// Copies cell `index` of a sheet laid out in rows of `width / cell` cells, or None
/// when the cell lies outside the sheet.
pub fn cut(sheet: &[u8], width: u32, cell: u32, index: u32) -> Option<Vec<u8>> {
    let columns = width.checked_div(cell).filter(|&columns| columns > 0)?;
    let (cell, width) = (cell as usize, width as usize);
    let x = cell * (index % columns) as usize;
    let y = cell * (index / columns) as usize;
    let mut out = Vec::with_capacity(cell * cell);
    for row in 0..cell {
        let start = (y + row).checked_mul(width)?.checked_add(x)?;
        out.extend_from_slice(sheet.get(start..start + cell)?);
    }
    Some(out)
}

/// Resamples a `source`×`source` cell to `target`×`target`. Steps are
/// `(source << 10) / target`; at the right edge both horizontal taps repeat, and on
/// the last source row the lower pair repeats the upper one's left tap and uses the
/// right tap of the upper row.
pub fn resample(cell: &[u8], source: u32, target: u32) -> Vec<u8> {
    let mut out = vec![0; (target * target) as usize];
    if source == 0 || target == 0 || cell.len() < (source * source) as usize {
        return out;
    }
    let step = (source << 10) / target;
    let last = source - 1;
    let at = |index: u32| u32::from(cell[index as usize]);
    let mut v = 0u32;
    for row in out.chunks_exact_mut(target as usize) {
        let line = source * (v >> 10);
        let mut u = 0u32;
        for pixel in row {
            let column = u >> 10;
            let fraction = u & 0x3ff;
            let top_left = line + column;
            let top_right = if column >= last { top_left } else { top_left + 1 };
            let (bottom_left, bottom_right) = if v >> 10 >= last {
                (top_left, top_right)
            } else {
                let below = top_left + source;
                (below, if column >= last { top_right } else { below + 1 })
            };
            let upper = (1024 - fraction) * at(top_left) + fraction * at(top_right);
            let lower = (1024 - fraction) * at(bottom_left) + fraction * at(bottom_right);
            *pixel = (((v & 0x3ff) * lower + (1024 - (v & 0x3ff)) * upper) >> 20) as u8;
            u += step;
        }
        v += step;
    }
    out
}

/// Paint an eight-bit square coverage cell onto a transparent DIB, with an
/// optional shadow offset by one pixel. The gradient uses signed 10-bit
/// increments and the colour and coverage accumulators use 256 as unity.
pub fn paint_cell(
    mask: &[u8], size: u32, colors: [[u8; 3]; 2], gradient: u32, shadow: Option<[u8; 3]>,
) -> anyhow::Result<crate::ImageSurface> {
    let side = size.checked_add(1).ok_or_else(|| anyhow::anyhow!("glyph size overflow"))?;
    let len = crate::rgba_len(side, side)?;
    anyhow::ensure!(size > 0 && mask.len() == size as usize * size as usize, "invalid glyph mask");
    let mut rgba = vec![0; len];
    let mut paint = |x: usize, y: usize, color: [u8; 3], coverage: u8| {
        let start = (y * side as usize + x) * 4;
        let pixel = &mut rgba[start..start + 4];
        let a = u32::from(coverage);
        for (channel, source) in pixel[..3].iter_mut().zip(color) {
            *channel = ((u32::from(source) * a + u32::from(*channel) * (256 - a)) >> 8) as u8;
        }
        pixel[3] = if pixel[3] == 0 { coverage }
            else { ((255 * a + u32::from(pixel[3]) * (256 - a)) >> 8) as u8 };
    };
    if let Some(color) = shadow {
        for (i, &a) in mask.iter().enumerate().filter(|(_, a)| **a != 0) {
            paint(i % size as usize + 1, i / size as usize + 1, color, a);
        }
    }
    let step: [i32; 3] = std::array::from_fn(|c|
        ((i32::from(colors[1][c]) - i32::from(colors[0][c])) << 10) / size as i32);
    for (i, &a) in mask.iter().enumerate().filter(|(_, a)| **a != 0) {
        let (x, y) = (i % size as usize, i / size as usize);
        let color = std::array::from_fn(|c| {
            let advance = match gradient {
                1 => step[c] * y as i32,
                2 => step[c] * x as i32,
                3 => (step[c] / 2) * (x + y) as i32,
                _ => 0,
            };
            (((i32::from(colors[0][c]) << 10) + advance) >> 10) as u8
        });
        paint(x, y, color, a);
    }
    Ok(crate::ImageSurface { width: side, height: side, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painted_cell_blends_shadow_and_face_and_advances_signed_gradient() -> anyhow::Result<()> {
        let cell = paint_cell(&[255, 0, 0, 128], 2, [[255, 0, 0], [0, 0, 255]], 2, Some([0, 255, 0]))?;
        assert_eq!(cell.rgba[..4], [254, 0, 0, 255]);
        assert_eq!(cell.rgba[16..20], [63, 127, 63, 255]);
        assert_eq!(cell.rgba[32..36], [0, 127, 0, 128]);
        Ok(())
    }

    #[test]
    fn cuts_cells_in_row_order() {
        let sheet: Vec<u8> = (0..32).collect();
        assert_eq!(cut(&sheet, 8, 2, 1), Some(vec![2, 3, 10, 11]));
        assert_eq!(cut(&sheet, 8, 2, 5), Some(vec![18, 19, 26, 27]));
        assert_eq!(cut(&sheet, 8, 2, 16), None);
    }

    #[test]
    fn halving_keeps_flat_cells_flat_and_samples_the_top_left() {
        assert_eq!(resample(&[200; 16], 4, 2), vec![200; 4]);
        let ramp = [0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255];
        assert_eq!(resample(&ramp, 4, 2), vec![0, 255, 0, 255]);
        // A quarter step mixes 256/1024 of the right tap.
        assert_eq!(resample(&[0, 255, 0, 255], 2, 4)[..4], [0, 127, 255, 255]);
    }
}
