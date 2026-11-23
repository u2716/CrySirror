//!4284B0 case15 dispatch policy. Frame pixels and sampled queues are engine-owned.
use super::super::sprite_ops::{SpriteCommand, SpriteRequest};
use super::*;
use crysirror_scene::layer::queued::SequenceOperation;
impl Machine {
    pub(super) fn finish_framebuffer_actors(
        &mut self,
        host: &mut dyn NativeHost,
        objects: &[u16],
        position_only: bool,
    ) -> Result<bool> {
        let mut commands = Vec::new();
        for &object in objects {
            commands.push(SpriteCommand {
                seed: self.state.native_live.sprite_seed(object)?,
                operations: vec![SequenceOperation::Finish {
                    cancel: false,
                    lanes: position_only.then_some(vec![0]),
                }],
            });
        }
        if !host.sprite(&SpriteRequest { commands })? {
            return Ok(false);
        }
        self.sync_sprite_states(host)?;
        Ok(true)
    }
    pub(super) fn execute_framebuffer(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        let complete = || {
            Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            })
        };
        let kind = a[0];
        let history = self.state.dispatch_a2 != 0;
        let layout = self.state.dispatch_a3;
        let capture = self.state.native_flags.get(&25104).copied().unwrap_or(0) != 0;
        if capture
            || (layout
                && !matches!(
                    kind,
                    3 | 4
                        | 5
                        | 6
                        | 7
                        | 9
                        | 10
                        | 11
                        | 13
                        | 14
                        | 15
                        | 16
                        | 17
                        | 20
                        | 21
                        | 23
                        | 24
                        | 25
                        | 30
                        | 31
                        | 32
                        | 51
                        | 52
                ))
        {
            if kind == 2 {
                if !host.emit(&Event::FallingEffect {
                    kind: 0,
                    parameter: 0,
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.native_live.framebuffer_falling_clear()?;
                self.clear_effect_pools();
                for flag in [66336, 66337, 66339, 66340, 66341, 66342] {
                    self.state.native_flags.insert(flag, 0);
                }
            }
            self.state.native_live.framebuffer_dispatch_skipped();
            return Ok(complete());
        }
        let fast = self.state.value(99, 193)? == 0
            && (self.state.value(99, 195)? != 0
                || self.state.native_flags.get(&11484).copied().unwrap_or(0) != 0
                || self.state.native_flags.get(&11488).copied().unwrap_or(0) != 0);
        if (matches!(kind, 15 | 16 | 17 | 20) && (layout || fast))
            || (kind == 21
                && (history
                    || layout
                    || (fast && self.state.value(9, 99)? == 0 && self.state.value(99, 192)? == 0)))
        {
            self.state.native_live.framebuffer_dispatch_skipped();
            return Ok(complete());
        }
        match kind {
            7 | 10 | 11 | 13 | 14 | 23 | 24 | 25 | 30 | 31 | 32 => {
                self.state.native_live.ignored_framebuffer_effect(kind);
                return Ok(complete());
            }
            9 => {
                //4284B0 mode9 commits the pending scene132/resource1 image
                //before4226B0 captures it, even within the same VM batch.
                if !self.flush_framebuffer_background(host)? {
                    return Ok(Some(CommandResult::Retry));
                }
                let record = self.state.native_live.object_record(0)?;
                let value =
                    |at: usize| f32::from_le_bytes(record[at - 8..at - 4].try_into().unwrap());
                let crop = if value(104) == value(96) || value(108) == value(100) {
                    [0, 0, 1280, 720]
                } else {
                    [
                        value(96) as i32,
                        value(100) as i32,
                        value(104) as i32,
                        value(108) as i32,
                    ]
                };
                if !host.effect_control(&EffectRequest::CaptureFrame { crop })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.native_live.capture_background()?;
                self.state.native_live.framebuffer_color_effect(kind);
                return Ok(complete());
            }
            3 => {
                if self.state.value(99, 10)? != 0 {
                    if !host.effect_control(&EffectRequest::Flash {
                        repeats: a[1].max(0) as u16,
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.transition_wait = None;
                    self.state.effect_ops.framebuffer_active = true;
                    self.state.effect_ops.framebuffer_clocked = false;
                    self.state.effect_ops.fadeout = None;
                }
                self.state.transition_color = Some([255; 3]);
                self.state.native_flags.insert(15612, a[1].into());
                self.state.native_live.framebuffer_color_effect(kind);
                return Ok(complete());
            }
            4 | 6 | 51 | 52 => {
                if !history && !fast {
                    let objects = if matches!(kind, 4 | 6) {
                        (0..64).collect::<Vec<u16>>()
                    } else {
                        let max = if kind == 51 { 4 } else { 5 };
                        (0..max)
                            .filter(|n| i32::from(a[1]) & (1 << n) != 0)
                            .map(|n| 10 * (n + 1) as u16)
                            .collect()
                    };
                    if !self.finish_framebuffer_actors(host, &objects, true)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    let mut commands = Vec::new();
                    for object in objects {
                        let seed = self.state.native_live.sprite_seed(object)?;
                        let p = [seed.values[0], seed.values[1]];
                        let horizontal = matches!(kind, 4 | 51);
                        let delta = if horizontal { [8., 0.] } else { [0., 8.] };
                        let second = if kind == 51 { 1. } else { -1. };
                        let operations = [1., 0., second, 0.]
                            .into_iter()
                            .map(|sign| {
                                primary_move([p[0] + delta[0] * sign, p[1] + delta[1] * sign], 10)
                            })
                            .collect();
                        commands.push(SpriteCommand { seed, operations });
                    }
                    if !commands.is_empty() {
                        if !host.sprite(&SpriteRequest { commands })? {
                            return Ok(Some(CommandResult::Retry));
                        }
                        self.sync_sprite_states(host)?;
                    }
                }
                self.state.native_live.framebuffer_queue_effect(kind);
                return Ok(complete());
            }
            _ => {}
        }
        if !self.finish_framebuffer_actors(
            host,
            &[10, 20, 30, 40, 50, 12, 22, 32, 42, 52],
            false,
        )? {
            return Ok(Some(CommandResult::Retry));
        }
        self.state.native_live.framebuffer_selected_finish_completed();
        if kind == 2 && fast && (history || self.state.value(9, 99)? == 0) {
            let protected = self.state.native_flags.get(&25088).copied().unwrap_or(0) != 0;
            if !host.effect_control(&EffectRequest::ClearFrame { protected })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.commit_framebuffer_clear(protected)?;
            return Ok(complete());
        }
        match kind {
            5 => {
                self.state.native_live.framebuffer_queue_effect(kind);
            }
            0 | 1 | 2 => {
                if kind <= 1 && self.state.transition_color.is_none() {
                    self.state.native_live.framebuffer_without_color();
                    return Ok(complete());
                }
                if kind == 2 {
                    if !host.effect_control(&EffectRequest::PartialFallingClear)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.native_live.framebuffer_partial_clear();
                    self.clear_effect_pools();
                    for flag in [66337, 66339, 66340, 66342] {
                        self.state.native_flags.insert(flag, 0);
                    }
                    self.state.transition_color = Some([0; 3]);
                    self.state.effect_ops.frame_image_ticks = 4 * (5 * i32::from(a[1]) + 5);
                }
                if kind <= 1 && fast && (history || self.state.value(9, 99)? == 0) {
                    self.state.native_live.framebuffer_dispatch_skipped();
                    return Ok(complete());
                }
                if self.state.value(99, 10)? == 0 {
                    self.state.native_live.framebuffer_color_effect(kind);
                    return Ok(complete());
                }
                let out = kind == 2;
                let color = if out {
                    [0; 3]
                } else {
                    self.state
                        .transition_color
                        .context("Circus fade colour resource missing")?
                };
                let base = 4 * (5 * i32::from(a[1]) + 5);
                let speed = self.state.value(99, 12)?;
                let adjusted = if speed < 5 {
                    base + ((f64::from(base / 2) / 5.) * f64::from(5 - speed)) as i32
                } else if speed > 5 {
                    base - ((f64::from(base / 2) / 6.) * f64::from(speed - 5)) as i32
                } else {
                    base
                };
                let protected = self.state.native_flags.get(&25088).copied().unwrap_or(0) != 0;
                if !host.effect_control(&EffectRequest::FlatFade {
                    out,
                    ticks: adjusted,
                    color,
                    protected,
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.transition_color = Some(color);
                self.state.transition_wait = None;
                self.state.effect_ops.framebuffer_active = true;
                self.state.effect_ops.framebuffer_clocked = false;
                self.state.effect_ops.fadeout = out.then_some(protected);
                self.state.native_live.framebuffer_color_effect(kind);
                if out {
                    self.state.effect_ops.frame_image_ticks = base;
                }
            }
            17 | 20 | 21 => {
                if kind == 17 {
                    let name = if a[1] == 1 {
                        "MSK\\MASK11A"
                    } else {
                        "MSK\\MASK04A"
                    };
                    if !host.emit(&Event::LoadImage {
                        slot: 21,
                        name: name.into(),
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.image_names.insert(21, name.into());
                    self.state.native_live.load_image(21, name)?;
                }
                if let Some(color) = self
                    .state
                    .transition_color
                    .filter(|_| self.state.image_names.contains_key(&21))
                {
                    if (self.state.value(99, 10)? != 0 || self.state.value(99, 192)? != 0)
                        && !host.effect_control(&EffectRequest::MaskFade {
                            reverse: kind != 20,
                            ticks: if kind == 17 { 60 } else { 12 * i32::from(a[1]) },
                            color,
                        })?
                    {
                        return Ok(Some(CommandResult::Retry));
                    }
                    if self.state.value(99, 10)? != 0 || self.state.value(99, 192)? != 0 {
                        self.state.transition_wait = None;
                        self.state.effect_ops.framebuffer_active = true;
                        self.state.effect_ops.framebuffer_clocked = false;
                        self.state.effect_ops.fadeout = None;
                    }
                }
                if host
                    .text(&super::super::text_ops::TextRequest::ClearUi)?
                    .is_none()
                {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.assign(99, 246, 0)?;
                self.state.native_live.framebuffer_color_effect(kind);
            }
            15 | 16 => {
                if !host.emit(&Event::LoadImage {
                    slot: 45,
                    name: "system\\effbreak".into(),
                })? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.image_names.insert(45, "system\\effbreak".into());
                self.state.native_live.load_image(45, "system\\effbreak")?;
                if self.state.effect_ops.framebuffer_started != Some(i.offset)
                    && self.state.value(99, 10)? != 0
                    && !host.effect_control(&EffectRequest::TileMask {
                        pattern: a[1],
                        color: self.state.transition_color,
                    })?
                {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.effect_ops.framebuffer_started = Some(i.offset);
                if self.state.value(99, 10)? != 0 {
                    self.state.transition_wait = None;
                    self.state.effect_ops.framebuffer_active = true;
                    self.state.effect_ops.framebuffer_clocked = false;
                    self.state.effect_ops.fadeout = None;
                }
                if kind == 16 {
                    if host
                        .text(&super::super::text_ops::TextRequest::ClearUi)?
                        .is_none()
                    {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.assign(99, 246, 0)?;
                    if !host.emit(&Event::FallingEffect {
                        kind: 0,
                        parameter: 0,
                    })? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.native_live.framebuffer_falling_clear()?;
                    self.state.native_live.falling_effect(0, 0)?;
                    self.clear_effect_pools();
                }
                self.state.native_live.framebuffer_color_effect(kind);
            }
            _ => {} // Original default only finishes the selected actor queues.
        }
        self.state.effect_ops.framebuffer_started = None;
        Ok(complete())
    }
}
