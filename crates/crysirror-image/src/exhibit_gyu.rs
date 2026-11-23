// SPDX-License-Identifier: MPL-2.0
//! ExHibit GYU stored and LZSS bitmap profiles. See docs/validation/exhibit-gyu.md.
use crate::{ImageDecodeContext, ImageFormat, ImageSurface, rgba_len};
use anyhow::{Context, Result, ensure};

pub(super) const ID: &str = "exhibit/gyu";
pub(super) struct Format;

impl ImageFormat for Format {
    fn id(&self) -> &'static str {
        ID
    }

    fn recognize(&self, _: &str, bytes: &[u8]) -> bool {
        bytes.starts_with(b"GYU\x1a")
    }

    fn decode(&self, bytes: &[u8], context: &ImageDecodeContext<'_>) -> Result<ImageSurface> {
        context.check_cancelled()?;
        let (header, mut payload) = bytes.split_at_checked(36).context("truncated GYU header")?;
        let field = |offset| {
            u32::from_le_bytes(
                header[offset..offset + 4]
                    .try_into()
                    .expect("fixed GYU field"),
            )
        };
        let flags = u16::from_le_bytes([header[4], header[5]]);
        let mode = u16::from_le_bytes([header[6], header[7]]);
        ensure!(
            matches!(mode, 0x0100 | 0x0200 | 0x0400 | 0x0800),
            "unsupported GYU compression mode {mode:#06x}"
        );
        let seed = field(8);
        ensure!(seed != 0, "external keyed GYU image requires a seed table");
        let depth = field(12);
        ensure!(
            matches!(depth, 8 | 24 | 32),
            "unsupported GYU depth {depth}"
        );
        let (width, height) = (field(16), field(20));
        let rgba_size = rgba_len(width, height)?;
        let (w, h) = (width as usize, height as usize);
        let pixel_bytes = depth as usize / 8;
        let color_stride = (w * pixel_bytes).next_multiple_of(4);
        let alpha_stride = w.next_multiple_of(4);
        let color_size = field(24) as usize;
        let alpha_size = field(28) as usize;
        let palette_count = field(32) as usize;
        ensure!(
            color_size > 0 && (mode != 0x0100 || color_size == h * color_stride),
            "invalid GYU stored color length"
        );
        ensure!(
            depth != 32 || alpha_size == 0,
            "unsupported GYU 32-bit color with separate alpha"
        );
        ensure!(
            palette_count <= 256 && (depth != 8 || palette_count > 0),
            "invalid GYU palette size"
        );

        let mut take = |len| -> Result<&[u8]> {
            let (plane, rest) = payload
                .split_at_checked(len)
                .context("truncated GYU payload")?;
            payload = rest;
            Ok(plane)
        };
        let palette = take(palette_count * 4)?;
        let mut colors = std::borrow::Cow::Borrowed(take(color_size)?);
        if seed != u32::MAX {
            let mut mt = crysirror_resource::legacy_stream::Mt19937Split::new(seed);
            let data = colors.to_mut();
            for _ in 0..10 {
                let a = mt.next_u32() as usize % data.len();
                let b = mt.next_u32() as usize % data.len();
                data.swap(a, b);
            }
        }
        if mode == 0x0800 {
            colors = expand_gyu(&colors, h * color_stride)?.into();
        } else if matches!(mode, 0x0200 | 0x0400) {
            colors =
                crysirror_resource::lzss::decompress_exact_12_4(&colors, h * color_stride)?.into();
        }
        let mut alpha = std::borrow::Cow::Borrowed(take(alpha_size)?);
        if alpha_size != 0 && alpha_size != h * alpha_stride {
            alpha = crysirror_resource::lzss::decompress_exact_12_4(&alpha, h * alpha_stride)
                .context("invalid GYU compressed alpha")?
                .into();
        }
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(rgba_size)?;
        for y in (0..h).rev() {
            context.check_cancelled()?;
            let row = &colors[y * color_stride..][..w * pixel_bytes];
            for (x, color) in row.chunks_exact(pixel_bytes).enumerate() {
                let bgr = if depth == 8 {
                    let index = usize::from(color[0]) * 4;
                    palette
                        .get(index..index + 3)
                        .context("GYU palette index out of bounds")?
                } else {
                    color
                };
                let opacity = if alpha.is_empty() {
                    255
                } else {
                    let sample = alpha[y * alpha_stride + x];
                    if flags == 3 {
                        sample
                    } else {
                        sample.saturating_mul(16)
                    }
                };
                rgba.extend_from_slice(&[bgr[2], bgr[1], bgr[0], opacity]);
            }
        }
        Ok(ImageSurface {
            width,
            height,
            rgba,
        })
    }
}

// Mode 0x0800 interleaves MSB-first control bits and raw token bytes on one
// input cursor. References are relative to the expanded color plane.
struct GyuBits<'a> {
    bytes: std::slice::Iter<'a, u8>,
    control: u8,
    mask: u8,
}
impl GyuBits<'_> {
    fn byte(&mut self) -> Result<u8> {
        self.bytes
            .next()
            .copied()
            .context("truncated GYU color token")
    }
    fn bit(&mut self) -> Result<u8> {
        if self.mask == 0 {
            self.control = self.byte()?;
            self.mask = 0x80;
        }
        let value = u8::from(self.control & self.mask != 0);
        self.mask >>= 1;
        Ok(value)
    }
}

fn expand_gyu(packed: &[u8], size: usize) -> Result<Vec<u8>> {
    let tokens = packed.get(4..).context("short GYU compressed plane")?;
    let mut src = GyuBits {
        bytes: tokens.iter(),
        control: 0,
        mask: 0,
    };
    let mut output = Vec::new();
    output.try_reserve_exact(size)?;
    output.push(src.byte()?);
    while output.len() < size {
        if src.bit()? != 0 {
            output.push(src.byte()?);
            continue;
        }
        let (distance, length) = if src.bit()? == 0 {
            let count = (src.bit()? << 1) | src.bit()?;
            (256 - usize::from(src.byte()?), usize::from(count) + 2)
        } else {
            let token = usize::from(u16::from_be_bytes([src.byte()?, src.byte()?]));
            let length = match token & 7 {
                0 => {
                    let extra = src.byte()?;
                    ensure!(extra != 0, "early end of GYU color plane");
                    usize::from(extra) + 1
                }
                n => n + 2,
            };
            (8192 - (token >> 3), length)
        };
        ensure!(
            distance <= output.len() && length <= size - output.len(),
            "invalid GYU color reference"
        );
        for _ in 0..length {
            output.push(output[output.len() - distance]);
        }
    }
    Ok(output)
}
