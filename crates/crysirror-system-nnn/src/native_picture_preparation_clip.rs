// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-owned native CharaClip reconstruction. The decoded resource remains
//! immutable and scoped; native code supplies header/padding policy while the
//! shared image query performs channel traversal with cancellation.
use crate::native_picture_clip::CharaClip;
use anyhow::{Context, Result, ensure};
use crysirror_core::image_processing::ImageResource;
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
enum Work {
    Pending(Task<CharaClip>),
    Ready(CharaClip),
}
pub(crate) struct PictureClipPreparation {
    pool: TaskPool,
    configuration: Option<(bool, Option<[i32; 4]>)>,
    work: Option<Work>,
}
impl PictureClipPreparation {
    pub(crate) fn new(pool: TaskPool) -> Self {
        Self {
            pool,
            configuration: None,
            work: None,
        }
    }
    pub(crate) fn configure(&mut self, has_mask: bool, header: Option<[i32; 4]>) -> Result<()> {
        ensure!(
            self.configuration.is_none() && self.work.is_none(),
            "NNN duplicate source clip metadata"
        );
        self.configuration = Some((has_mask, header));
        Ok(())
    }
    pub(crate) fn poll(&mut self, image: Arc<dyn ImageResource>) -> Result<Option<CharaClip>> {
        if self.work.is_none() {
            let (has_mask, header) = self
                .configuration
                .context("NNN image clip metadata absent")?;
            if !has_mask {
                self.work = Some(Work::Ready(CharaClip::full(image.size())));
            } else if let Some(insets) = header {
                self.work = Some(Work::Ready(CharaClip::header(image.size(), insets)));
            } else {
                self.work = Some(Work::Pending(self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let bytes = image.rgba();
                    let bounds = crysirror_image::coverage_bounds::CoverageBounds {
                        channel: 3,
                        not_equal: 0,
                    }
                    .query(image.size(), &bytes, &|| cancel.check())?;
                    CharaClip::from_coverage(image.size(), bounds)
                })?));
            }
        }
        if let Some(Work::Pending(task)) = self.work.as_mut() {
            let Some(clip) = task.try_take()? else {
                return Ok(None);
            };
            self.work = Some(Work::Ready(clip));
        }
        let Some(Work::Ready(clip)) = &self.work else {
            unreachable!();
        };
        ensure!(
            clip.bounds().iter().all(|v| v.unsigned_abs() <= 1_000_000),
            "NNN native source clip bounds"
        );
        Ok(Some(*clip))
    }
}
