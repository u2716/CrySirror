// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAMON's class-wide field identity over a scoped engine data-image lease.
//! CPU field creation and packed publication are retained, cancellable stages;
//! no native slot observes a half-prepared vector table or repeats its setter.
use crate::native_hamon::VectorKey;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState, ImageResource, PackedImageSpec},
    service::HostServices,
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Field {
    pub key: VectorKey,
    pub image: Arc<dyn ImageResource>,
}
enum Work {
    Compute(Task<PackedImageSpec>),
    Publish(ImageJob),
}
pub(crate) struct Preparation {
    client: ClientId,
    key: VectorKey,
    size: [u32; 2],
    work: Option<Work>,
    ready: Option<Field>,
    failed: Option<String>,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        key: VectorKey,
        old: Option<&Field>,
        pool: &TaskPool,
    ) -> Result<Self> {
        use crysirror_image::packed_radial_field::{PackedRadialField, RadialSquaredMath};
        ensure!(client.0 != 0, "NNN HAMON vector client");
        let spec = PackedRadialField {
            canvas: key.canvas,
            center: key.center,
            x_lanes: [0, 1, 1, 1],
            squared_math: RadialSquaredMath::Wrapping32,
        };
        let size = spec.storage_size()?;
        if let Some(old) = old.filter(|v| v.key == key) {
            ensure!(
                old.image.client() == client && old.image.size() == size,
                "NNN HAMON cached vector scope"
            );
            return Ok(Self {
                client,
                key,
                size,
                work: None,
                ready: Some(old.clone()),
                failed: None,
                api_calls: 0,
            });
        }
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            spec.prepare(&|| cancel.check())
        })?;
        Ok(Self {
            client,
            key,
            size,
            work: Some(Work::Compute(task)),
            ready: None,
            failed: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Field>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN HAMON vector preparation: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failed = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Field>> {
        loop {
            match self.work.take().context("NNN HAMON vector work absent")? {
                Work::Compute(mut task) => {
                    let Some(packed) = task.try_take()? else {
                        self.work = Some(Work::Compute(task));
                        return Ok(None);
                    };
                    self.work = Some(Work::Publish(
                        host.image_metadata(self.client)?
                            .begin_packed_pixels(packed)?,
                    ));
                    self.api_calls += 1;
                }
                Work::Publish(mut job) => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.work = Some(Work::Publish(job));
                        return Ok(None);
                    }
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("NNN HAMON vector publication: {error}")
                    }
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client && image.size() == self.size,
                            "NNN HAMON vector result scope"
                        );
                        let ready = Field {
                            key: self.key,
                            image,
                        };
                        self.ready = Some(ready.clone());
                        return Ok(Some(ready));
                    }
                },
            }
        }
    }
}
