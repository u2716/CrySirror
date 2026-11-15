//! Outline-design decorations and ink bounds, shared by all text clients.
use crate::{
    TextSystem,
    glyph::{GlyphCell, GlyphRaster},
};
use ab_glyph::{Font, GlyphId, PxScale, ScaleFont, point};
use anyhow::{Context, Result, ensure};
/// Enclose a rotated ink box in integer pixels, preserving empty boxes.
pub fn rotate_bounds(b: [i32; 4], degrees: f64) -> [i32; 4] {
    if b[2] <= b[0] || b[3] <= b[1] {
        return [0; 4];
    }
    let a = degrees.to_radians();
    let (s, c) = a.sin_cos();
    let mut out = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (x, y) in [(b[0], b[1]), (b[2], b[1]), (b[0], b[3]), (b[2], b[3])] {
        let x = f64::from(x);
        let y = f64::from(y);
        let p = [x * c - y * s, x * s + y * c];
        out = [
            out[0].min(p[0]),
            out[1].min(p[1]),
            out[2].max(p[0]),
            out[3].max(p[1]),
        ];
    }
    [
        out[0].floor() as i32,
        out[1].floor() as i32,
        out[2].ceil() as i32,
        out[3].ceil() as i32,
    ]
}
impl TextSystem {
    pub fn font_catalog(&self) -> Result<Vec<crysirror_core::service::FontCatalogFace>> {
        let mut entries = Vec::new();
        for alias in self.font_aliases()? {
            if self.precomputed_face(&alias)?.is_some() {
                entries.push(crysirror_core::service::FontCatalogFace {
                    names: vec![alias.clone()],
                    alias,
                    fixed_pitch: false,
                    outline: false,
                    code_pages: None,
                });
                continue;
            }
            let (bytes, index) = self.font_source(&alias)?;
            let face = rustybuzz::Face::from_slice(&bytes, index).context("catalogue font face")?;
            let os2 = face
                .raw_face()
                .table(rustybuzz::ttf_parser::Tag::from_bytes(b"OS/2"));
            let code_pages = os2
                .filter(|b| b.len() >= 86 && u16::from_be_bytes([b[0], b[1]]) >= 1)
                .map(|b| {
                    [
                        u32::from_be_bytes(b[78..82].try_into().unwrap()),
                        u32::from_be_bytes(b[82..86].try_into().unwrap()),
                    ]
                });
            entries.push(crysirror_core::service::FontCatalogFace {
                alias,
                names: crate::dynamic::family_names(&face)?,
                fixed_pitch: face.is_monospaced(),
                outline: true,
                code_pages,
            });
        }
        Ok(entries)
    }
    /// Baseline-relative line centre/thickness from the actual face tables.
    pub fn decoration_lines(
        &self,
        font: &str,
        size: f32,
        underline: bool,
        strikeout: bool,
    ) -> Result<Vec<[f32; 2]>> {
        if !underline && !strikeout {
            return Ok(vec![]);
        }
        let face = self.font(font)?;
        let (bytes, index) = self.font_source(font)?;
        let design = rustybuzz::Face::from_slice(&bytes, index).context("decoration face")?;
        let units = size / face.height_unscaled();
        let mut lines = Vec::new();
        for enabled in [underline, strikeout]
            .into_iter()
            .enumerate()
            .filter_map(|(i, v)| v.then_some(i))
        {
            let m = if enabled == 0 {
                design.underline_metrics()
            } else {
                design.strikeout_metrics()
            }
            .context("font has no decoration metrics")?;
            lines.push([
                -f32::from(m.position) * units,
                (f32::from(m.thickness).abs() * units).ceil().max(1.),
            ]);
        }
        Ok(lines)
    }
    pub fn decorate_glyph_cell(
        &self,
        font: &str,
        request: GlyphRaster,
        scale: f32,
        mut cell: GlyphCell,
        underline: bool,
        strikeout: bool,
    ) -> Result<GlyphCell> {
        let lines = self.decoration_lines(font, request.size, underline, strikeout)?;
        if lines.is_empty() {
            return Ok(cell);
        }
        let advance = self
            .glyph_metrics(font, request.glyph_id, request.size)?
            .advance
            * scale;
        let rects: Vec<[i32; 4]> = lines
            .into_iter()
            .map(|[y, t]| {
                [
                    request.baseline[0]
                        .min(request.baseline[0] + advance)
                        .floor() as i32,
                    (request.baseline[1] + y - t * 0.5).floor() as i32,
                    request.baseline[0]
                        .max(request.baseline[0] + advance)
                        .ceil() as i32,
                    (request.baseline[1] + y + t * 0.5).ceil() as i32,
                ]
            })
            .collect();
        let mut bounds = rects[0];
        for r in &rects {
            bounds = [
                bounds[0].min(r[0]),
                bounds[1].min(r[1]),
                bounds[2].max(r[2]),
                bounds[3].max(r[3]),
            ];
        }
        if cell.width > 0 && cell.height > 0 {
            bounds = [
                bounds[0].min(cell.origin[0]),
                bounds[1].min(cell.origin[1]),
                bounds[2].max(cell.origin[0] + cell.width as i32),
                bounds[3].max(cell.origin[1] + cell.height as i32),
            ];
        }
        let w = (bounds[2] - bounds[0]).max(0) as u32;
        let h = (bounds[3] - bounds[1]).max(0) as u32;
        ensure!(
            u64::from(w) * u64::from(h) <= 4_194_304,
            "decorated glyph cell budget"
        );
        let mut coverage = vec![0.; w as usize * h as usize];
        for y in 0..cell.height as usize {
            let dy = (cell.origin[1] - bounds[1]) as usize + y;
            let dx = (cell.origin[0] - bounds[0]) as usize;
            coverage[dy * w as usize + dx..dy * w as usize + dx + cell.width as usize]
                .copy_from_slice(
                    &cell.coverage[y * cell.width as usize..(y + 1) * cell.width as usize],
                );
        }
        for r in rects {
            for y in r[1]..r[3] {
                for x in r[0]..r[2] {
                    coverage[((y - bounds[1]) as u32 * w + (x - bounds[0]) as u32) as usize] = 1.;
                }
            }
        }
        cell.origin = [bounds[0], bounds[1]];
        cell.width = w;
        cell.height = h;
        cell.coverage = coverage;
        Ok(cell)
    }
    /// Ink bounds of the same simple layout, including actual design decoration
    /// metrics and synthetic bold. Empty strings/space-only runs have an empty box.
    pub fn ink_bounds_simple(
        &self,
        font: &str,
        size: f32,
        text: &str,
        bold: bool,
        underline: bool,
        strikeout: bool,
    ) -> Result<[i32; 4]> {
        let metrics = self.measure_simple(font, size, text)?;
        ensure!(text.len() <= 65536, "ink bounds text limit");
        let positions = self.position_simple(font, size as u16, text)?;
        let mut bounds: Option<[f32; 4]> = None;
        let mut add = |r: [f32; 4]| {
            if r[2] > r[0] && r[3] > r[1] {
                bounds = Some(bounds.map_or(r, |b| {
                    [
                        b[0].min(r[0]),
                        b[1].min(r[1]),
                        b[2].max(r[2]),
                        b[3].max(r[3]),
                    ]
                }));
            }
        };
        let bitmap = self.precomputed_face(font)?;
        for p in positions {
            let id = self.glyph_id(font, p.character)?;
            if let Some(face) = &bitmap {
                let g = face.glyph(id)?;
                let x = p.baseline[0] + g.bearing[0] as f32;
                let y = p.baseline[1] + g.bearing[1] as f32;
                add([x, y, x + f32::from(g.size[0]), y + f32::from(g.size[1])]);
            } else {
                let face = self.font(font)?;
                if let Some(g) = face.outline_glyph(GlyphId(id as u16).with_scale_and_position(
                    PxScale::from(size),
                    point(p.baseline[0], p.baseline[1]),
                )) {
                    let r = g.px_bounds();
                    add([
                        r.min.x,
                        r.min.y,
                        r.max.x + if bold { 1. } else { 0. },
                        r.max.y,
                    ]);
                }
            }
        }
        if !text.is_empty() {
            for [y, t] in self.decoration_lines(font, size, underline, strikeout)? {
                add([
                    0.,
                    metrics.ascent + y - t * 0.5,
                    metrics.advance,
                    metrics.ascent + y + t * 0.5,
                ]);
            }
        }
        Ok(bounds.map_or([0; 4], |r| {
            [
                r[0].floor() as i32,
                r[1].floor() as i32,
                r[2].ceil() as i32,
                r[3].ceil() as i32,
            ]
        }))
    }
}
