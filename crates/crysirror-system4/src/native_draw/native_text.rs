// Native Drawing policy only, GPL-2.0-or-later; no private pixel/font kernel.
use super::font_state::FontState;
use super::*;
use crysirror_core::{
    image_blend::*,
    image_processing::*,
    service::{FontLibrary, Glyph},
};
use std::sync::Arc;
struct Cell {
    position: [i32; 2],
    glyph: GlyphImageSpec,
}
pub(super) struct Run {
    call: ExternalCall,
    number: i32,
    base: ImageInput,
    cells: Vec<Cell>,
    options: GlyphRasterOptions,
    color: [u8; 3],
    alpha: bool,
    active: Option<Box<dyn GlyphImageBatchQuery>>,
    prepared: Vec<ImageCompositeLayer>,
    composite: Option<ImageJob>,
}
impl Run {
    pub(super) fn prepare(
        owner: ClientId,
        _machine: &Machine,
        call: &ExternalCall,
        args: &[Argument],
        state: &FontState,
        bridge: &Bridge,
        cg: &NativeCg,
        fonts: &dyn FontLibrary,
        alpha: bool,
    ) -> Result<Self> {
        let [Argument::Scalar {
            kind: 10,
            bits: number,
        }, Argument::Scalar { kind: 10, bits: x }, Argument::Scalar { kind: 10, bits: y }, Argument::String { bytes, .. }] =
            args
        else {
            bail!("native glyph string ABI");
        };
        if alpha {
            ensure!(
                bridge
                    .sact
                    .as_ref()
                    .and_then(|s| s.sprites.as_ref())
                    .and_then(|s| s.get(*number))
                    .is_some_and(|s| s.alpha),
                "native glyph destination lacks alpha plane"
            );
        }
        ensure!(
            (1..=170).contains(&state.size),
            "native3x font size profile"
        );
        ensure!(
            (0..=1000).contains(&state.weight),
            "native portable font weight profile"
        );

        let name = crysirror_resource::text_stream::decode_shift_jis(&state.name, 4096)?;
        let sact = bridge
            .sact
            .as_ref()
            .filter(|s| s.initialized)
            .context("native font before SACT")?;
        let bound = if state.name == FontState::default().name {
            Some(sact.font_aliases[0].clone())
        } else if name == "ＭＳ 明朝" {
            Some(sact.font_aliases[1].clone())
        } else {
            fonts.resolve_alias(&name)?
        };
        let font = fonts
            .resolve_alias(&bound.context("native font family not authorized/published")?)?
            .context("native font face not published")?;
        let face = fonts
            .face_style(&font)?
            .context("native portable outline font lacks actual OS/2 weight metadata")?;
        ensure!(
            !face.italic,
            "native regular-font profile cannot substitute an italic face"
        );
        let target = if state.weight == 0 { 400 } else { state.weight };
        ensure!(
            target == i32::from(face.weight)
                || (target >= 600 && face.weight >= 350 && face.weight <= 550),
            "native font weight outside reviewed mapped/synthetic outline profile"
        );
        // This is an explicit portable synthetic-bold profile, using the engine's
        // existing disk morphology. It does not claim Windows GDI hinting identity.
        let embolden = if target > i32::from(face.weight) {
            ((state.size as f32 * 3. * 0.025).ceil() as u8).min(16)
        } else {
            0
        };
        let raw = bytes.split(|b| *b == 0).next().unwrap();
        ensure!(raw.len() <= 4096, "native text bytes");
        let mut cells = Vec::new();
        let mut byte = 0usize;
        let mut advance = 0i32;
        while byte < raw.len() {
            ensure!(cells.len() < 254, "native text composition graph budget");
            let lead = raw[byte];
            let count = if (0x80..=0xa0).contains(&lead) || lead >= 0xe0 {
                2
            } else {
                1
            };
            let unit = raw
                .get(byte..byte + count)
                .context("native truncated CP932 text unit")?;
            let text = crysirror_resource::text_stream::decode_shift_jis(unit, 8)?;
            let mut chars = text.chars();
            let ch = chars.next().context("native empty text unit")?;
            ensure!(
                chars.next().is_none() && !ch.is_control(),
                "native CP932 font unit profile"
            );
            let id = fonts.glyph_id(&font, ch)?;
            let width = if count == 2 {
                state.size
            } else {
                state.size / 2
            };
            ensure!(width > 0, "native empty half-width cell");
            let position = [
                x.checked_add(advance).context("native text x overflow")?,
                *y,
            ];
            cells.push(Cell {
                position,
                glyph: GlyphImageSpec {
                    font: font.clone(),
                    size: state.size as u16,
                    canvas: [width as u32, state.size as u32],
                    glyphs: vec![Glyph {
                        glyph_id: id,
                        x_26_6: 0,
                        y_26_6: 0,
                        advance_26_6: width * 64,
                    }],
                },
            });
            let step = state
                .size
                .checked_add(state.space)
                .context("native font step overflow")?;
            advance = advance
                .checked_add(if count == 2 { step } else { step / 2 })
                .context("native font advance overflow")?;
            if count == 1 && step & 1 != 0 && byte & 1 != 0 {
                advance = advance
                    .checked_add(1)
                    .context("native half-cell remainder")?;
            }
            byte += count;
        }
        Ok(Self {
            call: call.clone(),
            number: *number,
            base: cg.sprite_input(owner, *number, bridge)?,
            cells,
            options: GlyphRasterOptions {
                size_basis: GlyphSizeBasis::Em,
                origin: GlyphImageOrigin::Top,
                supersample: 3,
                binary_cutoff: Some(128),
                coverage_lut: Some(Arc::from([0, 28, 56, 113, 141, 169, 198, 226, 254, 255])),
                // Portable explicit synthetic face policy. Final consumer must check
                // requested vs installed face weight instead of pretending GDI equality.
                embolden_radius: embolden,
                underline: state.underline,
                strikeout: state.strikeout,
                ..Default::default()
            },
            color: state.color,
            alpha,
            active: None,
            prepared: Vec::new(),
            composite: None,
        })
    }
    pub(super) fn poll(
        &mut self,
        owner: ClientId,
        images: &dyn ImageMetadata,
        host: &mut dyn HostServices,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        machine: &mut Machine,
        call: &ExternalCall,
    ) -> Result<Dispatch> {
        ensure!(self.call == *call, "native stale glyph completion");
        if self.active.is_none() && self.composite.is_none() {
            if let Some(first) = self.cells.first() {
                self.active = Some(
                    images.begin_glyph_image_batch(GlyphImageBatchSpec {
                        font: first.glyph.font.clone(),
                        size: first.glyph.size,
                        raster: self.options.clone(),
                        cells: self
                            .cells
                            .iter()
                            .map(|c| GlyphImageCell {
                                canvas: c.glyph.canvas,
                                glyphs: c.glyph.glyphs.clone(),
                            })
                            .collect(),
                    })?,
                );
            }
        }
        if let Some(active) = self.active.as_mut() {
            match active.poll()? {
                GlyphImageBatchState::Pending => {
                    return Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)))
                }
                GlyphImageBatchState::Failed(error) => bail!("native shared glyph batch: {error}"),
                GlyphImageBatchState::Cancelled => bail!("native shared glyph batch cancelled"),
                GlyphImageBatchState::Ready(batch) => {
                    ensure!(
                        batch.client() == owner && batch.images().len() == self.cells.len(),
                        "native glyph batch scope/count"
                    );
                    for (cell, resource) in self.cells.iter().zip(batch.images()) {
                        ensure!(
                            resource.client() == owner && resource.size() == cell.glyph.canvas,
                            "native glyph cell scope/size"
                        );
                        let math = if self.alpha {
                            ImageCompositeMath::CopyAlpha
                        } else {
                            ImageCompositeMath::Normalized(RgbaBlendState {
                                arithmetic: RgbaBlendArithmetic::Byte255 {
                                    source: RgbaByteRounding::Floor,
                                    destination: RgbaByteRounding::Floor,
                                },
                                color: RgbaBlendComponent {
                                    source: RgbaBlendFactor::SourceAlpha,
                                    destination: RgbaBlendFactor::OneMinusSourceAlpha,
                                    equation: RgbaBlendEquation::Add,
                                },
                                source_scale: [0., 0., 0., 1.],
                                source_bias: [
                                    f32::from(self.color[0]) / 255.,
                                    f32::from(self.color[1]) / 255.,
                                    f32::from(self.color[2]) / 255.,
                                    0.,
                                ],
                                write_mask: [true, true, true, false],
                                ..Default::default()
                            })
                        };
                        self.prepared.push(ImageCompositeLayer {
                            input: Box::new(ImageInput::Resource(resource.clone())),
                            source: [
                                0,
                                0,
                                cell.glyph.canvas[0] as i32,
                                cell.glyph.canvas[1] as i32,
                            ],
                            destination: cell.position,
                            clip: None,
                            opacity: 255,
                            math,
                        });
                    }
                    self.active = None;
                }
            }
        }
        if self.composite.is_none() {
            self.composite = Some(images.begin_process(
                ImageInput::Composite {
                    base: Box::new(self.base.clone()),
                    layers: self.prepared.clone(),
                },
                b"[]".as_slice().into(),
            )?);
        }
        match self.composite.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1))),
            ImageProcessState::Failed(error) => bail!("native shared glyph composition: {error}"),
            ImageProcessState::Ready(resource) => {
                cg.publish_draw_result(owner, self.number, resource, bridge, host)?;
                machine.complete_hll(call, HllReturn::Void)?;
                Ok(Dispatch::Completed)
            }
        }
    }
}
