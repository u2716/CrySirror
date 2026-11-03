//! Rectangle mapping around the stored-byte kernels, without copying the target.
use crate::ImageSurface;
use anyhow::{ensure, Result};
pub fn blit_stored_region(
    destination: &mut ImageSurface,
    df: u8,
    source: &ImageSurface,
    sf: u8,
    rect: [i32; 4],
    position: [i32; 2],
    mode: i32,
    parameter: i32,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    ensure!(df <= 3 && sf <= 3, "stored-byte bitmap format");
    ensure!(
        matches!(mode,0..=9|0x20..=0x27|0x40|0x41|0x80|0xc0|0xc1|0xf0|0xff)
            && (0..=256).contains(&parameter),
        "stored-byte blend mode/parameter"
    );
    ensure!(rect[2] >= 0 && rect[3] >= 0, "stored-byte source rectangle");
    let left = i64::from(rect[0]).max(0);
    let top = i64::from(rect[1]).max(0);
    let right = (i64::from(rect[0]) + i64::from(rect[2])).min(i64::from(source.width));
    let bottom = (i64::from(rect[1]) + i64::from(rect[3])).min(i64::from(source.height));
    if right <= left || bottom <= top {
        return Ok(());
    }
    let offset = [
        i64::from(position[0]) + left - i64::from(rect[0]),
        i64::from(position[1]) + top - i64::from(rect[1]),
    ];
    let [x, y] = [i32::try_from(offset[0])?, i32::try_from(offset[1])?];
    if i64::from(x) >= i64::from(destination.width)
        || i64::from(y) >= i64::from(destination.height)
        || i64::from(x) + right - left <= 0
        || i64::from(y) + bottom - top <= 0
    {
        return Ok(());
    }
    check()?;
    if rect == [0, 0, source.width as i32, source.height as i32] {
        ensure!(
            super::bitmap_blend::blit_mode(
                destination,
                i32::from(df),
                source,
                i32::from(sf),
                x,
                y,
                mode,
                parameter
            ),
            "stored-byte blend format pair"
        );
    } else {
        let width = (right - left) as u32;
        let height = (bottom - top) as u32;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(crate::rgba_len(width, height)?)?;
        for row in 0..height {
            check()?;
            let at = (((top as u32 + row) * source.width + left as u32) * 4) as usize;
            rgba.extend_from_slice(&source.rgba[at..at + width as usize * 4]);
        }
        let region = ImageSurface {
            width,
            height,
            rgba,
        };
        ensure!(
            super::bitmap_blend::blit_mode(
                destination,
                i32::from(df),
                &region,
                i32::from(sf),
                x,
                y,
                mode,
                parameter
            ),
            "stored-byte blend format pair"
        );
    }
    check()?;
    Ok(())
}
