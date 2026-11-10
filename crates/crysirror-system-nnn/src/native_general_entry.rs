// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonGeneral's entry countdown over the shared global overlap buffers.
//! Init captures the new mode through CreateStartScreen/Print before this
//! player starts. Native background scripts and mode input stay suspended while
//! an entry Calcu is active, including the final complete-image update.
use crate::config::Names;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::temporal_pattern::PatternRng;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Overlap,
    Wipe(i32),
    /// PrintEnterFadeIn has no branch for values below-1. GeneralInit still
    /// captures the mode and GeneralCalcu still spends the configured ticks.
    NoPrint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModeEntry {
    pub kind: EntryKind,
    pub frames: i32,
    /// Only the -1 entry overwrites the old overlap buffer with this colour.
    pub fill_old: Option<[u8; 4]>,
}
impl ModeEntry {
    pub(crate) fn read(table: &Names) -> Result<Option<Self>> {
        let code = table.integer(b"enterFadeInType")?.unwrap_or(0);
        if code == 0 {
            return Ok(None);
        }
        let frames = table.integer(b"enterFadeInTime")?.unwrap_or(10);
        ensure!(
            (-864000..=1000000).contains(&frames),
            "NNN General entry frame budget"
        );
        let kind = match code {
            -1 => EntryKind::Overlap,
            n if n >= 1 => {
                ensure!(
                    crate::native_simple_wipe::supported(n - 1),
                    "NNN General entry wipe {n}"
                );
                EntryKind::Wipe(n - 1)
            }
            _ => EntryKind::NoPrint,
        };
        let fill_old = if code == -1 {
            let r = table.integer(b"enterFadeInColorR")?.unwrap_or(0);
            let g = table.integer(b"enterFadeInColorG")?.unwrap_or(0);
            let b = table.integer(b"enterFadeInColorB")?.unwrap_or(0);
            let word = r.wrapping_shl(16) | g.wrapping_shl(8) | b;
            Some([(word >> 16) as u8, (word >> 8) as u8, word as u8, 255])
        } else {
            None
        };
        Ok(Some(Self {
            kind,
            frames,
            fill_old,
        }))
    }
}
pub(crate) struct EntryFadePlayer {
    client: ClientId,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    spec: ModeEntry,
    remaining: i32,
    frame_ms: u64,
    next_ms: Option<u64>,
    job: Option<ImageJob>,
    random: Option<PatternRng>,
    tables: crate::native_simple_wipe::Tables,
    active: bool,
    filling: bool,
}
impl EntryFadePlayer {
    pub(crate) fn begin(
        host: &mut dyn HostServices,
        client: ClientId,
        old: Arc<dyn ImageResource>,
        new: Arc<dyn ImageResource>,
        spec: ModeEntry,
        frame_ms: u64,
    ) -> Result<Self> {
        ensure!(
            old.client() == client
                && new.client() == client
                && old.size() == new.size()
                && (1..=60000).contains(&frame_ms),
            "NNN General entry image/clock scope"
        );
        let job = spec
            .fill_old
            .map(|color| {
                host.image_metadata(client)?.begin_process(
                    ImageInput::Solid {
                        size: old.size(),
                        color,
                    },
                    b"[]".as_slice().into(),
                )
            })
            .transpose()?;
        Ok(Self {
            client,
            old,
            new,
            spec,
            remaining: spec.frames,
            frame_ms,
            next_ms: None,
            filling: job.is_some(),
            job,
            random: None,
            tables: Default::default(),
            active: spec.frames > 0,
        })
    }
    pub(crate) fn dependency_pending(&self) -> bool {
        self.job.is_some()
    }
    pub(crate) fn active(&self) -> bool {
        self.active
    }
    pub(crate) fn resumes_mode_at(&self, now_ms: u64) -> bool {
        self.job.is_none() && self.remaining <= 0 && self.next_ms.is_none_or(|next| now_ms >= next)
    }
    pub(crate) fn final_frame_ready(&self) -> bool {
        self.job.is_none() && !self.filling && self.remaining <= 0
    }
    pub(crate) fn needs_random_context(&self) -> bool {
        matches!(self.spec.kind,EntryKind::Wipe(kind) if crate::native_simple_wipe::needs_random(kind))
            && self.random.is_none()
            && self.job.is_none()
            && self.remaining > 0
    }
    pub(crate) fn bind_random(
        &mut self,
        random: PatternRng,
        tables: crate::native_simple_wipe::Tables,
    ) {
        if self.random.is_none() {
            self.random = Some(random);
            self.tables = tables;
        }
    }
    pub(crate) fn take_random_state(
        &mut self,
    ) -> Option<(PatternRng, crate::native_simple_wipe::Tables)> {
        self.random.take().map(|r| (r, self.tables.clone()))
    }
    pub(crate) fn poll_dependency(&mut self) -> Result<Option<Arc<dyn ImageResource>>> {
        let Some(job) = self.job.as_mut() else {
            return Ok(None);
        };
        match job.poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN General shared entry fade: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.new.size(),
                    "NNN General entry result scope"
                );
                self.job = None;
                if self.filling {
                    self.old = image;
                    self.filling = false;
                    Ok(None)
                } else {
                    Ok(Some(image))
                }
            }
        }
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
        skip: bool,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if self.job.is_some() {
            return self.poll_dependency();
        }
        if !self.active {
            return Ok(None);
        }
        let now = host.now_ms();
        if self.next_ms.is_some_and(|next| now < next) {
            return Ok(None);
        }
        if self.remaining <= 0 {
            // The last entry Calcu still returns-1 even after it decremented
            // the counter to0. Ordinary mode Calcu starts on the next tick.
            self.active = false;
            return Ok(None);
        }
        self.remaining -= 1;
        // CommonGeneral's three skip-ok flags are constructor constants1;
        // GetFadeInOutSetup never reads similarly named configuration keys.
        if skip {
            self.remaining = 0;
        }
        self.next_ms = Some(
            now.checked_add(self.frame_ms)
                .context("NNN General entry clock overflow")?,
        );
        let count = (self.spec.frames - self.remaining) as u32;
        let duration = self.spec.frames.max(1) as u32;
        let kind = match self.spec.kind {
            EntryKind::NoPrint => return Ok(None),
            EntryKind::Overlap => 0,
            EntryKind::Wipe(kind) => kind,
        };
        let (input, effects) = if crate::native_simple_wipe::needs_random(kind) {
            crate::native_simple_wipe::plan_with_random(
                kind,
                self.old.clone(),
                self.new.clone(),
                count,
                duration,
                self.random
                    .as_mut()
                    .context("NNN General entry RNG owner absent")?,
                &mut self.tables,
            )?
        } else {
            crate::native_simple_wipe::plan(
                kind,
                self.old.clone(),
                self.new.clone(),
                count,
                duration,
            )?
        };
        self.job = Some(
            host.image_metadata(self.client)?
                .begin_process(input, serde_json::to_vec(&effects)?.into())?,
        );
        Ok(None)
    }
    /// The original overlap pair is global, shared by every General mode.
    /// Entry fills/captures remain available to a later exit type-1 path.
    pub(crate) fn retained_buffers(&self) -> (Arc<dyn ImageResource>, Arc<dyn ImageResource>) {
        (self.old.clone(), self.new.clone())
    }
}
