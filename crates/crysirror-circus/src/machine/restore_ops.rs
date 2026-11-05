//! Original40C6F0/425F50/425640 and1d restore transaction policy.
use super::save_ops::{RestoreOrigin, RestorePlan, SaveReply, SaveRequest};
use super::*;
use crate::native_save::{CapturedNativeBody, NativeSaveBody, SlotKind};

impl Machine {
    /// Called only at source432AB0 menu/backlog entry after its finalization
    /// fences. This is independent from41DD10's autosave mirror.
    pub(crate) fn capture_native_rollback(&mut self) -> Result<()> {
        self.state.saves.rollback = Some(self.state.native_live.freeze_rollback()?);
        self.state.resources.capture_rollback();
        Ok(())
    }
    pub(super) fn execute_native_restore(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<CommandResult> {
        if self.state.saves.pending_restore.is_none() {
            let origin = if i.opcode == 0x1d {
                RestoreOrigin::Rollback {
                    preserve_effects: a[1] != 0,
                }
            } else {
                RestoreOrigin::Slot
            };
            let body = if i.opcode == 0x1d {
                self.state
                    .saves
                    .rollback
                    .as_ref()
                    .context("Circus1d requires independent432AB0 menu/backlog rollback mirror")?
                    .admitted_body()?
                    .clone()
            } else {
                let slot = self.state.value(99, 254)?;
                let Some(reply) = host.save(&SaveRequest::Read {
                    slot,
                    kind: SlotKind::Normal,
                })?
                else {
                    return Ok(CommandResult::Retry);
                };
                let SaveReply::Read(bytes) = reply else {
                    bail!("Circus native slot read receipt mismatch")
                };
                let Some(bytes) = bytes else {
                    self.state.native_live.restoration_receipt(0x18);
                    return Ok(CommandResult::Complete {
                        next: i.next,
                        outcome: Outcome::Advanced,
                    });
                };
                NativeSaveBody::from_read_prefix(&bytes)?
            };
            let mut plan = RestorePlan::decode(body, origin)?;
            self.prepare_restore_audio(&mut plan)?;
            if matches!(
                origin,
                RestoreOrigin::Rollback {
                    preserve_effects: false
                }
            ) && self.state.system_data.dword(9) != 0
            {
                let speed = self.state.system_data.dword(11);
                let base = 20;
                plan.restore_fade_ticks = Some(if speed < 5 {
                    base + ((f64::from(base / 2) / 5.) * f64::from(5 - speed)) as i32
                } else if speed > 5 {
                    base - ((f64::from(base / 2) / 6.) * f64::from(speed - 5)) as i32
                } else {
                    base
                });
            }
            self.state.saves.pending_restore = Some(plan);
        }
        let plan = self.state.saves.pending_restore.as_ref().unwrap().clone();
        let Some(program) = host.scene(&plan.scene)
            .with_context(||format!("Circus425640 scene dependency path=Advdata/MES/{}.mes",plan.scene))? else {
            return Ok(CommandResult::Retry);
        };
        let history_load = plan.history_replay_allowed && plan.origin == RestoreOrigin::Slot && plan.body.native_word(66511 * 4)? != 0;
        program
            .instruction(if history_load { 3 } else { plan.resume_pc })
            .context("Circus original slot resume must be an actual native instruction boundary")?;
        let Some(scene_index) = host.scene_index(&plan.scene)? else {
            return Ok(CommandResult::Retry);
        };
        if history_load {
            // Native case24 enters406DB0 rather than425640. Clear retained
            //drawables through425DC0, then let the same VM rebuild the scene
            //with dispatch a2=1 and the original recorded random stream.
            let commands = (0..150).map(|object| -> Result<sprite_ops::SpriteCommand> {
                Ok(sprite_ops::SpriteCommand {
                    seed: self.state.native_live.sprite_seed(object)?,
                    operations: vec![crysirror_scene::layer::queued::SequenceOperation::Finish {
                        cancel: false, lanes: None,
                    }],
                })
            }).collect::<Result<Vec<_>>>()?;
            if !host.sprite(&sprite_ops::SpriteRequest { commands })? {
                return Ok(CommandResult::Retry);
            }
            self.sync_sprite_states(host)?;
            if host.text(&text_ops::TextRequest::Movie(movie_ops::MovieRequest::Prepare {
                protected: false,
            }))?.is_none() { return Ok(CommandResult::Retry); }
            for kind in 0..3 {
                if !host.emit(&Event::ReleaseAudioGroup { kind, forced: true, steps: 0 })? {
                    return Ok(CommandResult::Retry);
                }
            }
            if !self.reset_history_drawables(host)? {return Ok(CommandResult::Retry)}
            let mut candidate = self.apply_native_restore(&plan, scene_index)?;
            let mut live_body = plan.body.clone();
            let (target, limit) = live_body.prepare_history_load(scene_index)?;
            candidate.native_live = crate::native_live::NativeLiveState::from_original(&live_body)?;
            candidate.native_live.preserve_ephemeral_from(&self.state.native_live);
            candidate.native_live.restore_renderer_scope(&self.state.native_live, &BTreeSet::new(), &BTreeSet::new(), false)?;
            candidate.native_live.movie_screen_clear(false, true)?;
            for object in 10..64 {candidate.native_live.reset_object(object)?;}
            candidate.native_live.history_scene_entry(&plan.scene)?;
            candidate.native_flags.insert(66511, 0);
            candidate.assign(99, 195, 0)?;
            candidate.native_flags.insert(11484, 0);
            candidate.native_flags.insert(27962, 0);
            candidate.native_flags.insert(66508, 0);
            candidate.native_flags.insert(66510, 0);
            candidate.native_flags.insert(272090, 1);
            candidate.native_flags.insert(15616, 1);
            candidate.assign(99, 245, -1)?;
            candidate.assign(99, 13, 0)?;
            candidate.dialogue.auto_advance = false;
            candidate.last_music=None;
            candidate.dialogue.voice_source.clear();
            candidate.dialogue.speaker.clear();
            candidate.dialogue.body.clear();
            candidate.image_names.retain(|slot, _| (37..=44).contains(slot));
            candidate.read_run = None;
            candidate.dispatch_a2 = 1;
            candidate.dispatch_a3 = false;
            candidate.history.set_replay_target(target);
            candidate.history_replay = Some(HistoryReplay { target, phase: 0, limit });
            candidate.saves.capture = Some(CapturedNativeBody::capture(plan.body.clone(), BTreeSet::new()));
            candidate.native_live.restoration_receipt(i.opcode);
            self.state = candidate;
            self.program = Some(program);
            return Ok(CommandResult::Complete { next: 3, outcome: Outcome::Advanced });
        }
        if self.state.saves.pending_restore_state.is_none() {
            self.state.saves.pending_restore_state =
                Some(Box::new(self.apply_native_restore(&plan, scene_index)?));
        }
        if self.state.saves.pending_restore_audio.is_none() {
            let Some(reply) = host.save(&SaveRequest::Restore { plan: plan.clone() })
                .with_context(||format!("Circus425640 resource restore scene={} resume={:#x}",plan.scene,plan.resume_pc))? else {
                return Ok(CommandResult::Retry);
            };
            self.state.saves.pending_restore_audio = Some(match reply {
                SaveReply::Restored => Vec::new(),
                SaveReply::RestoredAudio(audio) => audio,
                SaveReply::RestoredMedia { audio, music } => {
                    self.state.saves.pending_restore_music = music;
                    audio
                },
                _ => bail!("Circus native resource restore receipt mismatch"),
            });
        }
        let audio = self.state.saves.pending_restore_audio.as_ref().unwrap().clone();
        let mut candidate = *self
            .state
            .saves
            .pending_restore_state
            .take()
            .context("Circus native restore lost candidate owner")?;
        if !self.state.saves.pending_restore_projected {
        let restored = plan
            .images
            .iter()
            .filter_map(|image| image.native_sprite.map(|object| object as u16))
            .collect::<BTreeSet<_>>();
        let reloaded = plan
            .images
            .iter()
            .filter(|image| !image.native_ui)
            .map(|image| image.slot)
            .collect::<BTreeSet<_>>();
        candidate.native_live.restore_renderer_scope(
            &self.state.native_live,
            &restored,
            &reloaded,
            matches!(
                plan.origin,
                RestoreOrigin::Rollback {
                    preserve_effects: true
                }
            ),
        )?;
        //425F50→425DC0 calls40EF40(0) on all150 renderers;447D20
        //copies mapped records again, then425640 explicitly unpauses them.
        candidate.native_live.menu_pause_all(false);
        for image in plan
            .images
            .iter()
            .filter(|image| !image.packed && !image.native_ui)
        {
            let dimensions=host.image_dimensions(image.slot)?;
            if dimensions.is_none()&&image.preloaded_archive_bitmap_reload() {
                //422CF0 keeps its uppercase filename even when the bitmap
                //reload of this archive declaration returns native null.
                candidate.native_live.unload_image(image.slot)?;
                candidate.native_live.image_name(image.slot,&image.name)?;
                candidate.image_names.insert(image.slot,image.name.to_ascii_uppercase());
                continue
            }
            let (allocation, logical)=dimensions
                .with_context(||format!("Circus restored image dimension receipt missing slot={} name={}",image.slot,image.name))?;
            candidate.native_live.image_name(image.slot, &image.name)?;
            candidate
                .native_live
                .attach_image(image.slot, allocation, logical)?;
        }
        for image in plan
            .images
            .iter()
            .filter(|image| image.packed && image.native_sprite.is_some())
        {
            let object = image.native_sprite.unwrap() as u16;
            let sprite = plan.body.sprite(object as usize)?;
            let raw = sprite.bytes();
            let looped = u32::from_le_bytes(raw[948..952].try_into()?) != 0;
            if looped {
                let (allocation, logical) = host
                    .packed_dimensions(object)?
                    .context("Circus restored packed frame dimension receipt")?;
                let frames = host
                    .packed_frame_count(33)?
                    .context("Circus restored packed frame count receipt")?;
                let hold = i16::try_from(i32::from_le_bytes(raw[916..920].try_into()?))?;
                candidate
                    .native_live
                    .packed_frame_attached(object, allocation, logical)?;
                candidate
                    .native_live
                    .packed_declared(object, 33, frames, hold, true,host.packed_lineage(object)?.context("Circus restored pack lineage receipt")?)?;
            }
        }
        if plan.falling[4] != 0 {
            let dimensions = host.image_dimensions(27)?;
            for (object, crop) in [(64, [0, 0, 3072, 576]), (65, [0, 600, 3072, 1176])] {
                candidate.native_live.reset_object(object)?;
                candidate.native_live.bind_resource(27, object, Some(crop), dimensions)?;
            }
        }
        let sprite_states = host.sprite_states()?;
        for (object, state) in &sprite_states {
            if *object < 150 {
                candidate
                    .native_live
                    .sprite_sequence_status(*object, state)?;
            }
        }
        let mut restored_machine = Machine {
            state: candidate,
            program: None,
        };
        restored_machine.sync_effect_frame(&host.effect_states()?)?;
        restored_machine.sync_comment_record(host)?;
        candidate = restored_machine.state;
        candidate.resources.restore_dispatch(&sprite_states);
        self.state.saves.pending_restore_projected = true;
        }
        let mut restored_machine = Machine { state: candidate, program: None };
        //425640 copies the current base renderer into each extra face.
        //444770 is a plain960-byte copy, with no448CE0 sampling. Retain the
        //completed resource receipt across asynchronous copy/binding retries.
        while self.state.saves.restore_extra_cursor < 6 {
            let cursor = self.state.saves.restore_extra_cursor;
            let (slot, source, target) = if cursor < 5 {
                (9 + cursor as u8, 10 * (cursor as u16 + 1), 12 + 10 * cursor as u16)
            } else { (32, 75, 76) };
            if !plan.body.image_name_bytes(usize::from(slot))?.is_empty() {
                if !restored_machine.resource_copy_object(host, source, target, slot, false, true)? {
                    self.state.saves.pending_restore_state = Some(Box::new(restored_machine.state));
                    return Ok(CommandResult::Retry);
                }
            }
            self.state.saves.restore_extra_cursor += 1;
        }
        if restored_machine.state.native_flags.get(&23876).copied().unwrap_or(0) != 0 {
            if !restored_machine.restore_dialogue_controls(host)? {
                self.state.saves.pending_restore_state = Some(Box::new(restored_machine.state));
                return Ok(CommandResult::Retry);
            }
            restored_machine.state.native_flags.insert(23876, 0);
        }
        let mut candidate = restored_machine.state;
        candidate.resources.restore_dispatch(&host.sprite_states()?);
        candidate.native_live.restored_queues_acknowledged();
        for (channel, name, looping) in audio {
            candidate
                .native_live
                .sound_loaded(channel, &name, looping)?;
        }
        if plan.origin == RestoreOrigin::Slot {
            candidate.last_music = self.state.saves.pending_restore_music.clone();
            candidate.saves.capture = Some(CapturedNativeBody::capture(
                plan.body.clone(),
                BTreeSet::new(),
            ));
        }
        candidate.native_live.restoration_receipt(i.opcode);
        //Case29's v516 prevents recapturing41DD10 when the restored6B is
        //decoded in this same dispatcher; its preceding mirror stays valid.
        candidate.saves.dialogue_checkpoint=None;
        candidate.saves.dialogue_checkpoint_suppressed=i.opcode==0x1d;
        self.state = candidate;
        self.program = Some(program);
        Ok(CommandResult::Complete {
            next: plan.resume_pc,
            outcome: Outcome::Advanced,
        })
    }
    pub(super) fn apply_native_restore(&self, plan: &RestorePlan, scene_index: i32) -> Result<State> {
        let body = &plan.body;
        let mut state = self.state.clone();
        state.saves.dialogue_checkpoint=None;
        state.saves.dialogue_checkpoint_suppressed=false;
        state.macros = macro_ops::MacroState::from_native_body(body, &self.state.macros)?;
        state.saves.pending_restore = None;
        state.saves.pending_restore_state = None;
        state.saves.pending_restore_audio = None;
        state.saves.pending_restore_music = None;
        state.saves.pending_restore_projected = false;
        state.saves.restore_extra_cursor = 0;
        if matches!(
            plan.origin,
            RestoreOrigin::Rollback {
                preserve_effects: false
            }
        ) {
            state.assign(99, 195, 0)?;
            state.native_flags.insert(11484, 0);
            state.assign(99, 13, 0)?;
            state.dialogue.auto_advance = false;
        }
        state.scene = plan.scene.clone();
        state.pc = plan.resume_pc;
        state.native_live = crate::native_live::NativeLiveState::from_original(body)?;
        state
            .native_live
            .preserve_ephemeral_from(&self.state.native_live);
        state.bytes.clear();
        for bank in 0..11u8 {
            for index in 0..100u8 {
                state.bytes.insert(
                    u16::from(bank) * 100 + u16::from(index),
                    body.byte_bank(bank, index)?,
                );
            }
        }
        for (index, at) in [(233, 115880), (241, 118888)] {
            state.system.insert(index, body.native_word(at)? as i32);
        }
        for index in [66508, 66509, 66510, 66511, 66512, 66513, 66515, 66517, 66584, 66329, 28970, 29722] {
            state
                .native_flags
                .insert(index, body.native_word(index as usize * 4)? as i32);
        }
        state.image_names.clear();
        for slot in 0..51u8 {
            let name = body.native_string(19496 + usize::from(slot) * 48, 48)?;
            if !name.is_empty() {
                state.image_names.insert(slot, name);
            }
        }
        state.pending_images.clear();
        state.builder = Group::default();
        state.current_button = None;
        //423070→41F2B0 clears four active groups at10348..10360,
        //not the persistent templates at10368+4*slot (including9/10/19).
        //Actual retained UI presentation is cleared by the resource bridge.

        state.dialogue.speaker = plan.prefixes[0].clone();
        state.dialogue.body = plan.prefixes[1].clone();
        //These names are live native buffers too. Retaining the pre-load
        //logical names would apply voice-prefix color/volume to another line.
        state.dialogue.voice_source = body.native_string(265392, 128)?;
        state.dialogue.voice_name = body.native_string(266332, crate::native_save::LIVE_BASE + crate::native_save::BODY_BYTES - 266332)?;
        state.dialogue.refresh_text_owner = None;
        state.dialogue.native_auto_started = None;
        state.dialogue.native_forward_pending=false;
        state.dialogue.pending_read_owner = None;
        state.dialogue.pending_inline_events.clear();
        state.dialogue.speaker_events.clear();
        state.menu_wait = false;
        state.wait_until = None;
        state.wait_click = false;
        state.wait_skippable = false;
        state.halted = false;
        state.transition_wait = None;
        state.transition_poll_at = None;
        state.wait_animations = false;
        state.wait_audio_mask = 0;
        state.savepoint = None;
        state.builder_offset = [0, 0];
        state.transition_color = None;
        state.read.clear();
        let read_index = body.read_run_index() as usize;
        let run = body.read_run(read_index)?;
        state.read_run = if (plan.history_replay_allowed && plan.origin == RestoreOrigin::Slot && body.native_word(66511 * 4)? != 0)
            || run.scene_index == u16::MAX || run.first_label == 0 {
            None
        } else {
            ensure!(
                i32::from(run.scene_index as i16) == scene_index,
                "Circus native current read-run scene mismatch"
            );
            Some((plan.scene.clone(), run.first_label, run.last_label))
        };
        state.version = body.native_string(265848, 128)?;
        state.protagonist_names = [
            body.native_string(265288, 20)?,
            body.native_string(265268, 20)?,
        ];
        let random_count = body.native_word(111864)? as usize;
        ensure!(
            random_count <= 200,
            "Circus native restored random history extent"
        );
        state.random_history = (0..random_count)
            .map(|n| body.native_word(115080 + n * 4).map(|v| v as i32))
            .collect::<Result<_>>()?;
        state.placement_tables.clear();
        for kind in 0..20usize {
            let count = body.native_word(106156 + kind * 4)? as usize;
            ensure!(count <= 100, "Circus native restored placement extent");
            let mut rows = Vec::new();
            for n in 0..count {
                let at = 26152 + kind * 4000 + n * 40;
                rows.push((
                    body.native_string(at, 32)?,
                    [
                        i16::try_from(body.native_word(at + 32)? as i32)?,
                        i16::try_from(body.native_word(at + 36)? as i32)?,
                    ],
                ));
            }
            if !rows.is_empty() {
                state.placement_tables.insert(kind as i16, rows);
            }
        }
        state.placement_declaration = None;
        let voice_count = body.native_word(269844)? as usize;
        ensure!(
            voice_count <= 50,
            "Circus native restored voice timing extent"
        );
        state.voice_timing = (0..voice_count)
            .map(|n| {
                Ok((
                    body.native_string(268012 + n * 36, 32)?,
                    body.native_word(268008 + n * 36)? as i32,
                ))
            })
            .collect::<Result<_>>()?;
        state.voice_timing_from_sprite = body.native_word(269848)? != 0;
        state.current_music = i16::try_from(body.native_word(21976)? as i32)?;
        state
            .media
            .restore_volume_backup(body.native_word(66514 * 4)? as i32);
        if plan.origin == RestoreOrigin::Slot {
            state.pending_audio.clear();
            state.pending_music = None;
            state.media.clear_playlist_for_restore();
            state.wait_text_idle = None;
            state.wait_process = 0;
            state.last_music = None;
        } else {
            state.callbacks.clear();
        }
        if let Some(backend @ (1 | 2)) = state.video_backend {
            let mut rows = Vec::new();
            let arena=if backend==1 {&plan.movie[..1640]} else {&body.bytes()[266340-crate::native_save::LIVE_BASE..266340-crate::native_save::LIVE_BASE+16400]};
            for raw in arena.chunks_exact(82) {
                if raw[0] == 0 { break; }
                let end = raw.iter().position(|b| *b == 0)
                    .context("Circus restored comment playlist terminator")?;
                let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..end]);
                ensure!(!error, "Circus restored comment playlist encoding");
                rows.push(text.into_owned());
            }
            state.media.restore_playlist(backend, rows);
        }
        Ok(state)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Host {
        program: Arc<Program>,
        body: Option<Vec<u8>>,
        reads: usize,
        pending: bool,
        fail: bool,
    }
    impl NativeHost for Host {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            Ok(Some(self.program.clone()))
        }
        fn scene_index(&mut self, _: &str) -> Result<Option<i32>> {
            Ok(Some(0))
        }
        fn emit(&mut self, _: &Event) -> Result<bool> {
            Ok(true)
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            Ok(self.body.is_some())
        }
        fn save(&mut self, request: &SaveRequest) -> Result<Option<SaveReply>> {
            match request {
                SaveRequest::Read { .. } => {
                    self.reads += 1;
                    Ok(Some(SaveReply::Read(self.body.clone())))
                }
                SaveRequest::Restore { .. } => {
                    if self.fail {
                        bail!("injected shared restore failure")
                    }
                    if self.pending {
                        self.pending = false;
                        return Ok(None);
                    }
                    Ok(Some(SaveReply::Restored))
                }
                _ => bail!("unexpected save request"),
            }
        }
    }
    fn fixture(scene: &str, value: u8) -> NativeSaveBody {
        let mut live = crate::native_live::NativeLiveState::fresh();
        live.byte_assignment(4, 17, value);
        live.audit_completed(0, scene, 13).unwrap();
        live.freeze_checkpoint(13)
            .unwrap()
            .admitted_body()
            .unwrap()
            .clone()
    }
    fn host(body: Option<Vec<u8>>) -> Host {
        Host {
            program: Arc::new(Program {
                profile: Profile::Dc4PlusHarmony,
                code_file_offset: 0,
                labels: vec![],
                label_metadata: vec![],
                raw_strings: Default::default(),
                instructions: vec![Instruction {
                    offset: 13,
                    next: 16,
                    opcode: 0,
                    arguments: Arguments::Bytes([0, 0]),
                    continuation: None,
                }],
            }),
            body,
            reads: 0,
            pending: true,
            fail: false,
        }
    }
    fn instruction(opcode: u8) -> Instruction {
        Instruction {
            offset: 3,
            next: 8,
            opcode,
            arguments: Arguments::Bytes([0, 0]),
            continuation: None,
        }
    }
    #[test]
    fn original_load_commits_pc_and_banks_only_after_resource_restore_receipt() {
        let body = fixture("loaded", 73);
        let mut host = host(Some(body.bytes().to_vec()));
        let mut machine = Machine::new("before");
        machine.state.assign(4, 17, 9).unwrap();
        let i = instruction(0x18);
        assert!(matches!(
            machine
                .execute_native_restore(&i, [0; 8], &mut host)
                .unwrap(),
            CommandResult::Retry
        ));
        assert_eq!(machine.state.scene, "before");
        assert_eq!(machine.state.pc, 3);
        assert_eq!(machine.state.value(4, 17).unwrap(), 9);
        assert!(matches!(
            machine
                .execute_native_restore(&i, [0; 8], &mut host)
                .unwrap(),
            CommandResult::Complete { next: 13, .. }
        ));
        assert_eq!(host.reads, 1);
        assert_eq!(machine.state.scene, "loaded");
        assert_eq!(machine.state.value(4, 17).unwrap(), 73);
        assert!(
            machine
                .state
                .saves
                .capture
                .as_ref()
                .unwrap()
                .admitted_body()
                .is_ok()
        );
    }
    #[test]
    fn failed_shared_restore_keeps_current_vm_and_missing_slot_advances() {
        let mut host = host(Some(fixture("loaded", 73).bytes().to_vec()));
        host.fail = true;
        let mut machine = Machine::new("before");
        assert!(
            machine
                .execute_native_restore(&instruction(0x18), [0; 8], &mut host)
                .is_err()
        );
        assert_eq!(machine.state.scene, "before");
        assert_eq!(machine.state.pc, 3);
        let mut missing = super::tests::host(None);
        let mut machine = Machine::new("before");
        assert!(matches!(
            machine
                .execute_native_restore(&instruction(0x18), [0; 8], &mut missing)
                .unwrap(),
            CommandResult::Complete { next: 8, .. }
        ));
    }
    #[test]
    fn rollback_uses_731976_mirror_and_never_reads_an_original_slot() {
        let mut machine = Machine::new("before");
        machine.state.saves.capture = Some(CapturedNativeBody::capture(
            fixture("autosave", 11),
            BTreeSet::new(),
        ));
        machine.state.saves.rollback = Some(CapturedNativeBody::capture(
            fixture("rollback", 29),
            BTreeSet::new(),
        ));
        let mut host = host(None);
        host.pending = false;
        let mut a = [0; 8];
        a[1] = 1;
        machine
            .execute_native_restore(&instruction(0x1d), a, &mut host)
            .unwrap();
        assert_eq!(host.reads, 0);
        assert_eq!(machine.state.scene, "rollback");
        assert_eq!(machine.state.value(4, 17).unwrap(), 29);
        assert_eq!(
            machine
                .state
                .saves
                .capture
                .as_ref()
                .unwrap()
                .admitted_body()
                .unwrap()
                .scene_name()
                .unwrap(),
            "autosave"
        );
    }
}
