// SPDX-License-Identifier: GPL-3.0-or-later
//! PB2A images. Mode 5 stores each numbered overlay relative to the same base.

use super::common::{bgr, image, purple_lz, Bits, C};
use anyhow::{bail, ensure, Context, Result};
use crysirror_image::ImageSurface;

const HEADER_LEN: usize = 32;
const FOOTER_LEN: usize = 27;
const RGB_CHANNELS: [usize; 4] = [2, 1, 0, 3];

struct Header {
    frames: u32,
    frame_table: usize,
    mode: u16,
    width: u32,
    height: u32,
    depth: u16,
    control: usize,
    data: usize,
}

impl Header {
    fn read(input: &[u8]) -> Result<Self> {
        ensure!(
            input.len() >= HEADER_LEN + FOOTER_LEN && input.starts_with(b"PB2A"),
            "invalid PB2A header"
        );
        let (body, footer) = input.split_at(input.len() - FOOTER_LEN);
        let mut decoded = [0; 24];
        for (index, byte) in decoded.iter_mut().enumerate() {
            *byte = (body[8 + index] ^ footer[24 + index % 2]).wrapping_sub(footer[index]);
        }
        let mut header = C::new(&decoded);
        let frames = header.u32()?;
        let frame_table = header.u32()? as usize;
        let mode = header.u16()?;
        let width = u32::from(header.u16()?);
        let height = u32::from(header.u16()?);
        let depth = header.u16()?;
        let control = header.u32()? as usize;
        let data = header.u32()? as usize;
        Ok(Self {
                frames,
                frame_table,
                mode,
                width,
                height,
                depth,
                control,
                data,
        })
    }
}

pub(crate) fn decode(input: &[u8], context: &crysirror_image::ImageDecodeContext<'_>) -> Result<ImageSurface> {
    let frame = context.parameter("purple-software/pb2-frame")
        .map(|value| value.as_u64().and_then(|v| u32::try_from(v).ok()).context("invalid PB2A frame index"))
        .transpose()?.unwrap_or(0);
    decode_frame(input, frame).map(|(image, _)| image)
}

pub(crate) fn decode_with_alpha(input: &[u8]) -> Result<(ImageSurface, bool)> {
    decode_frame(input, 0)
}

fn decode_frame(input: &[u8], frame: u32) -> Result<(ImageSurface, bool)> {
    let header = Header::read(input)?;
    let image = match header.mode {
        // CVNS 0x43d360, 0x43cd80 and 0x43d180 read their streams up to the end
        // of the file; the key bytes near the end can also be payload.
        1 | 2 => decode_tiles(&header, input),
        3 | 4 => decode_jbp(&header, input),
        5 => {
            ensure!(frame < header.frames, "PB2A frame {frame} outside {}", header.frames);
            let mut surface = decode_first_frame(&header, input)?;
            if frame != 0 {
                apply_frame(&header, input, frame, &mut surface)?;
            }
            Ok(surface)
        }
        6 => bail!("PB2A mode 6 requires a native variant name"),
        mode => bail!("unsupported PB2A mode {mode}"),
    }?;
    Ok((image, header.depth == 32 || header.mode == 5))
}

fn payload_at(bytes: &[u8], offset: usize) -> Result<&[u8]> {
    ensure!(offset >= HEADER_LEN, "PB2A offset inside header");
    bytes.get(offset..).context("PB2A offset outside payload")
}

fn decode_tiles(header: &Header, body: &[u8]) -> Result<ImageSurface> {
    let channels = match header.depth {
        24 => &RGB_CHANNELS[..3],
        32 => &RGB_CHANNELS[..],
        depth => bail!("unsupported PB2A depth {depth}"),
    };
    let mut surface = ImageSurface::filled(header.width, header.height, [0, 0, 0, 255])?;
    let pixels = surface.rgba.len() / 4;
    let mut control = C::new(payload_at(body, header.control)?);
    let mut data = C::new(payload_at(body, header.data)?);
    if header.mode == 1 {
        let planes = purple_lz(control.b, data.b, pixels * channels.len(), true)?;
        for (&channel, plane) in channels.iter().zip(planes.chunks_exact(pixels)) {
            unpack_tiles(&mut surface, channel, C::new(plane), None)?;
        }
        return Ok(surface);
    }

    let mut control_lengths = C::new(control.take(channels.len() * 4)?);
    let mut data_lengths = C::new(data.take(channels.len() * 4)?);
    for (index, &channel) in channels.iter().enumerate() {
        let last = index + 1 == channels.len();
        let control_size = if last {
            control.left()
        } else {
            control_lengths.u32()? as usize
        };
        let data_size = if last {
            data.left()
        } else {
            data_lengths.u32()? as usize
        };
        let mut stream = C::new(control.take(control_size)?);
        let controls = stream.u32()? as usize;
        let fills = stream.u32()? as usize;
        let expanded = stream.u32()? as usize;
        ensure!(expanded <= pixels, "PB2A tile data exceeds plane size");
        let mut filled_tiles = Bits::new(stream.take(controls)?, true);
        let mut values = C::new(stream.take(fills)?);
        let plane = purple_lz(
            stream.take(stream.left())?,
            data.take(data_size)?,
            expanded,
            true,
        )?;
        unpack_tiles(
            &mut surface,
            channel,
            C::new(&plane),
            Some((&mut filled_tiles, &mut values)),
        )?;
    }
    Ok(surface)
}

fn unpack_tiles(
    surface: &mut ImageSurface,
    channel: usize,
    mut literals: C<'_>,
    mut fills: Option<(&mut Bits<'_>, &mut C<'_>)>,
) -> Result<()> {
    let width = surface.width as usize;
    let height = surface.height as usize;
    for top in (0..height).step_by(8) {
        for left in (0..width).step_by(8) {
            let mut fill = None;
            if let Some((flags, values)) = &mut fills {
                if flags.get(1)? != 0 {
                    fill = Some(values.u8()?);
                }
            }
            for y in top..(top + 8).min(height) {
                for x in left..(left + 8).min(width) {
                    surface.rgba[(y * width + x) * 4 + channel] = match fill {
                        Some(value) => value,
                        None => literals.u8()?,
                    };
                }
            }
        }
    }
    ensure!(literals.left() == 0, "PB2A unused tile pixels");
    Ok(())
}

fn decode_jbp(header: &Header, body: &[u8]) -> Result<ImageSurface> {
    let (packed, alpha) = if header.depth == 32 {
        let alpha = payload_at(body, header.control)?;
        (&body[HEADER_LEN..header.control], Some(alpha))
    } else {
        (&body[HEADER_LEN..], None)
    };
    let raw = super::jbp1::decode(packed, Some([header.width, header.height]))?;
    let mut surface = bgr(header.width, header.height, &raw, 3, false)?;
    if let Some(alpha) = alpha {
        super::jbp1::apply_native_alpha(&mut surface, alpha)?;
    }
    Ok(surface)
}

fn decode_first_frame(header: &Header, body: &[u8]) -> Result<ImageSurface> {
    ensure!(header.frames > 0, "PB2A animation has no first frame");
    let mut offsets = C::at(body, HEADER_LEN)?;
    let mut surface = image(header.width, header.height)?;
    let pixels = surface.rgba.len() / 4;
    let planes_start = HEADER_LEN + 8 * RGB_CHANNELS.len();
    let planes = payload_at(body, planes_start)?;
    for channel in RGB_CHANNELS {
        let control = C::at(planes, offsets.u32()? as usize)?;
        let data = C::at(planes, offsets.u32()? as usize)?;
        let plane = purple_lz(&planes[control.p..], &planes[data.p..], pixels, true)?;
        for (pixel, value) in surface.rgba.chunks_exact_mut(4).zip(plane) {
            pixel[channel] = value;
        }
    }
    for rgba in surface.rgba.chunks_exact_mut(4) {
        rgba[0] ^= rgba[3];
        rgba[1] ^= rgba[0];
        rgba[2] ^= rgba[1];
    }
    Ok(surface)
}

/// 0x43c980. Frames after zero each replace a subset of complete 8x8 tiles.
/// A set mask bit preserves the base tile. Changed tiles contain opaque BGR.
fn apply_frame(header: &Header, bytes: &[u8], frame: u32, surface: &mut ImageSurface) -> Result<()> {
    let mut records = C::new(payload_at(bytes, header.frame_table)?);
    for _ in 1..frame {
        let length = records.u32()? as usize;
        records.skip(length.checked_sub(4).context("short PB2A frame record")?)?;
    }
    let length = records.u32()? as usize;
    let mut record = C::new(records.take(length.checked_sub(4).context("short PB2A frame record")?)?);
    let flags_len = record.u32()? as usize;
    record.skip(4)?;
    let unpacked = record.u32()? as usize;
    let control = record.take(flags_len)?;
    let data = purple_lz(control, record.take(record.left())?, unpacked, true)?;
    let mut delta = C::new(&data);
    let mask_len = delta.u32()? as usize;
    delta.skip(4)?;
    let mut mask = Bits::new(delta.take(mask_len)?, true);
    let width = surface.width as usize;
    for ty in 0..surface.height as usize / 8 {
        for tx in 0..width / 8 {
            if mask.get(1)? != 0 { continue; }
            for row in 0..8 {
                let start = 4 * ((ty * 8 + row) * width + tx * 8);
                for pixel in surface.rgba[start..start + 32].chunks_exact_mut(4) {
                    let bgr = delta.take(3)?;
                    pixel.copy_from_slice(&[bgr[2], bgr[1], bgr[0], 255]);
                }
            }
        }
    }
    Ok(())
}
