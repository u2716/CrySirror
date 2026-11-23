//! Alpha-border nine-patch parsing and stored RGBA resampling.
//! The first fully opaque run in the top/left border selects stretch cells;
//! bottom/right runs select content margins. Borders are not painted.
use crate::ImageSurface;
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug)]
pub struct NinePatchLayout {
    pub stretch: [u32; 4],
    pub margins: [i32; 4],
}
pub fn layout(
    size: [u32; 2],
    mut alpha: impl FnMut(u32, u32) -> Result<u8>,
) -> Result<Option<NinePatchLayout>> {
    let [w, h] = size;
    if w < 11 || h < 11 {
        return Ok(None);
    }
    fn run(len: u32, mut at: impl FnMut(u32) -> Result<u8>) -> Result<Option<[u32; 2]>> {
        let mut start = None;
        for p in 1..len - 1 {
            let a = at(p)?;
            if start.is_none() && a == 255 {
                start = Some(p);
            } else if let Some(start) = start {
                if a == 0 {
                    return Ok(Some([start, p]));
                }
            }
        }
        Ok(None)
    }
    let Some(x) = run(w, |x| alpha(x, 0))? else {
        return Ok(None);
    };
    let Some(y) = run(h, |y| alpha(0, y))? else {
        return Ok(None);
    };
    let mx = run(w, |x| alpha(x, h - 1))?;
    let my = run(h, |y| alpha(w - 1, y))?;
    let margins = [
        mx.map_or(-1, |r| r[0] as i32 - 1),
        my.map_or(-1, |r| r[0] as i32 - 1),
        mx.map_or(-1, |r| (w - r[1] - 1) as i32),
        my.map_or(-1, |r| (h - r[1] - 1) as i32),
    ];
    Ok(Some(NinePatchLayout {
        stretch: [x[0], y[0], x[1], y[1]],
        margins,
    }))
}
pub fn resize(
    source: &ImageSurface,
    size: [u32; 2],
    check: &dyn Fn() -> Result<()>,
) -> Result<ImageSurface> {
    source.validate()?;
    let [dw, dh] = size;
    let (sw, sh) = (source.width, source.height);
    ensure!(
        dw >= sw.saturating_sub(2) && dh >= sh.saturating_sub(2),
        "nine-patch destination smaller than interior"
    );
    let p = layout([sw, sh], |x, y| {
        Ok(source.rgba[((y * sw + x) * 4 + 3) as usize])
    })?
    .ok_or_else(|| anyhow::anyhow!("nine-patch stretch border missing"))?;
    let [left, top, right, bottom] = p.stretch;
    let axis = |v: u32, d: u32, s: u32, a: u32, b: u32| -> (u32, u32, u32) {
        let before = a - 1;
        let after = s - 1 - b;
        if v < before {
            return (v + 1, v + 1, 0);
        }
        if v >= d - after {
            return (b + v - (d - after), b + v - (d - after), 0);
        }
        let fixed =
            ((u64::from(v - before) * u64::from(b - a)) << 16) / u64::from(d - before - after);
        let x = a + (fixed >> 16) as u32;
        (x, (x + 1).min(s - 2), ((fixed & 65535) >> 8) as u32)
    };
    let length = crate::rgba_len(dw, dh)?;
    ensure!(
        u64::from(dw) * u64::from(dh) <= crate::effects::MAX_EFFECT_PIXELS,
        "nine-patch pixel budget"
    );
    let mut out = ImageSurface {
        width: dw,
        height: dh,
        rgba: vec![0; length],
    };
    for y in 0..dh {
        check()?;
        let (y0, y1, fy) = axis(y, dh, sh, top, bottom);
        for x in 0..dw {
            let (x0, x1, fx) = axis(x, dw, sw, left, right);
            for c in 0..4 {
                let get = |x, y| i32::from(source.rgba[((y * sw + x) * 4 + c) as usize]);
                let a = get(x0, y0) + ((get(x1, y0) - get(x0, y0)) * fx as i32 >> 8);
                let b = get(x0, y1) + ((get(x1, y1) - get(x0, y1)) * fx as i32 >> 8);
                out.rgba[((y * dw + x) * 4 + c) as usize] =
                    (a + ((b - a) * fy as i32 >> 8)).clamp(0, 255) as u8;
            }
        }
    }
    Ok(out)
}
