// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native4ff500 /502180/500bc0 message counters. The engine still owns glyph
//! reveal; these counters decide when its completed native actor may resume.
use super::*;

#[derive(Clone)]
pub(super) struct Timing {
    rates: [u32; 5],
    normal_rate: u32,
    voice_wait: i32,
    no_voice_wait: i32,
    script_wait: i32,
    script_voice_wait: i32,
    wait_voice: bool,
    script: bool,
    automatic: bool,
    voice: bool,
    user_remaining: i64,
    voice_remaining: i64,
    script_elapsed: i64,
    previous: Option<u64>,
}
impl Timing {
    pub(super) fn normal_speed(&mut self, index: usize) -> Result<u32> {
        self.normal_rate = *self.rates.get(index).context("NNN message speed index")?;
        Ok(self.normal_rate)
    }
    pub(super) fn new(normal_rate: u32) -> Self {
        Self {
            rates: [20, 40, 60, 120, 10000],
            normal_rate,
            voice_wait: 500,
            no_voice_wait: 2000,
            script_wait: 2000,
            script_voice_wait: 500,
            wait_voice: false,
            script: false,
            automatic: false,
            voice: false,
            user_remaining: 0,
            voice_remaining: 1,
            script_elapsed: 0,
            previous: None,
        }
    }
    pub(super) fn configure(&mut self, setup: &Names) -> Result<()> {
        let get =
            |name: &[u8], default| -> Result<i32> { Ok(setup.integer(name)?.unwrap_or(default)) };
        self.voice_wait = get(b"voiceLastWait", 500)?;
        self.no_voice_wait = get(b"noVoiceLastWait", 2000)?;
        self.script_wait = get(b"autoMessageLastWait", 2000)?;
        self.script_voice_wait = get(b"autoMessageVoiceLastWait", self.voice_wait)?;
        self.wait_voice = get(b"autoMessageWaitVoiceFlag", 0)? != 0;
        if get(b"customMessageSpeed", 0)? != 0 {
            for index in 0..5 {
                let value = get(
                    format!("messageSpeed{}", index + 1).as_bytes(),
                    self.rates[index] as i32,
                )?;
                self.rates[index] =
                    u32::try_from(value).context("NNN negative automatic glyph rate")?;
                ensure!(
                    self.rates[index] <= 1_000_000,
                    "NNN automatic glyph rate budget"
                );
            }
        }
        Ok(())
    }
    pub(super) fn start(
        &mut self,
        script: i32,
        automatic: bool,
        voice: bool,
        length: u32,
    ) -> Result<u32> {
        // Old500bc0 uses AutoMessage-1 directly as a five-entry array index.
        // Reject its undefined out-of-bounds domain instead of reading RAM.
        ensure!(
            script <= 5,
            "NNN automatic message speed outside native five-entry table"
        );
        self.script = script > 0;
        self.automatic = automatic;
        self.voice = voice;
        self.user_remaining = if voice {
            i64::from(length) + i64::from(self.voice_wait)
        } else {
            i64::from(self.no_voice_wait)
        };
        self.voice_remaining = if voice {
            i64::from(length) + i64::from(self.script_voice_wait)
        } else {
            1
        };
        self.script_elapsed = 0;
        self.previous = None;
        Ok(if script > 0 {
            self.rates[script as usize - 1]
        } else {
            self.normal_rate
        })
    }
    pub(super) fn arm(&mut self, now: u64) {
        self.previous = Some(now)
    }
    pub(super) fn script(&self) -> bool {
        self.script
    }
    pub(super) fn set_automatic(&mut self, enabled: bool) {
        self.automatic = enabled
    }
    pub(super) fn poll(&mut self, now: u64, complete: bool) -> Result<bool> {
        let Some(previous) = self.previous.replace(now) else {
            return Ok(false);
        };
        let elapsed = i64::try_from(
            now.checked_sub(previous)
                .context("NNN message clock moved backwards")?,
        )?;
        self.voice_remaining = self.voice_remaining.saturating_sub(elapsed);
        if !complete {
            return Ok(false);
        }
        if self.script {
            // 502180 checks the old autoMessageCount, then adds this update's
            // time. Voice-tail time is decremented before the reveal check.
            let advance = self.script_elapsed >= i64::from(self.script_wait)
                && (!self.wait_voice || self.voice_remaining <= 0);
            self.script_elapsed = self.script_elapsed.saturating_add(elapsed);
            return Ok(advance);
        }
        if self.automatic {
            self.user_remaining = self.user_remaining.saturating_sub(elapsed);
            return Ok(self.user_remaining <= 0);
        }
        // Original CheckAuto resets its counter to1 while auto mode is off.
        self.user_remaining = 1;
        Ok(false)
    }
}
