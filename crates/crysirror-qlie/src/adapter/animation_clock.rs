//! TABMP temporal bytecode (59EE58), separate from shared curve sampling and
//! Layer drawing. Each nested bundle owns its track clocks and wait counters.
use super::*;

/// The native ring (4FCBE0/4FCC3C), shared by all TABMPs in one VM. Its seed
/// comes from the host's asynchronous session-seed service, never a frame
/// timestamp. The original Delphi Random(range) uses the high product word.
#[derive(Clone)]
pub(super) struct RandomTable {
    words: Box<[u32; 1024]>,
    cursor: usize,
}
impl RandomTable {
    pub(super) fn new(mut seed: u32) -> Self {
        let mut words = Box::new([0; 1024]);
        for word in words.iter_mut() {
            seed = seed.wrapping_mul(134775813).wrapping_add(1);
            *word = ((u64::from(seed) * 0x7fffffff) >> 32) as u32;
        }
        Self { words, cursor: 0 }
    }
    pub(super) fn sample(&mut self, range: i64) -> i64 {
        self.cursor = (self.cursor + 1) & 1023;
        if range == 0 {
            return 0;
        }
        let value = i64::from(self.words[self.cursor]) % range.abs();
        if range < 0 {
            value - range.abs() / 2
        } else {
            value
        }
    }
    pub(super) fn words(&self) -> &[u32; 1024] {
        &self.words
    }
    pub(super) fn noise_words(&self) -> Vec<u16> {
        self.words.iter().map(|word| *word as u16).collect()
    }
}

///59F6B0 observes actual source PCM, not the voice's volume setting. An
///unbound or finished channel has zero level; pending mixer work exposes its
///last completed observation without blocking the VM owner.
pub(super) fn wave_level(
    h: &dyn HostServices,
    client: ClientId,
    object: &Object,
    channels: &BTreeMap<u32, u32>,
) -> Result<i32> {
    let Some(channel) = object
        .wave_channel
        .and_then(|native| channels.get(&native).copied())
        .filter(|channel| *channel != 0)
    else {
        return Ok(0);
    };
    let peak = h
        .audio_info(client, channel)?
        .map_or(0., |info| info.source_peak);
    let multiplier = object.wave_multiplier.unwrap_or(1.);
    ensure!(
        multiplier.is_finite() && multiplier >= 0.,
        "QLIE wave multiplier"
    );
    Ok((peak as f64 * f64::from(multiplier) * 256.)
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32)
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct TrackClock {
    time: i64,
    delay: i64,
    key: Option<usize>,
    visit: u64,
    /// Renderer enable byte (+3276) after `lv`; None keeps track+45.
    #[serde(default)]
    enabled: Option<bool>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Clock {
    input: u64,
    motion: usize,
    requested: usize,
    tracks: Vec<TrackClock>,
}
pub(super) struct Frame {
    pub motion: usize,
    pub tracks: Vec<(u64, u64)>,
    pub enabled: Vec<Option<bool>>,
}
/// Native TABMP keeps 256 per-track slots (+1228 time, +2252 delay, +3276
/// enable); a motion has at most 255 tracks.
pub(super) const TRACK_SLOTS: usize = 256;
impl Clock {
    /// Observe an already initialized paused reader without re-evaluating
    /// control events, audio gates or random branches on an unchanged clock.
    pub(super) fn frozen_frame(&self, requested_motion: usize) -> Option<Frame> {
        (!self.tracks.is_empty() && self.requested == requested_motion).then(|| self.frame())
    }
    fn reset(&mut self, motion: usize, count: usize) {
        self.motion = motion;
        self.tracks = (0..count.max(TRACK_SLOTS))
            .map(|_| TrackClock::default())
            .collect();
    }
    /// Initializes the slots for `motion` the way the next advance would.
    pub(super) fn prepare(&mut self, bundle: &resources::Bundle, motion: usize) -> Result<()> {
        if self.tracks.is_empty() || self.requested != motion {
            let count = bundle
                .animation
                .motions
                .get(motion)
                .context("QLIE animation clock motion")?
                .layers
                .len();
            self.reset(motion, count);
            self.requested = motion;
            self.input = 0;
        }
        Ok(())
    }
    pub(super) fn current_motion(&self) -> usize {
        self.motion
    }
    /// 59F668: `slot` >= 256 addresses every slot.
    pub(super) fn set_track_time(&mut self, slot: u32, time: f32) {
        for (i, t) in self.tracks.iter_mut().enumerate() {
            if slot as usize >= TRACK_SLOTS || i == slot as usize {
                t.time = time.trunc() as i64;
                t.delay = 0;
            }
        }
    }
    /// 5A150C.
    pub(super) fn set_track_delay(&mut self, slot: u32, delay: f32) {
        for (i, t) in self.tracks.iter_mut().enumerate() {
            if slot as usize >= TRACK_SLOTS || i == slot as usize {
                t.delay = delay.trunc() as i64;
            }
        }
    }
    pub(super) fn set_track_enabled(&mut self, slot: Option<usize>, enabled: bool) {
        for (i, t) in self.tracks.iter_mut().enumerate() {
            if slot.is_none_or(|s| s == i) {
                t.enabled = Some(enabled);
            }
        }
    }
    pub(super) fn track_time(&self, slot: usize) -> i64 {
        self.tracks.get(slot).map_or(0, |t| t.time)
    }
    pub(super) fn track_delay(&self, slot: usize) -> i64 {
        self.tracks.get(slot).map_or(0, |t| t.delay)
    }
    pub(super) fn track_enabled(&self, slot: usize) -> Option<bool> {
        self.tracks.get(slot).and_then(|t| t.enabled)
    }
    pub(super) fn seek(&mut self, time: u64) {
        self.input = time;
        for track in &mut self.tracks {
            track.time = time.min(i64::MAX as u64) as i64;
            track.delay = 0;
            track.key = None;
            track.visit = track.visit.wrapping_add(1);
        }
    }
    pub(super) fn advance(
        &mut self,
        bundle: &resources::Bundle,
        requested_motion: usize,
        input: u64,
        values: &animation::Values,
        flags: &[bool; 32],
        targets: &BTreeMap<u8, u8>,
        random: &mut RandomTable,
        audio_level: i32,
    ) -> Result<Frame> {
        if self.tracks.is_empty() || input < self.input || requested_motion != self.requested {
            let motion = bundle
                .animation
                .motions
                .get(requested_motion)
                .context("QLIE animation clock motion")?;
            self.reset(requested_motion, motion.layers.len());
            self.requested = requested_motion;
            self.input = 0;
        }
        let delta = input.saturating_sub(self.input).min(i64::MAX as u64) as i64;
        self.input = input;
        let motion = &bundle.animation.motions[self.motion];
        // +2252 is a remaining delay, not a speed multiplier. Advance every
        // track before evaluating events, in native index order.
        for (clock, track) in self.tracks.iter_mut().zip(&motion.layers) {
            let end = track.frames.last().map_or(0, |key| i64::from(key[0]));
            if clock.time >= end {
                clock.time = end;
            } else if delta > 0 {
                if clock.delay < delta {
                    clock.time = clock.time.saturating_add(delta.saturating_sub(clock.delay));
                    clock.delay = 0;
                } else {
                    clock.delay -= delta;
                }
            }
        }
        for index in 0..motion.layers.len() {
            let track = &motion.layers[index];
            let local = self.tracks[index].time.max(0) as u64;
            let Some(key_index) = track
                .frames
                .partition_point(|key| key[0] as u64 <= local)
                .checked_sub(1)
            else {
                self.tracks[index].key = None;
                continue;
            };
            let key = &track.frames[key_index];
            let next = track.frames.get(key_index + 1).unwrap_or(key);
            let value = |field: usize| -> Result<i64> {
                Ok(values
                    .number(key[field], local, key[0] as u64, next[0] as u64)?
                    .round() as i64)
            };
            let over = local.saturating_sub(key[0] as u64).min(i64::MAX as u64) as i64;
            let entered = self.tracks[index].key != Some(key_index);
            if entered {
                self.tracks[index].visit = self.tracks[index].visit.wrapping_add(1);
            }
            match key[52] {
                0 | 2 | 8 | 11 => {} //2 has no handler in the original switch.
                1 => {
                    let flag = value(53)?;
                    if !(1..32).contains(&flag) || !flags[flag as usize] {
                        let selector = value(54)?;
                        let selected = if (1..32).contains(&selector) {
                            i64::from(targets.get(&(selector as u8)).copied().unwrap_or(255))
                        } else {
                            255
                        };
                        let selected = if selected >= track.frames.len() as i64 {
                            value(55)?
                        } else {
                            selected
                        };
                        let target = usize::try_from(selected)
                            .ok()
                            .filter(|i| *i < track.frames.len())
                            .unwrap_or(0);
                        // Self-jumps pin the key. Other jumps retain overshoot.
                        self.tracks[index].time = i64::from(track.frames[target][0])
                            .saturating_add(if target == key_index { 0 } else { over });
                    }
                }
                3 | 9 | 14 if entered => {
                    let scale = match key[52] {
                        3 => 1000,
                        9 => 10,
                        _ => 1,
                    };
                    self.tracks[index].delay = random.sample(value(53)?.saturating_mul(scale));
                }
                4 => {
                    let threshold = value(53)?;
                    let threshold = if threshold == 0 { 32 } else { threshold };
                    if threshold < i64::from(audio_level) {
                        let target = usize::try_from(value(54)?)
                            .ok()
                            .filter(|i| *i < track.frames.len())
                            .unwrap_or(0);
                        self.tracks[index].time =
                            i64::from(track.frames[target][0]).saturating_add(over);
                    }
                }
                5 if entered => {
                    let target = usize::try_from(value(53)?)
                        .ok()
                        .and_then(|i| bundle.animation.motions.get(i).map(|m| (i, m.layers.len())))
                        .context("QLIE animation motion event target")?;
                    self.reset(target.0, target.1);
                    for clock in &mut self.tracks {
                        clock.time = over;
                    }
                    return Ok(self.frame());
                }
                6 | 10 | 15 if entered => {
                    let scale = match key[52] {
                        6 => 1000,
                        10 => 10,
                        _ => 1,
                    };
                    self.tracks[index].delay = value(53)?.saturating_mul(scale);
                }
                7 => {
                    for (clock, track) in self.tracks.iter_mut().zip(&motion.layers) {
                        clock.time = track.frames.last().map_or(0, |key| i64::from(key[0]));
                    }
                }
                12 | 13 if entered => {
                    let target = if key[52] == 12 {
                        value(53)?.saturating_add(over)
                    } else {
                        local as i64
                    };
                    for clock in &mut self.tracks {
                        clock.time = target;
                    }
                }
                3 | 5 | 6 | 9 | 10 | 12 | 13 | 14 | 15 => {}
                event => bail!("unsupported QLIE animation event {event}"),
            }
            self.tracks[index].key = Some(key_index);
            let end = track.frames.last().map_or(0, |key| i64::from(key[0]));
            self.tracks[index].time = self.tracks[index].time.min(end);
        }
        Ok(self.frame())
    }
    fn frame(&self) -> Frame {
        Frame {
            motion: self.motion,
            tracks: self
                .tracks
                .iter()
                .map(|clock| (clock.time.max(0) as u64, clock.visit))
                .collect(),
            enabled: self.tracks.iter().map(|clock| clock.enabled).collect(),
        }
    }
}
