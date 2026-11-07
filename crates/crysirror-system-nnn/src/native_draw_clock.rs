// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy5027c0 DRAW countdown over the shared adaptive step clock. Clock
//! admission and frame completion are separate; a worker pump never admits
//! elapsed time. DRAW0 reuses the class's prior kaeshita without repaying it.
use anyhow::{Context, Result};
use crysirror_scene::animation::AdaptiveStepClock;
#[derive(Clone)]
pub(crate) struct DrawClock {
    pub clock: AdaptiveStepClock,
    pub at: Option<u64>,
    pub last_steps: u32,
    pub remaining: u32,
    pub steps: u32,
    pub consumed: u32,
    pub background: bool,
    background_limit: Option<u32>,
    direct: bool,
}
impl DrawClock {
    pub(crate) fn new(
        clock: AdaptiveStepClock,
        at: Option<u64>,
        last_steps: u32,
        frames: u32,
    ) -> Result<Self> {
        clock.validate()?;
        Ok(Self {
            clock,
            at,
            last_steps,
            remaining: frames,
            steps: 0,
            consumed: 0,
            background: false,
            background_limit: None,
            direct: frames == 0,
        })
    }
    pub(crate) fn background(mut self, maximum: u32) -> Self {
        self.background = true;
        self.background_limit = Some(maximum);
        self
    }
    pub(crate) fn admit(&mut self, now: u64, skip: bool) -> Result<()> {
        if self.direct {
            self.steps = self.last_steps;
            return Ok(());
        }
        let elapsed = self
            .at
            .map(|at| {
                now.checked_sub(at)
                    .context("NNN DRAW native clock moved backwards")
            })
            .transpose()?
            .unwrap_or(0);
        let mut clock = self.clock.clone();
        clock.begin(elapsed.try_into()?)?;
        //5367d0 caps the requested count before asking CShakin. This is
        // independent of the global scheduler maximum/no-skip preference.
        let requested = self
            .background_limit
            .map_or(self.remaining.max(1), |maximum| {
                self.remaining.max(1).min(maximum)
            });
        let steps = clock.available(Some(requested))?;
        clock.consume_frames(steps)?;
        clock.end()?;
        self.clock = clock;
        self.at = Some(now);
        self.steps = steps;
        self.last_steps = steps;
        self.remaining = self.remaining.saturating_sub(steps);
        //502740/502130: skip changes the countdown after GetKaeshita; it
        // never invents effect advancement equal to the old remaining count.
        if skip {
            self.remaining = 0;
        }
        Ok(())
    }
    pub(crate) fn complete_frame(&mut self) -> Result<()> {
        self.consumed = self
            .consumed
            .checked_add(self.steps)
            .context("NNN DRAW native step total overflow")?;
        Ok(())
    }
    pub(crate) fn finished(&self) -> bool {
        self.direct || self.remaining == 0
    }
}
