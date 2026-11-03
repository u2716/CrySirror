// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/reallive/src/sound.rs (MPL-2.0).
//! The original `Sound` bookkeeping that scripts can observe (volumes and
//! their fades, names, loop modes, fade-outs, start times). Decoding and
//! mixing are the engine's shared audio services; only VM-visible state is
//! kept here.

/// A volume moving linearly to a target (original `Ramp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ramp {
    from: i32,
    to: i32,
    start: u64,
    duration: u64,
}

impl Default for Ramp {
    fn default() -> Self {
        Self::fixed(255)
    }
}

impl Ramp {
    pub fn fixed(value: i32) -> Self {
        Self {
            from: value,
            to: value,
            start: 0,
            duration: 0,
        }
    }

    pub fn value(&self, now: u64) -> i32 {
        if self.duration == 0 || now >= self.start + self.duration {
            return self.to;
        }
        let t = (now - self.start) as f64 / self.duration as f64;
        (f64::from(self.from) + f64::from(self.to - self.from) * t).round() as i32
    }

    pub fn retarget(&mut self, now: u64, to: i32, duration: u64) {
        *self = Self {
            from: self.value(now),
            to: to.clamp(0, 255),
            start: now,
            duration,
        };
    }
}

pub const WAV_CHANNELS: usize = 16;

/// What the original `Voice` exposes to scripts for one channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Track {
    pub name: String,
    pub looped: bool,
    /// When playback started (engine clock), for `bgmTimer`.
    pub started: u64,
    /// Set by a fade-out: the time the sound ends (`PCMFADECHECK`, status 2).
    pub ends_at: Option<u64>,
    pub paused_at: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct SoundState {
    pub bgm: Track,
    pub bgm_volume: Ramp,
    pub wav: [Track; WAV_CHANNELS],
    pub wav_volume: [Ramp; WAV_CHANNELS],
    pub se_volume: Ramp,
    pub koe_volume: Ramp,
    /// The last voice played and its character (`GET_LAST_KOE_COMMAND`).
    pub last_koe: (i32, i32),
}

impl SoundState {
    pub fn new() -> Self {
        Self {
            last_koe: (-1, 0),
            ..Default::default()
        }
    }

    /// The track playing on a channel (0 = BGM, 1..=16 = PCM 0..15).
    pub fn track_mut(&mut self, channel: Option<usize>) -> &mut Track {
        match channel {
            None => &mut self.bgm,
            Some(c) => &mut self.wav[c.min(WAV_CHANNELS - 1)],
        }
    }

    /// Whether the track is fading out at `now` (original `ends_at.is_some()`
    /// while playing).
    pub fn fading(track: &Track, now: u64) -> bool {
        track.ends_at.is_some_and(|end| now < end)
    }

    /// `bgmTimer`: milliseconds since the track started, paused time excluded.
    pub fn bgm_elapsed(&self, now: u64) -> i32 {
        let end = self.bgm.paused_at.unwrap_or(now);
        end.saturating_sub(self.bgm.started).min(i32::MAX as u64) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_matches_original_interpolation_and_clamp() {
        let mut r = Ramp::fixed(255);
        r.retarget(1000, 0, 1000);
        assert_eq!(r.value(1000), 255);
        assert_eq!(r.value(1500), 128);
        assert_eq!(r.value(2000), 0);
        r.retarget(2000, 400, 0);
        assert_eq!(r.value(2000), 255);
    }
}
