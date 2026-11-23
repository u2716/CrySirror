//! Native slot I/O and thumbnail worker fences. Rasterization and resizing are
//! shared engine operations; the adapter retains only native selection policy.
use super::*;
use crate::{
    machine::save_ops::{SaveReply, SaveRequest},
    native_save::{BODY_BYTES, CivilFields, NativeSaveBody, SlotKind},
};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    service::{CivilTimeQuery, CivilTimeState, StorageWrite, StorageWriteState},
};

#[derive(Default)]
pub(super) struct SaveService {
    active: Option<Pending>,
    pub(super) restore: Option<super::save_resources::RestoreResources>,
}
impl SaveService {
    pub(super) fn busy(&self)->bool {self.active.is_some()||self.restore.is_some()}
}
struct Pending {
    client: ClientId,
    request: SaveRequest,
    phase: Phase,
}
enum Phase {
    Clock(Box<dyn CivilTimeQuery>),
    Capture(ImageJob),
    Resize(ImageJob),
    Worker(Receiver<std::result::Result<SaveReply, String>>),
    Publish {
        job: Box<dyn StorageWrite>,
        body: Option<(VfsPath, Arc<[u8]>)>,
    },
}
fn spawn_worker(task: impl FnOnce() -> Result<SaveReply> + Send + 'static) -> Result<Phase> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("circus-native-save".into())
        .spawn(move || {
            let result = task().map_err(|e| format!("{e:#}"));
            let _ = tx.send(result);
        })?;
    Ok(Phase::Worker(rx))
}
fn thumbnail_canvas(resources: &[u8], client: ClientId) -> Result<u32> {
    ensure!(
        resources.len() <= 8 * 1024 * 1024,
        "Circus native thumbnail declaration budget"
    );
    let value: Value = serde_json::from_slice(resources)?;
    ensure!(
        value["owner"].as_u64() == Some(u64::from(client.0)),
        "Circus foreign checkpoint thumbnail"
    );
    // A generic all-UI capture does not prove native4226B0(1)'s selection.
    ensure!(
        value["native_thumbnail_selection"].as_str() == Some("4226b0:1"),
        "Circus checkpoint lacks source-correct native thumbnail selection"
    );
    Ok(u32::try_from(
        value["native_thumbnail_canvas"]
            .as_u64()
            .context("Circus checkpoint lacks frozen native thumbnail canvas")?,
    )?)
}
impl SaveService {
    fn begin(
        &mut self,
        request: &SaveRequest,
        client: ClientId,
        storage: Arc<dyn ClientStorage>,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(self.active.is_none(), "Circus native save already pending");
        let phase = match request {
            SaveRequest::CurrentThumbnailSource => {
                bail!("Circus current440 selection requires the owner resource bridge")
            }
            SaveRequest::Restore { .. } => {
                bail!("Circus resource restore requires owner resource bridge")
            }
            SaveRequest::CivilTime => Phase::Clock(host.begin_civil_time(client)?),
            SaveRequest::Thumbnail {
                checkpoint_resources,
            } => Phase::Capture(
                host.begin_canvas_image(client, thumbnail_canvas(checkpoint_resources, client)?)?,
            ),
            SaveRequest::Read { slot, kind } => {
                let path = VfsPath::new(Mount::Save, &kind.filename(*slot))?;
                spawn_worker(move || {
                    let Some(metadata) = storage.metadata(Mount::Save, path.relative())? else {
                        return Ok(SaveReply::Read(None));
                    };
                    ensure!(!metadata.is_directory, "Circus native slot is a directory");
                    let reader = storage.open_read(&path)?;
                    ensure!(
                        reader.len()? >= BODY_BYTES as u64,
                        "Circus native slot body truncated"
                    );
                    let mut bytes = vec![0; BODY_BYTES];
                    crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut bytes)?;
                    Ok(SaveReply::Read(Some(bytes)))
                })?
            }
            SaveRequest::Write {
                slot,
                kind,
                body,
                thumbnail_png,
                thumbnail_expected,
            } => {
                NativeSaveBody::from_exact(body)?;
                ensure!(
                    *thumbnail_expected == thumbnail_png.is_some()
                        && (!*thumbnail_expected || kind.needs_thumbnail(*slot)),
                    "Circus native slot thumbnail requirement mismatch"
                );
                let body_path = VfsPath::new(Mount::Save, &kind.filename(*slot))?;
                let body: Arc<[u8]> = Arc::from(body.clone());
                if let Some(png) = thumbnail_png {
                    ensure!(
                        png.len() <= 1024 * 1024,
                        "Circus native thumbnail byte budget"
                    );
                    // Produced by our shared resize+PNG worker. Decode again
                    // before publishing a caller-supplied or restored request.
                    let image = crysirror_image::decode_standard(png)?;
                    ensure!(
                        [image.width, image.height] == [128, 72],
                        "Circus native thumbnail must be128x72"
                    );
                    let path = VfsPath::new(Mount::Save, &format!("DC4PHDL{slot:02}.png"))?;
                    Phase::Publish {
                        job: storage.begin_write_atomic(&path, Arc::from(png.clone()))?,
                        body: Some((body_path, body)),
                    }
                } else {
                    Phase::Publish {
                        job: storage.begin_write_atomic(&body_path, body)?,
                        body: None,
                    }
                }
            }
        };
        self.active = Some(Pending {
            client,
            request: request.clone(),
            phase,
        });
        Ok(())
    }
    fn poll(
        &mut self,
        request: &SaveRequest,
        client: ClientId,
        storage: Arc<dyn ClientStorage>,
        host: &mut dyn HostServices,
    ) -> Result<Option<SaveReply>> {
        if self.active.is_none() {
            self.begin(request, client, storage.clone(), host)?;
            return Ok(None);
        }
        let pending = self.active.as_mut().unwrap();
        ensure!(
            pending.client == client && pending.request == *request,
            "Circus native save completion ownership mismatch"
        );
        let result = match &mut pending.phase {
            Phase::Clock(query) => {
                ensure!(
                    query.client() == client,
                    "Circus foreign native civil-clock query"
                );
                match query.poll()? {
                    CivilTimeState::Pending => return Ok(None),
                    CivilTimeState::Failed(e) => bail!("Circus native civil clock failed: {e}"),
                    CivilTimeState::Ready(time) => {
                        ensure!(
                            time.client == client,
                            "Circus foreign native civil-clock receipt"
                        );
                        Some(SaveReply::CivilTime(CivilFields {
                            year: u16::try_from(time.year)?,
                            month: time.month,
                            day: time.day,
                            hour: time.hour,
                            minute: time.minute,
                            second: time.second,
                        }))
                    }
                }
            }
            Phase::Capture(job) => match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(e) => {
                    bail!("Circus native thumbnail capture failed: {e}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == client,
                        "Circus foreign native thumbnail capture"
                    );
                    ensure!(
                        image.size() == [1280, 720],
                        "Circus native thumbnail source extent"
                    );
                    let effects =
                        serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::Resize {
                            width: 128,
                            height: 72,
                        }])?;
                    pending.phase = Phase::Resize(
                        host.image_metadata(client)?
                            .begin_process(ImageInput::Resource(image), Arc::from(effects))?,
                    );
                    return Ok(None);
                }
            },
            Phase::Resize(job) => match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(e) => bail!("Circus native thumbnail resize failed: {e}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == client && image.size() == [128, 72],
                        "Circus native thumbnail resize receipt mismatch"
                    );
                    pending.phase = spawn_worker(move || {
                        let image = ImageSurface {
                            width: 128,
                            height: 72,
                            rgba: image.rgba().to_vec(),
                        };
                        Ok(SaveReply::Thumbnail(crysirror_image::encode_png(&image)?))
                    })?;
                    return Ok(None);
                }
            },
            Phase::Worker(receiver) => match receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => bail!("Circus native save worker disconnected"),
                Ok(result) => Some(result.map_err(anyhow::Error::msg)?),
            },
            Phase::Publish { job, body } => match job.poll()? {
                StorageWriteState::Pending => return Ok(None),
                StorageWriteState::Failed(e) => {
                    bail!("Circus native slot atomic publication failed: {e}")
                }
                StorageWriteState::Ready => {
                    if let Some((path, bytes)) = body.as_ref() {
                        // Native421750 precedes40C360. Each file is atomic;
                        // a PNG receipt alone never acknowledges a slot body.
                        let next = storage.begin_write_atomic(path, bytes.clone())?;
                        *job = next;
                        *body = None;
                        return Ok(None);
                    }
                    Some(SaveReply::Written)
                }
            },
        };
        self.active = None;
        Ok(result)
    }
}
impl Bridge<'_> {
    pub(super) fn save_native(&mut self, request: &SaveRequest) -> Result<Option<SaveReply>> {
        if matches!(request, SaveRequest::CurrentThumbnailSource) {
            ensure!(self.services.saves.active.is_none() && self.services.saves.restore.is_none(), "Circus current440 selection during another native save job");
            return Ok(self.current_thumbnail_source()?.map(SaveReply::CurrentThumbnailSource));
        }
        if let SaveRequest::Restore { plan } = request {
            return self.restore_native_resources(plan);
        }
        ensure!(
            self.services.saves.restore.is_none(),
            "Circus native save request while resource restore pending"
        );
        let storage = self
            .services
            .storage
            .clone()
            .context("Circus native save storage unavailable")?;
        self.services
            .saves
            .poll(request, self.client, storage, self.host)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generic_checkpoint_or_foreign_selection_cannot_be_a_native_thumbnail() {
        let generic =
            serde_json::to_vec(&json!({"schema":1,"owner":7,"scene":{},"ui":{}})).unwrap();
        assert!(thumbnail_canvas(&generic, ClientId(7)).is_err());
        let selected=serde_json::to_vec(&json!({"owner":7,"native_thumbnail_selection":"4226b0:1","native_thumbnail_canvas":0x33000003u32})).unwrap();
        assert_eq!(
            thumbnail_canvas(&selected, ClientId(7)).unwrap(),
            0x33000003
        );
        assert!(thumbnail_canvas(&selected, ClientId(8)).is_err());
    }
}
