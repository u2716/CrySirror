//! Independent original-save transaction capability tests. No legacy VM/data,
//! audio device, renderer, window or system font is involved.
use crysirror_core::{
    image_processing::*,
    native_restore::*,
    service::*,
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_runtime::{runtime::VmFactory, AdapterRegistry, Project, Runtime};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Default)]
struct Control {
    gate: AtomicBool,
    dropped: AtomicUsize,
    steps: [AtomicUsize; 2],
    path: Mutex<Option<String>>,
    old_path: Mutex<Option<String>>,
    external_audio: Mutex<Option<Arc<dyn AudioReplacementResource>>>,
    font: Mutex<Option<String>>,
}
struct Factory(Arc<Control>);
impl VmFactory for Factory {
    fn create(
        &self,
        client: ClientId,
        _: &crysirror_runtime::model::ClientSpec,
    ) -> anyhow::Result<Box<dyn VmAdapter>> {
        Ok(Box::new(Vm {
            client,
            value: 7,
            control: self.0.clone(),
        }))
    }
}
struct Vm {
    client: ClientId,
    value: u8,
    control: Arc<Control>,
}
impl VmAdapter for Vm {
    fn engine_id(&self) -> &'static str {
        "restore-fixture"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        _: &mut dyn HostServices,
        _: InputFrame,
        _: u32,
    ) -> crysirror_core::Result<VmYield> {
        self.control.steps[self.client.0 as usize - 1].fetch_add(1, Ordering::SeqCst);
        Ok(VmYield::BudgetExhausted)
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Ok(vec![self.value])
    }
    fn restore(&mut self, bytes: &[u8]) -> crysirror_core::Result<()> {
        if bytes.len() != 1 {
            return Err(Error::CorruptSnapshot);
        }
        self.value = bytes[0];
        Ok(())
    }
    fn export_original_save(&mut self) -> crysirror_core::Result<Vec<u8>> {
        self.snapshot()
    }
    fn import_original_save(&mut self, bytes: &[u8]) -> crysirror_core::Result<()> {
        self.restore(bytes)
    }
    fn original_save_import_is_transactional(&self) -> bool {
        true
    }
    fn begin_original_restore(
        &self,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<Box<dyn OriginalRestorePreparation>> {
        let mode = *bytes.first().ok_or(Error::CorruptSnapshot)?;
        Ok(Box::new(Job {
            client: self.client,
            control: self.control.clone(),
            mode,
            image: None,
            ready: false,
            audio: None,
            prepared_audio: None,
        }))
    }
}
struct Job {
    client: ClientId,
    control: Arc<Control>,
    mode: u8,
    image: Option<ImageJob>,
    ready: bool,
    audio: Option<AudioReplacementJob>,
    prepared_audio: Option<Arc<dyn AudioReplacementResource>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
impl OriginalRestorePreparation for Job {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> crysirror_core::Result<OriginalRestoreProgress> {
        let path =
            crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, "unchanged")?;
        assert!(host.write_atomic(self.client, &path, b"bad").is_err());
        let storage = host.storage(self.client)?;
        assert!(storage.write_atomic(&path, b"bad").is_err());
        assert!(storage
            .begin_write_atomic(&path, b"bad".as_slice().into())
            .is_err());
        assert!(host.storage(ClientId(99)).is_err());
        assert!(host.image_metadata(ClientId(99)).is_err());
        assert!(host.submit(self.client, &[]).is_err());
        assert!(host.submit_engine(self.client, 1, br#"{}"#).is_err());
        assert!(host
            .submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: 0,
                    fade_ms: 0.
                }
            )
            .is_err());
        assert!(host
            .begin_audio(
                self.client,
                AudioRequest::Stop {
                    channel: 0,
                    fade_ms: 0.
                }
            )
            .is_err());
        assert!(host.font_library(self.client).is_err());
        if self.image.is_none() {
            self.image = Some(host.image_metadata(self.client)?.begin_process(
                ImageInput::Solid {
                    size: [2, 2],
                    color: [19, 37, 51, 255],
                },
                b"[]".as_slice().into(),
            )?);
        }
        if !self.ready {
            match self.image.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(OriginalRestoreProgress::Pending),
                ImageProcessState::Failed(error) => return Err(Error::Io(error)),
                ImageProcessState::Ready(image) => {
                    assert!(host
                        .publish_image_resource(ClientId(99), 30, image.clone())
                        .is_err());
                    let receipt = host.publish_image_resource(self.client, 30, image)?;
                    *self.control.path.lock().unwrap() = Some(receipt.path.relative().to_owned());
                    if self.mode == 5 {
                        host.release_image_resource(self.client, receipt.path.relative())?;
                    }
                    self.ready = true;
                }
            }
        }
        if self.mode == 2 {
            return Err(Error::Invalid("authored prepare failure"));
        }
        if (8..=12).contains(&self.mode) && self.prepared_audio.is_none() {
            if self.mode == 10 {
                self.prepared_audio = self.control.external_audio.lock().unwrap().clone();
            } else {
                if self.audio.is_none() {
                    let recipes = if self.mode == 8 {
                        vec![]
                    } else {
                        vec![AudioRequest::PlayPcm {
                            channel: 40,
                            source: PcmAudio::new(48000, vec![[0.25, 0.5]; 100])?,
                            parameters: AudioParameters::default(),
                        }]
                    };
                    self.audio = Some(host.begin_audio_replacement(self.client, recipes)?);
                }
                match self.audio.as_mut().unwrap().poll()? {
                    AudioReplacementState::Pending => return Ok(OriginalRestoreProgress::Pending),
                    AudioReplacementState::Failed(error) => return Err(error),
                    AudioReplacementState::Ready(resource) => {
                        *self.control.external_audio.lock().unwrap() = Some(resource.clone());
                        self.prepared_audio = Some(resource);
                    }
                }
            }
        }
        Ok(if self.control.gate.load(Ordering::SeqCst) {
            OriginalRestoreProgress::Ready
        } else {
            OriginalRestoreProgress::Pending
        })
    }
    fn finish(self: Box<Self>) -> crysirror_core::Result<OriginalRestoreCommit> {
        let path = self.control.path.lock().unwrap().clone().unwrap();
        let mut layers = vec![
            json!({"op":"put","layer":{"id":30,"rect":[0,0,2,2],"paint":{"kind":"image","asset":path}}}),
        ];
        if self.mode == 6 {
            layers.push(json!({"op":"begin_external_motion","id":30,"driver":2}));
        }
        if self.mode == 1 || self.mode == 11 {
            layers.push(json!({"op":"set_opacity","id":999,"opacity":1}));
        }
        if self.mode == 14 {
            let font = self
                .control
                .font
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| "missing".into());
            layers.push(json!({"op":"put","layer":{"id":31,"rect":[0,0,2,2],"paint":{"kind":"glyph_run","font":font,"size":16.,"glyphs":[{"character":"A","baseline":[0.,1.]}]}}}));
        }
        Ok(OriginalRestoreCommit {
            prepared_audio: self.prepared_audio.clone(),
            vm: Box::new(Vm {
                client: self.client,
                value: 42,
                control: self.control.clone(),
            }),
            commands: vec![],
            engine_batches: vec![OriginalRestoreBatch {
                schema: 1,
                bytes: serde_json::to_vec(&json!({"layers":layers}))
                    .unwrap()
                    .into(),
            }],
            audio: if self.mode == 3 {
                vec![AudioRequest::Stop {
                    channel: 0,
                    fade_ms: 0.,
                }]
            } else {
                vec![]
            },
            release_images: if self.mode == 7 {
                vec![self.control.old_path.lock().unwrap().clone().unwrap()]
            } else if self.mode == 4 || self.mode == 12 {
                vec!["__image_resource/not-owned".into()]
            } else {
                vec![]
            },
        })
    }
}
fn runtime() -> (tempfile::TempDir, Runtime, Arc<Control>) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("assets")).unwrap();
    let project:Project=serde_json::from_value(json!({"schema":2,"name":"original restore","content_revision":"1","assets":"assets","canvas":[4,4],"clients":[{"name":"a","adapter":"restore-fixture","game":"a"},{"name":"b","adapter":"restore-fixture","game":"b"}]})).unwrap();
    let control = Arc::new(Control::default());
    let mut registry = AdapterRegistry::default();
    registry
        .register("restore-fixture", Arc::new(Factory(control.clone())))
        .unwrap();
    let mut runtime =
        Runtime::with_asset_root(project, root.path(), &root.path().join("state"), registry)
            .unwrap();
    runtime
        .host
        .write_atomic(
            ClientId(1),
            &crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, "unchanged")
                .unwrap(),
            b"original",
        )
        .unwrap();
    (root, runtime, control)
}
fn settle(runtime: &mut Runtime, ticket: OriginalRestoreTicket) -> OriginalRestoreStatus {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let status = runtime
            .poll_original_save_restore(ticket.client, ticket)
            .unwrap();
        if status != OriginalRestoreStatus::Preparing {
            return status;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
}
#[test]
fn preparation_reserves_invisible_real_images_and_only_atomic_ready_commit_replaces_vm_and_scene() {
    let (_root, mut rt, control) = runtime();
    let before = serde_json::to_value(&rt.host.engine).unwrap();
    let ticket = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    assert!(rt
        .commit_original_save_restore(ClientId(1), ticket)
        .is_err());
    // Resource polling does not advance any VM/clock or mutate live state.
    let until = Instant::now() + Duration::from_secs(10);
    while control.path.lock().unwrap().is_none() {
        rt.poll_original_save_restore(ClientId(1), ticket).unwrap();
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let path = control.path.lock().unwrap().clone().unwrap();
    assert!(rt.assets.surface(ClientId(1), &path).is_none());
    assert_eq!(serde_json::to_value(&rt.host.engine).unwrap(), before);
    assert_eq!(rt.now_ms(), 0);
    assert_eq!(rt.export_original_save(ClientId(1)).unwrap(), [7]);
    assert!(rt.snapshot().is_err());
    rt.tick(10, Default::default()).unwrap();
    assert_eq!(control.steps[0].load(Ordering::SeqCst), 0);
    assert_eq!(control.steps[1].load(Ordering::SeqCst), 1);
    control.gate.store(true, Ordering::SeqCst);
    assert_eq!(
        rt.poll_original_save_restore(ClientId(1), ticket).unwrap(),
        OriginalRestoreStatus::Preparing
    );
    let inspection = rt.inspect();
    assert_eq!(
        inspection["native_restore"]["entries"][0]["dependency"],
        "scene_validation"
    );
    assert_eq!(
        inspection["native_restore"]["validation_tasks"]["workers"],
        1
    );
    assert_eq!(settle(&mut rt, ticket), OriginalRestoreStatus::Ready);
    rt.commit_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert_eq!(
        rt.original_save_restore_status(ClientId(1), ticket)
            .unwrap(),
        OriginalRestoreStatus::Committed
    );
    assert_eq!(rt.export_original_save(ClientId(1)).unwrap(), [42]);
    assert_eq!(rt.export_original_save(ClientId(2)).unwrap(), [7]);
    assert_eq!(
        &*rt.assets.surface(ClientId(1), &path).unwrap().rgba,
        [19, 37, 51, 255].repeat(4)
    );
    assert!(rt.assets.surface(ClientId(2), &path).is_none());
    assert!(rt.host.engine.layers.get(ClientId(1), 30).is_some());
    assert!(rt.host.engine.layers.get(ClientId(2), 30).is_none());
    assert_eq!(rt.now_ms(), 10);
    assert_eq!(rt.restored, 1);
    assert!(rt
        .commit_original_save_restore(ClientId(1), ticket)
        .is_err());
    rt.release_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert!(rt
        .original_save_restore_status(ClientId(1), ticket)
        .is_err());
}
#[test]
fn failed_preparation_bad_scene_and_unsupported_audio_leave_original_vm_scene_storage_and_images_unchanged(
) {
    for mode in [1, 2, 3, 4, 5, 6] {
        let (_root, mut rt, control) = runtime();
        control.gate.store(true, Ordering::SeqCst);
        let before = serde_json::to_value(&rt.host.engine).unwrap();
        let ticket = rt
            .begin_original_save_restore(ClientId(1), vec![mode].into())
            .unwrap();
        let status = settle(&mut rt, ticket);
        if mode == 2 {
            assert_eq!(
                status,
                OriginalRestoreStatus::Failed(Error::Invalid("authored prepare failure"))
            );
        } else if mode == 3 {
            assert_eq!(
                status,
                OriginalRestoreStatus::Failed(Error::Unsupported(
                    "atomic native audio restore requires prepared replacement channels"
                ))
            );
        } else {
            assert_eq!(status, OriginalRestoreStatus::Ready);
            assert!(rt
                .commit_original_save_restore(ClientId(1), ticket)
                .is_err());
        }
        assert!(matches!(
            rt.original_save_restore_status(ClientId(1), ticket)
                .unwrap(),
            OriginalRestoreStatus::Failed(_)
        ));
        assert_eq!(rt.export_original_save(ClientId(1)).unwrap(), [7]);
        assert_eq!(rt.now_ms(), 0);
        assert_eq!(rt.restored, 0);
        assert_eq!(serde_json::to_value(&rt.host.engine).unwrap(), before);
        let path = control.path.lock().unwrap().clone().unwrap();
        assert!(rt.assets.surface(ClientId(1), &path).is_none());
        assert_eq!(
            rt.host
                .read(
                    ClientId(1),
                    &crysirror_core::path::VfsPath::new(
                        crysirror_core::path::Mount::Save,
                        "unchanged"
                    )
                    .unwrap()
                )
                .unwrap(),
            b"original"
        );
        rt.tick(0, Default::default()).unwrap();
        assert_eq!(
            control.steps[0].load(Ordering::SeqCst),
            1,
            "failed preparations release the VM hold"
        );
    }
}
#[test]
fn scope_cancel_release_and_framework_epoch_never_admit_stale_results() {
    let (_root, mut rt, control) = runtime();
    let snapshot = rt.snapshot().unwrap();
    let ticket = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    assert!(rt
        .original_save_restore_status(ClientId(2), ticket)
        .is_err());
    let (_other, other, _) = runtime();
    assert!(other
        .original_save_restore_status(ClientId(1), ticket)
        .is_err());
    assert!(rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .is_err());
    rt.cancel_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert_eq!(
        rt.original_save_restore_status(ClientId(1), ticket)
            .unwrap(),
        OriginalRestoreStatus::Cancelled
    );
    assert!(rt
        .commit_original_save_restore(ClientId(1), ticket)
        .is_err());
    assert_eq!(control.dropped.load(Ordering::SeqCst), 1);
    control.gate.store(true, Ordering::SeqCst);
    let next = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    assert!(next.serial > ticket.serial);
    assert_eq!(settle(&mut rt, next), OriginalRestoreStatus::Ready);
    let path = control.path.lock().unwrap().clone().unwrap();
    rt.restore(&snapshot).unwrap();
    assert_eq!(
        rt.original_save_restore_status(ClientId(1), next).unwrap(),
        OriginalRestoreStatus::Cancelled
    );
    assert!(rt.commit_original_save_restore(ClientId(1), next).is_err());
    assert!(rt.assets.surface(ClientId(1), &path).is_none());
    rt.release_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert!(rt.poll_original_save_restore(ClientId(1), ticket).is_err());
    let final_ticket = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    assert!(final_ticket.epoch > next.epoch);
    rt.import_original_save(ClientId(2), &[9]).unwrap();
    assert_eq!(
        rt.original_save_restore_status(ClientId(1), final_ticket)
            .unwrap(),
        OriginalRestoreStatus::Cancelled
    );
    assert_eq!(rt.export_original_save(ClientId(2)).unwrap(), [9]);
}

#[test]
fn scoped_restore_does_not_retire_another_clients_live_producer() {
    let (_root, mut rt, control) = runtime();
    rt.host.submit_engine(ClientId(2),1,br#"{"layers":[{"op":"put","layer":{"id":90,"rect":[0,0,2,2],"paint":{"kind":"solid","color":[1,2,3,255]}}},{"op":"begin_external_motion","id":90,"driver":8}]}"#).unwrap();
    let stamp = rt
        .host
        .engine
        .layers
        .motion_status(ClientId(2), 90)
        .unwrap()
        .drivers[0]
        .stamp;
    control.gate.store(true, Ordering::SeqCst);
    let ticket = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    assert_eq!(settle(&mut rt, ticket), OriginalRestoreStatus::Ready);
    rt.commit_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert!(rt
        .host
        .engine
        .layers
        .has_external_motion_producers(ClientId(2)));
    assert!(!rt
        .host
        .engine
        .layers
        .has_external_motion_producers(ClientId(1)));
    assert_eq!(
        rt.host
            .engine
            .layers
            .motion_status(ClientId(2), 90)
            .unwrap()
            .drivers[0]
            .stamp,
        stamp
    );
}
#[test]
fn old_caller_image_release_is_atomic_and_scene_references_pin_its_immutable_generation() {
    let (_root, mut rt, control) = runtime();
    let mut job = rt
        .host
        .image_metadata(ClientId(1))
        .unwrap()
        .begin_process(
            ImageInput::Solid {
                size: [2, 2],
                color: [0, 0, 99, 255],
            },
            b"[]".as_slice().into(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let image = loop {
        match job.poll().unwrap() {
            ImageProcessState::Ready(image) => break image,
            ImageProcessState::Failed(error) => panic!("{error}"),
            ImageProcessState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    };
    let old = rt
        .host
        .publish_image_resource(ClientId(1), 7, image)
        .unwrap()
        .path
        .relative()
        .to_owned();
    rt.host.submit_engine(ClientId(1),1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":9,"rect":[0,0,2,2],"paint":{"kind":"image","asset":old}}}]})).unwrap()).unwrap();
    let pinned = rt.assets.surface(ClientId(1), &old).unwrap();
    *control.old_path.lock().unwrap() = Some(old.clone());
    control.gate.store(true, Ordering::SeqCst);
    let ticket = rt
        .begin_original_save_restore(ClientId(1), vec![7].into())
        .unwrap();
    assert_eq!(settle(&mut rt, ticket), OriginalRestoreStatus::Ready);
    rt.commit_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert!(
        rt.assets.surface(ClientId(1), &old).is_some(),
        "new scene still refers to ended caller lease"
    );
    rt.host
        .submit_engine(ClientId(1), 1, br#"{"layers":[{"op":"remove","id":9}]}"#)
        .unwrap();
    rt.tick(0, Default::default()).unwrap();
    assert!(rt.assets.surface(ClientId(1), &old).is_none());
    assert_eq!(&*pinned.rgba, [0, 0, 99, 255].repeat(4));
}

#[test]
fn cancellation_also_discards_the_generic_scene_validation_dependency_without_publication() {
    let (_root, mut rt, control) = runtime();
    control.gate.store(true, Ordering::SeqCst);
    let ticket = rt
        .begin_original_save_restore(ClientId(1), vec![0].into())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = rt.poll_original_save_restore(ClientId(1), ticket).unwrap();
        assert_eq!(status, OriginalRestoreStatus::Preparing);
        if rt.inspect()["native_restore"]["entries"][0]["dependency"] == "scene_validation" {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let path = control.path.lock().unwrap().clone().unwrap();
    rt.cancel_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert_eq!(rt.inspect()["native_restore"]["pending_bytes"], 0);
    assert_eq!(
        rt.poll_original_save_restore(ClientId(1), ticket).unwrap(),
        OriginalRestoreStatus::Cancelled
    );
    assert!(rt
        .commit_original_save_restore(ClientId(1), ticket)
        .is_err());
    assert!(rt.assets.surface(ClientId(1), &path).is_none());
    assert_eq!(rt.export_original_save(ClientId(1)).unwrap(), [7]);
    assert_eq!(rt.now_ms(), 0);
}

fn pcm_play(runtime: &mut Runtime, client: ClientId, channel: u32, value: [f32; 2]) {
    runtime
        .host
        .submit_audio(
            client,
            AudioRequest::PlayPcm {
                channel,
                source: PcmAudio::new(48000, vec![value; 100]).unwrap(),
                parameters: AudioParameters::default(),
            },
        )
        .unwrap();
}
fn prepared_audio(runtime: &mut Runtime, client: ClientId) -> Arc<dyn AudioReplacementResource> {
    let mut job = runtime
        .host
        .begin_audio_replacement(client, vec![])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match job.poll().unwrap() {
            AudioReplacementState::Ready(resource) => {
                assert!(matches!(
                    job.poll().unwrap(),
                    AudioReplacementState::Ready(_)
                ));
                return resource;
            }
            AudioReplacementState::Failed(error) => panic!("{error:?}"),
            AudioReplacementState::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    }
}
#[test]
fn prepared_audio_and_image_commit_swap_only_target_vm_channels_and_match_independent_pcm() {
    let (_dir, mut runtime, control) = runtime();
    let (_reference_dir, mut reference, _) = self::runtime();
    for rt in [&mut runtime, &mut reference] {
        pcm_play(rt, ClientId(2), 11, [0.125, 0.0625]);
    }
    pcm_play(&mut runtime, ClientId(1), 11, [0.75, 0.375]);
    let peer_before = runtime
        .host
        .channel_audio
        .playback_info(2, 11)
        .unwrap()
        .unwrap();
    let ticket = runtime
        .begin_original_save_restore(ClientId(1), vec![9].into())
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    // Hold the native resource gate while both real common jobs complete.
    loop {
        runtime
            .poll_original_save_restore(ClientId(1), ticket)
            .unwrap();
        if control.external_audio.lock().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let path = control.path.lock().unwrap().clone().unwrap();
    assert!(runtime.assets.surface(ClientId(1), &path).is_none());
    assert!(runtime.host.channel_audio.is_playing(1, 11));
    assert!(!runtime.host.channel_audio.is_playing(1, 40));
    assert_eq!(
        runtime
            .host
            .channel_audio
            .playback_info(2, 11)
            .unwrap()
            .unwrap()
            .position_frames,
        peer_before.position_frames
    );
    control.gate.store(true, Ordering::SeqCst);
    assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
    runtime
        .commit_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [42]);
    assert!(runtime.assets.surface(ClientId(1), &path).is_some());
    assert!(!runtime.host.channel_audio.is_playing(1, 11));
    assert!(runtime.host.channel_audio.is_playing(1, 40));
    assert!(runtime.host.channel_audio.is_playing(2, 11));
    pcm_play(&mut reference, ClientId(1), 40, [0.25, 0.5]);
    let mut output = vec![[0.; 2]; 32];
    let mut expected = output.clone();
    runtime
        .host
        .channel_audio
        .mixer
        .render(&mut output)
        .unwrap();
    reference
        .host
        .channel_audio
        .mixer
        .render(&mut expected)
        .unwrap();
    assert_eq!(
        output, expected,
        "actual PCM equals independently installed replacement plus unchanged peer"
    );
    assert_eq!(runtime.now_ms(), 0);
    let reused = runtime
        .begin_original_save_restore(ClientId(1), vec![10].into())
        .unwrap();
    assert!(
        matches!(
            settle(&mut runtime, reused),
            OriginalRestoreStatus::Failed(_)
        ),
        "a consumed replacement cannot install twice"
    );
    assert!(runtime.host.channel_audio.is_playing(1, 40));
    assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [42]);
}
#[test]
fn prepared_audio_foreign_consumed_cancelled_and_late_image_failure_never_partly_replace_live_state(
) {
    for mode in [8u8, 9, 11, 12] {
        let (_dir, mut runtime, control) = runtime();
        pcm_play(&mut runtime, ClientId(1), 11, [0.75, 0.375]);
        pcm_play(&mut runtime, ClientId(2), 11, [0.125, 0.0625]);
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        let ticket = runtime
            .begin_original_save_restore(ClientId(1), vec![mode].into())
            .unwrap();
        control.gate.store(true, Ordering::SeqCst);
        assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
        if mode == 9 {
            runtime
                .cancel_original_save_restore(ClientId(1), ticket)
                .unwrap();
        } else if mode >= 11 {
            assert!(runtime
                .commit_original_save_restore(ClientId(1), ticket)
                .is_err());
        } else {
            runtime
                .commit_original_save_restore(ClientId(1), ticket)
                .unwrap();
            assert!(
                !runtime.host.channel_audio.is_playing(1, 11),
                "empty explicit replacement clears target"
            );
            assert!(runtime.host.channel_audio.is_playing(2, 11));
            continue;
        }
        assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [7]);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
        assert!(runtime.host.channel_audio.is_playing(1, 11));
        assert!(!runtime.host.channel_audio.is_playing(1, 40));
        assert!(runtime.host.channel_audio.is_playing(2, 11));
    }
    let (_dir, mut runtime, control) = runtime();
    let (_foreign_dir, mut foreign, _) = self::runtime();
    // Both are genuine engine resources with the same numeric client. Runtime
    // provenance still differs and fails before Ready/publication.
    *control.external_audio.lock().unwrap() = Some(prepared_audio(&mut foreign, ClientId(1)));
    control.gate.store(true, Ordering::SeqCst);
    let ticket = runtime
        .begin_original_save_restore(ClientId(1), vec![10].into())
        .unwrap();
    assert!(matches!(
        settle(&mut runtime, ticket),
        OriginalRestoreStatus::Failed(_)
    ));
    assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [7]);
    *control.external_audio.lock().unwrap() = Some(prepared_audio(&mut runtime, ClientId(2)));
    let ticket = runtime
        .begin_original_save_restore(ClientId(1), vec![10].into())
        .unwrap();
    assert!(matches!(
        settle(&mut runtime, ticket),
        OriginalRestoreStatus::Failed(_)
    ));
    assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [7]);
}

#[test]
fn native_restore_rejects_target_host_media_world_continuations_and_keeps_peer_producers() {
    for kind in 0..3 {
        for owner in [ClientId(1), ClientId(2)] {
            let (_dir, mut runtime, control) = runtime();
            if kind == 1 {
                let entity =
                    serde_json::from_value(json!({"id":99,"name":"external simulation"})).unwrap();
                runtime.host.engine.world.install(owner, &[entity]).unwrap();
            } else if kind == 2 {
                runtime
                    .host
                    .submit(
                        owner,
                        &[crysirror_core::state::Command::PlayAudio {
                            channel: 99,
                            voice: crysirror_core::state::AudioVoice {
                                asset: crysirror_core::path::VfsPath::new(
                                    crysirror_core::path::Mount::Asset,
                                    "old.wav",
                                )
                                .unwrap(),
                                looping: false,
                                gain_milli: 1000,
                            },
                        }],
                    )
                    .unwrap();
            } else {
                runtime
                    .host
                    .media_request(
                        owner,
                        MediaRequest::Play {
                            id: 99,
                            asset: "missing.mpg".into(),
                            looping: false,
                            audio: false,
                            gain: 1.,
                        },
                    )
                    .unwrap();
                assert!(runtime.host.media.has_client_resources(owner));
            }
            let before = serde_json::to_value(&runtime.host.engine).unwrap();
            let ticket = runtime
                .begin_original_save_restore(ClientId(1), vec![0].into())
                .unwrap();
            control.gate.store(true, Ordering::SeqCst);
            assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
            if owner == ClientId(1) {
                assert!(runtime
                    .commit_original_save_restore(ClientId(1), ticket)
                    .is_err());
                assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [7]);
                assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
            } else {
                runtime
                    .commit_original_save_restore(ClientId(1), ticket)
                    .unwrap();
                assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [42]);
            }
            if kind == 1 {
                assert_eq!(runtime.host.engine.world.entities(owner).len(), 1);
            } else if kind == 2 {
                assert!(runtime
                    .host
                    .scene
                    .voices()
                    .keys()
                    .any(|id| id.client == owner));
            } else {
                assert!(runtime.host.media.has_client_resources(owner));
            }
        }
    }
}

#[test]
fn native_restore_font_gate_rejects_missing_or_dynamic_faces_without_publishing_partial_images() {
    for alias in ["missing", "__font/1/foreign"] {
        let (_dir, mut runtime, control) = runtime();
        *control.font.lock().unwrap() = Some(alias.into());
        control.gate.store(true, Ordering::SeqCst);
        let before = serde_json::to_value(&runtime.host.engine).unwrap();
        let ticket = runtime
            .begin_original_save_restore(ClientId(1), vec![14].into())
            .unwrap();
        assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
        assert!(runtime
            .commit_original_save_restore(ClientId(1), ticket)
            .is_err());
        assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [7]);
        assert_eq!(serde_json::to_value(&runtime.host.engine).unwrap(), before);
        let path = control.path.lock().unwrap().clone().unwrap();
        assert!(runtime.assets.surface(ClientId(1), &path).is_none());
    }
}
#[test]
#[ignore = "reads a runtime system font; no font file may be committed"]
fn native_restore_static_project_font_commits_glyph_resource_without_dynamic_lease_claim() {
    let (_dir, mut runtime, control) = runtime();
    let path = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ]
    .into_iter()
    .find(|path| std::path::Path::new(path).is_file())
    .expect("runtime system font required");
    runtime
        .host
        .text
        .add_font("restore-static", std::fs::read(path).unwrap())
        .unwrap();
    *control.font.lock().unwrap() = Some("restore-static".into());
    control.gate.store(true, Ordering::SeqCst);
    let ticket = runtime
        .begin_original_save_restore(ClientId(1), vec![14].into())
        .unwrap();
    assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
    runtime
        .commit_original_save_restore(ClientId(1), ticket)
        .unwrap();
    assert_eq!(runtime.export_original_save(ClientId(1)).unwrap(), [42]);
    assert!(runtime.host.engine.layers.get(ClientId(1), 31).is_some());
    assert_eq!(runtime.now_ms(), 0);
}

#[test]
fn native_restore_invalidates_only_target_ui_events_and_focus_after_successful_atomic_install() {
    for fail in [false, true] {
        for focused in [1u32, 2] {
            let (_dir, mut runtime, control) = runtime();
            for owner in [1u32, 2] {
                let widget = serde_json::from_value(
                    json!({"id":10,"kind":"button","rect":[(owner-1)*2,0,2,2]}),
                )
                .unwrap();
                runtime
                    .host
                    .engine
                    .ui
                    .apply_batch(
                        ClientId(owner),
                        &[crysirror_scene::ui::UiCommand::Put { widget }],
                    )
                    .unwrap();
                runtime
                    .host
                    .engine
                    .ui
                    .events
                    .push(crysirror_scene::ui::UiEvent {
                        widget: crysirror_scene::ui::WidgetId { owner, id: 10 },
                        kind: "activate".into(),
                        value: 1.,
                        menu_bar: None,
                    });
            }
            runtime
                .host
                .engine
                .ui
                .tick(
                    InputFrame {
                        buttons_down: buttons::MOUSE_LEFT,
                        buttons_pressed: buttons::MOUSE_LEFT,
                        pointer_x: if focused == 1 { 1 } else { 3 },
                        pointer_y: 1,
                    },
                    [4, 4],
                )
                .unwrap();
            assert!(
                runtime
                    .host
                    .engine
                    .ui
                    .interaction(focused, 10)
                    .unwrap()
                    .focused
            );
            control.gate.store(true, Ordering::SeqCst);
            let ticket = runtime
                .begin_original_save_restore(ClientId(1), vec![if fail { 12 } else { 0 }].into())
                .unwrap();
            assert_eq!(settle(&mut runtime, ticket), OriginalRestoreStatus::Ready);
            if fail {
                assert!(runtime
                    .commit_original_save_restore(ClientId(1), ticket)
                    .is_err());
                assert_eq!(runtime.host.engine.ui.events.len(), 2);
                assert!(
                    runtime
                        .host
                        .engine
                        .ui
                        .interaction(focused, 10)
                        .unwrap()
                        .focused
                );
            } else {
                runtime
                    .commit_original_save_restore(ClientId(1), ticket)
                    .unwrap();
                assert_eq!(runtime.host.engine.ui.events.len(), 1);
                assert_eq!(runtime.host.engine.ui.events[0].widget.owner, 2);
                assert_eq!(
                    runtime
                        .host
                        .engine
                        .ui
                        .interaction(focused, 10)
                        .unwrap()
                        .focused,
                    focused == 2
                );
            }
        }
    }
}
