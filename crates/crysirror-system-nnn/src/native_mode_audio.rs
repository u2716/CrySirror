// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Admission of base-General actors whose constructors do not read voice keys.
//! The ticket survives asynchronous audio work; completing it is not a second
//! Calcu. The native actor, not decoder polls or host refresh, owns its cadence.
use crate::{bridge::Bridge, config::Names, native_general_audio::GeneralAudioPolicy};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::HostServices;

pub(crate) struct ModeAudioAdmission {
    mode: i32,
    counter: u64,
    next_ms: Option<u64>,
    pending: bool,
    waiting_init: bool,
}
impl ModeAudioAdmission {
    pub(crate) fn new(mode: i32) -> Result<Self> {
        // PrintCG, PrintMovie and ChartMenu inherit the base defaults. A new
        // subclass that calls GetEnterExitVoiceFileName must supply its table.
        ensure!(
            matches!(mode, 23 | 27 | 34),
            "NNN base General admission profile"
        );
        Ok(Self {
            mode,
            counter: 0,
            next_ms: None,
            pending: false,
            waiting_init: false,
        })
    }
    pub(crate) fn pending(&self) -> bool {
        self.pending || self.waiting_init
    }
    pub(crate) fn resume_clock(&mut self, now: u64) {
        // Loading or a modal child does not accumulate a wall-time backlog of
        // GeneralCalcu calls. A pending ticket itself remains untouched.
        if self.next_ms.is_some() {
            self.next_ms = Some(now);
        }
    }
    pub(crate) fn begin(
        &mut self,
        host: &mut dyn HostServices,
        bridge: &mut Bridge,
        admit: bool,
    ) -> Result<bool> {
        if !bridge.poll_general_audio(host)? {
            self.waiting_init = true;
            return Ok(false);
        }
        self.waiting_init = false;
        if bridge.general_audio_mode() != Some(self.mode) {
            ensure!(
                !self.pending,
                "NNN General owner changed during an admitted actor frame"
            );
            if !admit {
                return Ok(false);
            }
            bridge.begin_general_audio(
                self.mode,
                GeneralAudioPolicy::for_mode(self.mode, &Names { tokens: vec![] })?,
            )?;
            self.counter = 0;
            self.next_ms = None;
        }
        Ok(true)
    }
    /// True admits one actor Calcu, including the completion of a previously
    /// admitted Calcu. A completion-only pump cannot create a new ticket.
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
        mut bridge: Option<&mut Bridge>,
        admit: bool,
        period_ms: u64,
    ) -> Result<bool> {
        if let Some(owner) = bridge.as_deref_mut() {
            if !self.begin(host, owner, admit)? {
                return Ok(false);
            }
        }
        let now = host.now_ms();
        if !self.pending {
            if !admit || self.next_ms.is_some_and(|next| now < next) {
                return Ok(false);
            }
            self.pending = true;
        }
        if let Some(owner) = bridge {
            if !owner.poll_general_update(host, self.mode, self.counter)? {
                return Ok(false);
            }
        }
        self.pending = false;
        self.counter = self
            .counter
            .checked_add(1)
            .context("NNN actor General update exhausted")?;
        self.next_ms = Some(
            now.checked_add(period_ms)
                .context("NNN actor General deadline overflow")?,
        );
        Ok(true)
    }
}
