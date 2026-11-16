// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Base GeneralInit/CreateStartScreen for non-button actors. The actor publishes
//! its initial Print, without Calcu; shared scene/UI image workers capture it.
use super::*;
use crate::native_general_entry::{EntryFadePlayer, EntryKind, ModeEntry};
use crysirror_core::{
    image_processing::ImageResource,
    service::{EngineQuery, buttons},
};
use serde_json::json;

pub(super) struct ActorEntry {
    spec: ModeEntry,
    mode: i32,
    display: u32,
    ui_root: Option<u32>,
    old: Option<Arc<dyn ImageResource>>,
    old_job: Option<ImageJob>,
    scene_job: Option<ImageJob>,
    frame_job: Option<ImageJob>,
    capture_path: Option<String>,
    display_path: Option<String>,
    ui: Vec<serde_json::Value>,
    started: bool,
    audio_started: bool,
    fade: Option<EntryFadePlayer>,
}
fn completed(job: &mut Option<ImageJob>) -> Result<Option<Arc<dyn ImageResource>>> {
    let Some(work) = job.as_mut() else {
        return Ok(None);
    };
    match work.poll()? {
        ImageProcessState::Pending => Ok(None),
        ImageProcessState::Failed(error) => {
            anyhow::bail!("NNN General actor entry capture: {error}")
        }
        ImageProcessState::Ready(image) => {
            *job = None;
            Ok(Some(image))
        }
    }
}
impl ActorEntry {
    /// Called before the first native Print changes the scene. The global old
    /// buffer is retained when available; the defined initial fallback captures
    /// the prior screen rather than accidentally capturing the new actor twice.
    pub(super) fn new(
        spec: ModeEntry,
        mode: i32,
        display: u32,
        ui_root: Option<u32>,
        vm: &NativeAdapter,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Self> {
        let old = vm.retained_mode_exit_frame();
        let old_job = if old.is_none() {
            Some(host.begin_layer_prefix_image_over(
                client,
                SIZE,
                [i32::MAX, i32::MAX],
                [0, 0, 0, 255],
            )?)
        } else {
            None
        };
        Ok(Self {
            spec,
            mode,
            display,
            ui_root,
            old,
            old_job,
            scene_job: None,
            frame_job: None,
            capture_path: None,
            display_path: None,
            ui: vec![],
            started: false,
            audio_started: false,
            fade: None,
        })
    }
    pub(super) fn prepare_old(&mut self) -> Result<bool> {
        if let Some(old) = completed(&mut self.old_job)? {
            self.old = Some(old);
        }
        Ok(self.old.is_some() || self.fade.is_some())
    }
    pub(super) fn pending(&self) -> bool {
        self.old_job.is_some()
            || self.scene_job.is_some()
            || self.frame_job.is_some()
            || self.fade.as_ref().is_some_and(|f| f.dependency_pending())
    }
    fn show(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let path = host
            .publish_image_resource(client, self.display, image)?
            .path
            .relative()
            .to_owned();
        if let Err(error)=host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":self.display,"rect":[0,0,800,600],"order":[i32::MAX-1,0],"paint":{"kind":"image","asset":path}}}]}))?){let _=host.release_image_resource(client,&path);return Err(error.into())}
        if let Some(old) = self.display_path.replace(path) {
            host.release_image_resource(client, &old)?;
        }
        Ok(())
    }
    pub(super) fn remove(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        if self.display_path.is_some() {
            host.submit_engine(
                client,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":self.display}]}))?,
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
    /// The caller has finished initial Print dependencies before entering here.
    /// A dependency-only retry may finish jobs but never advances GeneralCalcu.
    pub(super) fn poll(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut NativeAdapter,
        input: InputFrame,
        admit: bool,
    ) -> Result<bool> {
        if !self.audio_started {
            if !admit {
                return Ok(false);
            }
            vm.begin_general_audio(
                self.mode,
                crate::native_general_audio::GeneralAudioPolicy::for_mode(
                    self.mode,
                    &crate::config::Names { tokens: vec![] },
                )?,
            )?;
            self.audio_started = true;
        }
        if !vm.poll_general_audio(host)? {
            return Ok(false);
        }
        if !self.started {
            if !admit {
                return Ok(false);
            }
            self.scene_job = Some(host.begin_layer_prefix_image_over(
                client,
                SIZE,
                [i32::MAX, i32::MAX],
                [0, 0, 0, 255],
            )?);
            if let Some(root) = self.ui_root {
                self.ui = serde_json::from_slice(&host.query_engine(
                    client,
                    EngineQuery::UiSnapshot {
                        canvas: SIZE,
                        root: Some(root),
                    },
                )?)?;
                if self.spec.kind != EntryKind::NoPrint {
                    host.submit_engine(
                        client,
                        1,
                        &serde_json::to_vec(
                            &json!({"ui":[{"op":"set_visible","id":root,"visible":false}]}),
                        )?,
                    )?;
                }
            }
            self.started = true;
        }
        if let Some(root) = self.ui_root {
            let _ = host.query_engine(client, EngineQuery::UiSubtreeEvents(root))?;
        }
        if let Some(old) = completed(&mut self.old_job)? {
            self.old = Some(old);
        }
        if let Some(scene) = completed(&mut self.scene_job)? {
            if self.ui.is_empty() {
                self.frame_job = Some(
                    host.image_metadata(client)?
                        .begin_process(ImageInput::Resource(scene), b"[]".as_slice().into())?,
                );
            } else {
                let path = host
                    .publish_image_resource(client, self.display + 1, scene)?
                    .path
                    .relative()
                    .to_owned();
                self.capture_path = Some(path.clone());
                let mut commands = vec![
                    json!({"op":"put","layer":{"id":u32::MAX-1,"rect":[0,0,800,600],"order":[i32::MIN+1,0],"paint":{"kind":"image","asset":path}}}),
                ];
                commands.extend(
                    self.ui
                        .iter()
                        .map(|layer| json!({"op":"put","layer":layer})),
                );
                self.frame_job = Some(host.begin_detached_layers_image(
                    client,
                    SIZE,
                    [i32::MAX, i32::MAX],
                    &serde_json::to_vec(&commands)?,
                )?);
            }
        }
        if self.fade.is_none() && self.old.is_none() {
            return Ok(false);
        }
        if self.display_path.is_none()
            && self.spec.kind != EntryKind::NoPrint
            && self.spec.frames > 0
        {
            if let Some(old) = self.old.clone() {
                self.show(host, client, old)?;
            }
        }
        if let Some(frame) = completed(&mut self.frame_job)? {
            if let Some(path) = self.capture_path.take() {
                host.release_image_resource(client, &path)?;
            }
            self.fade = Some(EntryFadePlayer::begin(
                host,
                client,
                self.old.take().unwrap(),
                frame,
                self.spec,
                vm.frame_period_ms(),
            )?);
        }
        let Some(fade) = self.fade.as_mut() else {
            return Ok(false);
        };
        let image = if admit {
            if let Some(ticket) = fade.audio_ticket(host.now_ms()) {
                if !vm.poll_general_update(host, self.mode, ticket)? {
                    return Ok(false);
                }
            }
            vm.bind_entry_wipe_context(fade)?;
            let image = fade.poll(
                host,
                input.buttons_pressed & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT) != 0,
            )?;
            vm.commit_entry_wipe_context(fade)?;
            image
        } else {
            fade.poll_dependency()?
        };
        let done = !fade.active() && !fade.dependency_pending();
        if done {
            let (old, new) = fade.retained_buffers();
            vm.retain_general_overlap_buffers(old, new)?;
        }
        if let Some(image) = image {
            self.show(host, client, image)?;
        }
        if done {
            self.remove(host, client)?;
            if let Some(root) = self.ui_root {
                if self.spec.kind != EntryKind::NoPrint {
                    host.submit_engine(
                        client,
                        1,
                        &serde_json::to_vec(
                            &json!({"ui":[{"op":"set_visible","id":root,"visible":true}]}),
                        )?,
                    )?;
                }
            }
        }
        Ok(done)
    }
}
