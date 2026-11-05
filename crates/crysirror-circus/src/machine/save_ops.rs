//! Native41DD10/40C360/40C400 ownership and persistence policy.
//! The captured mirror is immutable across asynchronous worker retries.
use super::*;
use crate::native_save::{CapturedNativeBody, CivilFields, NativeSaveBody, SlotKind};
pub use crate::native_save::{RestoreOrigin, RestorePlan};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveRequest {
    CivilTime,
    ///421750 reads the currentCWnd440, independently of41DD10's body.
    CurrentThumbnailSource,
    /// Frozen native4226B0(1) selection, never the current desktop frame.
    Thumbnail {
        checkpoint_resources: Vec<u8>,
    },
    Write {
        slot: i32,
        kind: SlotKind,
        body: Vec<u8>,
        thumbnail_png: Option<Vec<u8>>,
        thumbnail_expected: bool,
    },
    Read {
        slot: i32,
        kind: SlotKind,
    },
    Restore {
        plan: RestorePlan,
    },
}
#[derive(Clone, Debug)]
pub enum SaveReply {
    CivilTime(CivilFields),
    CurrentThumbnailSource(Vec<u8>),
    Thumbnail(Vec<u8>),
    Written,
    Read(Option<Vec<u8>>),
    Restored,
    RestoredAudio(Vec<(u32, String, bool)>),
    RestoredMedia { audio: Vec<(u32, String, bool)>, music: Option<MusicAsset> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PendingWrite {
    scene: String,
    pc: u32,
    slot: i32,
    kind: SlotKind,
    mirror: NativeSaveBody,
    resources: Vec<u8>,
    time: Option<CivilFields>,
    thumbnail: Option<Vec<u8>>,
    thumbnail_expected: bool,
    #[serde(default)]
    thumbnail_source_ready: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveState {
    pub capture: Option<CapturedNativeBody>,
    pending: Option<PendingWrite>,
    #[serde(skip)]
    pub(crate) pending_restore: Option<RestorePlan>,
    #[serde(skip)]
    pub(crate) pending_restore_state: Option<Box<State>>,
    #[serde(skip)]
    pub(crate) pending_restore_audio: Option<Vec<(u32, String, bool)>>,
    #[serde(skip)]
    pub(crate) pending_restore_music: Option<MusicAsset>,
    #[serde(skip)]
    pub(crate) pending_restore_projected: bool,
    #[serde(skip)]
    pub(crate) restore_extra_cursor: usize,
    pub rollback: Option<CapturedNativeBody>,
    #[serde(default)]
    pub(crate) dialogue_checkpoint:Option<u32>,
    #[serde(default)]
    pub(crate) dialogue_checkpoint_suppressed:bool,
    #[serde(skip)]
    quick:Option<QuickSlot>,
}
impl SaveState {
    /// A newly captured41DD10 mirror does not inherit an in-flight write.
    pub(crate) fn captured(&mut self, body: CapturedNativeBody) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "Circus checkpoint during native slot write"
        );
        self.capture = Some(body);
        Ok(())
    }
    /// Remove retained mirrors before serializing a logical checkpoint. Native
    /// capture buffers are independent memory, not recursive logical snapshots.
    pub(crate) fn without_mirrors(&self) -> Self {
        Self::default()
    }
}
impl Machine {
    pub(crate) fn capture_dialogue_checkpoint(&mut self,host:&mut dyn NativeHost)->Result<()> {
        let Some(resume)=self.state.saves.dialogue_checkpoint else{return Ok(())};
        //4284B0's tail v523&&!v516 calls41DD10 after resource/audio/stage
        //consumers.25088 and66510 are41DD10's own guards; a skipped
        //capture never replaces its preceding mirror.
        if !self.state.saves.dialogue_checkpoint_suppressed
            &&self.state.native_flags.get(&25088).copied().unwrap_or(0)==0
            &&self.state.native_flags.get(&66510).copied().unwrap_or(0)==0 {
            self.sync_sprite_states(host)?;
            self.sync_effect_frame(&host.effect_states()?)?;
            self.sync_comment_record(host)?;
            let mirror=self.state.native_live.freeze_dialogue_checkpoint(resume)?;
            self.state.saves.captured(mirror)?;
        }
        self.state.saves.dialogue_checkpoint=None;
        Ok(())
    }
    pub(super) fn execute_save(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        if matches!(i.opcode, 0x1c | 0x1d) || (i.opcode == 0x18 && matches!(a[0], 0 | 4 | 24)) {
            ensure!(
                program.profile == Profile::Dc4PlusHarmony,
                "Circus original slot ABI is verified only for DC4PH profile0xabb6"
            );
        }
        let complete = |next| {
            Some(CommandResult::Complete {
                next,
                outcome: Outcome::Advanced,
            })
        };
        match i.opcode {
            0x1c => {
                let data = program.instruction(i.next)?;
                let Arguments::String(continuation) = &data.arguments else {
                    bail!("Circus checkpoint requires following native string metadata")
                };
                if self.state.native_flags.get(&25088).copied().unwrap_or(0) != 0
                    || self.state.native_flags.get(&66510).copied().unwrap_or(0) != 0
                {
                    //41DD10 explicitly preserves the previous mirror when
                    //suppressed; dispatch still consumes following metadata.
                    return Ok(complete(data.next));
                }
                let Some(shared_resources) = host.capture_checkpoint()? else {
                    return Ok(Some(CommandResult::Retry));
                };
                ensure!(
                    shared_resources.len() <= 8 * 1024 * 1024,
                    "Circus checkpoint resource declaration budget"
                );
                let mirror = self.state.native_live.freeze_checkpoint(i.next)?;
                let mut captured = self.state.clone();
                captured.checkpoint = None;
                captured.saves = captured.saves.without_mirrors();
                captured.pc = i.next;
                let vm_state = serde_json::to_vec(&captured)?;
                ensure!(
                    vm_state.len() <= 8 * 1024 * 1024,
                    "Circus checkpoint native declaration budget"
                );
                self.state.saves.captured(mirror)?;
                self.state.checkpoint = Some(NativeCheckpoint {
                    resume_pc: i.next,
                    continuation: continuation.clone(),
                    vm_state,
                    shared_resources,
                });
                Ok(complete(data.next))
            }
            0x18 if matches!(a[0], 4 | 24) => {
                //4284B0 case24/4 with a1!=0 suppresses the automatic
                //slot0 write during positive history dispatch.
                if a[0] == 4 && a[1] != 0 && self.state.dispatch_a2 != 0 {
                    self.state.native_live.syscall_completed(a[0]);
                    return Ok(complete(i.next));
                }
                let kind = if a[0] == 24 {
                    SlotKind::Pre
                } else {
                    SlotKind::Normal
                };
                let slot = if a[0] == 4 && a[1] != 0 {
                    0
                } else {
                    self.state.value(99, 254)?
                };
                if !self.write_native_slot(host,slot,kind,i.offset)?{return Ok(Some(CommandResult::Retry))}
                self.state.native_live.syscall_completed(a[0]);
                Ok(complete(i.next))
            }
            0x18 if a[0] == 0 => self.execute_native_restore(i, a, host).map(Some),
            0x1d => self.execute_native_restore(i, a, host).map(Some),
            _ => Ok(None),
        }
    }
}

impl Machine {
    fn write_native_slot(&mut self,host:&mut dyn NativeHost,slot:i32,kind:SlotKind,caller_pc:u32)->Result<bool> {
                if self.state.saves.pending.is_none() {
                    let mirror = self
                        .state
                        .saves
                        .capture
                        .as_ref()
                        .context("Circus native slot write requires41DD10 captured mirror")?
                        .admitted_body()?
                        .clone();
                    self.state.saves.pending = Some(PendingWrite {
                        scene: self.state.scene.clone(),
                        pc: caller_pc,
                        slot,
                        kind,
                        mirror,
                        resources: Vec::new(),
                        time: None,
                        thumbnail: None,
                        thumbnail_expected: false,
                        thumbnail_source_ready: !kind.needs_thumbnail(slot),
                    });
                }
                let pending = self.state.saves.pending.as_mut().unwrap();
                ensure!(
                    pending.pc == caller_pc
                        && pending.scene == self.state.scene
                        && pending.slot == slot
                        && pending.kind == kind,
                    "Circus native slot write continuation ownership mismatch"
                );
                if !pending.thumbnail_source_ready {
                    let Some(reply) = host.save(&SaveRequest::CurrentThumbnailSource)? else {
                        return Ok(false);
                    };
                    let SaveReply::CurrentThumbnailSource(resources) = reply else {
                        bail!("Circus current native440 selection receipt mismatch");
                    };
                    ensure!(resources.len() <= 8 * 1024 * 1024, "Circus current thumbnail declaration budget");
                    let declaration: serde_json::Value = serde_json::from_slice(&resources)?;
                    pending.thumbnail_expected = declaration["has_native_thumbnail"].as_bool()
                        .context("Circus current native440 provenance unavailable")?;
                    pending.resources = resources;
                    pending.thumbnail_source_ready = true;
                }
                if pending.thumbnail_expected && pending.thumbnail.is_none() {
                    let Some(reply) = host.save(&SaveRequest::Thumbnail {
                        checkpoint_resources: pending.resources.clone(),
                    })?
                    else {
                        return Ok(false);
                    };
                    let SaveReply::Thumbnail(png) = reply else {
                        bail!("Circus native thumbnail receipt mismatch")
                    };
                    ensure!(!png.is_empty(), "Circus native thumbnail is missing");
                    pending.thumbnail = Some(png);
                }
                if kind.needs_civil_time(slot) && pending.time.is_none() {
                    let Some(reply) = host.save(&SaveRequest::CivilTime)? else {
                        return Ok(false);
                    };
                    let SaveReply::CivilTime(time) = reply else {
                        bail!("Circus native civil-time receipt mismatch")
                    };
                    //40C2F0 changes the retained mirror date as well as this
                    // file copy; later slots>=110 retain that sampled date.
                    self.state
                        .saves
                        .capture
                        .as_mut()
                        .context("Circus native captured mirror disappeared during save")?
                        .stamp_civil_time(time)?;
                    pending.time = Some(time);
                }
                let body = pending.mirror.slot_bytes(kind, slot, pending.time)?;
                let request = SaveRequest::Write {
                    slot,
                    kind,
                    body,
                    thumbnail_png: pending.thumbnail.clone(),
                    thumbnail_expected: pending.thumbnail_expected,
                };
                let Some(reply) = host.save(&request)? else {
                    return Ok(false);
                };
                ensure!(
                    matches!(reply, SaveReply::Written),
                    "Circus native slot publication receipt mismatch"
                );
                self.state.saves.pending = None;
        Ok(true)
    }
}

#[derive(Clone,Debug)]
struct QuickSlot {write:bool,phase:u8,caller_pc:u32,old_auto:i32,toggle_skip:i32}
impl Machine {
    ///432CA0/4260D0 native fixed-slot2 owner. It never changes99/254.
    pub(in crate::machine) fn native_quick_slot(&mut self,host:&mut dyn NativeHost,write:bool)->Result<bool> {
        use crysirror_scene::layer::queued::SequenceOperation;
        if self.state.saves.quick.is_none() {
            if self.state.native_skipping()&&self.state.native_flags.get(&66510).copied().unwrap_or(0)==0{return Ok(true)}
            self.state.saves.quick=Some(QuickSlot{write,phase:0,caller_pc:self.state.pc,old_auto:0,toggle_skip:0});
        }
        ensure!(self.state.saves.quick.as_ref().unwrap().write==write,"Circus quick-slot owner mismatch");
        if write {
            if self.state.saves.quick.as_ref().unwrap().phase==0 {
                let paused=self.state.native_live.paused_sprite_objects();
                let selected=[0u16,10,20,30,40,50,12,22,32,42,52].into_iter().filter(|object|!paused.contains(object)).collect::<Vec<_>>();
                let commands=selected.iter().map(|object|Ok(sprite_ops::SpriteCommand{seed:self.state.native_live.sprite_seed(*object)?,operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}]})).collect::<Result<Vec<_>>>()?;
                if !host.sprite(&sprite_ops::SpriteRequest{commands})?{return Ok(false)}
                if !host.effect_control(&effect_ops::EffectRequest::FinishObjects{objects:selected})?{return Ok(false)}
                self.sync_sprite_states(host)?;self.sync_effect_frame(&host.effect_states()?)?;
                self.state.native_live.menu_pause_all(true);
                //422B70's all-renderer finalization is now paused; only its
                //independent overlay/text owner remains to finish.
                let mut seed=vec![0.;sprite_ops::CHANNELS];seed[16]=1.;
                if !host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{seed:sprite_ops::SpriteSeed{object:150,values:seed.clone(),retained:seed},operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}]}]})?{return Ok(false)}
                if host.text(&text_ops::TextRequest::FinishDrawableQueues)?.is_none(){return Ok(false)}
                self.sync_comment_record(host)?;
                if self.state.native_flags.get(&66510).copied().unwrap_or(0)==0 {
                    let mirror=self.state.native_live.freeze_menu_checkpoint()?;
                    self.state.saves.captured(mirror)?;
                }
                self.state.saves.quick.as_mut().unwrap().phase=1;
            }
            if self.state.saves.quick.as_ref().unwrap().phase==1 {
                let pc=self.state.saves.quick.as_ref().unwrap().caller_pc;
                if !self.write_native_slot(host,2,SlotKind::Normal,pc)?{return Ok(false)}
                let toggle=i32::from(self.state.native_flags.get(&11484).copied().unwrap_or(0)!=self.state.native_flags.get(&272077).copied().unwrap_or(0));
                let auto=self.state.value(99,13)?;
                let pending=self.state.saves.quick.as_mut().unwrap();pending.toggle_skip=toggle;pending.old_auto=auto;pending.phase=2;
            }
            if self.state.saves.quick.as_ref().unwrap().phase==2 {
                if !self.native_menu_entry(host,"mesmenu",0)?{return Ok(false)}
                self.state.native_live.menu_pause_all(false);
                self.state.saves.quick.as_mut().context("Circus quick-save continuation lost")?.phase=3;
            }
            if !self.native_skip_notice(host,0)?{return Ok(false)}
            let owner=self.state.saves.quick.take().unwrap();
            self.state.native_flags.insert(11484,owner.toggle_skip);self.state.native_flags.insert(272077,owner.toggle_skip);
            self.state.assign(99,13,owner.old_auto)?;
            self.state.dialogue.auto_advance=owner.old_auto!=0;
        }else{
            if self.state.saves.quick.as_ref().unwrap().phase==0 {
                let Some(reply)=host.save(&SaveRequest::Read{slot:2,kind:SlotKind::Normal})? else{return Ok(false)};
                let SaveReply::Read(bytes)=reply else{bail!("Circus quick-load slot receipt")};
                let Some(bytes)=bytes else{self.state.saves.quick=None;return Ok(true)};
                let mut plan=RestorePlan::decode(NativeSaveBody::from_read_prefix(&bytes)?,RestoreOrigin::Slot)?;
                plan.history_replay_allowed=false;self.prepare_restore_audio(&mut plan)?;
                self.state.saves.pending_restore=Some(plan);
                for index in [11480,11484,272077]{self.state.native_flags.insert(index,0);}
                self.state.assign(99,13,0)?;
                self.state.saves.quick.as_mut().unwrap().phase=1;
            }
            if self.state.saves.quick.as_ref().unwrap().phase==1 {
                let instruction=Instruction{offset:self.state.pc,next:self.state.pc,opcode:0x18,arguments:Arguments::Words([0;4]),continuation:None};
                match self.execute_native_restore(&instruction,[0;8],host)? {
                    CommandResult::Retry=>return Ok(false),
                    CommandResult::Complete{next,..}=>{self.state.pc=next;self.state.saves.quick.as_mut().context("Circus quick-load continuation lost")?.phase=2;}
                }
            }
            if !self.native_skip_notice(host,1)?{return Ok(false)}
            self.state.saves.quick=None;
        }
        Ok(true)
    }
}
