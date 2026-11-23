// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FULLANIME79's global last-print cache and original frame-name policy.
//! Resource reads and palette/index processing must be retained worker jobs.
use anyhow::{Result, ensure};

pub(crate) fn parameters(values: &[i32]) -> [i32; 16] {
    std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(match i {
            0 => 20,
            2 => 1,
            _ => 0,
        })
    })
    // Omitted4..15 were uninitialized native stack words. They are not read
    // by Print; zero defaults define them while preserving supplied/save data.
}
#[derive(Clone, Debug)]
pub(crate) struct FullAnimeClass {
    /// One field on CEffectFullAnime, shared across all assigned native slots.
    /// SetParam on ANY slot resets it; this is not an EFFECT save-body word.
    pub last_print: i32,
}
impl Default for FullAnimeClass {
    fn default() -> Self {
        // The original constructor left this field uninitialized before its
        // first SetParam. Define the same invalid-frame sentinel used there.
        Self { last_print: -1 }
    }
}

use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    path::{Mount, VfsPath},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::indexed_plane::IndexedPlane;
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
enum Work {
    Read(Task<Option<IndexedPlane>>),
    Pixels(ImageJob, Arc<[u8]>),
    Colors(ImageJob),
}
#[derive(Clone)]
pub(crate) struct Prepared(pub Option<Arc<dyn ImageResource>>);
/// The file-view and pool belong to the current native session. No ambient
/// filesystem path or separate GPK resource policy is introduced by FULLANIME.
#[derive(Clone)]
pub(crate) struct PreparationContext {
    pub resources: crate::resource_bridge::NativeResources,
    pub pool: TaskPool,
}
pub(crate) struct FullAnimePreparation {
    client: ClientId,
    size: [u32; 2],
    work: Option<Work>,
    ready: Option<Prepared>,
    failed: Option<String>,
    pub api_calls: u64,
}
impl FullAnimePreparation {
    pub(crate) fn from_context(
        client: ClientId,
        context: &PreparationContext,
        path: String,
        size: [u32; 2],
    ) -> Result<Self> {
        ensure!(
            context.resources.client() == client,
            "FULLANIME foreign file view"
        );
        Self::begin(
            client,
            context.resources.file_view(),
            path,
            size,
            &context.pool,
        )
    }
    pub(crate) fn begin(
        client: ClientId,
        open: crate::resource_bridge::SourceOpener,
        path: String,
        size: [u32; 2],
        pool: &TaskPool,
    ) -> Result<Self> {
        ensure!(client.0 != 0, "FULLANIME resource client");
        VfsPath::new(Mount::Asset, &path)?;
        let pixels = crysirror_image::rgba_len(size[0], size[1])? / 4;
        ensure!(
            pixels <= crysirror_image::effects::MAX_EFFECT_PIXELS as usize,
            "FULLANIME preparation pixels"
        );
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            let Some(reader) = open(&path, &cancel)? else {
                return Ok(None);
            };
            let bytes = crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
            cancel.check()?;
            let plane = crysirror_formats_blackcyc::full_anime::linear_indexed_dwq(&bytes, size)?;
            cancel.check()?;
            Ok(Some(plane))
        })?;
        Ok(Self {
            client,
            size,
            work: Some(Work::Read(task)),
            ready: None,
            failed: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Prepared>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("FULLANIME resource preparation: {error}");
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.poll_inner(host);
        if let Err(error) = &result {
            self.failed = Some(format!("{error:#}"));
        }
        result
    }
    fn poll_inner(&mut self, host: &mut dyn HostServices) -> Result<Option<Prepared>> {
        loop {
            let work = self
                .work
                .take()
                .ok_or_else(|| anyhow::anyhow!("FULLANIME worker state absent"))?;
            self.work = Some(match work {
                Work::Read(mut task) => {
                    let Some(plane) = task.try_take()? else {
                        self.work = Some(Work::Read(task));
                        return Ok(None);
                    };
                    let Some(plane) = plane else {
                        let ready = Prepared(None);
                        self.ready = Some(ready.clone());
                        return Ok(Some(ready));
                    };
                    let effects: Arc<[u8]> = serde_json::to_vec(&plane.effects()?)?.into();
                    let job = host
                        .image_metadata(self.client)?
                        .begin_packed_pixels(plane.packed()?)?;
                    self.api_calls += 1;
                    Work::Pixels(job, effects)
                }
                Work::Pixels(mut job, effects) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Pixels(job, effects));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("FULLANIME index worker: {error}")
                    }
                    ImageProcessState::Ready(raw) => {
                        ensure!(
                            raw.client() == self.client && raw.size() == self.size,
                            "FULLANIME foreign index plane"
                        );
                        let job = host
                            .image_metadata(self.client)?
                            .begin_process(ImageInput::Resource(raw), effects)?;
                        self.api_calls += 1;
                        Work::Colors(job)
                    }
                },
                Work::Colors(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Colors(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("FULLANIME palette worker: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client && image.size() == self.size,
                            "FULLANIME foreign colour plane"
                        );
                        let ready = Prepared(Some(image));
                        self.ready = Some(ready.clone());
                        return Ok(Some(ready));
                    }
                },
            });
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct FrameRequest {
    pub frame: i32,
    pub filename: String,
}
impl FullAnimeClass {
    pub(crate) fn set_param(&mut self) {
        self.last_print = -1;
    }
    pub(crate) fn request(
        &mut self,
        p: &[i32; 16],
        count: i32,
        frames: i32,
    ) -> Result<Option<FrameRequest>> {
        let (duration, mut frame) = if frames < 1 { (1, 0) } else { (frames, count) };
        if frame >= duration {
            frame = duration - 1;
        }
        frame /= p[2].max(1);
        if frame == self.last_print && p[3] != 0 {
            return Ok(None);
        }
        // Print writes the global number BEFORE opening the file. A missing
        // optional file therefore still suppresses a duplicate later Print.
        self.last_print = frame;
        let anime = p[1];
        let filename = format!(
            "dwq/an/an{}{}{}{}{}{}.dwq",
            anime / 100,
            (anime / 10) % 10,
            anime % 10,
            frame / 100,
            (frame / 10) % 10,
            frame % 10
        );
        ensure!(
            filename.len() < 1024,
            "FULLANIME original filename capacity"
        );
        Ok(Some(FrameRequest { frame, filename }))
    }
}
