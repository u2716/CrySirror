// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SystemMenu -> Title owns its capture, native countdown and resource task.
//! Cancelling the foreground actor never acknowledges its suspended command.
use super::*;
use crysirror_core::image_processing::ImageResource;
pub(super) struct TitleReturn {
    task: Option<Task<crate::native_shell::NativeStartupProfile>>,
    profile: Option<crate::native_shell::NativeStartupProfile>,
    capture: Option<ImageJob>,
    frame: Option<Arc<dyn ImageResource>>,
    returning: crate::native_general::ModeReturn,
    fade: Option<crate::native_general::ExitFadePlayer>,
    fade_started: bool,
    path: Option<String>,
    client: ClientId,
    frame_ms: u64,
}
impl TitleReturn {
    pub fn dependency_pending(&self)->bool {self.task.is_some()||self.capture.is_some()||self.fade.as_ref().is_some_and(|fade|fade.dependency_pending())}
    pub fn begin(
        vm: &NativeAdapter,
        table: &crate::config::Names,
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<Self> {
        let config = vm
            .configuration()
            .cloned()
            .context("NNN title return configuration absent")?;
        let storage = host.storage(client)?;
        let task = pool.spawn(move |cancel| {
            crate::native_shell::NativeStartupProfile::read(&config, storage.as_ref(), &cancel)
        })?;
        let returning = crate::native_general::ModeReturn::read(
            table,
            Some(crate::native_shell::NativeMode::Title),
        )?;
        let frame=if returning.capture_exit {None}else{Some(vm.retained_mode_exit_frame().context("NNN SystemMenu requires its initialized retained exit buffer")?)};
        let capture = if returning.capture_exit {Some(host.begin_layer_prefix_image_over(
            client,
            SIZE,
            [i32::MAX, i32::MAX],
            [0, 0, 0, 255],
        )?)}else{None};
        Ok(Self {
            task: Some(task),
            profile: None,
            capture,
            frame,
            returning,
            fade: None,
            fade_started: false,
            path: None,
            client,
            frame_ms: vm.frame_period_ms(),
        })
    }
    pub fn poll(&mut self, host: &mut dyn HostServices, admit: bool,vm:&mut NativeAdapter) -> Result<bool> {
        if let Some(task) = self.task.as_mut() {
            if let Some(profile) = task.try_take()? {
                self.profile = Some(profile);
                self.task = None;
            }
        }
        if let Some(capture) = self.capture.as_mut() {
            match capture.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN system-menu exit capture: {error}")
                }
                ImageProcessState::Ready(image) => {
                    self.frame = Some(image);
                    self.capture = None;
                }
            }
        }
        if self.profile.is_none() {
            return Ok(false);
        }
        if !self.fade_started {
            if let Some(spec) = self.returning.fade {
                self.fade = Some(crate::native_general::ExitFadePlayer::new_with_enter(
                    host,
                    self.client,
                    self.frame.as_ref().unwrap().clone(),
                    vm.retained_general_enter_frame(),
                    spec,
                    self.frame_ms,
                )?);
            }
            self.fade_started = true;
        }
        if let Some(fade) = self.fade.as_mut() {
            vm.bind_wipe_context(fade)?;
            let image = if admit {
                fade.poll(host)?
            } else {
                fade.poll_dependency()?
            };
            vm.commit_wipe_context(fade)?;
            if let Some(image) = image {
                if fade.finished() {
                    self.frame = Some(image);
                } else {
                    let next = host
                        .publish_image_resource(self.client, 0x6604fff0, image)?
                        .path
                        .relative()
                        .to_owned();
                    host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":0x6604fff0u32,"rect":[0,0,800,600],"order":[2147483647,0],"paint":{"kind":"image","asset":next}}}]}))?)?;
                    if let Some(old) = self.path.replace(next) {
                        host.release_image_resource(self.client, &old)?;
                    }
                }
            }
            if !fade.finished() {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn cancel(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.task = None;
        self.capture = None;
        self.fade = None;
        if let Some(path) = self.path.take() {
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(
                    &serde_json::json!({"layers":[{"op":"remove","id":0x6604fff0u32}]}),
                )?,
            )?;
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
    pub fn finish(
        mut self,
        host: &mut dyn HostServices,
    ) -> Result<(
        crate::native_shell::NativeStartupProfile,
        Arc<dyn ImageResource>,
    )> {
        self.cancel(host)?;
        Ok((
            self.profile
                .take()
                .context("NNN title return setup not ready")?,
            self.frame
                .take()
                .context("NNN title return frame not ready")?,
        ))
    }
}
