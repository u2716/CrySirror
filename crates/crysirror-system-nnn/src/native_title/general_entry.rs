// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonTitle::CreateStartScreen and General entry transition policy.
//! Start-screen construction uses the same shared button/image operations as
//! ordinary Print, without running a mode Calcu or consuming input.
use super::*;
use crate::native_general_entry::{EntryFadePlayer, EntryKind, ModeEntry};

pub(super) struct GeneralEntry {
    spec: ModeEntry,
    initial_started: bool,
    fade: Option<EntryFadePlayer>,
    done: bool,
}
impl GeneralEntry {
    pub(super) fn new(spec: ModeEntry) -> Self {
        Self {
            spec,
            initial_started: false,
            fade: None,
            done: false,
        }
    }
}
impl NativeTitlePlayer {
    pub(crate) fn configure_entry(&mut self, table: &Names) -> Result<()> {
        ensure!(
            self.inflight.is_none() && !self.widgets && self.counter == 0,
            "NNN General entry configured after first Title frame"
        );
        self.general_entry = ModeEntry::read(table)?.map(GeneralEntry::new);
        Ok(())
    }
    pub(super) fn restart_entry(&mut self) {
        if let Some(entry) = self.general_entry.as_mut() {
            *entry = GeneralEntry::new(entry.spec);
        }
    }
    pub(crate) fn entry_active(&self) -> bool {
        self.general_entry.as_ref().is_some_and(|entry| !entry.done)
    }
    pub(crate) fn entry_blocks_update(&self, now_ms: u64) -> bool {
        self.general_entry.as_ref().is_some_and(|entry| {
            !entry.done
                && entry
                    .fade
                    .as_ref()
                    .is_none_or(|fade| !fade.resumes_mode_at(now_ms))
        })
    }
    pub(super) fn entry_dependency_pending(&self) -> bool {
        self.general_entry
            .as_ref()
            .and_then(|entry| entry.fade.as_ref())
            .is_some_and(|fade| fade.dependency_pending())
    }
    pub(super) fn capture_only_for_entry(&self) -> bool {
        self.general_entry.as_ref().is_some_and(|entry| {
            entry.initial_started
                && entry.fade.is_none()
                && !entry.done
                && entry.spec.kind != EntryKind::NoPrint
        })
    }
    /// Complete only work already admitted by Init/Calcu. This pump must not
    /// begin another native update, consume a skip edge or tick the background.
    pub(super) fn poll_entry_dependency(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let Some(mut entry) = self.general_entry.take() else {
            return Ok(());
        };
        if let Some(fade) = entry.fade.as_mut() {
            if let Some(image) = fade.poll_dependency()? {
                self.present_return_frame(host, image)?;
                if fade.final_frame_ready() {
                    self.publish_widgets(
                        host,
                        self.counter >= self.spec.menu_wait.saturating_add(self.spec.menu_fade)
                            && self
                                .system_intro
                                .as_ref()
                                .is_none_or(|intro| intro.finished),
                    )?;
                }
            }
        }
        self.general_entry = Some(entry);
        Ok(())
    }
    pub(super) fn poll_entry(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        background: Arc<dyn ImageResource>,
        vm: &mut crate::adapter::NativeAdapter,
    ) -> Result<bool> {
        let Some(mut entry) = self.general_entry.take() else {
            return Ok(true);
        };
        if entry.done {
            self.general_entry = Some(entry);
            return Ok(true);
        }
        if !entry.initial_started {
            // GeneralInit -> CreateEnterScreen -> Title::CreateStartScreen:
            // CMenuButtonGroup::Calcu(NULL), then Print at menuStartCount0.
            // Button mode resets but its active animation timer does not tick.
            let mut states = self.states.clone();
            for (state, spec) in states.iter_mut().zip(&self.spec.buttons) {
                state.calcu_without_input(spec);
            }
            let mut frame = TitleFrame {
                background: background.clone(),
                counter: 0,
                states: states.clone(),
                jobs: Vec::new(),
                ready: Vec::new(),
                final_job: None,
                sounds: Default::default(),
                system_intro: self.system_intro.clone(),
            };
            if self.spec.menu_wait == 0 {
                let end = self.spec.menu_wait.saturating_add(self.spec.menu_fade);
                for (index, state) in states.iter_mut().enumerate() {
                    if !self.spec.buttons[index].exists {
                        continue;
                    }
                    let input = self.button_input(
                        index,
                        background.clone(),
                        state,
                        0,
                        frame.system_intro.as_ref(),
                    )?;
                    let effects = if matches!(input, ImageInput::Ternary { .. }) {
                        let (frames, percent) = state.frame(&self.spec.buttons[index]);
                        let image = self.images[index]
                            .image
                            .as_ref()
                            .context("NNN initial menu image absent")?;
                        vec![ImageEffect::MaskedPairBlend {
                            spec: MaskedPairBlend {
                                sources: [
                                    self.spec.buttons[index].source_rect(
                                        state.mode,
                                        frames[0],
                                        image.size(),
                                    )?,
                                    self.spec.buttons[index].source_rect(
                                        state.mode,
                                        frames[1],
                                        image.size(),
                                    )?,
                                ],
                                destination: [0, 0],
                                weights256: [percent * 256 / 100, (100 - percent) * 256 / 100],
                                clip: None,
                            },
                        }]
                    } else {
                        vec![]
                    };
                    frame.jobs.push((
                        index,
                        host.image_metadata(self.client)?
                            .begin_process(input, serde_json::to_vec(&effects)?.into())?,
                    ));
                    self.api_calls += 1;
                    if end == 0
                        && frame
                            .system_intro
                            .as_ref()
                            .is_none_or(|intro| intro.finished)
                    {
                        state.printed(&self.spec.buttons[index]);
                    } else {
                        state.animation = [0; 4];
                    }
                }
            }
            frame.states = states;
            self.inflight = Some(frame);
            self.publish_widgets(host, false)?;
            entry.initial_started = true;
            self.general_entry = Some(entry);
            return Ok(false);
        }
        if self.inflight.is_some() {
            self.general_entry = Some(entry);
            return Ok(false);
        }
        if entry.fade.is_none() {
            let new = self.completed_frame()?;
            // Overlap buffers are global across mode objects. The VM retains
            // that exit buffer independently of an executing background SPT.
            let old = vm.retained_mode_exit_frame().unwrap_or(background);
            entry.fade = Some(EntryFadePlayer::begin(
                host,
                self.client,
                old,
                new,
                entry.spec,
                self.frame_ms,
            )?);
        }
        let fade = entry.fade.as_mut().unwrap();
        vm.bind_entry_wipe_context(fade)?;
        let skip = input.buttons_pressed
            & (crysirror_core::service::buttons::MOUSE_LEFT
                | crysirror_core::service::buttons::MOUSE_RIGHT)
            != 0;
        let image = fade.poll(host, skip)?;
        vm.commit_entry_wipe_context(fade)?;
        if let Some(image) = image {
            self.present_return_frame(host, image)?;
        }
        if fade.final_frame_ready() {
            self.publish_widgets(
                host,
                self.counter >= self.spec.menu_wait.saturating_add(self.spec.menu_fade)
                    && self
                        .system_intro
                        .as_ref()
                        .is_none_or(|intro| intro.finished),
            )?;
        }
        let mut resume = false;
        if !fade.active() && !fade.dependency_pending() {
            let (old, new) = fade.retained_buffers();
            vm.retain_general_overlap_buffers(old, new)?;
            entry.done = true;
            self.clock_origin = None;
            resume = true;
            self.publish_widgets(
                host,
                self.counter >= self.spec.menu_wait.saturating_add(self.spec.menu_fade)
                    && self
                        .system_intro
                        .as_ref()
                        .is_none_or(|intro| intro.finished),
            )?;
        }
        self.general_entry = Some(entry);
        // A count-to-zero update still belongs to entry. Once its image and
        // next native deadline complete, this update executes ordinary Calcu.
        Ok(resume)
    }
}
