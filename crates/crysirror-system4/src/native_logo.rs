// Copyright (C)2026 CrySirror contributors. GPL-2.0-or-later.
//! AliceLogo2 native policy from the complete reviewed four-season native
//! functions and their called helpers (10001130..10009CA0). The pinned public
//! AliceLogo2.c has warning stubs, which are not adopted. ALK/QNT, byte sampling,
//! composition, PE data extraction, PCM and presentation belong to shared APIs.
mod timeline;
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    path::{Mount, VfsPath},
    service::{
        AudioParameters, AudioPreparePhase, AudioRequest, AudioResourceHandle, CivilTimeQuery,
        CivilTimeState, ClientStorage, EncodedAudio, HostServices, ImageMetadata, PcmBus,
    },
    state::ClientId,
};
use crysirror_resource::{Archive, DecodeContext};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use serde_json::json;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
const CHANNEL: u32 = 1022;
struct Catalog {
    archive: Arc<crysirror_formats_arc::alk::AlkCatalog>,
    waves: [Arc<[u8]>; 4],
}
struct Loaded {
    inputs: VecDeque<(i32, ImageInput)>,
    particles: Arc<[timeline::Particle]>,
}
struct Scene {
    season: i32,
    images: BTreeMap<i32, Arc<dyn ImageResource>>,
    particles: Arc<[timeline::Particle]>,
}
struct Frame {
    operations: VecDeque<ImageCompositeLayer>,
    job: Option<ImageJob>,
    base: ImageInput,
}
enum Phase {
    Civil(Box<dyn CivilTimeQuery>),
    Opening(Task<Loaded>),
    Decoding {
        inputs: VecDeque<(i32, ImageInput)>,
        jobs: Vec<(i32, ImageJob)>,
        images: BTreeMap<i32, Arc<dyn ImageResource>>,
        particles: Arc<[timeline::Particle]>,
        winter: bool,
    },
    AudioResource,
    AudioPlay,
    Clock,
    Plan(Task<VecDeque<ImageCompositeLayer>>),
    Frame(Frame),
    FinalFrame(ImageJob),
    Done,
}
struct Run {
    call: ExternalCall,
    season: i32,
    repeat: bool,
    phase: Phase,
    scene: Option<Arc<Scene>>,
    resource_ticket: Option<u64>,
    resource: Option<AudioResourceHandle>,
    play_ticket: Option<u64>,
    audio_owned: bool,
    start: u32,
    held: u64,
}
pub(crate) struct NativeLogo {
    owner: ClientId,
    pool: TaskPool,
    ini: Option<String>,
    catalog: Option<Arc<Catalog>>,
    init: Option<(ExternalCall, Task<Catalog>)>,
    images: Option<Arc<dyn ImageMetadata>>,
    run: Option<Run>,
    wave: [i32; 4],
}
impl NativeLogo {
    pub(crate) fn new(owner: ClientId, pool: TaskPool, ini: Option<String>) -> Self {
        Self {
            owner,
            pool,
            ini,
            catalog: None,
            init: None,
            images: None,
            run: None,
            wave: [0, 1, 2, 3],
        }
    }
    pub(crate) fn has_state(&self) -> bool {
        self.catalog.is_some() || self.init.is_some() || self.run.is_some()
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        json!({"initialized":self.catalog.is_some(),"preparing":self.init.is_some(),"running":self.run.is_some(),
            "season":self.run.as_ref().map(|r|r.season),"wave_selectors":self.wave,"window_surface":"canonical NativeCg WindowSurface"})
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.init = None;
        if let Some(run) = self.run.as_mut() {
            run.release_audio(self.owner, host)?;
        }
        self.run = None;
        self.catalog = None;
        self.images = None;
        // Native library exit stops sound, but never clears the host window
        // surface. The client lifecycle/reset transaction retires its image;
        // another native writer may already have replaced Layer16.
        Ok(())
    }
    fn publish(
        &mut self,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(
            resource.client() == self.owner && resource.size() == [800, 600],
            "native Logo foreign/invalid frame"
        );
        if bridge.sact.as_ref().is_some_and(|s| s.initialized) {
            ensure!(
                bridge.sact.as_ref().unwrap().canvas == [800, 600],
                "native Logo original800x600 window profile"
            );
        }
        // The canonical native WindowSurface lease exists independently of
        // SACT initialization and survives its creation of sprite(-1).
        cg.present_window_result(self.owner, resource, bridge, host)
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.run.is_none() && self.init.is_none(),
                "native Logo changed target"
            );
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        if library.name != b"AliceLogo2" {
            ensure!(
                self.run.is_none() && self.init.is_none(),
                "native Logo changed library"
            );
            return Ok(None);
        }
        ensure!(owner == self.owner, "native Logo foreign owner");
        let function = &library.functions[function];
        let args = machine.hll_arguments(call)?;
        let name = function.name.as_slice();
        ensure!(
            function.return_kind == if name == b"Init" { 10 } else { 0 },
            "native Logo return ABI"
        );
        match (name, args.as_slice()) {
            (
                b"SetWaveNum",
                [
                    Argument::Scalar {
                        kind: 10,
                        bits: index,
                    },
                    Argument::Scalar {
                        kind: 10,
                        bits: wave,
                    },
                ],
            ) => {
                ensure!(
                    self.run.is_none() && self.init.is_none(),
                    "native Logo SetWaveNum while pending"
                );
                if let Ok(index) = usize::try_from(*index) {
                    if let Some(slot) = self.wave.get_mut(index) {
                        *slot = *wave;
                    }
                }
                machine.complete_hll(call, HllReturn::Void)?;
                Ok(Some(Dispatch::Completed))
            }
            (b"Init", []) => {
                ensure!(self.run.is_none(), "native Logo Init during Run");
                ensure!(
                    machine.stack.len() < 65536,
                    "native Logo result stack budget"
                );
                if self.init.is_none() {
                    let storage = host.storage(owner)?;
                    let ini = self.ini.clone();
                    let permit = self.pool.reserve()?;
                    let images = host.image_metadata(owner)?;
                    ensure!(
                        !host.audio_playing(owner, CHANNEL)?,
                        "native Logo channel already owned"
                    );
                    self.init = Some((
                        call.clone(),
                        permit.spawn(move |cancel| {
                            load_catalog(owner, storage.as_ref(), ini.as_deref(), &cancel)
                        })?,
                    ));
                    self.images = Some(images);
                }
                let (pending, task) = self.init.as_mut().unwrap();
                ensure!(*pending == *call, "native Logo stale Init");
                let Some(catalog) = task.try_take()? else {
                    return Ok(Some(wait(host)));
                };
                self.init = None;
                self.catalog = Some(Arc::new(catalog));
                machine.complete_hll(call, HllReturn::Scalar(1))?;
                Ok(Some(Dispatch::Completed))
            }
            (
                b"Run",
                [
                    Argument::Scalar {
                        kind: 10,
                        bits: season,
                    },
                    Argument::Scalar {
                        kind: 10,
                        bits: repeat,
                    },
                ],
            ) => {
                ensure!(self.init.is_none(), "native Logo Run before Init receipt");
                let catalog = self
                    .catalog
                    .as_ref()
                    .context("native Logo Run before real Init")?
                    .clone();
                if self.run.is_none() {
                    if !(0..=4).contains(season) {
                        machine.complete_hll(call, HllReturn::Void)?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    let phase =
                        if *season == 0 {
                            Phase::Civil(host.begin_civil_time(owner)?)
                        } else {
                            let season = *season;
                            Phase::Opening(self.pool.spawn(move |cancel| {
                                load_images(owner, &catalog, season, &cancel)
                            })?)
                        };
                    self.run = Some(Run {
                        call: call.clone(),
                        season: *season,
                        repeat: *repeat != 0,
                        phase,
                        scene: None,
                        resource_ticket: None,
                        resource: None,
                        play_ticket: None,
                        audio_owned: false,
                        start: 0,
                        held: 0,
                    });
                }
                let images = self
                    .images
                    .as_ref()
                    .context("native Logo image authority absent")?
                    .clone();
                let run = self.run.as_mut().unwrap();
                ensure!(run.call == *call, "native Logo stale Run");
                let catalog = self.catalog.as_ref().unwrap();
                let frame = run.advance(
                    owner,
                    &self.pool,
                    catalog,
                    &self.wave,
                    images.as_ref(),
                    &bridge.input,
                    host,
                )?;
                if let Some(resource) = frame {
                    self.publish(resource, bridge, cg, host)?;
                    return Ok(Some(Dispatch::Presented));
                }
                if matches!(self.run.as_ref().unwrap().phase, Phase::Done) {
                    self.run.as_mut().unwrap().release_audio(owner, host)?;
                    self.run = None;
                    machine.complete_hll(call, HllReturn::Void)?;
                    return Ok(Some(Dispatch::Completed));
                }
                Ok(Some(wait(host)))
            }
            _ => anyhow::bail!(
                "unsupported native AliceLogo2 export/ABI {}",
                String::from_utf8_lossy(name)
            ),
        }
    }
}
fn wait(host: &dyn HostServices) -> Dispatch {
    Dispatch::WaitUntil(host.now_ms().saturating_add(1))
}
fn load_catalog(
    owner: ClientId,
    storage: &dyn ClientStorage,
    ini: Option<&str>,
    cancel: &Cancellation,
) -> Result<Catalog> {
    cancel.check()?;
    let ini = ini.context("native Logo requires original INI module folder")?;
    let ini = storage.open_asset(&VfsPath::new(Mount::Asset, ini)?)?;
    let ini = crysirror_resource::read_all(ini.as_ref(), 1024 * 1024)?;
    let document = crate::bootstrap::Document::parse(&ini)?;
    let folder = document
        .entries
        .iter()
        .rev()
        .find(|e| e.name == "DLL")
        .context("native Logo DLL directory absent")?;
    let crate::bootstrap::Value::String(folder) = &folder.value else {
        anyhow::bail!("native Logo DLL directory type")
    };
    let file = if folder.is_empty() {
        "AliceLogo2.dll".to_owned()
    } else {
        format!("{}/AliceLogo2.dll", folder.replace('\\', "/"))
    };
    let module = VfsPath::new(Mount::Asset, &file)?;
    let module =
        crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, &module)?
            .context("native Logo data module absent")?;
    let reader = storage.open_asset(&module)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), 16 * 1024 * 1024)?;
    cancel.check()?;
    let pe = crysirror_resource::pe::PeData::parse(&bytes)?;
    let rows = pe.raw_resources(
        &crysirror_resource::pe::PeResourceName::Name("WAVE".into()),
        &|| cancel.check(),
    )?;
    let mut waves: [Option<Arc<[u8]>>; 4] = std::array::from_fn(|_| None);
    for row in rows {
        cancel.check()?;
        let crysirror_resource::pe::PeResourceName::Id(id) = row.name else {
            continue;
        };
        if !(106..=109).contains(&id) {
            continue;
        }
        // The reviewed module has one Japanese1041 variant per resource.
        // Another language is accepted only when unique; ambiguity rejects.
        let slot = &mut waves[(id - 106) as usize];
        ensure!(
            slot.is_none(),
            "native Logo ambiguous WAVE language variants"
        );
        ensure!(
            row.bytes.starts_with(b"RIFF") && row.bytes.get(8..12) == Some(b"WAVE"),
            "native Logo embedded WAVE envelope"
        );
        *slot = Some(row.bytes.into());
    }
    let waves = waves
        .into_iter()
        .map(|v| v.context("native Logo WAVE106..109 resource missing"))
        .collect::<Result<Vec<_>>>()?;
    let path = VfsPath::new(Mount::Asset, "AliceLogo2.alk")?;
    let path =
        crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, &path)?
            .context("native Logo ALK absent")?;
    let cx = DecodeContext::new(owner).with_cancellation(cancel.clone());
    let archive = crysirror_formats_arc::alk::AlkCatalog::load(storage.open_asset(&path)?, &cx)?;
    ensure!(
        archive.entries().len() <= 256,
        "native Logo ALK slot budget"
    );
    Ok(Catalog {
        archive: Arc::new(archive),
        waves: waves
            .try_into()
            .ok()
            .context("native Logo WAVE table length")?,
    })
}
fn load_images(
    owner: ClientId,
    catalog: &Catalog,
    season: i32,
    cancel: &Cancellation,
) -> Result<Loaded> {
    let ids: Vec<i32> = match season {
        1 => (0..16).chain([17, 45]).collect(),
        2 => vec![4, 5, 6, 7, 44, 45, 30, 31, 32],
        3 => (18..22).chain([22, 38, 39, 40, 41, 45]).collect(),
        4 => vec![24, 25, 26, 27, 28, 43, 45],
        _ => anyhow::bail!("native Logo season"),
    };
    let mut registry = crysirror_image::DecoderRegistry::default();
    crysirror_formats_arc::register_images(&mut registry)?;
    let cx = DecodeContext::new(owner).with_cancellation(cancel.clone());
    let mut inputs = VecDeque::new();
    let mut total = 0usize;
    for id in ids {
        cancel.check()?;
        let reader = catalog.archive.open(id as u32, &cx)?;
        let bytes = crysirror_resource::read_all(reader.as_ref(), 16 * 1024 * 1024)?;
        total = total
            .checked_add(bytes.len())
            .context("native Logo encoded byte overflow")?;
        ensure!(
            total <= 32 * 1024 * 1024,
            "native Logo season encoded byte budget"
        );
        let name = format!("AliceLogo2/{id}.qnt");
        let info = registry
            .encoding_info_with(&name, &bytes, None, Some(&cx), Some("alice-soft/qnt"))?
            .context("native Logo QNT metadata absent")?;
        ensure!(
            info.size.iter().all(|v| *v <= 8192),
            "native Logo image dimensions"
        );
        inputs.push_back((
            id,
            ImageInput::Encoded {
                name,
                bytes: bytes.into(),
                size: info.size,
            },
        ));
    }
    Ok(Loaded {
        inputs,
        particles: timeline::particles(season)?,
    })
}
impl Run {
    fn release_audio(&mut self, owner: ClientId, host: &mut dyn HostServices) -> Result<()> {
        if let Some(ticket) = self.play_ticket {
            host.release_audio(owner, ticket)?;
            self.play_ticket = None;
        }
        if self.audio_owned {
            host.submit_audio(
                owner,
                AudioRequest::Stop {
                    channel: CHANNEL,
                    fade_ms: 0.,
                },
            )?;
            self.audio_owned = false;
        }
        if let Some(ticket) = self.resource_ticket {
            host.release_audio_resource_preparation(owner, ticket)?;
            self.resource_ticket = None;
        }
        if let Some(resource) = self.resource {
            host.release_audio_resource(owner, resource)?;
            self.resource = None;
        }
        Ok(())
    }
    fn play(&mut self, owner: ClientId, host: &mut dyn HostServices) -> Result<()> {
        self.play_ticket = Some(
            host.begin_audio(
                owner,
                AudioRequest::PlayResource {
                    channel: CHANNEL,
                    resource: self
                        .resource
                        .context("native Logo sound resource missing")?,
                    parameters: AudioParameters {
                        bus: PcmBus::Sfx,
                        ..Default::default()
                    },
                },
            )?,
        );
        self.phase = Phase::AudioPlay;
        // An admitted receipt may publish before it is polled. Cancellation
        // must stop this owned slot even in that Ready-but-unobserved window.
        self.audio_owned = true;
        Ok(())
    }
    fn advance(
        &mut self,
        owner: ClientId,
        pool: &TaskPool,
        catalog: &Arc<Catalog>,
        wave: &[i32; 4],
        images: &dyn ImageMetadata,
        input: &crate::input::Input,
        host: &mut dyn HostServices,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        match &mut self.phase {
            Phase::Civil(query) => {
                match query.poll()? {
                    CivilTimeState::Pending => return Ok(None),
                    CivilTimeState::Failed(error) => anyhow::bail!("native Logo month: {error}"),
                    CivilTimeState::Ready(time) => {
                        ensure!(time.client == owner, "native Logo foreign civil time");
                        self.season = match time.month {
                            3..=5 => 1,
                            6..=8 => 2,
                            9..=11 => 3,
                            _ => 4,
                        };
                    }
                }
                let season = self.season;
                let catalog = catalog.clone();
                self.phase = Phase::Opening(
                    pool.spawn(move |cancel| load_images(owner, &catalog, season, &cancel))?,
                );
            }
            Phase::Opening(task) => {
                let Some(loaded) = task.try_take()? else {
                    return Ok(None);
                };
                self.phase = Phase::Decoding {
                    inputs: loaded.inputs,
                    jobs: vec![],
                    images: BTreeMap::new(),
                    particles: loaded.particles,
                    winter: false,
                };
            }
            Phase::Decoding {
                inputs,
                jobs,
                images: resources,
                particles,
                winter,
            } => {
                let mut n = 0;
                while n < jobs.len() {
                    match jobs[n].1.poll()? {
                        ImageProcessState::Pending => n += 1,
                        ImageProcessState::Failed(error) => {
                            anyhow::bail!("native Logo image preparation: {error}")
                        }
                        ImageProcessState::Ready(resource) => {
                            ensure!(resource.client() == owner, "native Logo foreign image");
                            let (id, _) = jobs.remove(n);
                            resources.insert(id, resource);
                        }
                    }
                }
                while jobs.len() < 3 && !inputs.is_empty() {
                    let (id, input) = inputs.pop_front().unwrap();
                    jobs.push((id, images.begin_process(input, b"[]".as_slice().into())?));
                }
                if jobs.is_empty() && inputs.is_empty() {
                    if self.season == 4 && !*winter {
                        let source = resources
                            .get(&43)
                            .context("native Logo winter source absent")?
                            .clone();
                        let size = source.size();
                        for index in 0..8 {
                            let extent = 20 + index * 2;
                            let spec = json!([{"kind":"rect_resample_byte256","spec":{"size":[extent,extent],"source":[0.,0.,size[0],size[1]]}}]);
                            inputs.push_back((
                                100 + index,
                                ImageInput::Process {
                                    input: Box::new(ImageInput::Resource(source.clone())),
                                    effects: serde_json::to_vec(&spec)?.into(),
                                },
                            ));
                        }
                        *winter = true;
                    } else {
                        self.scene = Some(Arc::new(Scene {
                            season: self.season,
                            images: std::mem::take(resources),
                            particles: particles.clone(),
                        }));
                        let selector = wave[(self.season - 1) as usize];
                        let source = catalog
                            .waves
                            .get(usize::try_from(selector).unwrap_or(usize::MAX))
                            .context("native Logo WAVE selector outside0..3")?
                            .clone();
                        self.resource_ticket = Some(host.begin_encoded_audio_resource(
                            owner,
                            "AliceLogo2.wav",
                            EncodedAudio {
                                bytes: source,
                                frame_limit: None,
                            },
                            false,
                        )?);
                        self.phase = Phase::AudioResource;
                    }
                }
            }
            Phase::AudioResource => {
                let ticket = self
                    .resource_ticket
                    .context("native Logo lost sound receipt")?;
                let info = host.audio_resource_preparation(owner, ticket)?;
                match info.phase {
                    AudioPreparePhase::Pending => return Ok(None),
                    AudioPreparePhase::Failed => {
                        anyhow::bail!("native Logo sound preparation: {:?}", info.error)
                    }
                    AudioPreparePhase::Ready => {
                        self.resource =
                            Some(info.resource.context("native Logo ready sound absent")?);
                        ensure!(
                            self.resource.unwrap().client == owner.0,
                            "native Logo foreign sound resource"
                        );
                        host.release_audio_resource_preparation(owner, ticket)?;
                        self.resource_ticket = None;
                        self.play(owner, host)?;
                    }
                }
            }
            Phase::AudioPlay => {
                let ticket = self
                    .play_ticket
                    .context("native Logo lost playback receipt")?;
                let info = host.audio_preparation(owner, ticket)?;
                ensure!(
                    info.channel == CHANNEL,
                    "native Logo foreign playback channel"
                );
                match info.phase {
                    AudioPreparePhase::Pending => return Ok(None),
                    AudioPreparePhase::Failed => {
                        anyhow::bail!("native Logo playback: {:?}", info.error)
                    }
                    AudioPreparePhase::Ready => {
                        host.release_audio(owner, ticket)?;
                        self.play_ticket = None;
                        self.audio_owned = true;
                        self.start = host.now_ms() as u32;
                        self.phase = Phase::Clock;
                    }
                }
            }
            Phase::Clock => {
                let mut down = 0u64;
                for (n, key) in [1, 2, 32, 13, 17, 27].into_iter().enumerate() {
                    if input.key(key)? != 0 {
                        down |= 1 << n;
                    }
                }
                let skip = self.held & !down != 0;
                self.held |= down;
                let time = (host.now_ms() as u32).wrapping_sub(self.start);
                if skip || time >= 11000 && !self.repeat {
                    self.release_audio(owner, host)?;
                    self.phase = Phase::FinalFrame(images.begin_process(
                        ImageInput::Solid {
                            size: [800, 600],
                            color: [255; 4],
                        },
                        b"[]".as_slice().into(),
                    )?);
                } else if time >= 11000 {
                    host.submit_audio(
                        owner,
                        AudioRequest::Stop {
                            channel: CHANNEL,
                            fade_ms: 0.,
                        },
                    )?;
                    self.audio_owned = false;
                    self.play(owner, host)?;
                } else {
                    let scene = self
                        .scene
                        .as_ref()
                        .context("native Logo scene missing")?
                        .clone();
                    self.phase =
                        Phase::Plan(pool.spawn(move |cancel| {
                            timeline::sample(&scene, time, &|| cancel.check())
                        })?);
                }
            }
            Phase::Plan(task) => {
                let Some(operations) = task.try_take()? else {
                    return Ok(None);
                };
                self.phase = Phase::Frame(Frame {
                    operations,
                    job: None,
                    base: ImageInput::Solid {
                        size: [800, 600],
                        color: [0, 0, 0, 255],
                    },
                });
            }
            Phase::Frame(frame) => {
                if let Some(job) = &mut frame.job {
                    match job.poll()? {
                        ImageProcessState::Pending => return Ok(None),
                        ImageProcessState::Failed(error) => {
                            anyhow::bail!("native Logo composition: {error}")
                        }
                        ImageProcessState::Ready(resource) => {
                            ensure!(
                                resource.client() == owner && resource.size() == [800, 600],
                                "native Logo foreign frame"
                            );
                            frame.job = None;
                            if frame.operations.is_empty() {
                                self.phase = Phase::Clock;
                                return Ok(Some(resource));
                            }
                            frame.base = ImageInput::Resource(resource);
                        }
                    }
                }
                // At most128 native draws per bounded shared job. Chunks are
                // never presented separately, so a frame is one owner commit.
                let count = frame.operations.len().min(128);
                let layers = frame.operations.drain(..count).collect();
                frame.job = Some(images.begin_process(
                    ImageInput::Composite {
                        base: Box::new(frame.base.clone()),
                        layers,
                    },
                    b"[]".as_slice().into(),
                )?);
            }
            Phase::FinalFrame(job) => match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("native Logo final white frame: {error}")
                }
                ImageProcessState::Ready(resource) => {
                    self.phase = Phase::Done;
                    return Ok(Some(resource));
                }
            },
            Phase::Done => {}
        }
        Ok(None)
    }
}
