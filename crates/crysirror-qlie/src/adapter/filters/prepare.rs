//! Native filter continuation: asset policy and ordered shared image jobs.
//! Intermediate generations remain unpublished and are released between steps;
//! only the final resource replaces the object's visible lease.
use super::*;
use crysirror_core::image_processing::{ImageJob, ImageResource};
use crysirror_image::affine_blend::AffineBlendByte128;
use std::collections::VecDeque;

pub(super) enum Step {
    Effects(Arc<[u8]>),
    Blend {
        path: String,
        spec: AffineBlendByte128,
        back: bool,
    },
}
enum Pending {
    Read {
        path: String,
        spec: AffineBlendByte128,
        back: bool,
        task: Task<Arc<[u8]>>,
    },
    Decode {
        path: String,
        spec: AffineBlendByte128,
        back: bool,
        job: ImageJob,
    },
    Process(ImageJob),
}
pub(in super::super) struct Preparation {
    id: String,
    slot: usize,
    source: String,
    steps: VecDeque<Step>,
    current: Arc<dyn ImageResource>,
    pending: Option<Pending>,
}
impl Preparation {
    pub(super) fn new(
        id: &str,
        slot: usize,
        source: &str,
        steps: Vec<Step>,
        current: Arc<dyn ImageResource>,
    ) -> Self {
        Self {
            id: id.into(),
            slot,
            source: source.into(),
            steps: steps.into(),
            current,
            pending: None,
        }
    }
    pub(super) fn matches(&self, id: &str, slot: usize, source: &str) -> bool {
        self.id == id && self.slot == slot && self.source == source
    }
    pub(super) fn poll(
        &mut self,
        h: &mut dyn HostServices,
        client: ClientId,
        pool: &TaskPool,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        loop {
            if let Some(pending) = self.pending.take() {
                match pending {
                    Pending::Read {
                        path,
                        spec,
                        back,
                        mut task,
                    } => {
                        let Some(bytes) = task
                            .try_take()
                            .with_context(|| format!("QLIE blend source {path}"))?
                        else {
                            self.pending = Some(Pending::Read {
                                path,
                                spec,
                                back,
                                task,
                            });
                            return Ok(None);
                        };
                        let job = h.image_metadata(client)?.begin_encoded(&path, bytes)?;
                        self.pending = Some(Pending::Decode {
                            path,
                            spec,
                            back,
                            job,
                        });
                    }
                    Pending::Decode {
                        path,
                        spec,
                        back,
                        mut job,
                    } => match job.poll()? {
                        ImageProcessState::Pending => {
                            self.pending = Some(Pending::Decode {
                                path,
                                spec,
                                back,
                                job,
                            });
                            return Ok(None);
                        }
                        ImageProcessState::Failed(error) => {
                            bail!("QLIE blend source {path}: {error}")
                        }
                        ImageProcessState::Ready(external) => {
                            //55DB28 restores the base alpha after the draw;
                            //the shared affine operation retains it directly.
                            let current = ImageInput::Resource(self.current.clone());
                            let external = ImageInput::Resource(external);
                            let (destination, source) = if back {
                                (external, current)
                            } else {
                                (current, external)
                            };
                            let input = ImageInput::Raster {
                                destination: Box::new(destination),
                                source: Box::new(source),
                            };
                            let effects = serde_json::to_vec(&[
                                crysirror_image::effects::ImageEffect::AffineBlend { spec },
                            ])?;
                            self.pending = Some(Pending::Process(
                                h.image_metadata(client)?
                                    .begin_process(input, effects.into())?,
                            ));
                        }
                    },
                    Pending::Process(mut job) => match job.poll()? {
                        ImageProcessState::Pending => {
                            self.pending = Some(Pending::Process(job));
                            return Ok(None);
                        }
                        ImageProcessState::Failed(error) => {
                            bail!("QLIE resource filter worker: {error}")
                        }
                        ImageProcessState::Ready(image) => self.current = image,
                    },
                }
                continue;
            }
            match self.steps.pop_front() {
                None => return Ok(Some(self.current.clone())),
                Some(Step::Effects(effects)) => {
                    self.pending = Some(Pending::Process(
                        h.image_metadata(client)?
                            .begin_process(ImageInput::Resource(self.current.clone()), effects)?,
                    ));
                }
                Some(Step::Blend { path, spec, back }) => {
                    let storage = h.storage(client)?;
                    let source = path.clone();
                    let task = pool.spawn(move |cancel| {
                        cancel.check()?;
                        let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &source)?)?;
                        let bytes =
                            crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
                        cancel.check()?;
                        Ok(Arc::<[u8]>::from(bytes))
                    })?;
                    self.pending = Some(Pending::Read {
                        path,
                        spec,
                        back,
                        task,
                    });
                }
            }
        }
    }
}
