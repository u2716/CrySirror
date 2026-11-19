// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from arc_unpacker PB3 image decoder, Copyright (C) 2016 rr-.
use super::common::*;
use anyhow::{ensure, Context, Result};
use crysirror_image::{ImageDecodeContext, ImageSurface};

const HEADER_LEN: usize = 0x34;
const TAIL_LEN: usize = 0x2f;
fn read32(b: &[u8], at: usize) -> Result<usize> {
    Ok(C::at(b, at)?.u32()? as usize)
}
fn slice(b: &[u8], at: usize, n: usize) -> Result<&[u8]> {
    C::at(b, at)?.take(n)
}
fn header(input: &[u8]) -> Result<[u8; 0x34]> {
    ensure!(
        input.len() >= HEADER_LEN + TAIL_LEN && input.starts_with(b"PB3B"),
        "PB3 header"
    );
    let mut b: [u8; 0x34] = input[..0x34].try_into().expect("validated PB3 header");
    let tail = input.len() - TAIL_LEN;
    for i in 0..0x2c {
        b[8 + i] = (b[8 + i] ^ input[tail + 0x2c + i % 2]).wrapping_sub(input[tail + i]);
    }
    Ok(b)
}

pub fn bit_depth(input: &[u8]) -> Result<u16> {
    let b = header(input)?;
    let depth = C::at(&b, 0x22)?.u16()?;
    ensure!(matches!(depth, 24 | 32), "PB3 unsupported depth");
    Ok(depth)
}

pub fn decode(input: &[u8], ctx: &ImageDecodeContext<'_>) -> Result<ImageSurface> {
    let header = header(input)?;
    let mut b = input.to_vec();
    b[..header.len()].copy_from_slice(&header);
    let mut c = C::at(&b, 0x18)?;
    let sub = c.u32()? as usize;
    let mode = c.u16()?;
    let (w, h, depth) = (u32::from(c.u16()?), u32::from(c.u16()?), c.u16()?);
    ensure!(matches!(depth, 24 | 32), "PB3 unsupported depth");
    let native_memory = ctx
        .parameter(crate::cmvs_image::PROFILE)
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    if mode == 7 {
        let body = &b[..b.len() - TAIL_LEN];
        let mut streams = C::at(body, HEADER_LEN)?;
        let control = (streams.u32()? as usize)
            .checked_add(60)
            .context("PB3 control offset")?;
        let data = (streams.u32()? as usize)
            .checked_add(60)
            .context("PB3 data offset")?;
        let control = body.get(control..).context("PB3 control outside input")?;
        let data = body.get(data..).context("PB3 data outside input")?;
        let mut rgba = purple_lz(control, data, crysirror_image::rgba_len(w, h)?, true)?;
        crysirror_image::swap_red_blue(&mut rgba)?;
        if !native_memory && depth == 24 {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
        }
        return Ok(ImageSurface {
            width: w,
            height: h,
            rgba,
        });
    }
    let ch = depth as usize / 8;
    let mut out = image(w, h)?;
    let (wi, hi) = (w as usize, h as usize);
    match mode {
        1 => {
            ensure!(sub != 0, "PB3 zero block size");
            let ma = read32(&b, 0x2c)?;
            let da = read32(&b, 0x30)?;
            let mut m = C::at(&b, ma)?;
            let mut d = C::at(&b, da)?;
            let mut ms = Vec::new();
            let mut ds = Vec::new();
            for _ in 0..ch {
                ms.push(m.u32()? as usize);
                ds.push(d.u32()? as usize);
            }
            let (mut ma, mut da) = (ma + 4 * ch, da + 4 * ch);
            for channel in 0..ch {
                let (main, data) = if native_memory && channel + 1 == ch {
                    let body = &b[..b.len() - TAIL_LEN];
                    (
                        body.get(ma..).context("PB3 final tile control offset")?,
                        body.get(da..).context("PB3 final tile data offset")?,
                    )
                } else {
                    (slice(&b, ma, ms[channel])?, slice(&b, da, ds[channel])?)
                };
                if channel + 1 < ch {
                    ma = ma
                        .checked_add(ms[channel])
                        .context("PB3 tile control offset overflow")?;
                    da = da
                        .checked_add(ds[channel])
                        .context("PB3 tile data offset overflow")?;
                }
                let mut c = C::new(main);
                let (a, z, n) = (c.u32()? as usize, c.u32()? as usize, c.u32()? as usize);
                ensure!(n <= wi * hi, "PB3 tile data exceeds plane size");
                let controls = c.take(a)?;
                let fills = c.take(z)?;
                let plane = purple_lz(c.take(c.left())?, data, n, native_memory)?;
                let mut ctl = Bits::new(controls, true);
                let mut fill = C::new(fills);
                let mut plane = C::new(&plane);
                for by in (0..hi).step_by(sub) {
                    for bx in (0..wi).step_by(sub) {
                        let fill = if ctl.get(1)? != 0 {
                            Some(fill.u8()?)
                        } else {
                            None
                        };
                        for y in by..by.saturating_add(sub).min(hi) {
                            for x in bx..bx.saturating_add(sub).min(wi) {
                                out.rgba[(y * wi + x) * 4 + [2, 1, 0, 3][channel]] = match fill {
                                    Some(v) => v,
                                    None => plane.u8()?,
                                };
                            }
                        }
                    }
                }
            }
            if ch == 3 {
                for p in out.rgba.chunks_exact_mut(4) {
                    p[3] = 255;
                }
            }
        }
        5 => {
            let body = if native_memory {
                &b[..b.len() - TAIL_LEN]
            } else {
                &b
            };
            let channels = if native_memory { 4 } else { ch };
            let mut c = C::at(body, 0x34)?;
            let mut controls = Vec::new();
            let mut datas = Vec::new();
            for _ in 0..channels {
                controls.push(
                    (c.u32()? as usize)
                        .checked_add(0x54)
                        .context("PB3 plane control offset")?,
                );
                datas.push(
                    (c.u32()? as usize)
                        .checked_add(0x54)
                        .context("PB3 plane data offset")?,
                );
            }
            for channel in 0..channels {
                let ca = controls[channel];
                let da = datas[channel];
                let ce = *controls
                    .get(channel + 1)
                    .filter(|_| !native_memory)
                    .unwrap_or(&body.len());
                let de = *datas
                    .get(channel + 1)
                    .filter(|_| !native_memory)
                    .unwrap_or(&body.len());
                ensure!(ce >= ca && de >= da, "PB3 nonmonotonic plane offsets");
                let plane = purple_lz(
                    slice(body, ca, ce - ca)?,
                    slice(body, da, de - da)?,
                    wi * hi,
                    native_memory,
                )?;
                let mut acc = 0u8;
                for (p, v) in out.rgba.chunks_exact_mut(4).zip(plane) {
                    acc = acc.wrapping_add(v);
                    p[[2, 1, 0, 3][channel]] = acc;
                }
            }
        }
        6 | 8 => {
            let native_base = native_memory
                || mode == 8
                || ctx
                    .parameter("purple-software/pb3-native-base")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            let key = [
                0xa6, 0x75, 0xf3, 0x9c, 0xc5, 0x69, 0x78, 0xa3, 0x3e, 0xa5, 0x4f, 0x79, 0x59, 0xfe,
                0x3a, 0xc7,
            ];
            let name: Vec<u8> = slice(&b, 0x34, 32)?
                .iter()
                .enumerate()
                .map(|(i, v)| v ^ key[i % 16])
                .take_while(|v| *v != 0)
                .collect();
            let stem = match std::str::from_utf8(&name) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    let (s, _, errors) = encoding_rs::SHIFT_JIS.decode(&name);
                    ensure!(!errors, "PB3 reference name encoding");
                    s.into_owned()
                }
            };
            let mut found = false;
            if native_base || !stem.is_empty() {
                let extensions: &[&str] = if native_base {
                    &["pb3"]
                } else {
                    &["pb3", "png"]
                };
                for ext in extensions {
                    let path = format!("{stem}.{ext}");
                    if let Some(data) = ctx.try_read_relative(&path)? {
                        let base = if native_memory && !data.starts_with(b"PB3B") {
                            crate::cmvs_image::decode_raster(&data, &|| ctx.check_cancelled())?.0
                        } else {
                            ctx.decode_embedded(&path, &data)?
                        };
                        found = true;
                        if native_base {
                            ensure!(
                                [base.width, base.height] == [w, h],
                                "PB3 base dimensions differ"
                            );
                            out = base;
                            break;
                        }
                        for y in 0..hi.min(base.height as usize) {
                            for x in 0..wi.min(base.width as usize) {
                                let i = (y * wi + x) * 4;
                                let j = (y * base.width as usize + x) * 4;
                                out.rgba[i..i + 4].copy_from_slice(&base.rgba[j..j + 4]);
                            }
                        }
                    }
                }
            }
            ensure!(!native_base || found, "PB3 native base is missing");
            let ca = read32(&b, 0xc)?
                .checked_add(0x20)
                .context("PB3 control offset")?;
            let cs = read32(&b, 0x2c)?;
            let da = ca.checked_add(cs).context("PB3 data offset")?;
            let proxy = if native_base {
                let body = &b[..b.len() - TAIL_LEN];
                let data = body.get(da..).context("PB3 overlay data outside input")?;
                purple_lz(slice(body, ca, cs)?, data, sub, true)?
            } else {
                purple_lz(
                    slice(&b, ca, cs)?,
                    slice(&b, da, read32(&b, 0x30)?)?,
                    sub,
                    false,
                )?
            };
            let mut c = C::new(&proxy);
            let (cs, ds) = (c.u32()? as usize, c.u32()? as usize);
            let mut ctl = Bits::new(c.take(cs)?, true);
            let mut data = C::new(if native_base {
                c.take(c.left())?
            } else {
                c.take(ds)?
            });
            for by in (0..hi).step_by(8) {
                for bx in (0..wi).step_by(8) {
                    if ctl.get(1)? == 0 {
                        for y in by..(by + 8).min(hi) {
                            for x in bx..(bx + 8).min(wi) {
                                let p = data.take(4)?;
                                out.rgba[(y * wi + x) * 4..][..4]
                                    .copy_from_slice(&[p[2], p[1], p[0], p[3]]);
                            }
                        }
                    }
                }
            }
        }
        4 => anyhow::bail!("PB3 type 4 requires a native variant name"),
        2 | 3 => {
            if native_memory {
                let body = &b[..b.len() - TAIL_LEN];
                let mut data = C::at(body, HEADER_LEN)?;
                let alpha = if depth == 32 {
                    let start = read32(body, 0x2c)?;
                    ensure!(start >= HEADER_LEN, "PB3 alpha offset before JBP1");
                    Some(
                        body.get(start..)
                            .context("PB3 alpha offset outside input")?,
                    )
                } else {
                    None
                };
                let length = alpha.map_or(data.left(), |alpha| data.left() - alpha.len());
                let raw = super::jbp1::decode(data.take(length)?, Some([w, h]))?;
                let mut image = bgr(w, h, &raw, 3, false)?;
                if let Some(alpha) = alpha {
                    super::jbp1::apply_native_alpha(&mut image, alpha)?;
                }
                return Ok(image);
            }
            let (data, mask) = if mode == 2 {
                let ma = read32(&b, 0x2c)?;
                let ms = read32(&b, 0x30)?;
                ensure!(ma >= 0x34, "PB3 JBP1 offset");
                (slice(&b, 0x34, ma - 0x34)?, Some(slice(&b, ma, ms)?))
            } else {
                (&b[0x34..], None)
            };
            let raw = super::jbp1::decode(data, None)?;
            out = bgr(w, h, &raw, 3, false)?;
            if let Some(mask) = mask {
                let mut m = C::new(mask);
                let mut decoded = Vec::new();
                while m.left() != 0 {
                    let b = m.u8()?;
                    let n = if b == 0 || b == 255 {
                        m.u8()? as usize
                    } else {
                        1
                    };
                    ensure!(decoded.len() + n <= wi * hi, "PB3 mask output overflow");
                    decoded.resize(decoded.len() + n, b);
                }
                ensure!(decoded.len() == wi * hi, "PB3 short mask");
                for (p, a) in out.rgba.chunks_exact_mut(4).zip(decoded) {
                    p[3] = a;
                }
            }
        }
        _ => anyhow::bail!("unsupported PB3 main/subtype {mode}/{sub}"),
    };
    Ok(out)
}
