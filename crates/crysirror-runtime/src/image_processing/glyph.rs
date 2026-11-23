//! Worker publication of the same coverage used by engine GlyphRuns.
use super::*;
use crysirror_text::TextSystem;

pub(crate) fn begin(
    owner: ClientId,
    spec: GlyphImageSpec,
    text: TextSystem,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<ImageJob> {
    begin_with_options(owner, spec, Default::default(), text, pool, memory)
}
pub(crate) fn begin_styled(
    owner: ClientId,
    spec: StyledGlyphImageSpec,
    text: TextSystem,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<ImageJob> {
    begin_with_options(owner, spec.glyphs, spec.raster, text, pool, memory)
}
fn begin_with_options(
    owner: ClientId,
    spec: GlyphImageSpec,
    options: GlyphRasterOptions,
    text: TextSystem,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<ImageJob> {
    let work = admission(&spec, &options)?;
    // Resolve on the owner before queue admission. The job retains exactly one
    // authorized face, not the mutable catalog or any unrelated client fonts.
    let text = text
        .for_client(owner)
        .frozen_font(&spec.font)
        .map_err(|e| Error::Io(e.to_string()))?;
    let permit = pool.reserve().map_err(|e| Error::Io(e.to_string()))?;
    let bytes = work.bytes;
    let output = Charge::new(memory.clone(), bytes)?;
    let scratch = Charge::new(memory, work.scratch)?;
    let task = permit
        .spawn(move |cancel| {
            let _scratch = scratch;
            let rgba = text.rasterize_glyphs_styled_cancellable(
                &spec.font,
                spec.size,
                spec.canvas,
                &spec.glyphs,
                &options,
                &|| cancel.check(),
            )?;
            resource_from_surface(
                owner,
                ImageSurface {
                    width: spec.canvas[0],
                    height: spec.canvas[1],
                    rgba,
                },
                output,
                &|| cancel.check(),
            )
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(Job {
        task: Some(task),
        state: ImageProcessState::Pending,
    }))
}

pub(super) struct Admission {
    pub bytes: usize,
    pub scratch: usize,
    pub pixels: u64,
    pub work: u64,
}
pub(super) fn admission(spec: &GlyphImageSpec, options: &GlyphRasterOptions) -> Result<Admission> {
    admission_cell(
        &spec.font,
        spec.size,
        spec.canvas,
        spec.glyphs.len(),
        options,
    )
}
pub(super) fn admission_cell(
    font: &str,
    size: u16,
    canvas: [u32; 2],
    glyph_count: usize,
    options: &GlyphRasterOptions,
) -> Result<Admission> {
    options.validate()?;
    if font.is_empty() || font.len() > 256 || font.chars().any(char::is_control) {
        return Err(Error::Invalid("glyph image font alias"));
    }
    if !(1..=512).contains(&size) || canvas.iter().any(|v| *v == 0 || *v > 8192) {
        return Err(Error::Invalid("glyph image size/canvas"));
    }
    let pixels = u64::from(canvas[0]) * u64::from(canvas[1]);
    if pixels > 4 * 1024 * 1024 || glyph_count > 65536 {
        return Err(Error::Limit("glyph image pixels/count"));
    }
    let factor = u64::from(options.supersample);
    let working = pixels
        .checked_mul(factor * factor)
        .ok_or(Error::Limit("styled glyph working pixels"))?;
    let working_size = u64::from(size) * factor;
    let working_width = u64::from(canvas[0]) * factor;
    let working_height = u64::from(canvas[1]) * factor;
    let radius = u64::from(options.embolden_radius);
    let expanded = (working_width + 2 * radius) * (working_height + 2 * radius);
    if working > 4 * 1024 * 1024
        || working_size > 512
        || expanded * (2 * radius + 3) > 64 * 1024 * 1024
    {
        return Err(Error::Limit("styled glyph working/radius budget"));
    }
    let (stamp_pixels, stamp_work) = if let Some(offsets) = &options.coverage_offsets {
        // validate() has already bounded and rejected an empty stamp set.
        let min_x = offsets.iter().map(|p| i32::from(p[0])).min().unwrap();
        let min_y = offsets.iter().map(|p| i32::from(p[1])).min().unwrap();
        let max_x = offsets.iter().map(|p| i32::from(p[0])).max().unwrap();
        let max_y = offsets.iter().map(|p| i32::from(p[1])).max().unwrap();
        let width = working_width + 2 * radius + (max_x - min_x) as u64;
        let height = working_height + 2 * radius + (max_y - min_y) as u64;
        let pixels = width * height;
        let work = expanded * offsets.len() as u64;
        if pixels > 8 * 1024 * 1024 || work > 64 * 1024 * 1024 {
            return Err(Error::Limit("styled glyph stamp budget"));
        }
        (pixels, work)
    } else {
        (0, 0)
    };
    let side = working_size;
    if (glyph_count as u64)
        .checked_mul(side * side * 4)
        .is_none_or(|n| n > 64 * 1024 * 1024)
    {
        return Err(Error::Limit("glyph image work budget"));
    }
    // Output Vec->Arc transition plus bounded outline rasterizer scratch and
    // glyph positioning. Font source residency has its own retained lease.
    let plain = options.supersample == 1
        && options.binary_cutoff.is_none()
        && options.embolden_radius == 0
        && options.coverage_offsets.is_none()
        && options.coverage_lut.is_none()
        && options.size_basis == GlyphSizeBasis::LineHeight
        && options.origin == GlyphImageOrigin::Baseline
        && !options.underline
        && !options.strikeout;
    let extra = if plain {
        0
    } else {
        working as usize * 4 + expanded as usize * 2 + stamp_pixels as usize
    };
    Ok(Admission {
        bytes: pixels as usize * 4,
        scratch: pixels as usize * 4 + 32 * 1024 * 1024 + extra + glyph_count * 64,
        pixels,
        work: expanded * (2 * radius + 3) + stamp_work + (glyph_count as u64) * side * side * 4,
    })
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
    fn fixture() -> Arc<[u8]> {
        let mut b = b"TVP pre-rendered font\x1a".to_vec();
        b.extend([1, 2]);
        b.extend(1u32.to_le_bytes());
        b.extend(36u32.to_le_bytes());
        b.extend(38u32.to_le_bytes());
        b.extend((b'A' as u16).to_le_bytes());
        b.extend(58u32.to_le_bytes());
        for v in [2i16, 2, -1, 2, 3, 0, 4, 0] {
            b.extend(v.to_le_bytes());
        }
        b.extend([32, 0x43]);
        b.into()
    }
    fn face() -> (TextSystem, Arc<PreparedFontFace>, GlyphImageSpec) {
        let text = TextSystem::default();
        let face =
            Arc::new(PreparedFontFace::prepare_precomputed(fixture(), 24, &|| Ok(())).unwrap());
        text.publish_font(ClientId(1), "__font/1/authored", face.clone())
            .unwrap();
        (
            text,
            face,
            GlyphImageSpec {
                font: "__font/1/authored".into(),
                size: 24,
                canvas: [4, 3],
                glyphs: vec![Glyph {
                    glyph_id: 1,
                    x_26_6: 64,
                    y_26_6: 128,
                    advance_26_6: 192,
                }],
            },
        )
    }
    #[test]
    fn glyph_queue_cancellation_releases_face_and_all_charges_without_running_rasterizer() {
        for styled in [false, true] {
            let (text, face, mut spec) = face();
            let weak = Arc::downgrade(&face);
            let pool = TaskPool::new("glyph-cancel", 1, 2).unwrap();
            let (release, wait) = mpsc::channel();
            let (started, ready) = mpsc::channel();
            let mut blocker = pool
                .spawn(move |_| {
                    started.send(()).unwrap();
                    wait.recv().unwrap();
                    Ok(())
                })
                .unwrap();
            ready.recv_timeout(Duration::from_secs(5)).unwrap();
            let memory = Arc::new(AtomicUsize::new(0));
            let job = if styled {
                spec.size = 8;
                begin_styled(
                    ClientId(1),
                    StyledGlyphImageSpec {
                        glyphs: spec,
                        raster: GlyphRasterOptions {
                            supersample: 3,
                            binary_cutoff: Some(128),
                            embolden_radius: 1,
                            ..Default::default()
                        },
                    },
                    text.clone(),
                    pool.clone(),
                    memory.clone(),
                )
            } else {
                begin(
                    ClientId(1),
                    spec,
                    text.clone(),
                    pool.clone(),
                    memory.clone(),
                )
            }
            .unwrap();
            assert!(memory.load(Ordering::Acquire) > 0);
            drop(face);
            text.unpublish_font(ClientId(1), "__font/1/authored")
                .unwrap();
            drop(job);
            // Dropping a query never waits for an occupied worker. Its queued
            // closure owns the reservation until the cancellation fence executes.
            release.send(()).unwrap();
            let end = Instant::now() + Duration::from_secs(5);
            // The scheduler decrements outstanding before dropping the cancelled
            // queued closure. Wait for resource retirement itself, not that earlier
            // counter publication, to avoid observing a transient retained lease.
            while pool.stats().outstanding != 0
                || memory.load(Ordering::Acquire) != 0
                || weak.upgrade().is_some()
            {
                assert!(Instant::now() < end);
                let _ = blocker.try_take();
                std::thread::yield_now();
            }
            assert_eq!(memory.load(Ordering::Acquire), 0);
            assert!(weak.upgrade().is_none());
        }
    }
    #[test]
    fn glyph_admission_budget_and_queue_failure_leave_no_partial_reservations() {
        let (text, _face, spec) = face();
        let pool = TaskPool::new("glyph-budget", 1, 1).unwrap();
        let memory = Arc::new(AtomicUsize::new(BUDGET - 16));
        assert!(
            begin(
                ClientId(1),
                spec.clone(),
                text.clone(),
                pool.clone(),
                memory.clone()
            )
            .is_err()
        );
        assert_eq!(memory.load(Ordering::Acquire), BUDGET - 16);
        assert_eq!(pool.stats().outstanding, 0);
        memory.store(0, Ordering::Release);
        let permit = pool.reserve().unwrap();
        assert!(begin(ClientId(1), spec, text, pool.clone(), memory.clone()).is_err());
        assert_eq!(memory.load(Ordering::Acquire), 0);
        drop(permit);
    }
}
