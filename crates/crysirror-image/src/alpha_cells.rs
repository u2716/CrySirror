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

#[cfg(test)]
mod tests {
    use super::*;

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
