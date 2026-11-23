use super::{PalBitmapFontMetricsFooter32, PalBitmapGlyphRecord32, PalFontProfile};
use crate::native_asset::Nls;
use std::sync::Arc;

const MAX_COVERAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) struct PalBitmapFont {
    bytes: Arc<[u8]>,
    offsets: Arc<[u32]>,
    nls: Nls,
    pub metrics: PalBitmapFontMetricsFooter32,
}

#[derive(Clone, Debug)]
pub(super) struct BitmapGlyph {
    pub metrics: PalBitmapGlyphRecord32,
    pub stride: u32,
    pub coverage: Vec<u8>,
}

impl PalBitmapFont {
    pub fn parse(bytes: Vec<u8>, nls: Nls) -> Result<Self, &'static str> {
        if bytes.len() < 0x84 + 56 {
            return Err("font data has no complete lookup table and metrics footer");
        }
        let footer = bytes.len() - 56;
        let first_offset = word(&bytes, 0x80) as usize;
        if first_offset < 0x84 || first_offset > footer || first_offset % 4 != 0 {
            return Err("font lookup table has an invalid size");
        }
        let offsets: Vec<_> = bytes[..first_offset]
            .chunks_exact(4)
            .map(|value| u32::from_le_bytes(value.try_into().unwrap()))
            .collect();
        for &offset in &offsets {
            if offset == 0 {
                continue;
            }
            let offset = offset as usize;
            if offset < first_offset || offset.checked_add(24).is_none_or(|end| end > footer) {
                return Err("font glyph header is outside the resource");
            }
            let metrics = read_glyph_metrics(&bytes[offset..offset + 24]);
            let length = metrics.bitmap_bytes as usize;
            if length > MAX_COVERAGE_BYTES
                || offset
                    .checked_add(24)
                    .and_then(|start| start.checked_add(length))
                    .is_none_or(|end| end > footer)
            {
                return Err("font glyph coverage is outside the resource");
            }
            if length != 0 {
                let stride = metrics
                    .bitmap_bytes
                    .checked_div(metrics.black_box_height)
                    .ok_or("font bitmap has zero height")?
                    & !3;
                if stride < metrics.black_box_width
                    || stride.checked_mul(metrics.black_box_height) != Some(metrics.bitmap_bytes)
                {
                    return Err("font bitmap has an invalid row stride");
                }
            }
        }
        let metrics = PalBitmapFontMetricsFooter32 {
            height: word(&bytes, footer) as i32,
            ascent: word(&bytes, footer + 4) as i32,
            remaining_text_metrics: bytes[footer + 8..].try_into().unwrap(),
        };
        if metrics.height <= 0 {
            return Err("font metrics footer has a nonpositive cell height");
        }
        Ok(Self {
            bytes: bytes.into(),
            offsets: offsets.into(),
            nls,
            metrics,
        })
    }

    fn offset(&self, ch: char) -> Option<usize> {
        let encoded = self.nls.encode(&ch.to_string()).ok()?;
        let slot = match encoded.as_slice() {
            [low] => usize::from(*low),
            [high, low] => usize::from(high.wrapping_add(0x80)) * 255 + usize::from(*low),
            _ => return None,
        };
        let offset = *self.offsets.get(slot)? as usize;
        // Native code blindly interprets offset zero as a record. The portable
        // resource boundary treats an absent record as missing instead.
        (offset != 0).then_some(offset)
    }

    pub fn supports(&self, text: &str) -> bool {
        text.chars()
            .all(|ch| matches!(ch, ' ' | '\u{3000}' | '\n' | '\r') || self.offset(ch).is_some())
    }

    pub fn height(&self, size: u16) -> i32 {
        (self.metrics.height as f32 * self.scale(size)) as i32
    }

    pub fn baseline(&self, size: u16) -> i32 {
        (self.metrics.ascent as f32 * self.scale(size)) as i32
    }

    fn scale(&self, size: u16) -> f32 {
        f32::from(size) / self.metrics.height as f32
    }

    pub fn glyph_metrics(
        &self,
        ch: char,
        size: u16,
        profile: PalFontProfile,
    ) -> Result<Option<PalBitmapGlyphRecord32>, &'static str> {
        let Some(offset) = self.offset(ch) else {
            return Ok(None);
        };
        let original = read_glyph_metrics(&self.bytes[offset..offset + 24]);
        self.scaled_metrics(original, size, profile).map(Some)
    }

    fn scaled_metrics(
        &self,
        mut metrics: PalBitmapGlyphRecord32,
        size: u16,
        profile: PalFontProfile,
    ) -> Result<PalBitmapGlyphRecord32, &'static str> {
        let scale = self.scale(size);
        if scale == 1.0 || (metrics.bitmap_bytes == 0 && profile != PalFontProfile::Koikake) {
            return Ok(metrics);
        }
        if metrics.bitmap_bytes == 0 || metrics.black_box_height == 0 {
            return Err("native bitmap rescale would divide by zero");
        }
        let source_stride = (metrics.bitmap_bytes / metrics.black_box_height) & !3;
        let dimension = |value: u32| ((value as f32 * scale) as u32).max(1);
        metrics.black_box_width = dimension(source_stride);
        metrics.black_box_height = dimension(metrics.black_box_height);
        metrics.origin_x = nonzero((metrics.origin_x as f32 * scale) as i32);
        metrics.origin_y = nonzero((metrics.origin_y as f32 * scale) as i32);
        metrics.cell_inc_x = nonzero((f32::from(metrics.cell_inc_x) * scale) as i32 as i16);
        metrics.cell_inc_y = nonzero((f32::from(metrics.cell_inc_y) * scale) as i32 as i16);
        if profile != PalFontProfile::Totsulover {
            metrics.black_box_width = (u32::from(metrics.black_box_width as u16) + 3) & 0xfffc;
        }
        let stride = metrics
            .black_box_width
            .checked_add(3)
            .ok_or("font bitmap width overflow")?
            & !3;
        metrics.bitmap_bytes = stride
            .checked_mul(metrics.black_box_height)
            .ok_or("font bitmap size overflow")?;
        if metrics.bitmap_bytes as usize > MAX_COVERAGE_BYTES {
            return Err("scaled font bitmap exceeds the coverage limit");
        }
        let minimum_advance = metrics.black_box_width as i16;
        if metrics.cell_inc_x < minimum_advance {
            metrics.cell_inc_x = minimum_advance;
        }
        Ok(metrics)
    }

    pub fn glyph(
        &self,
        ch: char,
        size: u16,
        profile: PalFontProfile,
    ) -> Result<Option<BitmapGlyph>, &'static str> {
        let Some(offset) = self.offset(ch) else {
            return Ok(None);
        };
        let original = read_glyph_metrics(&self.bytes[offset..offset + 24]);
        let metrics = self.scaled_metrics(original, size, profile)?;
        let start = offset + 24;
        let coverage = &self.bytes[start..start + original.bitmap_bytes as usize];
        let source_stride = original
            .bitmap_bytes
            .checked_div(original.black_box_height)
            .unwrap_or(0)
            & !3;
        if self.scale(size) == 1.0 || coverage.is_empty() {
            return Ok(Some(BitmapGlyph {
                metrics,
                stride: source_stride,
                coverage: coverage.to_vec(),
            }));
        }
        let width = metrics.black_box_width;
        let height = metrics.black_box_height;
        let stride = width.checked_add(3).ok_or("font bitmap width overflow")? & !3;
        let sampled = resample_coverage(
            coverage,
            source_stride,
            original.black_box_height,
            width,
            height,
        )?;
        let mut output = vec![0; metrics.bitmap_bytes as usize];
        for (source, destination) in sampled
            .chunks_exact(width.max(1) as usize)
            .zip(output.chunks_exact_mut(stride.max(1) as usize))
        {
            destination[..source.len()].copy_from_slice(source);
        }
        Ok(Some(BitmapGlyph {
            metrics,
            stride,
            coverage: output,
        }))
    }
}

fn nonzero<T: Default + PartialEq + From<u8>>(value: T) -> T {
    if value == T::default() {
        T::from(1)
    } else {
        value
    }
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_glyph_metrics(bytes: &[u8]) -> PalBitmapGlyphRecord32 {
    PalBitmapGlyphRecord32 {
        black_box_width: word(bytes, 0),
        black_box_height: word(bytes, 4),
        origin_x: word(bytes, 8) as i32,
        origin_y: word(bytes, 12) as i32,
        cell_inc_x: i16::from_le_bytes(bytes[16..18].try_into().unwrap()),
        cell_inc_y: i16::from_le_bytes(bytes[18..20].try_into().unwrap()),
        bitmap_bytes: word(bytes, 20),
    }
}

/// PAL's coverage resampler uses 8-bit fractional coordinates and zero-valued
/// right/bottom neighbors. The source includes its native four-byte row padding.
pub(super) fn resample_coverage(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, &'static str> {
    let length = (width as usize)
        .checked_mul(height as usize)
        .ok_or("font bitmap size overflow")?;
    if length > MAX_COVERAGE_BYTES {
        return Err("scaled font bitmap exceeds the coverage limit");
    }
    if width == 0 || height == 0 {
        return Ok(vec![0; length]);
    }
    if source_width == 0
        || source_height == 0
        || source_width as usize * source_height as usize > source.len()
    {
        return Err("font resampling source extent is invalid");
    }
    let scale_x = width as f32 / source_width as f32;
    let scale_y = height as f32 / source_height as f32;
    let step_x = (256.0_f32 / scale_x).trunc() as u32;
    let step_y = (256.0_f32 / scale_y).trunc() as u32;
    let extent_x = width.min((source_width as f32 * scale_x) as u32);
    let extent_y = height.min((source_height as f32 * scale_y) as u32);
    let mut output = vec![0; length];
    let sample = |x: u32, y: u32| -> u32 {
        if x < source_width && y < source_height {
            u32::from(source[(y * source_width + x) as usize])
        } else {
            0
        }
    };
    for y in 0..extent_y {
        let coordinate_y = y.wrapping_mul(step_y);
        let iy = coordinate_y >> 8;
        let fy = coordinate_y & 255;
        for x in 0..extent_x {
            let coordinate_x = x.wrapping_mul(step_x);
            let ix = coordinate_x >> 8;
            let fx = coordinate_x & 255;
            let top = (256 - fx) * sample(ix, iy) + fx * sample(ix + 1, iy);
            let bottom = (256 - fx) * sample(ix, iy + 1) + fx * sample(ix + 1, iy + 1);
            output[(y * width + x) as usize] = (((256 - fy) * top + fy * bottom) >> 16) as u8;
        }
    }
    Ok(output)
}
