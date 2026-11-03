// Copyright (C) 2016 rr-; Copyright (c) 2026 CrySirror contributors.
// GPL-3.0-or-later. Control-plane codec derived from pinned arc_unpacker.
//! HG3 frame catalog and lazy pixels. Native IDs, origins and centers remain
//! metadata; shared Image owns bitmap copy/PNG/JPEG operations. No VM required.
use super::*;
use std::ops::Range;
#[derive(Clone, Debug)]
struct Tag {
    name: [u8; 8],
    payload: Range<usize>,
}
fn tags(length: usize, mut header: impl FnMut(usize) -> Result<[u8; 16]>) -> Result<Vec<Tag>> {
    let mut result = Vec::new();
    let mut at = 0usize;
    loop {
        ensure!(
            result.len() < 65536 && at.checked_add(16).is_some_and(|end| end <= length),
            "HG3 tag extent/count"
        );
        let h = header(at)?;
        let next = le32(&h, 8)? as usize;
        let size = le32(&h, 12)? as usize;
        let start = at + 16;
        let end = start.checked_add(size).context("HG3 tag overflow")?;
        ensure!(end <= length, "HG3 tag payload outside frame");
        result.push(Tag {
            name: h[..8].try_into().unwrap(),
            payload: start..end,
        });
        if next == 0 {
            break;
        }
        ensure!(next >= 16 + size, "HG3 tag link overlaps current payload");
        at = at.checked_add(next).context("HG3 tag link overflow")?;
    }
    Ok(result)
}
#[derive(Clone, Debug)]
struct Info {
    size: [u32; 2],
    canvas: [u32; 2],
    origin: [i32; 2],
    depth: u32,
    flags: Option<u32>,
    base: Option<[i32; 2]>,
}
impl Info {
    fn parse(data: &[u8]) -> Result<Self> {
        ensure!(data.len() >= 28, "HG3 stdinfo extent");
        let info = Self {
            size: [le32(data, 0)?, le32(data, 4)?],
            depth: le32(data, 8)?,
            origin: [le32(data, 12)? as i32, le32(data, 16)? as i32],
            canvas: [le32(data, 20)?, le32(data, 24)?],
            flags: if data.len() >= 32 {
                Some(le32(data, 28)?)
            } else {
                None
            },
            base: if data.len() >= 40 {
                Some([le32(data, 32)? as i32, le32(data, 36)? as i32])
            } else {
                None
            },
        };
        rgba_len(info.size[0], info.size[1])?;
        rgba_len(info.canvas[0], info.canvas[1])?;
        Ok(info)
    }
}
fn one<'a>(tags: &'a [Tag], name: &[u8; 8]) -> Result<Option<&'a Tag>> {
    let mut found = tags.iter().filter(|tag| &tag.name == name);
    let result = found.next();
    ensure!(found.next().is_none(), "HG3 duplicate singleton tag");
    Ok(result)
}
pub(super) fn open(source: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    ensure!(magic(source, 0, b"HG-3")?, "missing HG-3");
    let length = length(source)?;
    ensure!(length >= 20, "HG3 file header/frame extent");
    let expand = if let Some(options) = cx.parameters.get("cat-system/hg3") {
        ensure!(options.is_object(), "HG3 options must be an object");
        options
            .get("expand_canvas")
            .map(|value| value.as_bool().context("HG3 expand_canvas must be boolean"))
            .transpose()?
            .unwrap_or(true)
    } else {
        true
    };
    let native = match cx
        .parameters
        .get("cat-system/hg3")
        .and_then(|v| v.get("pixel_profile"))
    {
        None => false,
        Some(v) => match v.as_str() {
            Some("archive") => false,
            Some("native") => true,
            _ => bail!("HG3 pixel_profile must be archive or native"),
        },
    };
    let mut at = 12u64;
    let mut rows = Vec::new();
    while at < length {
        cx.check_cancelled()?;
        limits(cx, rows.len() + 1)?;
        let frame = bytes(source, at, 8, 8)?;
        let next = le32(&frame, 0)? as u64;
        let frame_id = le32(&frame, 4)?;
        let stored = if next == 0 {
            length - at - 8
        } else {
            ensure!(next >= 24, "HG3 frame link extent");
            next - 8
        };
        range(length, at + 8, stored)?;
        let n = usize::try_from(stored).context("HG3 frame address extent")?;
        let frame_tags = tags(n, |offset| {
            cx.check_cancelled()?;
            Ok(bytes(source, at + 8 + offset as u64, 16, 16)?
                .try_into()
                .unwrap())
        })?;
        let standard = one(&frame_tags, b"stdinfo\0")?.context("HG3 stdinfo missing")?;
        let info = Info::parse(&bytes(
            source,
            at + 8 + standard.payload.start as u64,
            standard.payload.len().min(40) as u64,
            40,
        )?)?;
        if native {
            ensure!(
                info.flags.is_some() && info.base.is_some(),
                "HG3 native profile requires complete 40-byte stdinfo"
            );
        }
        let small = |tag: &Tag, count: usize| -> Result<Vec<u8>> {
            ensure!(tag.payload.len() >= count, "HG3 metadata tag extent");
            bytes(
                source,
                at + 8 + tag.payload.start as u64,
                count as u64,
                count as u64,
            )
        };
        let mode = one(&frame_tags, b"imgmode\0")?
            .map(|tag| small(tag, 4).and_then(|b| le32(&b, 0)))
            .transpose()?;
        let compression_type = one(&frame_tags, b"cptype\0\0")?
            .map(|tag| small(tag, 4).and_then(|b| le32(&b, 0)))
            .transpose()?;
        let mut attributes = Vec::new();
        for tag in &frame_tags {
            let name = zstr(&tag.name);
            if name.len() >= 7
                && name.starts_with(b"ats")
                && name[3..].iter().all(u8::is_ascii_digit)
            {
                let id = std::str::from_utf8(&name[3..])?.parse::<u32>()?;
                let data = small(tag, 20)?;
                attributes.push(serde_json::json!({"id":id,
                    "rect":[le32(&data,0)? as i32,le32(&data,4)? as i32,le32(&data,8)? as i32,le32(&data,12)? as i32],
                    "color":le32(&data,16)?}));
            }
        }
        let decoded_size = if expand { info.canvas } else { info.size };
        let output = output_size(decoded_size[0], decoded_size[1], cx)?;
        let metadata = serde_json::json!({"hg3":{
            "ordinal":rows.len(),"frame_id":frame_id,"size":info.size,"canvas":info.canvas,
            "origin":info.origin,"base":info.base,"color_bits":info.depth,"flags":info.flags,
            "decoded_size":decoded_size,"expanded":expand,
            "pixel_profile":if native{"native"}else{"archive"},
            "image_mode":mode,"compression_type":compression_type,"attributes":attributes,
            "tags":frame_tags.iter().map(|tag|String::from_utf8_lossy(zstr(&tag.name)).into_owned()).collect::<Vec<_>>()}});
        let bound = cx.limits.max_entry_size;
        rows.push(
            Row::packed(
                format!("{:03}.png", rows.len()),
                at + 8,
                stored,
                output,
                decode_context(move |data, _, cx| {
                    decode_frame(data, &info, bound, expand, native, mode, cx)
                }),
            )
            .meta(metadata),
        );
        at = at
            .checked_add(8 + stored)
            .context("HG3 next frame overflow")?;
    }
    Ok(rows)
}
fn decode_frame(
    data: &[u8],
    info: &Info,
    bound: u64,
    expand: bool,
    native: bool,
    mode: Option<u32>,
    cx: &DecodeContext,
) -> Result<Vec<u8>> {
    cx.check_cancelled()?;
    let bound = bound.min(cx.limits.max_entry_size);
    let mut work = cx.limits.max_resident_bytes;
    let chunks = tags(data.len(), |at| {
        cx.check_cancelled()?;
        Ok(data[at..at + 16].try_into().unwrap())
    })?;
    let mut controlled = Vec::new();
    for tag in &chunks {
        if tag.name.starts_with(b"img") && tag.name[3..7].iter().all(u8::is_ascii_digit) {
            ensure!(tag.name[7] == 0, "HG3 control tag ordinal layout");
            let ordinal = std::str::from_utf8(&tag.name[3..7])?.parse::<u32>()?;
            controlled.push((ordinal, tag));
        }
    }
    controlled.sort_by_key(|(ordinal, _)| *ordinal);
    for (expected, (ordinal, _)) in controlled.iter().enumerate() {
        ensure!(
            *ordinal as usize == expected,
            "HG3 control segment ordinal gap/duplicate"
        );
    }
    let jpeg = one(&chunks, b"img_jpg\0")?;
    let webp = one(&chunks, b"img_wbp\0")?;
    let alpha = one(&chunks, b"img_al\0\0")?;
    ensure!(
        one(&chunks, b"img_jpa\0")?.is_none(),
        "HG3 img_jpa color profile unsupported"
    );
    ensure!(
        usize::from(!controlled.is_empty())
            + usize::from(jpeg.is_some())
            + usize::from(webp.is_some())
            <= 1,
        "HG3 mixed color payloads"
    );
    let mut image = if !controlled.is_empty() {
        ensure!(
            alpha.is_none(),
            "HG3 control-plane with separate alpha profile unsupported"
        );
        if native {
            let mut output = ImageSurface::filled(info.size[0], info.size[1], [0; 4])?;
            let mut covered = vec![false; info.size[1] as usize];
            for (_, tag) in controlled {
                cx.check_cancelled()?;
                let (start, part) =
                    control(&data[tag.payload.clone()], info, bound, true, cx, &mut work)?;
                let y = info.size[1] - start - part.height;
                //Original6A1340 reads segments in ordinal order. Overlap is
                //defined: later decoded rows overwrite earlier ones. Gaps
                //would expose native uninitialized memory and are rejected.
                blit(&mut output, &part, 0, y as i32)?;
                covered[y as usize..(y + part.height) as usize].fill(true);
            }
            ensure!(
                covered.into_iter().all(|v| v),
                "HG3 native control segments leave undefined rows"
            );
            output
        } else {
            ensure!(
                controlled.len() == 1,
                "HG3 segmented controls require native pixel_profile"
            );
            control(
                &data[controlled[0].1.payload.clone()],
                info,
                bound,
                false,
                cx,
                &mut work,
            )?
            .1
        }
    } else if let Some(tag) = jpeg {
        ensure!(
            matches!(info.depth, 24 | 32),
            "HG3 JPEG color depth profile"
        );
        let jpeg = &data[tag.payload.clone()];
        ensure!(
            jpeg.starts_with(&[0xff, 0xd8]),
            "HG3 img_jpg JPEG signature"
        );
        let mut image = crysirror_image::decode_standard(jpeg)?;
        ensure!(
            [image.width, image.height] == info.size,
            "HG3 JPEG/stdinfo dimensions disagree"
        );
        if let Some(tag) = alpha.filter(|_| !native || info.depth == 32) {
            let mut c = Cursor::new(&data[tag.payload.clone()]);
            let packed = c.le32()? as usize;
            let size = c.le32()? as u64;
            ensure!(
                size == u64::from(info.size[0]) * u64::from(info.size[1]) && size <= bound,
                "HG3 alpha plane extent"
            );
            let alpha = inflate_index(c.take(packed)?, size)?;
            ensure!(
                alpha.len() as u64 == size && c.left() == 0,
                "HG3 alpha length mismatch"
            );
            for (pixel, alpha) in image.rgba.chunks_exact_mut(4).zip(alpha) {
                pixel[3] = alpha;
            }
        }
        image
    } else if let Some(tag) = webp {
        ensure!(info.depth == 32, "HG3 WebP native32-bit surface profile");
        let payload = &data[tag.payload.clone()];
        ensure!(
            payload.starts_with(b"RIFF") && payload.get(8..12) == Some(b"WEBP"),
            "HG3 WebP signature"
        );
        let image = crysirror_image::decode_standard(payload)?;
        ensure!(
            [image.width, image.height] == info.size,
            "HG3 WebP/stdinfo dimensions disagree"
        );
        //6A1340's WebP branch skips img_al: embedded WebP alpha is retained.
        image
    } else {
        bail!("HG3 frame has no supported color data (alpha-only/img_jpa profile)");
    };
    if native {
        //6A2100 defaults JPEG to mode1; WebP/control stay0.68D9B0 swaps
        //native stored R/B only for mode1. Shared JPEG/WebP decoders already
        //produce canonical RGBA (native libraries use RGB/RGBA output), while
        //control data was normalized from its default BGR(A) byte profile.
        let decoded_rgb = jpeg.is_some() || webp.is_some();
        let mode = mode.unwrap_or(if jpeg.is_some() { 1 } else { 0 });
        let swap = if decoded_rgb { mode != 1 } else { mode == 1 };
        if swap {
            for pixel in image.rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
    }
    cx.check_cancelled()?;
    image.validate()?;
    if !expand {
        return encode_png_uncompressed(&image);
    }
    let mut destination = ImageSurface::filled(info.canvas[0], info.canvas[1], [0; 4])?;
    blit(&mut destination, &image, info.origin[0], info.origin[1])?;
    encode_png_uncompressed(&destination)
}
fn charge(work: &mut u64, bytes: u64) -> Result<()> {
    *work = work
        .checked_sub(bytes)
        .context("HG3 frame decode work byte limit")?;
    Ok(())
}
fn control(
    data: &[u8],
    info: &Info,
    bound: u64,
    native: bool,
    cx: &DecodeContext,
    work: &mut u64,
) -> Result<(u32, ImageSurface)> {
    ensure!(
        matches!(info.depth, 24 | 32),
        "HG3 control-plane color depth"
    );
    let [width, frame_height] = info.size;
    let channels = info.depth as usize / 8;
    let mut c = Cursor::new(data);
    let start = c.le32()?;
    let stored_height = c.le32()?;
    let height = if native {
        ensure!(
            stored_height > 0
                && start
                    .checked_add(stored_height)
                    .is_some_and(|end| end <= frame_height),
            "HG3 native control segment rows outside frame"
        );
        stored_height
    } else {
        ensure!(
            start == 0 && (stored_height == 0 || stored_height == frame_height),
            "HG3 archive control descriptor profile"
        );
        frame_height
    };
    let packed_stride = width as usize * channels;
    let stride = if native {
        packed_stride
    } else {
        (packed_stride + 3) & !3
    };
    let bytes = stride
        .checked_mul(height as usize)
        .context("HG3 control extent overflow")?;
    let expected = if native {
        bytes
            .checked_add(3)
            .context("HG3 control padding overflow")?
            & !3
    } else {
        bytes
    };
    ensure!(expected as u64 <= bound, "HG3 control output limit");
    let dc = c.le32()? as usize;
    let ds = c.le32()? as u64;
    let cc = c.le32()? as usize;
    let cs = c.le32()? as u64;
    ensure!(ds <= bound && cs <= bound, "HG3 inflated chunk limits");
    charge(work, ds.checked_add(cs).context("HG3 chunk work overflow")?)?;
    cx.check_cancelled()?;
    let d = inflate_index(c.take(dc)?, ds)?;
    cx.check_cancelled()?;
    let ctl = inflate_index(c.take(cc)?, cs)?;
    cx.check_cancelled()?;
    ensure!(
        d.len() as u64 == ds && ctl.len() as u64 == cs && c.left() == 0,
        "HG3 control chunk lengths"
    );
    let mut bits = Bits {
        b: &ctl,
        at: 0,
        lsb: true,
    };
    let mut copy = bits.bit()? != 0;
    let size = bits.gamma()?;
    ensure!(
        size % 4 == 0
            && if native {
                size >= bytes && size as u64 <= bound
            } else {
                size == expected
            },
        "HG3 image control output length"
    );
    charge(
        work,
        (size as u64)
            .checked_mul(2)
            .and_then(|v| v.checked_add(u64::from(width) * u64::from(height) * 4))
            .context("HG3 pixel work overflow")?,
    )?;
    let mut tmp = vec![0; size];
    let (mut out, mut source) = (0, 0);
    while out < size {
        cx.check_cancelled()?;
        let n = bits.gamma()?;
        ensure!(n <= size - out, "HG3 run outside output");
        if copy {
            let end = source + n;
            let part = d.get(source..end).context("HG3 literal outside input")?;
            tmp[out..out + n].copy_from_slice(part);
            source = end;
        }
        out += n;
        copy = !copy;
    }
    let mut pixels = vec![0; size];
    let plane = size / 4;
    for i in 0..plane {
        if i % 4096 == 0 {
            cx.check_cancelled()?;
        }
        let mut value = 0u32;
        for p in 0..4 {
            let v = tmp[p * plane + i] as u32;
            let mut spread = 0;
            for shift in [6, 4, 2, 0] {
                spread = (spread << 6) | (v & (3 << shift));
            }
            value |= spread << (6 - p * 2);
        }
        for j in 0..4 {
            let v = (value >> (j * 8)) as u8;
            pixels[i * 4 + j] = if v & 1 != 0 { (v >> 1) ^ 255 } else { v >> 1 };
        }
    }
    for x in channels..stride {
        pixels[x] = pixels[x].wrapping_add(pixels[x - channels]);
    }
    for y in 1..height as usize {
        cx.check_cancelled()?;
        for x in 0..stride {
            let i = y * stride + x;
            pixels[i] = pixels[i].wrapping_add(pixels[i - stride]);
        }
    }
    let mut rgba = Vec::with_capacity(rgba_len(width, height)?);
    for row in pixels.chunks_exact(stride).take(height as usize).rev() {
        cx.check_cancelled()?;
        for pixel in row[..width as usize * channels].chunks_exact(channels) {
            rgba.extend([
                pixel[2],
                pixel[1],
                pixel[0],
                if channels == 4 { pixel[3] } else { 255 },
            ]);
        }
    }
    Ok((
        start,
        ImageSurface {
            width,
            height,
            rgba,
        },
    ))
}
