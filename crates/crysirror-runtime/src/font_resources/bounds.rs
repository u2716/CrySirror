//! Scoped, bounded glyph coverage metadata preparation, without native VMs.
use super::*;

struct Resource {
    owner: ClientId,
    font: String,
    digest: String,
    glyphs: Vec<GlyphBounds>,
    _snapshot: TextSystem,
    _charge: Arc<Charge>,
}
impl GlyphBoundsResource for Resource {
    fn client(&self) -> ClientId {
        self.owner
    }
    fn font(&self) -> &str {
        &self.font
    }
    fn digest(&self) -> &str {
        &self.digest
    }
    fn glyphs(&self) -> &[GlyphBounds] {
        &self.glyphs
    }
}
struct Query {
    task: Option<Task<Arc<dyn GlyphBoundsResource>>>,
    state: GlyphBoundsProgress,
}
impl GlyphBoundsQuery for Query {
    fn poll(&mut self) -> crysirror_core::Result<GlyphBoundsProgress> {
        if matches!(self.state, GlyphBoundsProgress::Pending) {
            let task = self
                .task
                .as_mut()
                .ok_or(crysirror_core::Error::Invalid("glyph bounds task missing"))?;
            match task.try_take() {
                Ok(Some(resource)) => {
                    self.state = GlyphBoundsProgress::Ready(resource);
                    self.task = None;
                }
                Ok(None) => {}
                Err(error) => {
                    self.state = GlyphBoundsProgress::Failed(err(error));
                    self.task = None;
                }
            }
        }
        Ok(self.state.clone())
    }
    fn cancel(&mut self) -> crysirror_core::Result<()> {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.state = GlyphBoundsProgress::Cancelled;
        Ok(())
    }
}

pub(super) fn begin(
    client: &ClientFonts,
    spec: GlyphBoundsSpec,
) -> crysirror_core::Result<Box<dyn GlyphBoundsQuery>> {
    use crysirror_text::glyph::GlyphRaster;
    let prepare = || -> Result<Query> {
        ensure!(
            !spec.font.is_empty()
                && spec.font.len() <= 256
                && !spec.font.chars().any(char::is_control),
            "glyph bounds font alias"
        );
        ensure!(spec.glyphs.len() <= 65536, "glyph bounds request count");
        let mut work = 0u64;
        for request in &spec.glyphs {
            ensure!(
                request.size.is_finite()
                    && (1.0..=512.0).contains(&request.size)
                    && request
                        .baseline
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                "glyph bounds request size/baseline"
            );
            let side = request.size.ceil() as u64;
            work = work
                .checked_add(side * side * 4)
                .context("glyph bounds work overflow")?;
            ensure!(work <= 64 * 1024 * 1024, "glyph bounds work budget");
        }
        // Scope/identity validation occurs before work is admitted. A frozen
        // snapshot retains the exact immutable face even if its alias is later
        // released; new unrelated clients cannot discover it through this job.
        let text = client.text.frozen_font(&spec.font)?;
        let digest = text.font_digest(&spec.font)?;
        let permit = client.resources.0.cpu.reserve()?;
        let charge = charge_precomputed(
            &client.resources.0.used,
            spec.glyphs.len() * core::mem::size_of::<GlyphBounds>() + 32 * 1024,
        )?;
        // rasterize_glyph_cell may hold four million f32 coverage samples.
        // Scratch is released on completion; only metadata/font leases persist.
        let scratch = charge_precomputed(
            &client.resources.0.used,
            16 * 1024 * 1024 + spec.glyphs.len() * core::mem::size_of::<GlyphBoundsRequest>(),
        )?;
        let owner = client.owner;
        let task = permit.spawn(move |cancel| {
            let _scratch = scratch;
            let mut glyphs = Vec::with_capacity(spec.glyphs.len());
            let mut actual_work = 0u64;
            for request in spec.glyphs {
                cancel.check()?;
                let glyph_id = text.glyph_id(&spec.font, request.character)?;
                let cell = text.rasterize_glyph_cell(
                    &spec.font,
                    GlyphRaster {
                        glyph_id,
                        size: request.size,
                        baseline: request.baseline,
                    },
                )?;
                actual_work = actual_work
                    .checked_add(u64::from(cell.width) * u64::from(cell.height))
                    .context("glyph bounds coverage work overflow")?;
                ensure!(
                    actual_work <= 64 * 1024 * 1024,
                    "glyph bounds coverage work budget"
                );
                glyphs.push(GlyphBounds {
                    glyph_id,
                    origin: cell.origin,
                    size: [cell.width, cell.height],
                });
            }
            cancel.check()?;
            Ok(Arc::new(Resource {
                owner,
                font: spec.font,
                digest,
                glyphs,
                _snapshot: text,
                _charge: charge,
            }) as Arc<dyn GlyphBoundsResource>)
        })?;
        Ok(Query {
            task: Some(task),
            state: GlyphBoundsProgress::Pending,
        })
    };
    Ok(Box::new(prepare().map_err(err)?))
}
