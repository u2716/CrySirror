// SPDX-License-Identifier: GPL-3.0-or-later
//! Native PB2A memory images. The CMVS dispatcher selects frame zero and
//! supplies no variant name for memory inputs. See the format SATD record for
//! the named variant and additional frame entry points.

use super::common::{bgr, image, purple_lz, Bits, C};
use anyhow::{bail, ensure, Context, Result};
use crysirror_image::ImageSurface;

const HEADER_LEN: usize = 32;
const FOOTER_LEN: usize = 27;
const RGB_CHANNELS: [usize; 4] = [2, 1, 0, 3];

struct Header {
    frames: u32,
    mode: u16,
    width: u32,
    height: u32,
    depth: u16,
    control: usize,
    data: usize,
}

impl Header {
    fn read(input: &[u8]) -> Result<(Self, &[u8])> {
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
        header.skip(4)?; // Offset of the additional frame table.
        let mode = header.u16()?;
        let width = u32::from(header.u16()?);
        let height = u32::from(header.u16()?);
        let depth = header.u16()?;
        let control = header.u32()? as usize;
        let data = header.u32()? as usize;
        Ok((
            Self {
                frames,
                mode,
                width,
                height,
                depth,
                control,
                data,
            },
            body,
        ))
    }
}

pub(crate) fn decode(input: &[u8]) -> Result<ImageSurface> {
    decode_with_alpha(input).map(|(image, _)| image)
}

pub(crate) fn decode_with_alpha(input: &[u8]) -> Result<(ImageSurface, bool)> {
    let (header, body) = Header::read(input)?;
    let image = match header.mode {
        1 | 2 => decode_tiles(&header, body),
        3 | 4 => decode_jbp(&header, body),
        5 => decode_first_frame(&header, body),
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
