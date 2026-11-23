// SPDX-License-Identifier: MPL-2.0
use super::{Effect, Program};
use anyhow::{Context, Result, bail};
use glam::Vec3;

#[derive(Debug)]
enum Action {
    Delay,
    Move { actor: String, from: Vec3, to: Vec3 },
    Fade { front: bool, to: f32, started: bool },
    Dither { actor: i32, from: f32, to: f32, detached: bool },
}

#[derive(Debug)]
struct Running {
    id: u64,
    background: bool,
    start: u64,
    duration: u64,
    waiting_delay: bool,
    last_sample: Option<u64>,
    action: Action,
}

#[derive(Default)]
pub(super) struct Timeline {
    now: u64,
    serial: u64,
    actions: Vec<Running>,
    alpha: [f32; 2],
    /// Each SceneFadeCanvas owns one tween. Restarting it cancels the old
    /// tween; every command waiting for this canvas observes the replacement.
    fades: [Option<(u64, u64, f32, f32)>; 2],
    dithers: std::collections::HashMap<i32, f32>,
    pub(super) background: bool,
}

impl Timeline {
    pub(super) fn pending(&self, background: bool) -> bool {
        self.actions.iter().any(|action| action.background == background && !matches!(action.action, Action::Dither { detached: true, .. }))
    }

    fn enqueue(&mut self, delay: f32, seconds: f32, action: Action) {
        self.serial += 1;
        self.actions.push(Running {
            id: self.serial,
            background: self.background,
            start: self.now.saturating_add((delay * 1000.) as u64),
            duration: (seconds * 1000.) as u64,
            waiting_delay: delay > 0.,
            last_sample: None,
            action,
        });
    }

    /// CommandList.ProcessEnd calls Result(false). A move snaps to its
    /// endpoint; a delayed fade cancels and a timer is discarded.
    pub(super) fn end_background(&mut self, effects: &mut Vec<Effect>) {
        self.actions.retain_mut(|work| {
            if work.background {
                match &mut work.action {
                    Action::Move { actor, to, .. } => effects.push(Effect::MoveOffset {
                        id: work.id, actor: actor.clone(), offset: to.to_array(), done: true,
                    }),
                    Action::Fade { front, .. } => self.fades[*front as usize] = None,
                    Action::Dither { actor, detached, .. } => {
                        // Result(false) writes 1 and releases the command. Its
                        // separately scheduled LitMotion has no cancel handle.
                        if !*detached { publish_dither(&mut self.dithers, *actor, 1., effects); }
                        *detached = true;
                        return true;
                    }
                    Action::Delay => {}
                }
            }
            !work.background
        });
    }

    fn update(&mut self, now: u64, effects: &mut Vec<Effect>) {
        self.now = now;
        for (lane, tween) in self.fades.iter_mut().enumerate() {
            if let Some((start, duration, from, to)) = *tween {
                let t = (now.saturating_sub(start) as f32 / duration as f32).min(1.);
                let alpha = from + (to - from) * t;
                if alpha != self.alpha[lane] {
                    self.alpha[lane] = alpha;
                    effects.push(Effect::CanvasAlpha { front: lane == 1, alpha });
                }
                if t == 1. { *tween = None; }
            }
        }
        self.actions.retain_mut(|work| {
            if now < work.start || work.last_sample == Some(now) { return true; }
            work.last_sample = Some(now);
            if work.waiting_delay {
                // FadeDelay.Process clamps the timer at the delay, then the
                // following frame starts the fade from the current opacity.
                work.waiting_delay = false;
                return true;
            }
            let elapsed = now - work.start;
            match &mut work.action {
                Action::Delay => elapsed < work.duration,
                Action::Dither { actor, from, to, detached } => {
                    let t = if work.duration == 0 { 1. } else { (elapsed as f32 / work.duration as f32).min(1.) };
                    publish_dither(&mut self.dithers, *actor, *from + (*to - *from) * t, effects);
                    if elapsed >= work.duration && !*detached {
                        // Both native Result methods (A09230) write 1.0,
                        // including DitherOut once its tween has reached 0.
                        publish_dither(&mut self.dithers, *actor, 1., effects);
                    }
                    elapsed < work.duration
                }
                Action::Move { actor, from, to } => {
                    // Mathf.InverseLerp(0,0,t) returns zero. Result(false)
                    // still explicitly snaps to the endpoint on cancellation.
                    let t = if work.duration == 0 { 0. } else { (elapsed as f32 / work.duration as f32).min(1.) };
                    effects.push(Effect::MoveOffset {
                        id: work.id, actor: actor.clone(),
                        offset: from.lerp(*to, t).to_array(), done: elapsed >= work.duration,
                    });
                    elapsed < work.duration
                }
                Action::Fade { front, to, started } => {
                    let lane = *front as usize;
                    if !*started {
                        *started = true;
                        let alpha = self.alpha[lane];
                        if work.duration == 0 {
                            self.alpha[lane] = *to;
                            self.fades[lane] = None;
                            effects.push(Effect::CanvasAlpha { front: *front, alpha: *to });
                        } else {
                            // FadeCanvas.StartAsync (32CB0B0) interpolates
                            // between 10 ms and duration by remaining opacity.
                            let distance = (*to - alpha).abs().clamp(0., 1.);
                            let duration = (10. + (work.duration as f32 - 10.) * distance).round() as u64;
                            self.fades[lane] = Some((now, duration.max(1), alpha, *to));
                        }
                    }
                    self.fades[lane].is_some()
                }
            }
        });
    }
}

fn publish_dither(values: &mut std::collections::HashMap<i32, f32>, actor: i32, value: f32, effects: &mut Vec<Effect>) {
    let old = values.get(&actor).copied().unwrap_or(1.);
    if old != value {
        values.insert(actor, value);
        effects.push(Effect::Dither { actor, value });
    }
}

impl Program {
    pub(crate) fn inherit_dither(&mut self, actor: i32, value: f32) {
        self.timeline.dithers.insert(actor, value);
    }
    pub(crate) fn poll(&mut self, now: u64, effects: &mut Vec<Effect>) {
        self.timeline.update(now, effects);
    }

    pub(crate) fn has_work(&self, background: bool) -> bool {
        self.timeline.pending(background)
    }

    /// Schedule the Process/Result commands used by the installed rest
    /// scenario in either CommandController list.
    pub(super) fn schedule(&mut self, name: &str, args: &[String], effects: &mut Vec<Effect>) -> Result<()> {
        let number = |i: usize, default: f32| -> Result<f32> {
            match args.get(i) {
                Some(s) if !s.is_empty() => Ok(s.trim().parse::<f32>()?),
                _ => Ok(default),
            }
        };
        match name {
            "CharaSetDither" | "CharaDitherIn" | "CharaDitherOut" => {
                let arg = args.first().context("missing dither actor")?;
                let actor = if arg == "NowID" { 0 } else { arg.trim().parse()? };
                let value: f32 = args.get(1).context("missing dither value or duration")?.trim().parse()?;
                anyhow::ensure!(value.is_finite(), "nonfinite dither operand");
                let old = self.timeline.dithers.get(&actor).copied().unwrap_or(1.);
                if name == "CharaSetDither" {
                    publish_dither(&mut self.timeline.dithers, actor, value.clamp(0., 1.), effects);
                } else {
                    let fade_in = name == "CharaDitherIn";
                    if fade_in == (old <= 0.) {
                        self.timeline.enqueue(0., value, Action::Dither { actor, from: old, to: f32::from(fade_in), detached: false });
                    } else {
                        publish_dither(&mut self.timeline.dithers, actor, 1., effects);
                    }
                }
            }
            "WaitAbs" => self.timeline.enqueue(0., number(0, 0.)?, Action::Delay),
            "Fade" | "FadeDelay" => {
                let delayed = name == "FadeDelay";
                let front = args.get(if delayed { 3 } else { 2 }).is_some_and(|a| a.eq_ignore_ascii_case("front"));
                let cover = args.first().is_none_or(|a| a.eq_ignore_ascii_case("in"));
                let delay = if delayed { number(2, 1.)? } else { 0. };
                self.timeline.enqueue(delay, number(1, 0.5)?, Action::Fade { front, to: f32::from(cover), started: false });
            }
            "CharaLerpVectorMove" => {
                let vector = |i: usize| -> Result<Vec3> {
                    let key = args.get(i).context("missing vector name")?;
                    Ok(Vec3::from_array(*self.vectors.get(key).with_context(|| format!("unknown vector {key:?}"))?))
                };
                if args.get(4).is_some_and(|s| !s.eq_ignore_ascii_case("true")) {
                    bail!("world-space CharaLerpVectorMove needs the native parent transform");
                }
                let actor = args.first().context("missing move actor")?.clone();
                self.timeline.enqueue(0., number(1, 0.)?, Action::Move { actor, from: vector(2)?, to: vector(3)? });
            }
            _ => unreachable!(),
        }
        // Capture the actor's starting transform in command order, before
        // subsequent commands can move it, and apply immediate canvas changes.
        self.timeline.update(self.timeline.now, effects);
        Ok(())
    }
}
