// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original dataN restore assembled from native policy and engine-owned ready
//! resources. Only Runtime commits the replacement VM/Scene/audio together.
use crate::{
    adapter::{NativeAdapter, ScriptLoader},
    bridge::{Bridge, BridgeRestoreSeed},
    expression::Variables,
    native_bootstrap::NativeConfiguration,
    native_control::ScriptSession,
    native_save::{LegacySaveProfile, NativeSaveFile},
    native_save_audio::SavedAudioPreparation,
    native_save_images::SavedPictures,
    native_save_wait::{SavedWait, SavedWaitPreparation},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    native_restore::{
        OriginalRestoreBatch, OriginalRestoreCommit, OriginalRestorePreparation,
        OriginalRestoreProgress,
    },
    service::{AudioReplacementResource, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use serde_json::{json, Value};
use std::sync::Arc;
struct Parsed {
    file: Arc<NativeSaveFile>,
    profile: LegacySaveProfile,
    session: ScriptSession,
    seed: BridgeRestoreSeed,
    pictures: Option<SavedPictures>,
    audio: Option<SavedAudioPreparation>,
    wait: Option<SavedWait>,
    wait_job: Option<SavedWaitPreparation>,
    base: Option<ImageJob>,
    pattern_init: Option<(i32, crysirror_core::service::PatternJob)>,
    frame: Option<crate::native_presentation::NativeFrameWork>,
    frame_ready: Option<crate::native_presentation::NativeFrameResult>,
    prepared_audio: Option<Arc<dyn AudioReplacementResource>>,
    layers: Vec<Value>,
    audio_done: bool,
    pictures_done: bool,
    ui: Vec<Value>,
}
pub(crate) struct PreparedNativeRestore {
    pub session: ScriptSession,
    pub bridge: Bridge,
    pub batches: Vec<OriginalRestoreBatch>,
    pub audio: Arc<dyn AudioReplacementResource>,
    pub release_images: Vec<String>,
    pub original: Arc<NativeSaveFile>,
    pub profile: LegacySaveProfile,
}
impl Parsed {
    fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self.audio_done {
            if !self.audio.as_mut().unwrap().poll(host)? {
                return Ok(false);
            }
            let prepared = self.audio.take().unwrap().finish()?;
            let (bridge, audio) = self
                .seed
                .audio
                .take()
                .context("NNN saved audio seed consumed")?
                .finish(prepared)?;
            self.seed.bridge.configure_audio(bridge)?;
            self.prepared_audio = Some(audio);
            self.audio_done = true;
        }
        if !self.pictures_done {
            if !self.pictures.as_mut().unwrap().poll(host)? {
                return Ok(false);
            }
            let pictures = self.pictures.take().unwrap();
            let effects = pictures.effects().to_vec();
            for picture in pictures.into_prepared()? {
                self.seed.bridge.insert_saved_picture(picture)?;
            }
            self.seed
                .bridge
                .load_saved_effect_state(&effects, self.file.status_word(48)? as u32)?;
            self.layers = self.seed.bridge.saved_picture_operations(host)?;
            self.pictures_done = true;
            self.base = Some(
                host.image_metadata(self.seed.bridge.client())?
                    .begin_process(
                        ImageInput::Solid {
                            size: self.seed.bridge.canvas_size(),
                            color: [0, 0, 0, 255],
                        },
                        b"[]".as_slice().into(),
                    )?,
            );
        }
        if self.pattern_init.is_none() {
            if let Some(slot) = self.seed.bridge.next_saved_suna() {
                let random = self
                    .session
                    .machine()
                    .variables()
                    .context("NNN saved SUNA variables absent")?
                    .random_state()
                    .context("NNN saved SUNA needs current native RNG")?;
                let spec = crysirror_image::temporal_pattern::TemporalPatternSpec {
                    size: self.seed.bridge.canvas_size(),
                    cell: [2, 2],
                };
                spec.validate()?;
                let job = host.begin_pattern(
                    self.seed.bridge.client(),
                    crysirror_core::service::PatternRequest::Create {
                        spec: serde_json::to_vec(&spec)?.into(),
                        rng: serde_json::to_vec(&random)?.into(),
                    },
                )?;
                self.pattern_init = Some((slot, job));
            }
        }
        if let Some((slot, job)) = &mut self.pattern_init {
            use crysirror_core::service::{PatternOutput, PatternProcessState};
            match job.poll()? {
                PatternProcessState::Pending => return Ok(false),
                PatternProcessState::Failed(error) => {
                    anyhow::bail!("NNN original SUNA InitParameter: {error}")
                }
                PatternProcessState::Ready(PatternOutput::Resource {
                    resource,
                    rng_after,
                }) => {
                    self.seed.bridge.complete_saved_suna(*slot, resource)?;
                    let random = serde_json::from_slice(&rng_after)?;
                    let mut variables = self
                        .session
                        .machine()
                        .variables()
                        .context("NNN saved SUNA variables absent")?
                        .clone();
                    variables.replace_random_state(random)?;
                    self.session.machine_mut().replace_variables(variables)?;
                    self.pattern_init = None;
                    return Ok(false);
                }
                PatternProcessState::Ready(_) => {
                    anyhow::bail!("NNN SUNA init returned image instead of pattern resource")
                }
            }
        }
        if let Some(job) = &mut self.base {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => anyhow::bail!("NNN saved frame base: {error}"),
                ImageProcessState::Ready(image) => {
                    let random = self
                        .session
                        .machine()
                        .variables()
                        .context("NNN saved variable bank absent")?
                        .random_state()
                        .context("NNN saved frame requires current original RNG")?;
                    self.frame = Some(self.seed.bridge.begin_saved_frame(image, random, host)?);
                    self.base = None;
                }
            }
        }
        if let Some(frame) = &mut self.frame {
            let Some(done) = frame.poll(host)? else {
                return Ok(false);
            };
            let mut variables = self
                .session
                .machine()
                .variables()
                .context("NNN saved variables absent")?
                .clone();
            variables.replace_random_state(done.random.clone())?;
            self.seed.bridge.bind_preference_variables(&mut variables)?;
            let _ = variables.bind_calculated_value(b"mode", self.file.status_word(20)?)?;
            self.session.machine_mut().replace_variables(variables)?;
            self.layers
                .push(self.seed.bridge.install_saved_frame(&done, host)?);
            self.frame_ready = Some(done);
            self.frame = None;
        }
        if let Some(wait) = self.wait.take() {
            self.seed.bridge.validate_saved_wait_effects()?;
            self.wait_job = Some(
                wait.begin(
                    Some(&self.seed.text),
                    Some(&self.seed.selection),
                    self.frame_ready
                        .as_ref()
                        .context("NNN saved frame absent")?
                        .target
                        .clone(),
                    host,
                )?,
            );
        }
        if let Some(wait) = &mut self.wait_job {
            if !wait.poll(host)? {
                return Ok(false);
            }
            let wait = self.wait_job.take().unwrap().finish()?;
            let (batch, releases) = self.seed.bridge.install_saved_wait(wait)?;
            self.layers.extend(
                batch["layers"]
                    .as_array()
                    .context("NNN saved wait layers")?
                    .iter()
                    .cloned(),
            );
            let ui = batch
                .get("ui")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            // Keep the prepared menu separate until serialization; its UI
            // mutations are still inside the same Runtime staged transaction.
            self.ui = ui;
            self.seed.release_images.extend(releases);
        }
        Ok(self.wait_job.is_none() && self.wait.is_none() && self.frame_ready.is_some())
    }
    fn encode(self, cancel: crysirror_tasks::Cancellation) -> Result<PreparedNativeRestore> {
        cancel.check()?;
        let mut layers = self
            .seed
            .remove_layers
            .into_iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        layers.extend(self.layers);
        let mut ui = self
            .seed
            .remove_ui
            .into_iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        ui.extend(self.ui);
        let bytes = serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?;
        cancel.check()?;
        let release_images = self
            .seed
            .release_images
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(PreparedNativeRestore {
            session: self.session,
            bridge: self.seed.bridge,
            batches: vec![OriginalRestoreBatch {
                schema: 1,
                bytes: bytes.into(),
            }],
            audio: self
                .prepared_audio
                .context("NNN prepared audio replacement absent")?,
            release_images,
            original: self.file,
            profile: self.profile,
        })
    }
}
pub(crate) struct NativeRestoreJob {
    client: ClientId,
    pool: TaskPool,
    configuration: Arc<NativeConfiguration>,
    loader: ScriptLoader,
    dormant: Option<(Arc<[u8]>, BridgeRestoreSeed, Variables)>,
    parse: Option<Task<Parsed>>,
    resources: Option<Parsed>,
    encode: Option<Task<PreparedNativeRestore>>,
    ready: Option<PreparedNativeRestore>,
    failed: Option<String>,
}
impl NativeRestoreJob {
    pub(crate) fn new(
        client: ClientId,
        bytes: Arc<[u8]>,
        seed: BridgeRestoreSeed,
        current: Variables,
        configuration: Arc<NativeConfiguration>,
        loader: ScriptLoader,
        pool: TaskPool,
    ) -> Self {
        Self {
            client,
            pool,
            configuration,
            loader,
            dormant: Some((bytes, seed, current)),
            parse: None,
            resources: None,
            encode: None,
            ready: None,
            failed: None,
        }
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.ready.is_some() {
            return Ok(true);
        }
        if self.dormant.is_some() {
            let permit = self.pool.reserve()?;
            let (bytes, mut seed, current) = self.dormant.take().unwrap();
            let config = self.configuration.clone();
            let loader = self.loader.clone();
            let client = self.client;
            self.parse=Some(permit.spawn(move|cancel|{
                cancel.check()?;let profile=LegacySaveProfile::for_configuration(&config)?;
                ensure!(!profile.omake&&!profile.cutin&&profile.extension_sizes.is_empty(),
                    "NNN native Omake/Cutin/extensions require complete saved actor consumers");
                let file=Arc::new(NativeSaveFile::parse(&bytes,&profile)?);
                ensure!(matches!(file.status_word(20)?,9|10),"NNN original restore requires saved message/selection actor; other native modes need their own wait consumers");
                let wait=file.saved_wait()?.context("NNN saved native wait absent")?;
                seed.bridge.restore_native_status(&file)?;
                let latent=seed.text.dormant(file.messages(file.position()?.message_rows as usize)?,
                    file.status_word(30)? as u32,file.status_word(476)? as u32,file.status_word(44)?!=0,file.status_word(485)?!=0)?;
                seed.bridge.configure_text(latent)?;seed.bridge.configure_selection(seed.selection.dormant()?)?;
                let session=file.prepare_script_contexts(&profile,&config,&current,|name,cancel|loader(name,cancel),&cancel)?;
                let pictures=SavedPictures::new(client,&file,&profile,&config,seed.bridge.image_resources()?)?;
                let audio=seed.audio.as_ref().context("NNN saved audio seed absent")?.prepare(&file,&profile)?;cancel.check()?;
                Ok(Parsed{file,profile,session,seed,pictures:Some(pictures),audio:Some(audio),wait:Some(wait),
                    wait_job:None,base:None,pattern_init:None,frame:None,frame_ready:None,prepared_audio:None,layers:vec![],
                    audio_done:false,pictures_done:false,ui:vec![]})
            })?);
            return Ok(false);
        }
        if let Some(task) = &mut self.parse {
            let Some(parsed) = task.try_take()? else {
                return Ok(false);
            };
            self.parse = None;
            self.resources = Some(parsed);
        }
        if let Some(resources) = &mut self.resources {
            if !resources.poll(host)? {
                return Ok(false);
            }
            let permit = self.pool.reserve()?;
            let resources = self.resources.take().unwrap();
            self.encode = Some(permit.spawn(move |cancel| resources.encode(cancel))?);
            return Ok(false);
        }
        if let Some(task) = &mut self.encode {
            let Some(ready) = task.try_take()? else {
                return Ok(false);
            };
            self.ready = Some(ready);
            self.encode = None;
        }
        Ok(self.ready.is_some())
    }
    pub(crate) fn finish_native(
        mut self,
    ) -> Result<(
        NativeAdapter,
        Vec<OriginalRestoreBatch>,
        Arc<dyn AudioReplacementResource>,
        Vec<String>,
    )> {
        ensure!(
            self.failed.is_none(),
            "NNN original restore preparation failed"
        );
        let ready = self
            .ready
            .take()
            .context("NNN original restore is not ready")?;
        let vm = NativeAdapter::from_original_restore(
            ready.session,
            ready.bridge,
            self.configuration,
            self.loader,
            self.pool,
            ready.original,
            ready.profile,
        )?;
        Ok((vm, ready.batches, ready.audio, ready.release_images))
    }
}
impl OriginalRestorePreparation for NativeRestoreJob {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> crysirror_core::Result<OriginalRestoreProgress> {
        if let Some(error) = &self.failed {
            return Err(crysirror_core::Error::Io(error.clone()));
        }
        match self.advance(host) {
            Ok(true) => Ok(OriginalRestoreProgress::Ready),
            Ok(false) => Ok(OriginalRestoreProgress::Pending),
            Err(error) => {
                let text = format!("NNN original restore: {error:#}");
                self.failed = Some(text.clone());
                Err(crysirror_core::Error::Io(text))
            }
        }
    }
    fn finish(self: Box<Self>) -> crysirror_core::Result<OriginalRestoreCommit> {
        let (vm, engine_batches, audio, release_images) = self
            .finish_native()
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))?;
        Ok(OriginalRestoreCommit {
            vm: Box::new(vm),
            commands: vec![],
            engine_batches,
            audio: vec![],
            prepared_audio: Some(audio),
            release_images,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        adapter::NativeAdapter,
        bridge::{CallbackProfile, SceneBindings},
        config::Names,
        expression::VariableProfile,
        machine::Machine,
        native_bootstrap::PreparedBoot,
        native_resources::ResourceNames,
        native_system_file::{NativeSystemFile, SystemLimits, SystemNames, SystemStamp},
        native_text::{MessageBindings, MessageProfile, NativeTextBridge},
        native_ui::{NativeSelectionBridge, SelectionProfile},
        Program, XorProfile,
    };
    use crysirror_core::{
        native_restore::OriginalRestoreStatus,
        service::{buttons, AudioParameters, AudioRequest, InputFrame, PcmAudio},
        vm::VmAdapter,
    };
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
        time::{Duration, Instant},
    };
    fn names(value: &str) -> Names {
        Names::parse(value.as_bytes(), false).unwrap()
    }
    pub(crate) fn native_fixture() -> (
        NativeAdapter,
        NativeSaveFile,
        LegacySaveProfile,
        Arc<AtomicUsize>,
    ) {
        let client = ClientId(1);
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([3, 0x66660002, 0x55560005]);
        words[18] = words.len() as u32;
        let program = Arc::new(
            Program::parse(
                &words
                    .into_iter()
                    .flat_map(|word| (!word).to_le_bytes())
                    .collect::<Vec<_>>(),
                XorProfile::default(),
            )
            .unwrap(),
        );
        let setup = names(
            "saveHeaderName wait-fixture lowVersion 1 frameTime 20 dwqPackLevel 0 fontSize 4 nameFontSize 4 printX 0 printY 0 nextY 4 LPrintX 0 LPrintY 0 LNextY 4 fullMessageNumber 3 windowMessageNumber 3 nameFlag 0 nameColorChangeFlag 0",
        );
        let empty = names("");
        let resources = ResourceNames::new("").unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        variables.write_stored(0, 111);
        variables.write_stored(250, 333);
        variables.write_stored(300, 555);
        variables
            .replace_random_state(crysirror_image::temporal_pattern::PatternRng::msvc(123))
            .unwrap();
        let boot = PreparedBoot {
            session: ScriptSession::new(program.clone(), Machine::with_variables(variables)),
            resources: resources.clone(),
            setup: setup.clone(),
            main_control: None,
            variable_names: empty.clone(),
            project: empty.clone(),
            commands: empty.clone(),
            functions: empty.clone(),
            initial: None,
            module_sha256: crate::native_bootstrap::MUGEN2_MODULE_SHA256.into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        };
        let profile = LegacySaveProfile::for_configuration(&boot.configuration()).unwrap();
        let mut bridge = Bridge::new(
            client,
            CallbackProfile::Mugen2Legacy,
            SceneBindings {
                root: 1,
                overlap_canvas: 7,
                effect_base: 100,
                size: [16, 16],
                layers: 16,
                music_channel: 0,
            },
        )
        .unwrap();
        bridge
            .configure_system_file(
                NativeSystemFile::defaults(
                    SystemLimits::new(2, 1, 1),
                    SystemStamp([2026, 11, 1, 0, 10, 20, 30, 400]),
                    &SystemNames {
                        font: b"fixture".to_vec(),
                        surname: b"old-family".to_vec(),
                        given_name: b"old-name".to_vec(),
                    },
                )
                .unwrap(),
                &setup,
            )
            .unwrap();
        let message = MessageProfile::from_legacy_setup(
            &setup,
            MessageBindings {
                client,
                root: 1,
                body_layer: 200,
                name_layer: 201,
                order: 30,
                canvas: [16, 16],
                window: None,
            },
            "fixture",
            2,
            None,
            None,
            buttons::MOUSE_LEFT,
        )
        .unwrap();
        bridge
            .configure_selection(
                NativeSelectionBridge::new(SelectionProfile {
                    client,
                    root: 1,
                    widget_base: 300,
                    layer_base: 400,
                    order: 30,
                    canvas: [16, 16],
                    face: message.body.clone(),
                    rows: vec![],
                    message_origins: vec![],
                    single_byte_map: None,
                })
                .unwrap(),
            )
            .unwrap();
        bridge
            .configure_text(NativeTextBridge::new(message).unwrap())
            .unwrap();
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = reads.clone();
        bridge
            .configure_resources(
                crate::resource_bridge::NativeResources::blackcyc(
                    client,
                    Arc::new(move |_, cancel| {
                        cancel.check()?;
                        observed.fetch_add(1, Ordering::SeqCst);
                        Ok(None)
                    }),
                )
                .unwrap(),
                resources,
                0,
            )
            .unwrap();
        let audio = crate::native_audio::NativeAudioPolicy::new(
            crate::native_services_bootstrap::NativePreferences::original_defaults().audio,
            crate::native_audio::AudioBindings {
                music: 0,
                sound_base: 16,
                voice_base: 32,
            },
        )
        .unwrap();
        bridge
            .configure_audio(
                crate::audio_bridge::AudioBridge::new(
                    client,
                    audio,
                    empty.clone(),
                    empty,
                    Arc::new(|_, _| {
                        anyhow::bail!("inactive authored audio source must not resolve")
                    }),
                )
                .unwrap(),
            )
            .unwrap();
        let loader_reads = reads.clone();
        let loader: ScriptLoader = Arc::new(move |name, cancel| {
            cancel.check()?;
            ensure!(name == b"sys", "authored original controller");
            loader_reads.fetch_add(1, Ordering::SeqCst);
            Ok(program.clone())
        });
        let vm = NativeAdapter::from_bootstrap(boot, loader, bridge).unwrap();
        let (mut file, _) = crate::native_save_wait_tests::save_file();
        for (index, value) in [
            (20, 10),
            (28, 7),
            (30, 0x22220001),
            (36, -1),
            (44, 1),
            (45, 20),
            (472, 32),
            (476, 0x12340007),
        ] {
            crate::native_save_wait_tests::word(&mut file, index, value);
        }
        let mut stored = vec![0; 1000];
        stored[0] = 10;
        stored[250] = 20;
        stored[300] = 30;
        file.replace_variables(&stored).unwrap();
        for (index, value) in [
            (0, 1),
            (1, -1),
            (2, 54),
            (27, 7),
            (28, 99999),
            (30, 10),
            (31, 20),
            (32, 30),
        ] {
            let at = 3 * 1024 + index * 4;
            file.blocks[5].payload[at..at + 4].copy_from_slice(&(value as i32).to_le_bytes());
        }
        for (index, name) in [
            (448, b"new-family".as_slice()),
            (452, b"new-name".as_slice()),
        ] {
            let start = (index - 8) * 4;
            file.blocks[2].payload[start..start + name.len()].copy_from_slice(name);
        }
        file.validate(&profile).unwrap();
        (vm, file, profile, reads)
    }
    fn runtime() -> (tempfile::TempDir, crysirror_runtime::Runtime) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"native original restore",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"target","adapter":"crysirror-script-v1","game":"authored"},{"name":"peer","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        for client in [ClientId(1), ClientId(2)] {
            rt.host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,16,16],"paint":{"kind":"solid","color":[4,5,6,255]}}}]})).unwrap()).unwrap();
            rt.host
                .submit_audio(
                    client,
                    AudioRequest::PlayPcm {
                        channel: 11,
                        source: PcmAudio::new(48000, vec![[0.2, 0.1]; 100]).unwrap(),
                        parameters: AudioParameters::default(),
                    },
                )
                .unwrap();
        }
        (root, rt)
    }
    #[track_caller]
    fn spin(mut poll: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while !poll() {
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
    }
    #[test]
    fn real_native_job_prepares_original_records_resources_audio_and_vm_without_live_scene_or_playback_changes(
    ) {
        let (_root, mut rt) = runtime();
        let (old, file, profile, reads) = native_fixture();
        let bytes = file.encode(&profile).unwrap();
        let old_diag = old.diagnostics();
        let scene = serde_json::to_value(&rt.host.engine).unwrap();
        let mut job = old.original_restore_job(bytes.clone().into()).unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        spin(|| job.poll(&mut rt.host).unwrap() == OriginalRestoreProgress::Ready);
        assert_eq!(
            job.poll(&mut rt.host).unwrap(),
            OriginalRestoreProgress::Ready
        );
        assert_eq!(
            reads.load(Ordering::SeqCst),
            1,
            "one original controller source, no inactive asset source"
        );
        assert_eq!(old.diagnostics(), old_diag);
        assert_eq!(rt.now_ms(), 0);
        assert_eq!(serde_json::to_value(&rt.host.engine).unwrap(), scene);
        assert!(rt.host.audio_playing(ClientId(1), 11).unwrap());
        assert!(rt.host.audio_playing(ClientId(2), 11).unwrap());
        let ready = job.ready.as_ref().unwrap();
        assert_eq!(ready.original.encode(&ready.profile).unwrap(), bytes);
        assert_eq!(ready.session.machine().pc(), 32);
        assert_eq!(ready.session.executed(), 0);
        let vars = ready.session.machine().variables().unwrap();
        assert_eq!(vars.read_stored(0), 10);
        assert_eq!(vars.read_stored(250), 333);
        assert_eq!(vars.read_stored(300), 30);
        assert_eq!(
            vars.random_state().unwrap(),
            crysirror_image::temporal_pattern::PatternRng::msvc(123)
        );
        assert_eq!(ready.bridge.effect_counter(3), Some((7, 99999)));
        let batch: Value = serde_json::from_slice(&ready.batches[0].bytes).unwrap();
        let display = batch["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["layer"]["id"] == 116)
            .unwrap();
        let path = display["layer"]["paint"]["asset"].as_str().unwrap();
        assert_eq!(
            &*rt.assets.surface(ClientId(1), path).unwrap().rgba,
            [10, 20, 30, 255].repeat(256)
        );
        assert!(rt.assets.surface(ClientId(2), path).is_none());
        let commit = Box::new(job).finish().unwrap();
        assert!(commit.audio.is_empty());
        assert_eq!(commit.prepared_audio.unwrap().info().channels, 0);
        let diag: Value = serde_json::from_slice(&commit.vm.diagnostics()).unwrap();
        assert_eq!(diag["original_save_restored"], true);
        assert_eq!(diag["instructions"], 0);
    }
    #[test]
    fn invalid_original_profile_or_window_fails_before_source_admission_and_retains_original_vm() {
        let (_root, mut rt) = runtime();
        let (old, file, profile, reads) = native_fixture();
        let before = old.diagnostics();
        let scene = serde_json::to_value(&rt.host.engine).unwrap();
        for bytes in [vec![0; 8], {
            let mut file = file.clone();
            crate::native_save_wait_tests::word(&mut file, 180, 1);
            file.encode(&profile).unwrap()
        }] {
            let mut job = old.original_restore_job(bytes.into()).unwrap();
            let mut error = None;
            spin(|| match job.poll(&mut rt.host) {
                Ok(_) => false,
                Err(reason) => {
                    error = Some(reason);
                    true
                }
            });
            assert_eq!(job.poll(&mut rt.host).unwrap_err(), error.unwrap());
            assert!(Box::new(job).finish().is_err());
            assert_eq!(reads.load(Ordering::SeqCst), 0);
            assert_eq!(old.diagnostics(), before);
            assert_eq!(serde_json::to_value(&rt.host.engine).unwrap(), scene);
            assert_eq!(rt.now_ms(), 0);
            assert!(rt.host.audio_playing(ClientId(1), 11).unwrap());
            assert!(rt.host.audio_playing(ClientId(2), 11).unwrap());
        }
    }
    enum FixtureVm {
        Native(NativeAdapter),
        Installed(crate::installed::InstalledAdapter),
    }
    struct Factory(Mutex<Option<FixtureVm>>);
    impl crysirror_runtime::runtime::VmFactory for Factory {
        fn create(
            &self,
            client: ClientId,
            _: &crysirror_runtime::model::ClientSpec,
        ) -> Result<Box<dyn VmAdapter>> {
            ensure!(client == ClientId(1), "authored native client");
            let vm = self
                .0
                .lock()
                .unwrap()
                .take()
                .context("authored factory already consumed")?;
            Ok(match vm {
                FixtureVm::Native(vm) => Box::new(vm),
                FixtureVm::Installed(vm) => Box::new(vm),
            })
        }
    }
    #[test]
    #[ignore = "reads caller-supplied licensed runtime font; no font bundled"]
    fn actual_runtime_original_transaction_keeps_old_state_until_commit_then_waits_and_continues_native_pc(
    ) {
        for installed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir(root.path().join("game")).unwrap();
            let (vm, file, profile, _) = native_fixture();
            let mut registry = crysirror_runtime::AdapterRegistry::default();
            registry
                .register(
                    "system-nnn",
                    Arc::new(Factory(Mutex::new(Some(if installed {
                        FixtureVm::Installed(
                            crate::installed::InstalledAdapter::original_restore_fixture(vm)
                                .unwrap(),
                        )
                    } else {
                        FixtureVm::Native(vm)
                    })))),
                )
                .unwrap();
            let project:crysirror_runtime::Project=serde_json::from_value(json!({"schema":2,"name":"native dataN atomic",
            "content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"target","adapter":"system-nnn","game":"authored"},{"name":"peer","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
            let mut rt = crysirror_runtime::Runtime::with_asset_root(
                project,
                root.path(),
                &root.path().join("state"),
                registry,
            )
            .unwrap();
            rt.host
                .text
                .add_font(
                    "fixture",
                    std::fs::read(
                        std::env::var_os("CRYSIRROR_TEST_FONT")
                            .expect("licensed runtime font path"),
                    )
                    .unwrap(),
                )
                .unwrap();
            for client in [ClientId(1), ClientId(2)] {
                rt.host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,16,16],"paint":{"kind":"solid","color":[4,5,6,255]}}}]})).unwrap()).unwrap();
                rt.host
                    .submit_audio(
                        client,
                        AudioRequest::PlayPcm {
                            channel: 11,
                            source: PcmAudio::new(48000, vec![[0.2, 0.1]; 100]).unwrap(),
                            parameters: AudioParameters::default(),
                        },
                    )
                    .unwrap();
            }
            let scene = serde_json::to_value(&rt.host.engine).unwrap();
            let before = rt.inspect()["sessions"][0]["native"].clone();
            let ticket = rt
                .begin_original_save_restore(ClientId(1), file.encode(&profile).unwrap().into())
                .unwrap();
            assert!(rt.poll_original_save_restore(ClientId(2), ticket).is_err());
            spin(
                || match rt.poll_original_save_restore(ClientId(1), ticket).unwrap() {
                    OriginalRestoreStatus::Ready => true,
                    OriginalRestoreStatus::Preparing => false,
                    state => panic!("{state:?}"),
                },
            );
            eprintln!("prepared original native transaction installed={installed}");
            assert_eq!(rt.inspect()["sessions"][0]["native"], before);
            assert_eq!(serde_json::to_value(&rt.host.engine).unwrap(), scene);
            assert!(rt.host.audio_playing(ClientId(1), 11).unwrap());
            assert!(rt.host.audio_playing(ClientId(2), 11).unwrap());
            rt.commit_original_save_restore(ClientId(1), ticket)
                .unwrap();
            eprintln!("committed original native transaction installed={installed}");
            assert_eq!(rt.now_ms(), 0);
            assert!(!rt.host.audio_playing(ClientId(1), 11).unwrap());
            assert!(rt.host.audio_playing(ClientId(2), 11).unwrap());
            let native = &rt.inspect()["sessions"][0]["native"];
            assert_eq!(native["original_save_restored"], true);
            assert_eq!(native["wait_kind"], "saved_message_input");
            assert_eq!(native["instructions"], 0);
            rt.tick_control(
                16,
                InputFrame {
                    buttons_down: buttons::MOUSE_LEFT,
                    buttons_pressed: buttons::MOUSE_LEFT,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                rt.inspect()["sessions"][0]["native"]["instructions"],
                0,
                "loading click only arms the restored wait"
            );
            // Each later click has a genuine release edge. Repeating pressed
            // while the physical button stays held is not a desktop event trace.
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut ticks = 0usize;
            // With a native script loader, END restarts controller context0;
            // it does not halt the installed game. Observe actual execution of
            // the saved nextPC rather than imposing data-VM halt semantics.
            while rt.inspect()["sessions"][0]["native"]["instructions"]
                .as_u64()
                .unwrap()
                == 0
            {
                rt.tick_control(16, InputFrame::default()).unwrap();
                rt.tick_control(
                    16,
                    InputFrame {
                        buttons_down: buttons::MOUSE_LEFT,
                        buttons_pressed: buttons::MOUSE_LEFT,
                        ..Default::default()
                    },
                )
                .unwrap();
                ticks += 1;
                if ticks <= 2 {
                    eprintln!(
                        "restored native installed={installed}: {}",
                        rt.inspect()["sessions"]
                    );
                }
                assert!(
                    Instant::now() < deadline,
                    "restored native wait installed={installed}: {}",
                    rt.inspect()["sessions"]
                );
                std::thread::yield_now();
            }
            assert!(
                rt.inspect()["sessions"][0]["native"]["instructions"]
                    .as_u64()
                    .unwrap()
                    >= 1
            );
            assert_ne!(
                rt.inspect()["sessions"][0]["native"]["wait_kind"],
                "saved_message_input"
            );
            assert_eq!(rt.inspect()["sessions"][0]["native"]["effect_counters"]["3"], json!([8,99999]),
                "static native fill still advances its original CountUp through the shared zero-lead scheduler");
            assert_eq!(
                rt.inspect()["sessions"][0]["native"]["factory_registered"],
                installed
            );
        }
    }
}
