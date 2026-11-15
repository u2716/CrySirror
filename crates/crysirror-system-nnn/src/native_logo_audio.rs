// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonLogo's soundCount/played array. Calcu checks these after its skip
//! early-return and before phase changes; the counter spans all three phases.
//! Only names and ordering live here. The shared Audio service owns playback.
use crate::config::Names;
use anyhow::{Result, ensure};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(crate) struct SoundSpec {
    kind: i32,
    logo: i32,
    at: i32,
    filename: Option<Vec<u8>>,
}
pub(crate) fn read(table: &Names) -> Result<Vec<SoundSpec>> {
    let number = table.integer(b"soundNumber")?.unwrap_or(0);
    ensure!(number <= 1024, "NNN logo timed audio allocation budget");
    let mut specs = Vec::new();
    for i in 1..=number.max(0) {
        let filename = table
            .parameter(format!("soundFilename{i}").as_bytes())
            .map(|v| v.to_vec());
        if let Some(name) = &filename {
            ensure!(
                name.len() <= 243 && !name.contains(&0),
                "NNN logo timed audio filename bounds"
            );
        }
        specs.push(SoundSpec {
            kind: table
                .integer(format!("soundType{i}").as_bytes())?
                .unwrap_or(0),
            logo: table
                .integer(format!("soundLogo{i}").as_bytes())?
                .unwrap_or(1),
            at: table
                .integer(format!("soundTime{i}").as_bytes())?
                .unwrap_or(0),
            filename,
        });
    }
    Ok(specs)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    SystemWave(Vec<u8>),
    ReplayVoice(Vec<u8>),
}
#[derive(Default)]
pub(crate) struct Clock {
    specs: Vec<SoundSpec>,
    played: Vec<bool>,
    count: i32,
    queue: VecDeque<Event>,
}
impl Clock {
    pub(crate) fn new(specs: Vec<SoundSpec>) -> Self {
        Self {
            played: vec![false; specs.len()],
            specs,
            ..Self::default()
        }
    }
    pub(crate) fn next_logo(&mut self) {
        self.count = 0;
    }
    pub(crate) fn calcu(&mut self, logo: usize) {
        self.count = self.count.wrapping_add(1);
        for (spec, played) in self.specs.iter().zip(&mut self.played) {
            if !*played && spec.logo == logo as i32 + 1 && self.count >= spec.at {
                // Native marks even a NULL filename played before testing it.
                // Other nonzero soundType values all select ReplayVoice.
                *played = true;
                if let Some(name) = &spec.filename {
                    self.queue.push_back(if spec.kind == 0 {
                        Event::SystemWave(name.clone())
                    } else {
                        Event::ReplayVoice(name.clone())
                    });
                }
            }
        }
    }
    pub(crate) fn pending(&self) -> Option<&Event> {
        self.queue.front()
    }
    pub(crate) fn complete(&mut self, event: &Event) -> Result<()> {
        ensure!(
            self.queue.front() == Some(event),
            "NNN logo timed audio receipt changed"
        );
        self.queue.pop_front();
        Ok(())
    }
}
