// SPDX-License-Identifier: MPL-2.0
//! Scripted step list `0x2280..=0x228b` (engine +4760, 0x14002a1b0..0x14002aa30 of the
//! 2021-10-07 x64 build; the 2016 build has the same layout). A 20-byte header precedes
//! sixteen 96-byte records; the adapter plays them against objects and the camera.

use super::resources::path_string;
use serde_json::{json, Value};

pub const LIMIT: usize = 16;

/// Record +20.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0x2286`: blocks until `duration` ms after the previous wait.
    Wait,
    /// `0x2288` (sub 0), `0x2289` (sub 1) and `0x228a` (sub 2) on script object `slot`.
    Object,
    /// `0x2287`: camera 0 eye; `sub` is the easing mode.
    Camera,
    /// `0x228b`: image of part `slot` of message window 0.
    Window,
    Other(u32),
}

impl Kind {
    fn code(self) -> u32 {
        match self {
            Self::Wait => 1,
            Self::Object => 3,
            Self::Camera => 4,
            Self::Window => 5,
            Self::Other(code) => code,
        }
    }

    fn of(code: u32) -> Self {
        match code {
            1 => Self::Wait,
            3 => Self::Object,
            4 => Self::Camera,
            5 => Self::Window,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Step {
    pub kind: Kind,
    /// +24.
    pub sub: u32,
    /// +28.
    pub slot: u32,
    /// +32, a 64-byte `lstrcpyA` buffer.
    name: Vec<u8>,
    /// +96.
    pub duration: u32,
    /// +100..+108 as float bits.
    pub values: [u32; 3],
    /// +112, the world track easing of subs 1 and 2.
    pub mode: u32,
}

impl Step {
    pub fn name(&self) -> Option<String> {
        path_string(&self.name).ok()
    }

    pub fn position(&self) -> [f32; 3] {
        self.values.map(f32::from_bits)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Sequence {
    /// +0.
    pub running: bool,
    steps: Vec<Step>,
    /// +8.
    pub index: u32,
    /// +12: the start clock, rebased on restore.
    pub start: u32,
    /// +16: milliseconds consumed by completed waits.
    pub waited: u32,
}

impl Sequence {
    /// `0x2280` (0x14002a650).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn count(&self) -> u32 {
        self.steps.len() as u32
    }

    /// `0x2281`: the current record while running, else -1.
    pub fn current(&self) -> u32 {
        if self.running { self.index } else { u32::MAX }
    }

    /// `0x2282` (0x14002a9d0): restarts a stopped, non-empty list.
    pub fn start(&mut self, now: u32) {
        if !self.running && !self.steps.is_empty() {
            self.start = now;
            self.waited = 0;
            self.index = 0;
            self.running = true;
        }
    }

    /// `0x2283` (0x14002aa30): stops without clearing.
    pub fn stop(&mut self) {
        if self.running && !self.steps.is_empty() {
            self.running = false;
        }
    }

    pub fn step(&self, index: u32) -> Option<&Step> {
        self.steps.get(index as usize)
    }

    /// Appends a record; the native list ignores pushes past sixteen.
    pub fn push(&mut self, kind: Kind, sub: u32, slot: u32, name: &[u8], duration: u32, values: [u32; 3], mode: u32) {
        if self.steps.len() < LIMIT {
            let name = name.iter().copied().take_while(|&byte| byte != 0).take(63).collect();
            self.steps.push(Step { kind, sub, slot, name, duration, values, mode });
        }
    }

    pub fn snapshot(&self) -> Value {
        let steps: Vec<_> = self
            .steps
            .iter()
            .map(|step| {
                json!({
                    "kind": step.kind.code(), "sub": step.sub, "slot": step.slot, "name": step.name,
                    "duration": step.duration, "values": step.values, "mode": step.mode,
                })
            })
            .collect();
        json!({
            "running": self.running, "index": self.index, "start": self.start,
            "waited": self.waited, "steps": steps,
        })
    }

    /// Accepts the five-word header earlier checkpoints stored, which had no records.
    pub fn from_snapshot(value: &Value) -> Option<Self> {
        if let Some(words) = value.as_array() {
            let words: Vec<u32> = words.iter().map(|word| word.as_u64().map(|word| word as u32)).collect::<Option<_>>()?;
            let [running, _, index, start, waited]: [u32; 5] = words.try_into().ok()?;
            return Some(Self { running: running != 0, steps: Vec::new(), index, start, waited });
        }
        let word = |value: &Value| value.as_u64().map(|word| word as u32);
        let mut steps = Vec::new();
        for step in value["steps"].as_array()?.iter().take(LIMIT) {
            let values: Vec<u32> = step["values"].as_array()?.iter().map(word).collect::<Option<_>>()?;
            steps.push(Step {
                kind: Kind::of(word(&step["kind"])?),
                sub: word(&step["sub"])?,
                slot: word(&step["slot"])?,
                name: step["name"].as_array()?.iter().map(|byte| byte.as_u64().map(|byte| byte as u8)).collect::<Option<_>>()?,
                duration: word(&step["duration"])?,
                values: values.try_into().ok()?,
                mode: word(&step["mode"])?,
            });
        }
        Some(Self {
            running: value["running"].as_bool()?,
            steps,
            index: word(&value["index"])?,
            start: word(&value["start"])?,
            waited: word(&value["waited"])?,
        })
    }

    /// 0x14002a5b0: a restored running list resumes its current wait from now.
    pub fn rebase(&mut self, now: u32) {
        if self.running && !self.steps.is_empty() {
            self.start = now.wrapping_sub(self.waited);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_sixteen_records_and_round_trips() {
        let mut sequence = Sequence::default();
        for slot in 0..20 {
            sequence.push(Kind::Object, 0, slot, b"bg01\0junk", 300, [0; 3], 0);
        }
        assert_eq!(sequence.count(), 16);
        assert_eq!(sequence.current(), u32::MAX);
        sequence.start(1000);
        assert_eq!(sequence.current(), 0);
        let restored = Sequence::from_snapshot(&sequence.snapshot()).unwrap();
        assert_eq!(restored.count(), 16);
        assert_eq!(restored.step(3).unwrap().name().as_deref(), Some("bg01"));
        assert!(restored.running);
    }
}
