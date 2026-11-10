// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonGeneral entry/exit voice and GameCallBack's global delayed audio.
//! These counters advance on admitted native updates, never on I/O completion.
//! Events retain original names/numbers; shared Audio owns loading and playback.
use crate::config::Names;
use anyhow::{Result, ensure};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GeneralAudioEvent {
    /// The existing system-sound API takes the original one-based UI number.
    Sound(i32),
    /// PlaySystemVoiceByFileName uses sys resources and the two alternating
    /// system-sound channels, but VOICESWITCH/VOICEVOLUME and voice-name gates.
    Voice(Vec<u8>),
}
#[derive(Clone, Debug)]
pub(crate) struct GeneralAudioPolicy {
    enter_sound: i32,
    exit_sound: i32,
    voices: bool,
    enter_voice: Option<Vec<u8>>,
    exit_voice: Option<Vec<u8>>,
    enter_wait: i32,
    exit_wait: i32,
}
impl GeneralAudioPolicy {
    pub(crate) fn read(table: &Names) -> Result<Self> {
        let voices = table.integer(b"enterExitVoiceFlag")?.unwrap_or(0) != 0;
        let name = |key: &[u8]| -> Result<Option<Vec<u8>>> {
            if !voices {
                return Ok(None);
            }
            table
                .parameter(key)
                .map(|raw| {
                    ensure!(
                        raw.len() < 256 && !raw.contains(&0),
                        "NNN General voice name bounds"
                    );
                    Ok(raw.to_vec())
                })
                .transpose()
        };
        Ok(Self {
            enter_sound: table.integer(b"enterSENumber")?.unwrap_or(-1),
            exit_sound: table.integer(b"exitSENumber")?.unwrap_or(-1),
            voices,
            enter_voice: name(b"enterVoiceFileName")?,
            exit_voice: name(b"exitVoiceFileName")?,
            enter_wait: if voices {
                table.integer(b"enterVoiceWaitTime")?.unwrap_or(20)
            } else {
                20
            },
            // Constructor's20 is overwritten by GetEnterExitVoiceFileName's5
            // only when enterExitVoiceFlag is enabled, exactly as the source.
            exit_wait: if voices {
                table.integer(b"exitVoiceWaitTime")?.unwrap_or(5)
            } else {
                20
            },
        })
    }
    pub(crate) fn init(&self, global: &mut GeneralAudioDelays, omake: bool) -> GeneralAudioClock {
        // OmakeGeneralInit calls only Init, bypassing GeneralInit's delayed SE
        // and enter voice setup. It must not accidentally replay a mode voice.
        if !omake && self.enter_sound != -1 {
            global.on_enter_sound(self.enter_sound);
        }
        GeneralAudioClock {
            policy: self.clone(),
            enter_count: if omake { 0 } else { self.enter_wait },
            exit_count: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct GeneralAudioClock {
    policy: GeneralAudioPolicy,
    enter_count: i32,
    // These are native hover/reset words. GeneralCalcu's exit voice block is
    // explicitly commented out: hover never emits the GeneralEndMode voice.
    exit_count: i32,
}
impl GeneralAudioClock {
    pub(crate) fn reinitialize(&mut self, global: &mut GeneralAudioDelays, omake: bool) {
        // OmakeGeneralInit calls only the subclass Init: a reused native
        // object retains these General counters, rather than clearing them.
        if !omake {
            self.enter_count = self.policy.enter_wait;
            self.exit_count = 0;
            if self.policy.enter_sound != -1 {
                global.on_enter_sound(self.policy.enter_sound);
            }
        }
    }
    pub(crate) fn calcu(&mut self) -> Option<GeneralAudioEvent> {
        if self.policy.voices && self.enter_count > 0 {
            self.enter_count -= 1;
            if self.enter_count == 0 {
                return self
                    .policy
                    .enter_voice
                    .clone()
                    .map(GeneralAudioEvent::Voice);
            }
        }
        None
    }
    pub(crate) fn end_mode(&self, global: &mut GeneralAudioDelays, omake: bool) {
        if self.policy.exit_sound != -1 {
            global.on_exit_sound(self.policy.exit_sound);
        }
        // OmakeGeneralEndMode schedules its exit SE but bypasses exit voice.
        if !omake && self.policy.voices && self.policy.exit_wait > 0 {
            if let Some(name) = &self.policy.exit_voice {
                global.on_exit_voice(name.clone(), self.policy.exit_wait);
            }
        }
    }
    pub(crate) fn enter_exit_area(&mut self) {
        if !self.policy.voices {
            return;
        }
        if self.exit_count == 0 {
            self.exit_count = self.policy.exit_wait;
        }
        self.enter_count = 0;
    }
    pub(crate) fn leave_exit_area(&mut self) {
        if self.policy.voices {
            self.exit_count = 0;
            self.enter_count = 0;
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct GeneralAudioDelays {
    enter_sound: Option<(i32, i32)>,
    exit_sound: Option<(i32, i32)>,
    exit_voice: Option<(Vec<u8>, i32)>,
    enter_wait: i32,
    exit_wait: i32,
}
impl GeneralAudioDelays {
    pub(crate) fn read(setup: &Names) -> Result<Self> {
        Ok(Self {
            enter_sound: None,
            exit_sound: None,
            exit_voice: None,
            enter_wait: setup.integer(b"delayEnterSEWait")?.unwrap_or(20),
            exit_wait: setup.integer(b"delayExitSEWait")?.unwrap_or(5),
        })
    }
    fn on_enter_sound(&mut self, number: i32) {
        self.enter_sound = Some((number, self.enter_wait));
    }
    fn on_exit_sound(&mut self, number: i32) {
        self.exit_sound = Some((number, self.exit_wait));
    }
    fn on_exit_voice(&mut self, name: Vec<u8>, wait: i32) {
        self.exit_voice = Some((name, wait));
    }
    pub(crate) fn calcu(&mut self) -> Vec<GeneralAudioEvent> {
        let mut events = Vec::with_capacity(3);
        // CGameCallBack::CalcuGame calls SE first, then delayed voice. Keep
        // replacement order when both select the same two system channels.
        for pending in [&mut self.enter_sound, &mut self.exit_sound] {
            if let Some((number, count)) = pending.as_mut() {
                if *count > 0 {
                    *count -= 1;
                    if *count == 0 {
                        // Original PlaySystemSound receives number-1; the
                        // adapter's existing API uses one-based numbers.
                        // Nonpositive numbers still rotate/stop the two
                        // system channels before PlaySystemSound returns for
                        // its negative table index; preserve that event too.
                        events.push(GeneralAudioEvent::Sound(*number));
                        *pending = None;
                    }
                }
            }
        }
        if let Some((name, count)) = self.exit_voice.as_mut() {
            if *count > 0 {
                *count -= 1;
                if *count == 0 {
                    events.push(GeneralAudioEvent::Voice(name.clone()));
                }
            }
            // The original retains the filename after the one-shot counter
            // reaches0; a later GeneralEndMode replaces that global slot.
        }
        events
    }
}

/// One GameCallBack owns delayed audio, while each General object keeps its
/// own entry/hover counters across mode changes. Completion pumps consume the
/// queue without advancing those clocks. The caller supplies an admitted
/// update identity so a retry cannot repeat a decrement or a voice request.
#[derive(Clone, Debug)]
pub(crate) struct GeneralAudioOwner {
    delays: GeneralAudioDelays,
    modes: std::collections::BTreeMap<i32, GeneralAudioClock>,
    queue: std::collections::VecDeque<GeneralAudioEvent>,
    global_update: Option<u64>,
    mode_update: Option<(u64, i32)>,
}
impl GeneralAudioOwner {
    pub(crate) fn read(setup: &Names) -> Result<Self> {
        Ok(Self {
            delays: GeneralAudioDelays::read(setup)?,
            modes: Default::default(),
            queue: Default::default(),
            global_update: None,
            mode_update: None,
        })
    }
    pub(crate) fn install(&mut self, mode: i32, policy: GeneralAudioPolicy) -> Result<()> {
        ensure!((0..=256).contains(&mode), "NNN General mode audio identity");
        ensure!(
            !self.modes.contains_key(&mode),
            "NNN General audio object installed twice"
        );
        // A native General constructor leaves both counters zero. GeneralInit
        // or OmakeGeneralInit below decides which fields actually change.
        self.modes.insert(
            mode,
            GeneralAudioClock {
                policy,
                enter_count: 0,
                exit_count: 0,
            },
        );
        Ok(())
    }
    pub(crate) fn begin(&mut self, mode: i32, omake: bool) -> Result<()> {
        self.modes
            .get_mut(&mode)
            .ok_or_else(|| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))?
            .reinitialize(&mut self.delays, omake);
        Ok(())
    }
    pub(crate) fn end(&mut self, mode: i32, omake: bool) -> Result<()> {
        self.modes
            .get(&mode)
            .ok_or_else(|| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))?
            .end_mode(&mut self.delays, omake);
        Ok(())
    }
    pub(crate) fn enter_exit_area(&mut self, mode: i32) -> Result<()> {
        self.modes
            .get_mut(&mode)
            .ok_or_else(|| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))?
            .enter_exit_area();
        Ok(())
    }
    pub(crate) fn leave_exit_area(&mut self, mode: i32) -> Result<()> {
        self.modes
            .get_mut(&mode)
            .ok_or_else(|| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))?
            .leave_exit_area();
        Ok(())
    }
    pub(crate) fn admit_global(&mut self, update: u64) -> Result<()> {
        if self.global_update == Some(update) {
            return Ok(());
        }
        ensure!(
            self.global_update.is_none_or(|old| update > old),
            "NNN delayed audio update moved backwards"
        );
        self.queue.extend(self.delays.calcu());
        ensure!(self.queue.len() <= 1024, "NNN delayed audio queue budget");
        self.global_update = Some(update);
        Ok(())
    }
    pub(crate) fn admit_mode(&mut self, update: u64, mode: i32, omake: bool) -> Result<()> {
        if self.mode_update == Some((update, mode)) {
            return Ok(());
        }
        ensure!(
            self.global_update == Some(update),
            "NNN General audio before global callback update"
        );
        if !omake {
            if let Some(event) = self
                .modes
                .get_mut(&mode)
                .ok_or_else(|| anyhow::anyhow!("NNN General audio object absent for mode{mode}"))?
                .calcu()
            {
                self.queue.push_back(event);
            }
        }
        ensure!(self.queue.len() <= 1024, "NNN General audio queue budget");
        self.mode_update = Some((update, mode));
        Ok(())
    }
    pub(crate) fn pending(&self) -> Option<&GeneralAudioEvent> {
        self.queue.front()
    }
    pub(crate) fn complete(&mut self, event: &GeneralAudioEvent) -> Result<()> {
        ensure!(
            self.queue.front() == Some(event),
            "NNN General audio completion identity changed"
        );
        self.queue.pop_front();
        Ok(())
    }
}
