// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original CCommonMessageWindow variant binding over shared resource/image
//! workers. The VM retains filename/plane policy, never a private raster path.
use crate::{
    native_services_bootstrap::{
        NativeWindowSource, PreparedNativeServices, resolved_window_resource,
    },
    native_text::MessageWindow,
    resource_bridge::NativeResources,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    service::HostServices,
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
struct Input {
    source: NativeWindowSource,
    input: ImageInput,
    effects: Arc<[u8]>,
}
struct Work {
    source: NativeWindowSource,
    job: ImageJob,
    ready: Option<MessageWindow>,
}
struct Pending {
    number: i32,
    task: Option<Task<Vec<Input>>>,
    work: Vec<Work>,
    paths: Vec<String>,
}
struct Policy {
    services: PreparedNativeServices,
    resources: NativeResources,
    levels: u32,
    pool: TaskPool,
    ids: [u32; 2],
}
pub(crate) struct WindowBinding {
    policy: Arc<Policy>,
    loaded: Option<i32>,
    pending: Option<Pending>,
    owned: Vec<String>,
    pub(crate) api_calls: u64,
}
impl WindowBinding {
    pub(crate) fn new(
        services: PreparedNativeServices,
        resources: NativeResources,
        levels: u32,
        pool: TaskPool,
        ids: [u32; 2],
    ) -> Self {
        Self {
            policy: Arc::new(Policy {
                services,
                resources,
                levels,
                pool,
                ids,
            }),
            loaded: Some(0),
            pending: None,
            owned: Vec::new(),
            api_calls: 0,
        }
    }
    pub(crate) fn detached(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            loaded: None,
            pending: None,
            owned: Vec::new(),
            api_calls: 0,
        }
    }
    pub(crate) fn owned_paths(&self) -> &[String] {
        &self.owned
    }
    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn loaded(&self, number: i32) -> bool {
        self.loaded == Some(number)
    }
    pub(crate) fn poll(
        &mut self,
        client: ClientId,
        number: i32,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<MessageWindow>>> {
        if self.loaded(number) {
            return Ok(None);
        }
        if self.pending.is_none() {
            let policy = self.policy.clone();
            let permit = policy.pool.reserve()?;
            let task = permit.spawn(move |cancel| {
                cancel.check()?;
                let mut inputs = Vec::new();
                if let Some(windows) = policy.services.windows(number, policy.levels)? {
                    for source in std::iter::once(windows.body).chain(windows.name) {
                        let decoded = policy.resources.image_source(&source.picture, &cancel)?;
                        let effects = if decoded.has_mask {
                            Vec::new()
                        } else {
                            vec![crysirror_image::effects::ImageEffect::SpriteAlpha {
                                spec: crysirror_image::sprite_alpha::SpriteAlpha::CornerKey {
                                    right: false,
                                },
                            }]
                        };
                        inputs.push(Input {
                            source,
                            input: decoded.input,
                            effects: serde_json::to_vec(&effects)?.into(),
                        });
                    }
                }
                cancel.check()?;
                Ok(inputs)
            })?;
            self.pending = Some(Pending {
                number,
                task: Some(task),
                work: Vec::new(),
                paths: Vec::new(),
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(
            pending.number == number,
            "NNN pending window variant changed"
        );
        if let Some(task) = pending.task.as_mut() {
            let Some(inputs) = task.try_take()? else {
                return Ok(None);
            };
            pending.task = None;
            for input in inputs {
                pending.work.push(Work {
                    source: input.source,
                    job: host
                        .image_metadata(client)?
                        .begin_process(input.input, input.effects)?,
                    ready: None,
                });
                self.api_calls += 1;
            }
        }
        for (index, work) in pending.work.iter_mut().enumerate() {
            if work.ready.is_some() {
                continue;
            }
            match work.job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN native window binding: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(image.client() == client, "NNN native window foreign image");
                    let id = *self
                        .policy
                        .ids
                        .get(index)
                        .context("NNN native window source count")?;
                    let published = host.publish_image_resource(
                        client,
                        0x66010000 + index as u32,
                        image.clone(),
                    )?;
                    self.api_calls += 1;
                    let path = published.path.relative().to_owned();
                    pending.paths.push(path.clone());
                    work.ready = Some(resolved_window_resource(id, &work.source, path, image)?);
                }
            }
        }
        let pending = self.pending.take().unwrap();
        for path in self.owned.drain(..) {
            host.release_image_resource(client, &path)?;
            self.api_calls += 1;
        }
        self.owned = pending.paths;
        self.loaded = Some(number);
        Ok(Some(
            pending
                .work
                .into_iter()
                .map(|work| work.ready.unwrap())
                .collect(),
        ))
    }
}
