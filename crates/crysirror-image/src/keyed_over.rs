// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Exact integer "source over" stamping between colour-keyed and
//! straight-alpha stored RGBA, writing into the destination in place.
//!
//! A keyed surface is opaque except texels whose RGB equals its key; its
//! alpha byte is not read and stays as stored. With `t = 255 - alpha`
//! (transparency) for alpha surfaces, a source texel with `t == 255` is
//! skipped and one with `t == 0` is copied. Otherwise, over an alpha
//! destination: if `dt == 255` the texel is copied; if `dt == 0` each RGB is
//! `floor(((255-st)*s + st*d) / 255)`; else the transparency becomes
//! `floor(st*dt/255)` and each RGB `floor((255*(255-st)*s + st*(255-dt)*d) /
//! (65025 - st*dt))`. Over a keyed destination, a key texel takes the source
//! RGB, otherwise the `dt == 0` form applies. A keyed source over an alpha
//! destination writes alpha 255. Behaviour reference: D.C.II P.C. 40BB40.
use crate::ImageSurface;
use anyhow::{ensure, Result};

/// Stamp `source` onto `destination` with its top-left at `origin`.
/// `*_key` is `Some(rgb)` for keyed surfaces and `None` for straight alpha.
pub fn keyed_over(
    destination: &mut ImageSurface,
    destination_key: Option<[u8; 3]>,
    source: &ImageSurface,
    source_key: Option<[u8; 3]>,
    origin: [i32; 2],
) -> Result<()> {
    destination.validate()?;
    source.validate()?;
    let [ox, oy] = origin.map(i64::from);
    let (dw, dh) = (i64::from(destination.width), i64::from(destination.height));
    let (sw, sh) = (i64::from(source.width), i64::from(source.height));
    let (x0, y0) = (ox.max(0), oy.max(0));
    let (x1, y1) = ((ox + sw).min(dw), (oy + sh).min(dh));
    ensure!(sw > 0 && sh > 0, "keyed over source extent");
    for y in y0..y1 {
        for x in x0..x1 {
            let s_at = (((y - oy) * sw + (x - ox)) * 4) as usize;
            let d_at = ((y * dw + x) * 4) as usize;
            let s: [u8; 4] = source.rgba[s_at..s_at + 4].try_into()?;
            let d = &mut destination.rgba[d_at..d_at + 4];
            let st = match source_key {
                Some(key) if s[..3] == key => continue,
                Some(_) => 0u32,
                None => 255 - u32::from(s[3]),
            };
            if st == 255 {
                continue;
            }
            let mix = |s: u8, d: u8| (((255 - st) * u32::from(s) + st * u32::from(d)) / 255) as u8;
            match destination_key {
                Some(key) => {
                    if st == 0 || d[..3] == key {
                        d[..3].copy_from_slice(&s[..3]);
                    } else {
                        for c in 0..3 {
                            d[c] = mix(s[c], d[c]);
                        }
                    }
                }
                None => {
                    let dt = 255 - u32::from(d[3]);
                    if st == 0 || dt == 255 {
                        d[..3].copy_from_slice(&s[..3]);
                        d[3] = (255 - st) as u8;
                    } else if dt == 0 {
                        for c in 0..3 {
                            d[c] = mix(s[c], d[c]);
                        }
                    } else {
                        let divisor = 65025 - st * dt;
                        for c in 0..3 {
                            d[c] = ((255 * (255 - st) * u32::from(s[c])
                                + st * (255 - dt) * u32::from(d[c]))
                                / divisor) as u8;
                        }
                        d[3] = (255 - st * dt / 255) as u8;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn one(rgba: [u8; 4]) -> ImageSurface {
        ImageSurface { width: 1, height: 1, rgba: rgba.to_vec() }
    }
    #[test]
    fn keyed_source_skips_key_and_copies_opaque() {
        let mut d = one([1, 2, 3, 0]);
        keyed_over(&mut d, None, &one([255, 0, 255, 255]), Some([255, 0, 255]), [0, 0]).unwrap();
        assert_eq!(d.rgba, [1, 2, 3, 0]);
        keyed_over(&mut d, None, &one([9, 8, 7, 0]), Some([255, 0, 255]), [0, 0]).unwrap();
        assert_eq!(d.rgba, [9, 8, 7, 255]);
    }
    #[test]
    fn partial_over_partial_uses_exact_division() {
        // st = 127, dt = 127: transparency 63; rgb (255*128*200 + 127*128*100) / 48896.
        let mut d = one([100, 100, 100, 128]);
        keyed_over(&mut d, None, &one([200, 200, 200, 128]), None, [0, 0]).unwrap();
        assert_eq!(d.rgba, [((255 * 128 * 200 + 127 * 128 * 100) / 48896) as u8; 3]
            .iter().copied().chain([192]).collect::<Vec<_>>());
    }
    #[test]
    fn keyed_destination_takes_source_on_key() {
        let mut d = one([255, 0, 255, 255]);
        keyed_over(&mut d, Some([255, 0, 255]), &one([10, 20, 30, 128]), None, [0, 0]).unwrap();
        assert_eq!(d.rgba, [10, 20, 30, 255]);
    }
}
