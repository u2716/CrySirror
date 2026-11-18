// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Zoom (1xx) and shrink (2xx) screen transitions of the Shapeshifter build
//! (sub_43B390 dispatch).
//!
//! Anchors follow the numpad (sub_43BA20): 7 8 9 on the top edge, 4 and 6
//! on the left and right middles, 1 2 3 on the bottom edge, anything else
//! the centre. Zoom (sub_453A60, sampler sub_4542C0) magnifies the old
//! frame: for i = 0, 20 .. 600 the screen shows the rectangle of
//! 100/(i+100) of the screen centred on the first anchor (kept inside the
//! screen); then the new frame from i = 600 down to 0 on the second
//! anchor. Shrink (sub_4537E0, sub_451B60) draws the old frame at
//! 100/(i+100) of the screen around the first anchor on black for
//! i = 0, 50 .. 2000, then the new frame from 2000 down to 0 around the
//! second. Each step is one present and a 5 ms sleep.
//!
//! The old frame is the presented frame captured when the transition
//! starts; the new one is the layer prefix below the transition's own
//! layers, captured once it is presented.
use crate::vm::Vm;
use anyhow::{Result, bail};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    service::HostServices,
};

pub const STEP_MS: u64 = 5;
/// Draw order reserved for the transition's black backdrop and image.
pub const LAYER_BASE: u32 = 12_002;

pub fn anchor(digit: i32, [w, h]: [i32; 2]) -> [i32; 2] {
    match digit {
        1 => [0, h],
        2 => [w / 2, h],
        3 => [w, h],
        4 => [0, h / 2],
        6 => [w, h / 2],
        7 => [0, 0],
        8 => [w / 2, 0],
        9 => [w, 0],
        _ => [w / 2, h / 2],
    }
}

/// Origin of a `size` box centred on `at`, kept inside the screen.
fn clamp_box(at: [i32; 2], size: [i32; 2], screen: [i32; 2]) -> [i32; 2] {
    let mut o = [0, 0];
    for k in 0..2 {
        let v = (at[k] - size[k] / 2).max(0);
        o[k] = if v + size[k] <= screen[k] { v } else { screen[k] - size[k] };
    }
    o
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    CaptureOld,
    Out,
    CaptureNew,
    In,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// 1xx: magnified frames, each blended at half level over the last.
    Zoom,
    /// 2xx: scaled frames on black.
    Shrink,
    /// State switch (sub_44FCF0(16, 20)): the old frame darkens to black
    /// in 17 steps of 20 ms, then the new state shows.
    Darken,
}

pub struct Zoom {
    pub kind: Kind,
    pub shrink: bool,
    pub from: [i32; 2],
    pub to: [i32; 2],
    phase: Phase,
    step: u32,
    job: Option<ImageJob>,
    next_at: u64,
    /// The frame being drawn (asset, size).
    pub image: Option<(String, [u32; 2])>,
    /// Zoom: the latest drawn steps, oldest first (blend kind 8 at level
    /// 255 composes each step at half level over the previous result).
    pub trail: std::collections::VecDeque<(String, [i32; 4])>,
}

/// Steps after which a half-level trail contributes below 1/256.
const TRAIL: usize = 9;

impl Zoom {
    pub fn new(kind: Kind, from: [i32; 2], to: [i32; 2], job: ImageJob) -> Self {
        let shrink = kind == Kind::Shrink;
        Self { kind, shrink, from, to, phase: Phase::CaptureOld, step: 0, job: Some(job), next_at: 0, image: None, trail: Default::default() }
    }
    fn step_ms(&self) -> u64 {
        if self.kind == Kind::Darken { 20 } else { STEP_MS }
    }
    /// Darken: the black level of the current step (0..255).
    pub fn darkness(&self) -> Option<u8> {
        (self.kind == Kind::Darken && self.phase == Phase::Out).then(|| (255 * self.step.min(16) / 16) as u8)
    }
    /// Records the drawn step for the zoom trail.
    fn push_trail(&mut self, screen: [i32; 2]) {
        if self.kind != Kind::Zoom {
            return;
        }
        if let (Some((r, _)), Some((asset, _))) = (self.rect(screen), &self.image) {
            self.trail.push_back((asset.clone(), r));
            while self.trail.len() > TRAIL {
                self.trail.pop_front();
            }
        }
    }
    pub fn capturing_old(&self) -> bool {
        self.phase == Phase::CaptureOld
    }
    fn steps(&self) -> u32 {
        match self.kind {
            Kind::Shrink => 41,
            Kind::Zoom => 31,
            Kind::Darken => 17,
        }
    }
    fn scale_index(&self) -> u32 {
        let unit = if self.shrink { 50 } else { 20 };
        let i = self.step.min(self.steps() - 1) * unit;
        match self.phase {
            Phase::In => (self.steps() - 1) * unit - i,
            _ => i,
        }
    }
    /// The drawn image rectangle and whether black shows around it.
    pub fn rect(&self, screen: [i32; 2]) -> Option<([i32; 4], bool)> {
        if !matches!(self.phase, Phase::Out | Phase::In) {
            return None;
        }
        if self.kind == Kind::Darken {
            return Some(([0, 0, screen[0], screen[1]], false));
        }
        let i = self.scale_index() as i32;
        let at = if self.phase == Phase::Out { self.from } else { self.to };
        let part = [100 * screen[0] / (i + 100), 100 * screen[1] / (i + 100)];
        let o = clamp_box(at, part, screen);
        if self.shrink {
            Some(([o[0], o[1], part[0], part[1]], true))
        } else {
            // The part fills the screen: the frame scales by screen/part.
            let k = [screen[0] as f32 / part[0].max(1) as f32, screen[1] as f32 / part[1].max(1) as f32];
            let r = [(-o[0] as f32 * k[0]) as i32, (-o[1] as f32 * k[1]) as i32, (screen[0] as f32 * k[0]) as i32, (screen[1] as f32 * k[1]) as i32];
            Some((r, false))
        }
    }
}

impl Vm {
    /// The state switch's fade to black over the frame shown last.
    pub(crate) fn begin_switch_fade(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.zoom.is_some() {
            return Ok(());
        }
        let job = h.begin_frame_capture(self.client)?;
        self.zoom = Some(Zoom::new(Kind::Darken, [0, 0], [0, 0], job));
        Ok(())
    }
    /// Advances the zoom; true when it has finished.
    pub(crate) fn poll_zoom(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        let now = self.now;
        let Some(z) = self.zoom.as_mut() else { return Ok(true) };
        if let Some(job) = z.job.as_mut() {
            let image = match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("RScript zoom transition frame capture: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            z.job = None;
            let size = image.size();
            self.next_image += 1;
            let published = h.publish_image_resource(self.client, self.next_image, image)?;
            let z = self.zoom.as_mut().unwrap();
            z.image = Some((published.path.relative().to_owned(), size));
            z.phase = if z.phase == Phase::CaptureOld { Phase::Out } else { Phase::In };
            z.step = 0;
            z.next_at = now + z.step_ms();
            let screen = self.screen();
            self.zoom.as_mut().unwrap().push_trail(screen);
            self.dirty = true;
            return Ok(false);
        }
        if now < z.next_at {
            return Ok(false);
        }
        z.step += 1;
        z.next_at = now + z.step_ms();
        self.dirty = true;
        if z.step < z.steps() {
            let screen = self.screen();
            self.zoom.as_mut().unwrap().push_trail(screen);
            return Ok(false);
        }
        let z = self.zoom.as_mut().unwrap();
        match z.phase {
            Phase::Out if z.kind != Kind::Darken => {
                // The new scene is presented under the transition's black
                // layer, then captured without it.
                z.phase = Phase::CaptureNew;
                z.image = None;
                let below = self.presented_order(LAYER_BASE);
                let screen = [self.apini.width, self.apini.height];
                let job = h.begin_layer_prefix_image(self.client, screen, [below, 0])?;
                self.zoom.as_mut().unwrap().job = Some(job);
                Ok(false)
            }
            _ => {
                self.zoom = None;
                Ok(true)
            }
        }
    }
}
