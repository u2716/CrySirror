// SPDX-License-Identifier: MPL-2.0
//! Trial40FAB0/4447D0/448280: LoadFont creates a non-drawing name holding
//! per-character cache entries; it neither loads a file nor changes SetFont.
//! Field0xB60 is only stored/initialized and released by457BA0/457E50.
//! Portable shared coverage leases replace opaque random GDI cache IDs. Native
//! paint/weight/edge identity remains part of the cache key; no glyph image is
//! exposed as a script texture. Host outline substitution is not GDI raster parity.
use super::*;
use crysirror_core::{
    image_processing::{
        GlyphImageBatchQuery, GlyphImageBatchResource, GlyphImageBatchSpec, GlyphImageBatchState,
        GlyphImageCell, GlyphRasterOptions, GlyphSizeBasis,
    },
    service::{Glyph, GlyphBoundsProgress, GlyphBoundsQuery, GlyphBoundsRequest, GlyphBoundsSpec},
};

#[derive(Clone, PartialEq)]
struct Key {
    font: String,
    size: i32,
    colors: [[u8; 4]; 2],
    weight: i32,
    edge: u16,
    text: String,
}
pub(super) struct Cache {
    key: Key,
    cells: Option<Arc<dyn GlyphImageBatchResource>>,
}
impl Cache {
    fn bytes(&self) -> u64 {
        self.cells.as_ref().map_or(0, |r| {
            r.images()
                .iter()
                .map(|image| {
                    let [w, h] = image.size();
                    u64::from(w) * u64::from(h) * 4
                })
                .sum()
        })
    }
}
pub(super) struct Preparing {
    key: Key,
    bounds: Option<Box<dyn GlyphBoundsQuery>>,
    spec: Option<GlyphImageBatchSpec>,
    batch: Option<Box<dyn GlyphImageBatchQuery>>,
    bytes: u64,
}
fn weight(text: &str) -> i32 {
    match text.to_ascii_uppercase().as_str() {
        "DONTCARE" => 0,
        "THIN" => 100,
        "EXTRALIGHT" => 200,
        "LIGHT" => 300,
        "NORMAL" => 400,
        "MEDIUM" => 500,
        "SEMIBOLD" => 600,
        "BOLD" => 700,
        "EXTRABOLD" => 800,
        "HEAVY" => 900,
        _ => text.parse().unwrap_or(400),
    }
}
fn edge(text: &str) -> u16 {
    match text.to_ascii_uppercase().as_str() {
        "LEFT" => 1,
        "RIGHT" => 16,
        "DOWN" => 256,
        "UP" => 4096,
        "LEFTDOWN" => 257,
        "RIGHTDOWN" => 272,
        "LEFTUP" => 4097,
        "RIGHTUP" => 4112,
        "AROUND" => 4369,
        _ => 0,
    }
}
impl N2SystemVm {
    pub(super) fn native_font_cache(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 8, "N2System LoadFont signature");
        let name = args.string(0)?;
        if state.glyph_preparation.is_none() {
            let requested = args.string(1)?;
            let library = self.fonts.as_ref().unwrap();
            let font = library
                .resolve_alias(requested.trim_start_matches('@'))?
                .or_else(|| {
                    (requested.is_empty()
                        || requested
                            .trim_start_matches('@')
                            .replace(' ', "")
                            .starts_with("ＭＳ"))
                    .then(|| self.style.font.clone())
                })
                .with_context(|| format!("N2System LoadFont family unavailable: {requested}"))?;
            let size = args.integer(2)?;
            // 444680 forwards zero to LOGFONT's platform font mapper. The
            // portable substituted face uses a 16 px default cell at 96 DPI;
            // it is independent of SetFont/dialogue size and display scale.
            // Exact Windows mapper metrics depend on the installed face.
            let size = if size == 0 { 16 } else { size };
            ensure!(
                size.unsigned_abs() <= 512,
                "N2System LoadFont explicit size must be 1..512 in magnitude"
            );
            let text = args.string(7)?;
            ensure!(
                text.chars().count() <= 1024,
                "N2System LoadFont character capacity"
            );
            if self.config.encoding == "shift-jis" {
                let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(&text);
                ensure!(
                    !errors && encoded.len() < 2048,
                    "N2System LoadFont native character buffer"
                );
            }
            let key = Key {
                font,
                size,
                colors: [args.color(3)?, args.color(4)?],
                weight: weight(&args.string(5)?),
                edge: edge(&args.string(6)?),
                text,
            };
            if let Some(cache) = self.glyph_caches.values().find(|cache| cache.key == key) {
                let cells = cache.cells.clone();
                let id = self.create(host, current, &name, Kind::Name, [0.; 2])?;
                self.glyph_caches.insert(id, Cache { key, cells });
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            state.glyph_preparation = Some(Preparing {
                key,
                bounds: None,
                spec: None,
                batch: None,
                bytes: 0,
            });
        }
        let other_bytes = self
            .threads
            .values()
            .filter_map(|state| state.glyph_preparation.as_ref())
            .map(|p| p.bytes)
            .sum::<u64>();
        let preparation = state.glyph_preparation.as_mut().unwrap();
        if !preparation.key.text.is_empty() {
            let library = self.fonts.as_ref().unwrap();
            if preparation.spec.is_none() && preparation.batch.is_none() {
                if preparation.bounds.is_none() {
                    let key = &preparation.key;
                    let scale = if key.size < 0 {
                        library.line_height_for_em(&key.font, key.size.unsigned_abs() as f32)?
                    } else {
                        key.size as f32
                    };
                    match library.begin_glyph_bounds(GlyphBoundsSpec {
                        font: key.font.clone(),
                        glyphs: key
                            .text
                            .chars()
                            .map(|character| GlyphBoundsRequest {
                                character,
                                size: scale,
                                baseline: [0.; 2],
                            })
                            .collect(),
                    }) {
                        Ok(query) => preparation.bounds = Some(query),
                        Err(Error::Limit("font CPU task capacity")) => return Ok(false),
                        Err(e) => return Err(e.into()),
                    }
                }
                match preparation.bounds.as_mut().unwrap().poll()? {
                    GlyphBoundsProgress::Pending => return Ok(false),
                    GlyphBoundsProgress::Failed(error) => return Err(error.into()),
                    GlyphBoundsProgress::Cancelled => {
                        bail!("N2System font cache preparation cancelled")
                    }
                    GlyphBoundsProgress::Ready(bounds) => {
                        ensure!(
                            bounds.client() == self.client && bounds.font() == preparation.key.font,
                            "N2System font cache bounds owner"
                        );
                        let key = &preparation.key;
                        let margin = if key.edge != 0 { 3 } else { 1 };
                        let cells = bounds
                            .glyphs()
                            .iter()
                            .map(|b| GlyphImageCell {
                                canvas: [
                                    (b.size[0] + 2 * margin + 3) & !3,
                                    (b.size[1] + 2 * margin).max(1),
                                ],
                                glyphs: vec![Glyph {
                                    glyph_id: b.glyph_id,
                                    x_26_6: (margin as i32 - b.origin[0]) * 64,
                                    y_26_6: (margin as i32 - b.origin[1]) * 64,
                                    advance_26_6: 0,
                                }],
                            })
                            .collect::<Vec<_>>();
                        let bytes = cells
                            .iter()
                            .map(|c| u64::from(c.canvas[0]) * u64::from(c.canvas[1]) * 4)
                            .sum::<u64>();
                        let retained = self.glyph_caches.values().map(Cache::bytes).sum::<u64>();
                        ensure!(
                            retained + other_bytes + bytes
                                <= if cfg!(target_os = "vita") {
                                    8 * 1024 * 1024
                                } else {
                                    32 * 1024 * 1024
                                },
                            "N2System font cache byte budget"
                        );
                        let actual = library
                            .face_style(&key.font)?
                            .map_or(400, |style| style.weight);
                        preparation.spec = Some(GlyphImageBatchSpec {
                            font: key.font.clone(),
                            size: key.size.unsigned_abs() as u16,
                            cells,
                            raster: GlyphRasterOptions {
                                size_basis: if key.size < 0 {
                                    GlyphSizeBasis::Em
                                } else {
                                    GlyphSizeBasis::LineHeight
                                },
                                embolden_radius: u8::from(key.weight >= 600 && actual < 600),
                                ..Default::default()
                            },
                        });
                        preparation.bounds = None;
                        preparation.bytes = bytes;
                    }
                }
            }
            if preparation.batch.is_none() {
                match self
                    .images
                    .as_ref()
                    .unwrap()
                    .begin_glyph_image_batch(preparation.spec.as_ref().unwrap().clone())
                {
                    Ok(query) => {
                        preparation.batch = Some(query);
                        preparation.spec = None;
                    }
                    Err(Error::Limit("glyph batch task capacity")) => return Ok(false),
                    Err(e) => return Err(e.into()),
                }
            }
            match preparation.batch.as_mut().unwrap().poll()? {
                GlyphImageBatchState::Pending => return Ok(false),
                GlyphImageBatchState::Failed(error) => return Err(error.into()),
                GlyphImageBatchState::Cancelled => bail!("N2System font cache cancelled"),
                GlyphImageBatchState::Ready(cells) => {
                    ensure!(cells.client() == self.client, "N2System font cache owner");
                    let key = state.glyph_preparation.take().unwrap().key;
                    let id = self.create(host, current, &name, Kind::Name, [0.; 2])?;
                    self.glyph_caches.insert(
                        id,
                        Cache {
                            key,
                            cells: Some(cells),
                        },
                    );
                }
            }
        } else {
            let key = state.glyph_preparation.take().unwrap().key;
            let id = self.create(host, current, &name, Kind::Name, [0.; 2])?;
            self.glyph_caches.insert(id, Cache { key, cells: None });
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
