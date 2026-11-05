//! TABMP temporal bytecode (59EE58), separate from shared curve sampling and
//! Layer drawing. Each nested bundle owns its track clocks and wait counters.
use super::*;

/// The native ring (4FCBE0/4FCC3C), shared by all TABMPs in one VM. Its seed
/// comes from the host's asynchronous session-seed service, never a frame
/// timestamp. The original Delphi Random(range) uses the high product word.
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
    fn sample(&mut self, range: i64) -> i64 {
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
}

#[derive(Default)]
struct TrackClock {
    time: i64,
    delay: i64,
    key: Option<usize>,
    visit: u64,
}
#[derive(Default)]
pub(super) struct Clock {
    input: u64,
    motion: usize,
    requested: usize,
    tracks: Vec<TrackClock>,
}
pub(super) struct Frame {
    pub motion: usize,
    pub tracks: Vec<(u64, u64)>,
}
impl Clock {
    fn reset(&mut self, motion: usize, count: usize) {
        self.motion = motion;
        self.tracks = (0..count).map(|_| TrackClock::default()).collect();
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
                4 => bail!("QLIE conditional animation jump requires audio-wave observation"),
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
        }
    }
}
