// SPDX-License-Identifier: MPL-2.0
//! Independent signed horizontal shift of every pixel row. Pixels shifted in
//! from outside the row are fully transparent, matching a strip drawn at an
//! offset over a clipped destination (N-System Wave 433E80).
use crate::ImageSurface;
use anyhow::{Result, ensure};

pub fn apply(
    input: &ImageSurface,
    offsets: &[i32],
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    input.validate()?;
    ensure!(
        offsets.len() == input.height as usize,
        "row shift needs one offset per row"
    );
    ensure!(
        u64::from(input.width) * u64::from(input.height) <= crate::effects::MAX_EFFECT_PIXELS,
        "row shift pixel budget"
    );
    let width = i64::from(input.width);
    let mut out = ImageSurface {
        width: input.width,
        height: input.height,
        rgba: vec![0; input.rgba.len()],
    };
    let row_bytes = input.width as usize * 4;
    for (y, &shift) in offsets.iter().enumerate() {
        if y % 64 == 0 {
            check()?;
        }
        let shift = i64::from(shift);
        // Destination x in [max(shift,0), min(width, width+shift)).
        let (first, last) = (shift.max(0), (width + shift).min(width));
        if first >= last {
            continue;
        }
        let count = (last - first) as usize * 4;
        let dst = y * row_bytes + first as usize * 4;
        let src = y * row_bytes + (first - shift) as usize * 4;
        out.rgba[dst..dst + count].copy_from_slice(&input.rgba[src..src + count]);
    }
    check()?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rows_shift_independently_with_transparent_fill() {
        let image = ImageSurface {
            width: 3,
            height: 2,
            rgba: (1..=6u8).flat_map(|v| [v, v, v, 255]).collect(),
        };
        let out = apply(&image, &[1, -2], &|| Ok(())).unwrap();
        assert_eq!(&out.rgba[..12], &[0, 0, 0, 0, 1, 1, 1, 255, 2, 2, 2, 255]);
        assert_eq!(&out.rgba[12..], &[6, 6, 6, 255, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
