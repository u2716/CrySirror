// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Atomic glyph-cell preparation; pixels use the existing TextSystem kernel.
use super::*;
use crysirror_text::TextSystem;
struct Batch {
    owner: ClientId,
    images: Vec<Arc<dyn ImageResource>>,
}
impl GlyphImageBatchResource for Batch {
    fn client(&self) -> ClientId {
        self.owner
    }
    fn images(&self) -> &[Arc<dyn ImageResource>] {
        &self.images
    }
}
struct Query {
    owner: ClientId,
    task: Option<Task<Arc<dyn GlyphImageBatchResource>>>,
    state: GlyphImageBatchState,
}
impl GlyphImageBatchQuery for Query {
    fn client(&self) -> ClientId {
        self.owner
    }
    fn poll(&mut self) -> Result<GlyphImageBatchState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(batch)) => {
                    self.task = None;
                    self.state = GlyphImageBatchState::Ready(batch);
                }
                Err(e) => {
                    self.task = None;
                    self.state = GlyphImageBatchState::Failed(
                        e.downcast_ref::<Error>()
                            .cloned()
                            .unwrap_or_else(|| Error::Io(e.to_string())),
                    );
                }
            }
        }
        Ok(self.state.clone())
    }
    fn cancel(&mut self) -> Result<()> {
        self.task = None;
        self.state = GlyphImageBatchState::Cancelled;
        Ok(())
    }
}
pub(crate) fn begin(
    owner: ClientId,
    spec: GlyphImageBatchSpec,
    text: TextSystem,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<Box<dyn GlyphImageBatchQuery>> {
    if spec.cells.is_empty() || spec.cells.len() > 1024 {
        return Err(Error::Limit("glyph batch cell count"));
    }
    let mut pixels = 0u64;
    let mut work = 0u64;
    let mut glyphs = 0usize;
    let mut max_scratch = 0usize;
    let mut outputs = Vec::with_capacity(spec.cells.len());
    // Full admission and independent output leases precede queue admission.
    // An error drops all preceding reservations without exposing any resource.
    for cell in &spec.cells {
        let admitted = super::glyph::admission_cell(
            &spec.font,
            spec.size,
            cell.canvas,
            cell.glyphs.len(),
            &spec.raster,
        )?;
        pixels += admitted.pixels;
        work += admitted.work;
        glyphs += cell.glyphs.len();
        if pixels > 4 * 1024 * 1024 || work > 64 * 1024 * 1024 || glyphs > 65536 {
            return Err(Error::Limit("glyph batch total pixels/work/count"));
        }
        max_scratch = max_scratch.max(admitted.scratch);
        outputs.push(Charge::new(memory.clone(), admitted.bytes)?);
    }
    let text = text
        .for_client(owner)
        .frozen_font(&spec.font)
        .map_err(|e| Error::Io(e.to_string()))?;
    let _scratch = Charge::new(memory, max_scratch + glyphs * 64 + spec.cells.len() * 128)?;
    let permit = pool.reserve().map_err(|e| Error::Io(e.to_string()))?;
    let task = permit
        .spawn(move |cancel| {
            let _scratch = _scratch;
            let mut images = Vec::with_capacity(spec.cells.len());
            for (cell, output) in spec.cells.into_iter().zip(outputs) {
                cancel.check()?;
                let rgba = text.rasterize_glyphs_styled_cancellable(
                    &spec.font,
                    spec.size,
                    cell.canvas,
                    &cell.glyphs,
                    &spec.raster,
                    &|| cancel.check(),
                )?;
                images.push(resource_from_surface(
                    owner,
                    ImageSurface {
                        width: cell.canvas[0],
                        height: cell.canvas[1],
                        rgba,
                    },
                    output,
                    &|| cancel.check(),
                )?);
            }
            cancel.check()?;
            Ok(Arc::new(Batch { owner, images }) as Arc<dyn GlyphImageBatchResource>)
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(Query {
        owner,
        task: Some(task),
        state: GlyphImageBatchState::Pending,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::Glyph;
    use crysirror_text::dynamic::PreparedFontFace;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    fn setup() -> (TextSystem, Arc<PreparedFontFace>, GlyphImageBatchSpec) {
        let mut b = b"TVP pre-rendered font\x1a".to_vec();
        b.extend([1, 2]);
        b.extend(1u32.to_le_bytes());
        b.extend(36u32.to_le_bytes());
        b.extend(38u32.to_le_bytes());
        b.extend((b'A' as u16).to_le_bytes());
        b.extend(58u32.to_le_bytes());
        for value in [2i16, 2, -1, 2, 3, 0, 4, 0] {
            b.extend(value.to_le_bytes());
        }
        b.extend([32, 0x43]);
        let face =
            Arc::new(PreparedFontFace::prepare_precomputed(b.into(), 24, &|| Ok(())).unwrap());
        let text = TextSystem::default();
        text.publish_font(ClientId(1), "__font/1/batch", face.clone())
            .unwrap();
        let spec = GlyphImageBatchSpec {
            font: "__font/1/batch".into(),
            size: 24,
            raster: Default::default(),
            cells: (0..32)
                .map(|_| GlyphImageCell {
                    canvas: [4, 3],
                    glyphs: vec![Glyph {
                        glyph_id: 1,
                        x_26_6: 64,
                        y_26_6: 128,
                        advance_26_6: 192,
                    }],
                })
                .collect(),
        };
        (text, face, spec)
    }
    #[test]
    fn batch_queue_cancel_has_one_task_and_releases_all_cell_leases_without_joining() {
        let (text, face, spec) = setup();
        let weak = Arc::downgrade(&face);
        let pool = TaskPool::new("batch-cancel", 1, 2).unwrap();
        let (send, recv) = mpsc::channel();
        let (started, ready) = mpsc::channel();
        let mut blocker = pool
            .spawn(move |_| {
                started.send(()).unwrap();
                recv.recv().unwrap();
                Ok(())
            })
            .unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let memory = Arc::new(AtomicUsize::new(0));
        let mut query = begin(
            ClientId(1),
            spec,
            text.clone(),
            pool.clone(),
            memory.clone(),
        )
        .unwrap();
        assert_eq!(pool.stats().outstanding, 2);
        assert!(memory.load(Ordering::Acquire) > 32 * 48);
        drop(face);
        text.unpublish_font(ClientId(1), "__font/1/batch").unwrap();
        query.cancel().unwrap();
        assert!(matches!(
            query.poll().unwrap(),
            GlyphImageBatchState::Cancelled
        ));
        // This completes while the worker remains occupied by the unrelated task.
        send.send(()).unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        while pool.stats().outstanding != 0
            || memory.load(Ordering::Acquire) != 0
            || weak.upgrade().is_some()
        {
            assert!(Instant::now() < end);
            let _ = blocker.try_take();
            std::thread::yield_now();
        }
    }
    #[test]
    fn batch_failure_rolls_back_prior_cells_and_retained_cells_keep_only_their_own_charge() {
        let (text, _face, mut spec) = setup();
        let pool = TaskPool::new("batch-leases", 1, 2).unwrap();
        let memory = Arc::new(AtomicUsize::new(0));
        spec.cells.last_mut().unwrap().glyphs[0].glyph_id = 65535;
        let mut query = begin(
            ClientId(1),
            spec.clone(),
            text.clone(),
            pool.clone(),
            memory.clone(),
        )
        .unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            match query.poll().unwrap() {
                GlyphImageBatchState::Pending => {
                    assert!(Instant::now() < end);
                    std::thread::yield_now();
                }
                GlyphImageBatchState::Failed(_) => break,
                _ => panic!("partial publication"),
            }
        }
        while memory.load(Ordering::Acquire) != 0 {
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        spec.cells.last_mut().unwrap().glyphs[0].glyph_id = 1;
        let mut query = begin(ClientId(1), spec, text, pool.clone(), memory.clone()).unwrap();
        let batch = loop {
            match query.poll().unwrap() {
                GlyphImageBatchState::Pending => {
                    assert!(Instant::now() < end);
                    std::thread::yield_now();
                }
                GlyphImageBatchState::Ready(b) => break b,
                _ => panic!("valid batch failed"),
            }
        };
        let one = batch.images()[4].clone();
        drop(batch);
        drop(query);
        while pool.stats().outstanding != 0 || memory.load(Ordering::Acquire) != 48 {
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        assert_eq!(one.size(), [4, 3]);
        drop(one);
        assert_eq!(memory.load(Ordering::Acquire), 0);
    }
}
