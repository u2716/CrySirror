// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Plain-menu GeneralInit -> CreateStartScreen/Print -> enter countdown.
//! The UI snapshot and its text are rendered by shared detached-layer workers;
//! no VM framebuffer or UI rasterizer is introduced here.
use super::*;
use crate::native_general_entry::{EntryFadePlayer, EntryKind, ModeEntry};

const CAPTURE: u32 = ROOT + 4800;
const DISPLAY: u32 = ROOT + 4801;

pub(super) struct PlainEntry {
    spec: ModeEntry,
    started: bool,
    old_job: Option<ImageJob>,
    scene_job: Option<ImageJob>,
    frame_job: Option<ImageJob>,
    old: Option<Arc<dyn ImageResource>>,
    new: Option<Arc<dyn ImageResource>>,
    ui: Vec<Value>,
    capture_path: Option<String>,
    display_path: Option<String>,
    fade: Option<EntryFadePlayer>,
}
impl PlainEntry {
    pub(super) fn new(spec: ModeEntry) -> Self {
        Self {
            spec,
            started: false,
            old_job: None,
            scene_job: None,
            frame_job: None,
            old: None,
            new: None,
            ui: vec![],
            capture_path: None,
            display_path: None,
            fade: None,
        }
    }
    pub(super) fn dependency_pending(&self) -> bool {
        self.old_job.is_some()
            || self.scene_job.is_some()
            || self.frame_job.is_some()
            || self
                .fade
                .as_ref()
                .is_some_and(|fade| fade.dependency_pending())
    }
    pub(super) fn remove(&mut self, client: ClientId, host: &mut dyn HostServices) -> Result<()> {
        if self.display_path.is_some() {
            host.submit_engine(
                client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":DISPLAY}]}))?,
            )?;
        }
        for path in [self.capture_path.take(), self.display_path.take()]
            .into_iter()
            .flatten()
        {
            host.release_image_resource(client, &path)?;
        }
        Ok(())
    }
    fn present(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let path = host
            .publish_image_resource(client, DISPLAY, image)?
            .path
            .relative()
            .to_owned();
        if let Err(error)=host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{
            "id":DISPLAY,"rect":[0,0,SIZE[0],SIZE[1]],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":path}
        }}]}))?) {let _=host.release_image_resource(client,&path);return Err(error.into())}
        if let Some(old) = self.display_path.replace(path) {
            host.release_image_resource(client, &old)?;
        }
        Ok(())
    }
}
fn completed(job: &mut Option<ImageJob>) -> Result<Option<Arc<dyn ImageResource>>> {
    let Some(work) = job.as_mut() else {
        return Ok(None);
    };
    match work.poll()? {
        ImageProcessState::Pending => Ok(None),
        ImageProcessState::Failed(error) => {
            anyhow::bail!("NNN General menu entry capture: {error}")
        }
        ImageProcessState::Ready(image) => {
            *job = None;
            Ok(Some(image))
        }
    }
}
impl Menu {
    /// Ordinary menu Calcu/input/background work is suspended throughout entry.
    /// Dependency pumps may only complete admitted captures/effects. In
    /// particular they cannot decrement the countdown or consume a skip click.
    pub(in crate::installed) fn poll_plain_entry(
        &mut self,
        client: ClientId,
        font: &str,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        vm: &mut NativeAdapter,
    ) -> Result<bool> {
        let Some(mut entry) = self.entry.take() else {
            return Ok(true);
        };
        if !entry.started {
            if !admit {
                self.entry = Some(entry);
                return Ok(false);
            }
            entry.old = vm.retained_mode_exit_frame();
            if entry.old.is_none() {
                // The original global old buffer is not initialized by this
                // mode. Match Title's defined fallback to the preceding frame.
                entry.old_job = Some(host.begin_layer_prefix_image_over(
                    client,
                    SIZE,
                    [i32::MAX, i32::MAX],
                    [0, 0, 0, 255],
                )?);
            }
            self.build(client, font, host)?;
            entry.scene_job = Some(host.begin_layer_prefix_image_over(
                client,
                SIZE,
                [i32::MAX, i32::MAX],
                [0, 0, 0, 255],
            )?);
            entry.ui = serde_json::from_slice(&host.query_engine(
                client,
                EngineQuery::UiSnapshot {
                    canvas: SIZE,
                    root: Some(ROOT),
                },
            )?)?;
            if entry.spec.kind != EntryKind::NoPrint {
                self.hide_controls(client, host)?;
            }
            entry.started = true;
        }
        // Activations during entry must not be replayed into the first normal
        // Calcu; CheckSkipClick is sampled from this admitted native frame only.
        let _ = host.query_engine(client, EngineQuery::UiSubtreeEvents(ROOT))?;
        if let Some(old) = completed(&mut entry.old_job)? {
            entry.old = Some(old);
        }
        if let Some(scene) = completed(&mut entry.scene_job)? {
            let path = host
                .publish_image_resource(client, CAPTURE, scene)?
                .path
                .relative()
                .to_owned();
            entry.capture_path = Some(path.clone());
            let mut commands = vec![
                json!({"op":"put","layer":{"id":u32::MAX-1,"rect":[0,0,SIZE[0],SIZE[1]],"order":[i32::MIN+1,0],"paint":{"kind":"image","asset":path}}}),
            ];
            for layer in &entry.ui {
                commands.push(json!({"op":"put","layer":layer}));
            }
            entry.frame_job = Some(host.begin_detached_layers_image(
                client,
                SIZE,
                [i32::MAX, i32::MAX],
                &serde_json::to_vec(&commands)?,
            )?);
        }
        if let Some(frame) = completed(&mut entry.frame_job)? {
            if let Some(path) = entry.capture_path.take() {
                host.release_image_resource(client, &path)?;
            }
            entry.new = Some(frame);
        }
        if entry.fade.is_none() && entry.old.is_some() && entry.new.is_some() {
            entry.fade = Some(EntryFadePlayer::begin(
                host,
                client,
                entry.old.clone().unwrap(),
                entry.new.take().unwrap(),
                entry.spec,
                vm.frame_period_ms(),
            )?);
        }
        let mut done = false;
        if let Some(fade) = entry.fade.as_mut() {
            if admit {
                if let Some(ticket) = fade.audio_ticket(host.now_ms()) {
                    if !vm.poll_general_update(host, self.kind.native_mode(), ticket)? {
                        self.entry = Some(entry);
                        return Ok(false);
                    }
                }
                vm.bind_entry_wipe_context(fade)?;
                let skip =
                    input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT) != 0;
                let image = fade.poll(host, skip)?;
                vm.commit_entry_wipe_context(fade)?;
                if let Some(image) = image {
                    entry.present(client, host, image)?;
                }
            } else if let Some(image) = fade.poll_dependency()? {
                entry.present(client, host, image)?;
            }
            let fade = entry.fade.as_ref().unwrap();
            done = !fade.active() && !fade.dependency_pending();
            if done {
                let (old, new) = fade.retained_buffers();
                vm.retain_general_overlap_buffers(old, new)?;
            }
        }
        if done {
            // GeneralCalcu's count-to-zero frame was still entry. Its next
            // admitted tick restores ordinary painting and resumes mode Calcu.
            entry.remove(client, host)?;
            self.published = false;
            self.build(client, font, host)?;
            self.general_next_ms = None;
            return self.poll_general_audio(host, vm, true);
        }
        self.entry = Some(entry);
        Ok(false)
    }
}
