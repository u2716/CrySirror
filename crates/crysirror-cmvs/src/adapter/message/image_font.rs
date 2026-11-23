// SPDX-License-Identifier: MPL-2.0
//! Image glyph font `0x23c0..=0x23c2` (engine +4464, 0x140018b30..0x140018d80). Scripts
//! load 8-bit glyph sheets, cut a character's cell from each, and may give a cell its
//! own advance. The rasterizer (0x14006bd20) draws a character from here before asking
//! GDI: an exact cell size is used as is; otherwise, while the character has fewer than
//! sixteen cells, the smallest larger cell is resampled to the requested size and kept.

use super::super::visual::DecodedImage;
use anyhow::Result;
use crysirror_core::{
    service::{HostServices, TextExtent, TextMeasurement},
    state::ClientId,
};
use crysirror_image::{alpha_cells, ImageSurface};
use std::sync::Arc;

#[derive(Clone)]
struct Sheet {
    cell: u32,
    width: u32,
    coverage: Vec<u8>,
}

#[derive(Clone)]
struct Cell {
    size: u16,
    advance: u16,
    coverage: Arc<[u8]>,
    asset: Option<Arc<str>>,
}

#[derive(Clone)]
struct Glyph {
    code: u16,
    cells: Vec<Cell>,
}

#[derive(Clone, Default)]
pub(in crate::adapter) struct ImageFont {
    sheets: Vec<Sheet>,
    glyphs: Vec<Glyph>,
    published: u32,
}

/// First id of the published cell images.
const ASSET_BASE: u32 = 730_000;
const CELL_LIMIT: usize = 16;

impl ImageFont {
    /// `0x23c0`: a sheet of `cell`-pixel squares. The original keeps only 8-bit images;
    /// decoded pictures arrive as RGBA, so the red channel carries the sheet's values.
    pub fn add_sheet(&mut self, image: &DecodedImage, cell: u32) {
        let [width, _] = image.size();
        let coverage = image.rgba().chunks_exact(4).map(|pixel| pixel[0]).collect();
        self.sheets.push(Sheet { cell, width, coverage });
    }

    /// `0x23c1`: the character's cell `index` from every sheet, in sheet order.
    pub fn add_glyph(&mut self, index: u32, code: u16) {
        let cells = self
            .sheets
            .iter()
            .filter_map(|sheet| {
                let coverage = alpha_cells::cut(&sheet.coverage, sheet.width, sheet.cell, index)?;
                let size = sheet.cell as u16;
                Some(Cell { size, advance: size, coverage: coverage.into(), asset: None })
            })
            .collect();
        self.glyphs.push(Glyph { code, cells });
    }

    /// `0x23c2`: the advance of the character's `size` cell. False when either is missing.
    pub fn set_advance(&mut self, code: u16, size: u32, advance: u16) -> bool {
        let cell = self
            .glyphs
            .iter_mut()
            .find(|glyph| glyph.code == code)
            .and_then(|glyph| glyph.cells.iter_mut().find(|cell| u32::from(cell.size) == size));
        cell.map(|cell| cell.advance = advance).is_some()
    }

    fn glyph(&self, code: u16) -> Option<&Glyph> {
        self.glyphs.iter().find(|glyph| glyph.code == code)
    }

    /// The published cell and its advance, once [`Self::prepare`] has made it.
    pub fn lookup(&self, code: u16, size: u32) -> Option<(Arc<str>, u32)> {
        let cell = self.glyph(code)?.cells.iter().find(|cell| u32::from(cell.size) == size)?;
        Some((cell.asset.clone()?, u32::from(cell.advance)))
    }

    /// Makes and publishes the cells the text in `bytes` needs at `sizes`.
    pub fn prepare(&mut self, host: &mut dyn HostServices, client: ClientId, bytes: &[u8], sizes: &[u32]) -> Result<()> {
        if self.glyphs.is_empty() {
            return Ok(());
        }
        let mut index = 0;
        while index < bytes.len() && bytes[index] != 0 {
            let byte = bytes[index];
            let code = if matches!(byte, 0x81..=0x9f | 0xe0..=0xfc) && index + 1 < bytes.len() {
                index += 2;
                u16::from_be_bytes([byte, bytes[index - 1]])
            } else {
                index += 1;
                byte as i8 as i16 as u16
            };
            for &size in sizes {
                self.ensure(host, client, code, size)?;
            }
        }
        Ok(())
    }

    /// One character's cell at `size`, as [`Self::prepare`] makes them.
    pub fn prepare_code(&mut self, host: &mut dyn HostServices, client: ClientId, code: u16, size: u32) -> Result<()> {
        if self.glyphs.is_empty() {
            return Ok(());
        }
        self.ensure(host, client, code, size)
    }

    fn ensure(&mut self, host: &mut dyn HostServices, client: ClientId, code: u16, size: u32) -> Result<()> {
        let Some(glyph) = self.glyphs.iter_mut().find(|glyph| glyph.code == code) else {
            return Ok(());
        };
        let position = match glyph.cells.iter().position(|cell| u32::from(cell.size) == size) {
            Some(position) => position,
            None => {
                // 0x140018d80: the smallest cell larger than the request, below 1024.
                let source = glyph
                    .cells
                    .iter()
                    .filter(|cell| u32::from(cell.size) > size && cell.size < 1024)
                    .min_by_key(|cell| cell.size);
                let Some(source) = source.filter(|_| glyph.cells.len() < CELL_LIMIT && size > 0) else {
                    return Ok(());
                };
                let coverage = alpha_cells::resample(&source.coverage, u32::from(source.size), size);
                let advance = if source.advance == source.size {
                    size as u16
                } else {
                    (size * u32::from(source.advance) / u32::from(source.size)) as u16
                };
                glyph.cells.push(Cell { size: size as u16, advance, coverage: coverage.into(), asset: None });
                glyph.cells.len() - 1
            }
        };
        let cell = &mut glyph.cells[position];
        if cell.asset.is_none() {
            let side = u32::from(cell.size);
            let rgba = cell.coverage.iter().flat_map(|&value| [255, 255, 255, value]).collect();
            let image = DecodedImage::from_surface(client, ImageSurface { width: side, height: side, rgba }, Some(true))?;
            let uploaded = image.publish(host, client, ASSET_BASE + self.published)?;
            self.published += 1;
            cell.asset = Some(uploaded.path.relative().into());
        }
        Ok(())
    }
}

/// The CP932 character word the layout uses: big-endian pairs, sign-extended bytes.
pub(super) fn code(character: char) -> Option<u16> {
    let mut buffer = [0; 4];
    let (bytes, _, bad) = encoding_rs::SHIFT_JIS.encode(character.encode_utf8(&mut buffer));
    match (bad, bytes.as_ref()) {
        (false, &[byte]) => Some(byte as i8 as i16 as u16),
        (false, &[lead, trail]) => Some(u16::from_be_bytes([lead, trail])),
        _ => None,
    }
}

/// Proportional layout measurements with image font advances in front of the host font.
pub(super) struct Metrics<'a> {
    pub inner: &'a dyn TextMeasurement,
    pub font: &'a ImageFont,
}

impl TextMeasurement for Metrics<'_> {
    fn measure_simple(&self, font: &str, size: f32, text: &str) -> crysirror_core::Result<TextExtent> {
        let mut characters = text.chars();
        if let (Some(character), None) = (characters.next(), characters.next()) {
            if let Some((_, advance)) = code(character).and_then(|code| self.font.lookup(code, size as u32)) {
                let extent = self.inner.measure_simple(font, size, text).ok();
                return Ok(TextExtent {
                    advance: advance as f32,
                    line_height: extent.map_or(size, |extent| extent.line_height),
                    ascent: extent.map_or(size, |extent| extent.ascent),
                    descent: extent.map_or(0.0, |extent| extent.descent),
                });
            }
        }
        self.inner.measure_simple(font, size, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_follow_the_cells_they_came_from() {
        let mut font = ImageFont::default();
        font.sheets.push(Sheet { cell: 4, width: 8, coverage: vec![255; 32] });
        font.add_glyph(1, 0x8740);
        assert!(font.set_advance(0x8740, 4, 3));
        assert!(!font.set_advance(0x8740, 5, 3));
        let glyph = font.glyph(0x8740).unwrap();
        assert_eq!((glyph.cells.len(), glyph.cells[0].advance), (1, 3));
    }
}
