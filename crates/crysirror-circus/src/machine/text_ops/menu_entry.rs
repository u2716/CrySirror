//! Native432B90/432AB0/422B70 menu entry: the rollback mirror and autosave
//! mirror are distinct snapshots, on opposite sides of the pause declaration.
use super::*;
use crysirror_scene::layer::queued::SequenceOperation;
#[derive(Clone, Debug)]
pub struct PendingMenuEntry {
    offset: u32,
    name: String,
    program: Arc<Program>,
    resources: Option<Vec<u8>>,
    mirror: Option<crate::native_save::CapturedNativeBody>,
}
impl Machine {
    ///432F90/433680 call the same432B90 owner outside bytecode execution.
    ///Only its real scene/PC transition is committed; no invented3E audit or
    ///instruction count is added to the source program.
    pub(in crate::machine) fn native_menu_entry(&mut self,host:&mut dyn NativeHost,name:&str,now:u64)->Result<bool> {
        let program=self.program.clone().context("Circus native menu caller scene")?;
        let instruction=Instruction{offset:self.state.pc,next:self.state.pc,opcode:0x3e,arguments:Arguments::String(name.into()),continuation:None};
        match self.execute_menu_entry(&program,&instruction,name,host,now)? {
            CommandResult::Retry=>Ok(false),
            CommandResult::Complete{next,..}=>{self.state.pc=next;Ok(true)},
        }
    }
    pub(super) fn execute_menu_entry(
        &mut self,
        program: &Program,
        i: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<CommandResult> {
        if self.state.dialogue.menu_entry_pending.is_none() {
            let (name, target) = match host.scene(text) {
                Ok(Some(target)) => (text.to_owned(), target),
                Ok(None) => return Ok(CommandResult::Retry),
                Err(_) => (
                    self.state.scene.clone(),
                    self.program.clone().context("Circus menu scene fallback")?,
                ),
            };
            //432AB0 consumes every pending inline instruction before freezing.
            if !self.poll_text_events(host, u64::MAX)? {
                return Ok(CommandResult::Retry);
            }
            let commands = (0..150)
                .map(
                    |object| -> Result<super::super::sprite_ops::SpriteCommand> {
                        Ok(super::super::sprite_ops::SpriteCommand {
                            seed: self.state.native_live.sprite_seed(object)?,
                            operations: vec![SequenceOperation::Finish {
                                cancel: false,
                                lanes: None,
                            }],
                        })
                    },
                )
                .collect::<Result<Vec<_>>>()?;
            if !host.sprite(&super::super::sprite_ops::SpriteRequest { commands })? {
                return Ok(CommandResult::Retry);
            }
            self.sync_sprite_states(host)?;
            if !host.effect_control(&super::super::effect_ops::EffectRequest::Finish)? {
                return Ok(CommandResult::Retry);
            }
            let had44 = host.image_dimensions(44)?.is_some();
            if host.text(&TextRequest::MenuEntryFinish)?.is_none() {
                return Ok(CommandResult::Retry);
            }
            self.state.native_live.unload_image(44)?;
            if had44 {
                self.state.image_names.remove(&44);
            }
            self.state.native_flags.insert(272088, 0);
            self.state.native_flags.insert(272077, 0);
            self.state
                .native_live
                .advance_pc(&self.state.scene, i.next)?;
            self.state.native_live.menu_pause_all(false);
            self.capture_native_rollback()?;
            self.state.native_live.menu_pause_all(true);
            self.state.dialogue.menu_entry_pending = Some(PendingMenuEntry {
                offset: i.offset,
                name,
                program: target,
                resources: None,
                mirror: None,
            });
        }
        ensure!(
            self.state
                .dialogue
                .menu_entry_pending
                .as_ref()
                .unwrap()
                .offset
                == i.offset,
            "Circus menu-entry continuation identity"
        );
        if self
            .state
            .dialogue
            .menu_entry_pending
            .as_ref()
            .unwrap()
            .resources
            .is_none()
        {
            let Some(resources) = host.capture_checkpoint()? else {
                return Ok(CommandResult::Retry);
            };
            let mirror = self.state.native_live.freeze_menu_checkpoint()?;
            let pending = self.state.dialogue.menu_entry_pending.as_mut().unwrap();
            pending.resources = Some(resources);
            pending.mirror = Some(mirror);
        }
        if !host.emit(&Event::ReleaseAudioGroup {
            kind: 1,
            forced: false,
            steps: 30,
        })? {
            return Ok(CommandResult::Retry);
        }
        if host.text(&TextRequest::ClearDialogue)?.is_none() {
            return Ok(CommandResult::Retry);
        }
        let pending = self.state.dialogue.menu_entry_pending.take().unwrap();
        let mut captured = self.state.clone();
        captured.checkpoint = None;
        captured.saves = captured.saves.without_mirrors();
        captured.pc = i.next;
        self.state.saves.captured(pending.mirror.unwrap())?;
        let continuation = self
            .state
            .checkpoint
            .as_ref()
            .map_or_else(String::new, |c| c.continuation.clone());
        self.state.checkpoint = Some(NativeCheckpoint {
            resume_pc: i.next,
            continuation,
            vm_state: serde_json::to_vec(&captured)?,
            shared_resources: pending.resources.unwrap(),
        });
        let ui = [
            self.state.groups.contains_key(&0),
            self.state.groups.contains_key(&1),
        ];
        self.state.native_live.menu_entry(&pending.name, ui)?;
        self.state.assign(99, 245, -1)?;
        self.state.native_flags.insert(66512, 0);
        self.state.native_flags.insert(66510, 0);
        self.state.dialogue.body.clear();
        self.state.dialogue.speaker.clear();
        self.state.dialogue.voice_source.clear();
        self.state.dialogue.pending_inline_events.clear();
        self.state.wait_click = false;
        self.state.menu_wait = false;
        self.state.scene = pending.name;
        self.program = Some(pending.program);
        let _ = (program, now);
        Ok(CommandResult::Complete {
            next: 3,
            outcome: Outcome::Advanced,
        })
    }
}
