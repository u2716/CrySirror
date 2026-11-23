//!433AA0's pre-process input policy. Native key stock remains a shared-owner
//!observation; action/menu/history implementations are the existing VM exits.
use super::*;
use crysirror_core::service::buttons as b;
#[derive(Clone, Debug, Default)]
pub(in crate::machine) struct PrefixState {
    frame: Option<u64>,
    sample: Option<PrefixInput>,
    phase: u8,
    done: bool,
    blocked: bool,
    gesture_epoch: u64,
    right_mode: i32,
    objects: Option<Vec<u16>>,
}
impl Machine {
    pub(crate) fn finish_native_prefix_frame(&mut self) {
        //A live confirmation/menu/resource phase retains its owner across
        //rendered modal frames. Only a completed input pass can be retired.
        if self.state.system_ops.input_prefix.done {self.state.system_ops.input_prefix=Default::default();}
        self.state.system_ops.finish_callback_frame();
    }
    pub(crate) fn native_prefix_pending(&self) -> bool {
        !self.state.system_ops.input_prefix.done && self.state.system_ops.input_prefix.phase != 0
    }
    fn prefix_read_run(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        if let Some((scene, start, end)) = &self.state.read_run {
            let Some(index) = host.scene_index(scene)? else {
                return Ok(false);
            };
            if index >= 0 && *start != 0 {
                self.state
                    .system_data
                    .mark_read(index as usize, *start, *end)?;
            }
        }
        Ok(true)
    }
    fn prefix_sample(&mut self, host: &mut dyn NativeHost) -> Result<Option<PrefixInput>> {
        match self.gesture_request(host, GestureRequest::PrefixInput)? {
            Some(GestureReply::PrefixInput(input)) => Ok(Some(input)),
            None => Ok(None),
            _ => bail!("Circus433AA0 prefix input receipt"),
        }
    }
    fn prefix_consume(&mut self, host: &mut dyn NativeHost, mask: u64) -> Result<bool> {
        Ok(matches!(
            self.gesture_request(host, GestureRequest::ConsumeInput { mask })?,
            Some(GestureReply::Done)
        ))
    }
    fn prefix_clear_skip(&mut self) -> Result<()> {
        //421C90 clears only272077 and the two CWnd Ctrl/toggle latches.
        self.state.assign(99, 195, 0)?;
        self.state.native_flags.insert(11484, 0);
        self.state.native_flags.insert(11488, 0);
        Ok(())
    }
    fn prefix_finish(&mut self, blocked: bool) -> Result<bool> {
        self.state.system_ops.input_prefix.done = true;
        self.state.system_ops.input_prefix.blocked = blocked;
        Ok(!blocked)
    }
    pub(crate) fn poll_native_prefix(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<bool> {
        if self.state.system_ops.input_prefix.done {
            if self.state.system_ops.input_prefix.frame == Some(now) {
                return Ok(!self.state.system_ops.input_prefix.blocked);
            }
            self.state.system_ops.input_prefix = Default::default();
        }
        if self.state.system_ops.input_prefix.phase == 0 {
            let Some(input) = self.prefix_sample(host)? else {
                return Ok(false);
            };
            self.state.system_ops.input_prefix.frame = Some(now);
            if input.control_trigger
                && self.state.native_flags.get(&15632).copied().unwrap_or(0) != 0
            {
                self.state.native_flags.insert(11488, 1);
                self.state.native_flags.insert(11484, 1);
            }
            if !input.control_stock
                && self.state.native_flags.get(&11488).copied().unwrap_or(0) != 0
            {
                self.prefix_clear_skip()?;
            }
            if !input.enabled {
                self.state.native_flags.insert(11488, 0);
                self.state.native_flags.insert(11484, 0);
            }
            if self.state.native_flags.get(&23880).copied().unwrap_or(0) != 0
                && (self.state.wait_process != 6 || self.state.history.active())
            {
                self.state.native_flags.insert(23880, 0);
                if !self.prefix_consume(host, u64::MAX)? {
                    return Ok(false);
                }
            }
            self.state.system_ops.input_prefix.sample = Some(input);
            self.state.system_ops.input_prefix.phase = 1;
            if self.state.wait_process != 6 || self.state.history.active() {
                return self.prefix_finish(false);
            }
        }
        if self.state.system_ops.input_prefix.phase == 1 {
            let input = self.state.system_ops.input_prefix.sample.clone().unwrap();
            if input.released & b::SPACE != 0 {
                if !self.prefix_read_run(host)? {
                    return Ok(false);
                }
                let hide =
                    i32::from(self.state.native_flags.get(&11480).copied().unwrap_or(0) == 0);
                self.state.native_flags.insert(11480, hide);
                self.state.assign(99, 13, 0)?;
                self.prefix_clear_skip()?;
            }
            if self.state.native_flags.get(&11480).copied().unwrap_or(0) != 0
                && input.released & (b::ENTER | b::MOUSE_LEFT) != 0
            {
                self.state.native_flags.insert(11480, 0);
                if !self.prefix_consume(host, b::ENTER | b::MOUSE_LEFT)? {
                    return Ok(false);
                }
            }
            self.state.system_ops.input_prefix.gesture_epoch =
                self.state.system_ops.gestures.completed;
            self.state.system_ops.input_prefix.phase = 2;
        }
        if self.state.system_ops.input_prefix.phase == 2 {
            if !self.poll_native_gestures(host, now)? {
                //A held gesture is an ordinary rendered frame. An async
                //action/confirmation retains its owner instead of resampling.
                if !self.native_action_pending() {
                    self.state.system_ops.input_prefix.done = true;
                    self.state.system_ops.input_prefix.blocked = true;
                }
                return Ok(false);
            }
            if self.state.system_ops.gestures.completed
                != self.state.system_ops.input_prefix.gesture_epoch
            {
                return self.prefix_finish(false);
            }
            self.state.system_ops.input_prefix.phase = 3;
        }
        if self.state.system_ops.input_prefix.phase == 3 {
            let Some(input) = self.prefix_sample(host)? else {
                return Ok(false);
            };
            self.state.system_ops.input_prefix.sample = Some(input.clone());
            if input.released & (b::MOUSE_RIGHT | b::ESCAPE) != 0 && !self.state.history.active() {
                if self.state.native_flags.get(&11480).copied().unwrap_or(0) != 0
                    || self.state.native_skipping()
                    || self.state.value(99, 13)? != 0
                {
                    self.state.native_flags.insert(11480, 0);
                    self.state.assign(99, 13, 0)?;
                    self.prefix_clear_skip()?;
                    return self.prefix_finish(true);
                }
                let mode = if self.state.value(9, 7)? == 1 || input.released & b::ESCAPE != 0 {
                    0
                } else {
                    self.state.value(99, 211)?
                };
                if mode == 1 {
                    self.state.native_flags.insert(11480, 1);
                    return self.prefix_finish(true);
                }
                self.state.system_ops.input_prefix.right_mode = mode;
                self.state.system_ops.input_prefix.phase = 4;
            } else {
                self.state.system_ops.input_prefix.phase = 8;
            }
        }
        if self.state.system_ops.input_prefix.phase == 4 {
            if self.state.system_ops.input_prefix.objects.is_none() {
                let Some(GestureReply::Objects(objects)) =
                    self.gesture_request(host, GestureRequest::DialogueObjects)?
                else {
                    return Ok(false);
                };
                self.state.system_ops.input_prefix.objects = Some(objects);
            }
            let objects = self.state.system_ops.input_prefix.objects.clone().unwrap();
            let mut commands = Vec::new();
            for &object in &objects {
                commands.push(sprite_ops::SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![crysirror_scene::layer::queued::SequenceOperation::Finish {
                        cancel: false,
                        lanes: None,
                    }],
                });
            }
            if !host.sprite(&sprite_ops::SpriteRequest { commands })? {
                return Ok(false);
            }
            if !host.effect_control(&effect_ops::EffectRequest::FinishObjects { objects })? {
                return Ok(false);
            }
            self.sync_sprite_states(host)?;
            self.sync_effect_frame(&host.effect_states()?)?;
            self.state.system_ops.input_prefix.phase = 5;
        }
        if self.state.system_ops.input_prefix.phase == 5 {
            if self.state.system_ops.input_prefix.right_mode == 0 {
                if !self.prefix_read_run(host)? {
                    return Ok(false);
                }
                self.state.system_ops.input_prefix.phase = 6;
            } else {
                self.state.system_ops.input_prefix.phase = 8;
            }
        }
        if self.state.system_ops.input_prefix.phase == 6 {
            if host
                .publish_audio(&media_ops::AudioDeclaration {
                    name: "se\\201".into(),
                    voice: false,
                    looping: false,
                    voice_volume: None,
                })?
                .is_none()
            {
                return Ok(false);
            }
            self.state.system_ops.input_prefix.phase = 7;
        }
        if self.state.system_ops.input_prefix.phase == 7 {
            if !self.native_menu_entry(host, "advmenu", now)? {
                return Ok(false);
            }
            self.state.system_ops.input_prefix.phase = 8;
        }
        if self.state.system_ops.input_prefix.phase == 8 {
            if self.state.native_flags.get(&23872).copied().unwrap_or(0) != 0 {
                self.state.native_flags.insert(11480, 0);
            }
            if self.state.native_flags.get(&11480).copied().unwrap_or(0) != 0 {
                return self.prefix_finish(true);
            }
            let Some(input) = self.prefix_sample(host)? else {
                return Ok(false);
            };
            for (key, action) in [
                (b::F1, 10),
                (b::F2, 0),
                (b::F3, 2),
                (b::F4, 9),
                (b::F5, 7),
                (b::F6, 8),
                (b::F7, 4),
                (b::F8, 5),
                (b::F9, 6),
            ] {
                if input.released & key != 0 {
                    self.state.system_ops.input_prefix.right_mode = action;
                    self.state.system_ops.input_prefix.phase = 9;
                    break;
                }
            }
            if self.state.system_ops.input_prefix.phase == 8 {
                return self.prefix_finish(false);
            }
        }
        if self.state.system_ops.input_prefix.phase == 9 {
            let action = self.state.system_ops.input_prefix.right_mode;
            if !self.native_system_action(host, action, now)? {
                return Ok(false);
            }
            //432F90 calls442B00 before its switch, so later F-key release
            //tests in this same source pass see cleared transient stock.
            return self.prefix_finish(false);
        }
        unreachable!()
    }
}
