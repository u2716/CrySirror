// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Client-scoped Core capability around the engine's detached audio sources.
use crysirror_audio::channel_replacement::{ChannelReplacementJob, PreparedChannelReplacement};
use crysirror_core::{service::*, state::ClientId, Error, Result};
use std::sync::{Arc, Mutex};

struct Resource {
    owner: Arc<()>,
    info: AudioReplacementInfo,
    prepared: Mutex<Option<PreparedChannelReplacement>>,
}
impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioReplacementResource")
            .field("info", &self.info)
            .finish()
    }
}
impl AudioReplacementResource for Resource {
    fn client(&self) -> ClientId {
        self.info.client
    }
    fn info(&self) -> AudioReplacementInfo {
        self.info
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
struct Job {
    owner: Arc<()>,
    info: AudioReplacementInfo,
    job: Option<ChannelReplacementJob>,
    state: AudioReplacementState,
}
fn error(error: anyhow::Error) -> Error {
    error
        .downcast_ref::<Error>()
        .cloned()
        .unwrap_or_else(|| Error::Io(format!("{error:#}")))
}
impl AudioReplacementProcess for Job {
    fn client(&self) -> ClientId {
        self.info.client
    }
    fn poll(&mut self) -> Result<AudioReplacementState> {
        if let Some(job) = &mut self.job {
            match job.poll() {
                Ok(None) => {}
                Ok(Some(prepared)) => {
                    self.state = AudioReplacementState::Ready(Arc::new(Resource {
                        owner: self.owner.clone(),
                        info: self.info,
                        prepared: Mutex::new(Some(prepared)),
                    }));
                    self.job = None;
                }
                Err(failure) => {
                    self.state = AudioReplacementState::Failed(error(failure));
                    self.job = None;
                }
            }
        }
        Ok(self.state.clone())
    }
}
pub(crate) fn begin(
    audio: &crysirror_audio::channels::ChannelAudio,
    owner: Arc<()>,
    client: ClientId,
    requests: Vec<AudioRequest>,
    storage: Arc<dyn ClientStorage>,
    pool: crysirror_tasks::TaskPool,
) -> Result<AudioReplacementJob> {
    let info = AudioReplacementInfo {
        client,
        channels: requests.len(),
    };
    let job = audio
        .prepare_client_replacement(client.0, requests, storage, pool)
        .map_err(error)?;
    Ok(Box::new(Job {
        owner,
        info,
        job: Some(job),
        state: AudioReplacementState::Pending,
    }))
}
/// Validate the real implementation/host/client and consume its one detached
/// value. Any later failure only drops prepared sources; live audio is intact.
pub(crate) fn take(
    owner: &Arc<()>,
    client: ClientId,
    source: &Arc<dyn AudioReplacementResource>,
) -> Result<PreparedChannelReplacement> {
    let actual = resource(owner, client, source)?;
    actual
        .prepared
        .lock()
        .map_err(|_| Error::Io("audio replacement resource poisoned".into()))?
        .take()
        .ok_or(Error::Invalid("audio replacement already consumed"))
}
fn resource<'a>(
    owner: &Arc<()>,
    client: ClientId,
    source: &'a Arc<dyn AudioReplacementResource>,
) -> Result<&'a Resource> {
    let actual = source
        .as_any()
        .downcast_ref::<Resource>()
        .ok_or(Error::Invalid("foreign audio replacement implementation"))?;
    if actual.info.client != client || !Arc::ptr_eq(owner, &actual.owner) {
        return Err(Error::Invalid("foreign audio replacement owner"));
    }
    Ok(actual)
}
pub(crate) fn validate(
    owner: &Arc<()>,
    client: ClientId,
    source: &Arc<dyn AudioReplacementResource>,
) -> Result<()> {
    if resource(owner, client, source)?
        .prepared
        .lock()
        .map_err(|_| Error::Io("audio replacement resource poisoned".into()))?
        .is_none()
    {
        return Err(Error::Invalid("audio replacement already consumed"));
    }
    Ok(())
}
