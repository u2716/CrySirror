// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Main General audio admission. A native update ticket is separate from a
//! host poll and from decoder completion. Only the shared Audio owner plays.
use super::*;
use crate::native_general_audio::{GeneralAudioEvent, GeneralAudioOwner, GeneralAudioPolicy};
#[derive(Clone, Debug)]
pub(super) struct ForegroundAudio {
    message: GeneralAudioPolicy,
    selection: GeneralAudioPolicy,
    mode: Option<i32>,
    selection_receipt: Option<u64>,
    counter: u64,
    next_ms: Option<u64>,
    pub paused: bool,
}
impl ForegroundAudio {
    pub(super) fn detached(&self) -> Self {
        let mut value = self.clone();
        value.mode = None;
        value.selection_receipt = None;
        value.next_ms = None;
        value.paused = false;
        value
    }
}
impl Bridge {
    pub(crate) fn configure_foreground_general_audio(
        &mut self,
        message: &crate::config::Names,
        selection: &crate::config::Names,
    ) -> Result<()> {
        ensure!(
            self.general_foreground.is_none(),
            "NNN foreground General audio configured twice"
        );
        self.general_foreground = Some(ForegroundAudio {
            message: GeneralAudioPolicy::for_mode(10, message)?,
            selection: GeneralAudioPolicy::for_mode(9, selection)?,
            mode: None,
            selection_receipt: None,
            counter: 0,
            next_ms: None,
            paused: false,
        });
        Ok(())
    }
    pub(crate) fn pause_foreground_general_audio(&mut self, paused: bool) {
        if let Some(driver) = self.general_foreground.as_mut() {
            driver.paused = paused;
            driver.next_ms = None;
        }
    }
    pub(crate) fn poll_foreground_general_audio(
        &mut self,
        host: &mut dyn HostServices,
        call: Option<&NativeCall>,
        admit: bool,
    ) -> Result<bool> {
        if !self.poll_general_audio(host)? {
            return Ok(false);
        }
        let Some(driver) = self.general_foreground.as_ref() else {
            return Ok(true);
        };
        if driver.paused || self.background_read_marks.is_some() || self.chart_mode_active() {
            return Ok(true);
        }
        // DRAW and OVERRAP are PRINTMESSAGE's internal printMode, not
        // separate General objects11/14 (the source never constructs those).
        let retained=driver.mode.filter(|mode|self.general_audio.as_ref().and_then(|owner|owner.primary_mode())==Some(*mode));
        let mode = match call.map(|c| (&c.payload, c.identify, c.code)) {
            Some((Payload::Message { .. }, _, _)) if !self.skip.next_command => Some(10),
            Some((Payload::Selection { .. }, _, _)) => Some(9),
            Some((Payload::Words { words, .. }, 0x66660006, 0x22220004))
                if words.first().is_some_and(|n| *n > 0) && !self.skip.next_command =>
            {
                Some(10)
            }
            Some((Payload::Words { .. }, 0x66660006, 0x22220005))
                if !(self.skip.next_command && self.skip.effect_command) =>
            {
                Some(10)
            }
            None => match self.wait_kind(None) {
                "saved_message_input" => Some(10),
                "saved_selection_input" => Some(9),
                _ => retained,
            },
            _ => retained,
        };
        let Some(mode) = mode else {
            return Ok(true);
        };
        // Continuing SELECT's General through unrelated SPT instructions is
        // not another SELECT entry: only a real new selection receipt resets it.
        let selected = if mode==9{call.filter(|call|matches!(call.payload,Payload::Selection{..})).map(|call|call.serial)}else{None};
        let changed = driver.mode != Some(mode)
            || self.general_audio.as_ref().and_then(|a| a.primary_mode()) != Some(mode)
            || (mode == 9 && selected.is_some() && driver.selection_receipt != selected);
        if !admit {
            return Ok(!changed);
        }
        if changed {
            let policy = if mode == 9 {
                driver.selection.clone()
            } else {
                driver.message.clone()
            };
            self.begin_general_audio(mode, policy)?;
            let driver = self.general_foreground.as_mut().unwrap();
            driver.mode = Some(mode);
            driver.selection_receipt = selected;
            driver.counter = 0;
            driver.next_ms = None;
        }
        // A resource pump completes the already-admitted Calcu. It cannot
        // spend another General counter while that native frame is pending.
        if self.dependency_pending() {
            return Ok(true);
        }
        let driver = self.general_foreground.as_ref().unwrap();
        let now = host.now_ms();
        if driver.next_ms.is_some_and(|next| now < next) {
            return Ok(true);
        }
        let ticket = driver.counter;
        if !self.poll_general_update(host, mode, ticket)? {
            return Ok(false);
        }
        let period = self.frame_period_ms();
        let driver = self.general_foreground.as_mut().unwrap();
        driver.counter = driver
            .counter
            .checked_add(1)
            .context("NNN foreground General update exhausted")?;
        driver.next_ms = Some(
            now.checked_add(period)
                .context("NNN foreground General deadline overflow")?,
        );
        Ok(true)
    }
    pub(crate) fn configure_general_audio(&mut self, setup: &crate::config::Names) -> Result<()> {
        ensure!(
            self.general_audio.is_none(),
            "NNN General audio configured twice"
        );
        self.general_audio = Some(GeneralAudioOwner::read(setup)?);
        Ok(())
    }
    pub(crate) fn begin_general_audio(
        &mut self,
        mode: i32,
        policy: GeneralAudioPolicy,
    ) -> Result<()> {
        if self.general_audio.is_none() {
            // Explicit standalone components use the source constructor's
            // delay defaults; installed sessions install their game setup.
            self.general_audio = Some(GeneralAudioOwner::read(&crate::config::Names {
                tokens: vec![],
            })?);
        }
        self.general_audio
            .as_mut()
            .context("NNN General audio owner absent")?
            .begin_primary(mode, policy)
    }
    pub(crate) fn end_current_general_audio(&mut self) -> Result<()> {
        if let Some(owner) = self.general_audio.as_mut() {
            owner.end_current_primary()?;
        }
        Ok(())
    }
    pub(crate) fn general_audio_mode(&self) -> Option<i32> {
        self.general_audio
            .as_ref()
            .and_then(|owner| owner.primary_mode())
    }
    pub(crate) fn end_general_audio(&mut self, mode: i32) -> Result<()> {
        self.general_audio
            .as_mut()
            .context("NNN General audio owner absent")?
            .end_primary(mode)
    }
    pub(crate) fn poll_general_audio(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        loop {
            let event = self
                .general_audio
                .as_ref()
                .and_then(|owner| owner.pending())
                .cloned();
            let Some(event) = event else {
                return Ok(true);
            };
            let done = match &event {
                GeneralAudioEvent::Sound(number) => self.poll_system_sound(host, *number)?,
                GeneralAudioEvent::Voice(name) => self.poll_system_voice(host, name)?,
            };
            if !done {
                return Ok(false);
            }
            self.general_audio.as_mut().unwrap().complete(&event)?;
        }
    }
    pub(crate) fn poll_general_update(
        &mut self,
        host: &mut dyn HostServices,
        mode: i32,
        ticket: u64,
    ) -> Result<bool> {
        if !self.poll_general_audio(host)? {
            return Ok(false);
        }
        let fresh=self.general_audio
            .as_mut()
            .context("NNN General audio owner absent")?
            .admit_primary(mode, ticket)?;
        if fresh{self.voice_speak.observe_control(host.now_ms())?;}
        self.poll_general_audio(host)
    }
}
