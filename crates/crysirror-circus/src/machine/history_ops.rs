//! Native CWnd10344 history/log policy (4049A0,406140,405340,4065B0).
//! Retained target is the log object's first DWORD, never a decoder PC or
//! transient HistoryReplay cursor. Presentation/hit testing belongs to shared UI.
use super::*;
use crate::native_save::NativeSaveBody;
#[path="history_text.rs"]
mod history_text;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryRow {
    pub flat_index: i32,
    pub body: String,
    pub speaker: String,
    pub voice: Option<String>,
    pub scene: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryAction {
    Open, Back, Previous, Next, First, Last, ToggleThumbnail,
    Row(u8), Voice(u8), Confirm(bool), ToggleConfirmation,
}
#[derive(Clone, Debug)]
pub enum HistoryRequest {
    Present { rows: Vec<HistoryRow>, total: i32, page: i32, thumbnail_enabled: bool },
    Poll { active: bool, can_open: bool, right_opens: bool },
    Confirm { kind:i16, row: HistoryRow, template: Option<Group>, warp_pointer:bool },
    CapturePreview { flat_index:i32 },
    ShowPreview { flat_index:i32, row:u8, large:bool },
    HidePreview,
    PreviewShell,
    Close,
}
#[derive(Clone, Debug)]
pub enum HistoryReply {
    Ready,
    Input { action: Option<HistoryAction>, hover_row: Option<u8>, slider: Option<i32> },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct HistoryState {
    pub(super) target: i32,
    active: bool,
    #[serde(default)] confirming:bool,
    snapshot: Option<NativeSaveBody>,
    starts: Vec<i32>,
    total: i32,
    page: i32,
    run_index: usize,
    selected: i32,
    rows: Vec<HistoryRow>,
    dirty: bool,
    #[serde(skip)] opening: u8,
    #[serde(skip)] opening_sound:bool,
    #[serde(skip)] pending_action: Option<HistoryAction>,
    #[serde(skip)] last_poll: Option<u64>,
    #[serde(skip)] preview:Option<Box<HistoryPreview>>,
    #[serde(skip)] last_hover:Option<u8>,
    #[serde(skip)] large_preview:Option<i32>,
}
#[derive(Clone,Debug)]
struct HistoryPreview {
    original:Arc<State>,
    program:Option<Arc<Program>>,
    flat:i32,
    row:u8,
    large:bool,
    phase:u8,
    restore:Arc<super::save_ops::RestorePlan>,
}
impl Default for HistoryState {
    fn default() -> Self {
        //4049A0 initializes first DWORD to-1, independently of this[1]=0.
        Self { target:-1,active:false,confirming:false,snapshot:None,starts:Vec::new(),total:0,
            page:0,run_index:0,selected:0,rows:Vec::new(),dirty:false,
            opening:0,opening_sound:true,pending_action:None,last_poll:None,preview:None,last_hover:None,large_preview:None }
    }
}
impl HistoryState {
    pub(super) fn active(&self)->bool {self.active||self.opening!=0||self.confirming}
    pub(super) fn previewing(&self)->bool {self.preview.is_some()}
    pub(super) fn set_replay_target(&mut self, target: i32) { self.target=target; self.active=false;self.confirming=false; }
    fn locate(&self, flat: i32) -> Result<(usize,u16)> {
        let body=self.snapshot.as_ref().context("Circus history snapshot owner")?;
        let index=self.starts.iter().rposition(|start| *start<=flat)
            .context("Circus history flat row precedes first run")?;
        let run=body.read_run(index)?;
        let label=i32::from(run.first_label).wrapping_add(flat.wrapping_sub(self.starts[index]));
        ensure!(label>=0&&label<=i32::from(run.last_label),"Circus history flat row outside run");
        Ok((index,label as u16))
    }
}
impl Machine {
    pub(super) fn open_native_history(&mut self,host:&mut dyn NativeHost,now:u64,play_sound:bool)->Result<bool> {
        if self.state.history.opening==0 {self.state.history.opening=1;self.state.history.opening_sound=play_sound;}
        self.open_history(host,now)
    }
    fn history_reply(&mut self, host:&mut dyn NativeHost, request:HistoryRequest)->Result<Option<HistoryReply>> {
        match host.text(&text_ops::TextRequest::History(request))? {
            Some(text_ops::TextReply::History(reply))=>Ok(Some(reply)),
            None=>Ok(None), _=>bail!("Circus history UI receipt"),
        }
    }
    ///406140(0) after432F90(3)'s read/voice/432AB0 fences.
    fn open_history(&mut self,host:&mut dyn NativeHost,now:u64)->Result<bool>{
        use crysirror_scene::layer::queued::SequenceOperation;
        if self.state.history.opening==1 {
            if let Some((scene,start,end))=&self.state.read_run {
                let Some(index)=host.scene_index(scene)? else{return Ok(false)};
                if index>=0&&*start!=0 {self.state.system_data.mark_read(index as usize,*start,*end)?;}
            }
            self.state.history.opening=2;
        }
        if self.state.history.opening==2 {
            if self.state.history.opening_sound&&host.publish_audio(&media_ops::AudioDeclaration{name:"se\\201".into(),voice:false,looping:false,voice_volume:None})?.is_none(){return Ok(false)}
            self.state.history.opening=3;
        }
        if self.state.history.opening==3 {
            if !host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:0})? {return Ok(false)}
            if !self.poll_text_events(host,u64::MAX)? {return Ok(false)}
            let mut commands=(0..150).map(|object|Ok(sprite_ops::SpriteCommand {
                seed:self.state.native_live.sprite_seed(object)?,
                operations:vec![SequenceOperation::Finish{cancel:false,lanes:None},
                    SequenceOperation::Capture{enabled:self.state.sprite.capture,repeats:0,forever:self.state.sprite.forever}],
            })).collect::<Result<Vec<_>>>()?;
            let mut extra=vec![0.;sprite_ops::CHANNELS];extra[16]=self.state.sprite.extra_rate.unwrap_or(1.);
            commands.push(sprite_ops::SpriteCommand{seed:sprite_ops::SpriteSeed{object:150,values:extra.clone(),retained:extra},
                operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}]});
            if !host.sprite(&sprite_ops::SpriteRequest{commands})? {return Ok(false)}
            if !host.effect_control(&effect_ops::EffectRequest::Finish)? {return Ok(false)}
            if host.text(&text_ops::TextRequest::FinishDrawableQueues)?.is_none(){return Ok(false)}
            self.sync_sprite_states(host)?;
            self.sync_effect_frame(&host.effect_states()?)?;
            self.capture_native_rollback()?;
            let body=self.state.saves.rollback.as_ref().unwrap().observed_body().clone();
            let last=body.read_run_index() as usize;
            ensure!(last<crate::native_save::READ_RUN_COUNT,"Circus history directory extent");
            let mut starts=Vec::new();let mut total=0i32;
            for index in 0..=last {
                let run=body.read_run(index)?;
                starts.push(total);
                total=total.checked_add(i32::from(run.last_label)-i32::from(run.first_label)+1)
                    .context("Circus history directory total overflow")?;
            }
            self.state.history.snapshot=Some(body);
            self.state.history.starts=starts;
            self.state.history.total=total;
            self.state.history.page=total-2;
            self.state.history.run_index=last;
            self.state.history.target=total-2;
            self.state.history.active=true;
            self.state.history.rows.clear();
            self.state.history.dirty=true;
            self.state.history.opening=0;
            self.state.history.last_poll=Some(now);
            //433E50 observes active4065B0 and disables auto while the log owns input.
            self.state.native_flags.insert(23872,1);
            self.state.assign(99,13,0)?;
        }
        Ok(true)
    }
    pub(super) fn poll_history(&mut self,host:&mut dyn NativeHost,now:u64)->Result<bool>{
        if !self.state.history.active && !self.state.history.confirming
            && self.state.history.preview.is_none() && self.state.history.pending_action.is_none()
            && self.state.native_flags.get(&23872).copied().unwrap_or(0)!=0 {
            //406DB0/402750 mutated the same retained owner during402BC0.
            //The next4065B0 returns0; its GUI is not resurrected from Buf.
            if self.history_reply(host,HistoryRequest::Close)?.is_none(){return Ok(false)}
            self.state.native_flags.insert(23872,0);
            self.state.history.last_poll=Some(now);
            return Ok(true);
        }
        if self.state.history.opening!=0&&!self.open_history(host,now)? {return Ok(false)}
        if self.state.history.dirty&&!self.rebuild_history(host)? {return Ok(false)}
        if self.state.history.last_poll==Some(now){return Ok(true)}
        if self.state.history.pending_action.is_none() {
            let request=HistoryRequest::Poll{active:self.state.history.active||self.state.history.confirming,
                can_open:self.state.wait_process==6,
                right_opens:self.state.system_data.dword(26)==2};
            let Some(reply)=self.history_reply(host,request)? else{return Ok(false)};
            let HistoryReply::Input{action,hover_row,slider}=reply else{bail!("Circus history input receipt")};
            if let Some(page)=slider.filter(|_|self.state.history.active&&self.state.history.total>4){
                self.state.history.page=page.clamp(2,self.state.history.total-2);
                self.state.history.dirty=true;
            }
            self.state.history.pending_action=action.clone();
            if hover_row.is_some()&&self.state.system_data.dword(40)!=0 {
                //402BC0 owns an actual replay thumbnail; presentation cannot
                //replace this with a slot PNG or the current game frame.
                self.prepare_history_preview(hover_row.unwrap(),false)?;
                if self.state.history.preview.is_some(){return Ok(true)}
            }
            if hover_row.is_none()&&self.state.history.last_hover.is_some() {
                if self.history_reply(host,HistoryRequest::HidePreview)?.is_none(){return Ok(false)}
                self.state.history.last_hover=None;
            }
            self.state.history.pending_action=action;
        }
        let Some(action)=self.state.history.pending_action.clone() else{
            self.state.history.last_poll=Some(now);return Ok(true)
        };
        match action {
            HistoryAction::Open if !self.state.history.active=>{
                self.state.history.opening=1;
                self.state.history.opening_sound=true;
                if !self.open_history(host,now)? {return Ok(false)}
            }
            HistoryAction::Back=>{
                if self.history_reply(host,HistoryRequest::Close)?.is_none(){return Ok(false)}
                self.state.history.active=false;self.state.history.target=-1;
                self.state.native_flags.insert(23872,0);
            }
            HistoryAction::Previous=>{
                if self.state.history.page>0&&self.state.history.page+1>=4 {
                    self.state.history.page-=1;
                    let index=self.state.history.run_index;
                    if index>0&&self.state.history.starts[index]>self.state.history.page+1 {self.state.history.run_index-=1;}
                    self.state.history.dirty=true;
                }
            }
            HistoryAction::Next=>{
                if self.state.history.page>=self.state.history.total-2 {
                    if self.history_reply(host,HistoryRequest::Close)?.is_none(){return Ok(false)}
                    self.state.history.active=false;self.state.history.target=-1;
                    self.state.native_flags.insert(23872,0);
                } else {
                    self.state.history.page+=1;
                    let index=self.state.history.run_index;
                    if index+1<self.state.history.starts.len()&&self.state.history.starts[index+1]<=self.state.history.page+1 {self.state.history.run_index+=1;}
                    self.state.history.dirty=true;
                }
            }
            HistoryAction::First=>{self.state.history.page=2;self.state.history.dirty=true;}
            HistoryAction::Last=>{self.state.history.page=self.state.history.total-2;self.state.history.dirty=true;}
            HistoryAction::ToggleThumbnail=>{
                let old=self.state.system_data.dword(40);
                self.state.assign(99,188,i32::from(old==0))?;
                self.state.history.dirty=true;
            }
            HistoryAction::Voice(row)=>{
                if let Some(name)=self.state.history.rows.get(usize::from(row)).and_then(|row|row.voice.clone()) {
                    if let Some(volume)=self.state.native_voice_volume(&name)? {
                        if host.publish_audio(&media_ops::AudioDeclaration{name,voice:true,looping:false,voice_volume:Some(volume)})?.is_none(){return Ok(false)}
                    }
                }
            }
            HistoryAction::Row(row_index)=>{
                let row=self.state.history.rows.get(usize::from(row_index)).context("Circus history selected row")?.clone();
                self.state.history.selected=row.flat_index;
                if self.state.system_data.dword(40)!=0&&self.state.history.large_preview!=Some(row.flat_index) {
                    self.prepare_history_preview(row_index,true)?;
                    if let Some(preview)=self.state.history.preview.as_mut(){Arc::make_mut(&mut preview.original).history.pending_action=Some(HistoryAction::Row(row_index));}
                    return Ok(true);
                }
                self.state.history.large_preview=None;
                if self.state.value(93,1)?!=0 {
                    self.state.history.pending_action=Some(HistoryAction::Confirm(true));
                    return Ok(true);
                }
                let mut template=self.state.groups.get(&12).cloned();
                if let Some(group)=template.as_mut(){for button in &mut group.buttons {button.check_index=1;}self.resolve_group_values(host,group)?;}
                if self.history_reply(host,HistoryRequest::Confirm{kind:1,row,template,warp_pointer:self.state.system_data.dword(33)!=0})?.is_none(){return Ok(false)}
                self.state.native_flags.insert(23872,2);
                self.state.history.confirming=true;
            }
            HistoryAction::ToggleConfirmation=>{let value=self.state.value(93,1)?;self.state.assign(93,1,i32::from(value==0))?;}
            HistoryAction::Confirm(true)=>{if !self.resume_history_selection(host,now)? {return Ok(false)}}
            HistoryAction::Confirm(false)=>{self.state.history.confirming=false;self.state.native_flags.insert(23872,1);}
            _=>{}
        }
        self.state.history.pending_action=None;
        self.state.history.last_poll=Some(now);
        Ok(true)
    }
}
impl Machine {
    ///404E30 maps the flat history directory to the original scene label;
    ///402860 reads native6A/4D/6B strings without executing scene opcodes.
    fn history_row(&mut self,host:&mut dyn NativeHost,flat:i32)->Result<Option<Option<HistoryRow>>>{
        if flat<0||flat>=self.state.history.total{return Ok(Some(None))}
        let (index,label)=self.state.history.locate(flat)?;
        let native_scene=self.state.history.snapshot.as_ref().unwrap().read_run(index)?.scene_index as i16;
        let scene=if native_scene<0 {self.state.scene.clone()}else{
            let Some(name)=host.scene_name(i32::from(native_scene))? else{return Ok(None)};name
        };
        let Some(program)=host.scene(&scene)? else{return Ok(None)};
        let Some(offset)=program.labels.get(usize::from(label)).map(|offset|offset&0x7fff_ffff) else{return Ok(Some(None))};
        let Some(start)=program.instructions.iter().position(|i|i.offset==offset) else{return Ok(Some(None))};
        let mut body=String::new();let mut speaker=String::new();let mut voice=None;
        for instruction in &program.instructions[start..] {
            match (&instruction.arguments,instruction.opcode) {
                (Arguments::String(text),0x6a)=>{speaker=text.clone();body=text.clone();}
                (Arguments::String(text),0x4d)=>{voice=(!text.is_empty()).then(||text.clone());}
                (Arguments::String(text),0x6b)=>{body=text.clone();break;}
                (_,0|0x16)=>break,
                _=>{}
            }
        }
        if body.is_empty()||body.contains("◆タイトル名◆"){return Ok(Some(None))}
        let changed=self.state.native_flags.get(&66516).copied().unwrap_or(0)!=0;
        let body=history_text::preprocess(&body,&self.state.protagonist_names,changed)?;
        let speaker=history_text::preprocess(&speaker,&self.state.protagonist_names,changed)?;
        Ok(Some(Some(HistoryRow{flat_index:flat,body,speaker,voice,scene})))
    }
    fn rebuild_history(&mut self,host:&mut dyn NativeHost)->Result<bool>{
        let total=self.state.history.total;
        let page=self.state.history.page;
        let mut rows=Vec::new();
        let mut count=total.min(4).max(0);let mut skipped=0;let mut n=0;
        while n<count {
            let flat=page-n+1;
            if flat<0{break}
            let Some(row)=self.history_row(host,flat)? else{return Ok(false)};
            if total<=4 {
                if let Some(row)=row {rows.push(row)}else{count+=1;}
            } else if row.is_none(){skipped+=1;count+=1;}
            n+=1;
            ensure!(n<=self.state.history.total+4,"Circus history filtering progress");
        }
        if total>4 {
            for n in 0..4 {
                let flat=page-n+skipped+1;
                if flat<0{break}
                let Some(row)=self.history_row(host,flat)? else{return Ok(false)};
                if let Some(row)=row{rows.push(row)}
            }
        }
        rows.reverse();
        let base=if total<4{0}else{page-total.min(4)+2};
        for (n,row) in rows.iter_mut().enumerate(){row.flat_index=base+n as i32;}
        let request=HistoryRequest::Present{rows:rows.clone(),total,page,
            thumbnail_enabled:self.state.system_data.dword(40)!=0};
        if self.history_reply(host,request)?.is_none(){return Ok(false)}
        self.state.assign(99,245,-1)?;
        self.state.history.rows=rows;
        self.state.history.last_hover=None;
        self.state.history.large_preview=None;
        self.state.history.target=0; //405340's final store, not the selected row.
        self.state.history.dirty=false;
        Ok(true)
    }
}
impl Machine {
    fn resume_history_selection(&mut self,host:&mut dyn NativeHost,now:u64)->Result<bool>{
        let selected=self.state.history.selected;
        self.begin_history_replay(host,selected,now,false)
    }
    fn begin_history_replay(&mut self,host:&mut dyn NativeHost,flat:i32,_now:u64,preview:bool)->Result<bool>{
        use super::save_ops::{RestoreOrigin,RestorePlan};
        use crysirror_scene::layer::queued::SequenceOperation;
        let (run_index,label)=self.state.history.locate(flat)?;
        let snapshot=self.state.history.snapshot.as_ref().context("Circus retained log snapshot")?.clone();
        let native_scene=snapshot.read_run(run_index)?.scene_index as i16;
        let scene=if native_scene<0 {self.state.scene.clone()}else{
            let Some(name)=host.scene_name(i32::from(native_scene))? else{return Ok(false)};name
        };
        let Some(program)=host.scene(&scene)? else{return Ok(false)};
        program.instruction(3).context("Circus log scene entry boundary")?;
        let Some(scene_index)=host.scene_index(&scene)? else{return Ok(false)};
        //432D90 accepted1/11 reaches425E60; source433E50 restarts this scene
        //with15616 and the retained numerical label, not a bytecode seek.
        let commands=(0..150).map(|object|Ok(sprite_ops::SpriteCommand {
            seed:self.state.native_live.sprite_seed(object)?,
            operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}],
        })).collect::<Result<Vec<_>>>()?;
        if !host.sprite(&sprite_ops::SpriteRequest{commands})? {return Ok(false)}
        self.sync_sprite_states(host)?;
        if host.text(&text_ops::TextRequest::Movie(movie_ops::MovieRequest::Prepare{protected:false}))?.is_none(){return Ok(false)}
        if !preview {
            for kind in 0..3 {
                if !host.emit(&Event::ReleaseAudioGroup{kind,forced:true,steps:0})? {return Ok(false)}
            }
        }
        let body=self.state.native_live.history_jump(&snapshot,run_index,self.state.history.run_index,&scene)?;
        if !self.reset_history_drawables(host)? {return Ok(false)}
        let mut plan=RestorePlan::decode(body.clone(),RestoreOrigin::Rollback{preserve_effects:false})?;
        self.prepare_restore_audio(&mut plan)?;
        let mut candidate=self.apply_native_restore(&plan,scene_index)?;
        candidate.native_live=self.state.native_live.clone();
        candidate.native_live.preserve_ephemeral_from(&self.state.native_live);
        candidate.native_live.restore_renderer_scope(&self.state.native_live,&BTreeSet::new(),&BTreeSet::new(),false)?;
        candidate.native_live.movie_screen_clear(false,true)?;
        for object in 10..64 {candidate.native_live.reset_object(object)?;}
        candidate.native_live.history_scene_entry(&scene)?;
        candidate.scene=scene;
        candidate.pc=3;
        candidate.assign(99,195,0)?;
        candidate.assign(99,245,-1)?;
        candidate.assign(99,13,0)?;
        for (address,value) in [(11484,0),(27962,0),(66508,0),(66510,0),(66515,0),(272090,1),(15616,1),(23872,0)] {
            candidate.native_flags.insert(address,value);
        }
        if preview {candidate.native_flags.insert(23872,self.state.native_flags.get(&23872).copied().unwrap_or(0));}
        candidate.image_names.retain(|slot,_|(37..=44).contains(slot));
        candidate.dialogue.speaker.clear();candidate.dialogue.body.clear();
        candidate.dialogue.voice_source.clear();candidate.dialogue.auto_advance=false;candidate.dialogue.native_auto_started=None;candidate.dialogue.native_forward_pending=false;
        candidate.last_music=None;
        candidate.read_run=None;
        candidate.wait_process=0;candidate.wait_click=false;candidate.wait_until=None;
        candidate.menu_wait=false;candidate.wait_text_idle=None;candidate.dialogue.panel_wait=false;
        candidate.pending_audio.clear();candidate.pending_music=None;
        candidate.dispatch_a2=1;candidate.dispatch_a3=false;
        let target=i32::from(label);
        let mut limit=if scene_index<0{2}else{i32::from(snapshot.read_run(0)?.first_label)};
        if scene_index>=0 {
            for n in 0..=snapshot.read_run_index() as usize {
                let run=snapshot.read_run(n)?;
                if i32::from(run.scene_index as i16)==scene_index {limit=i32::from(run.first_label);}
            }
        }
        candidate.history.set_replay_target(target);
        candidate.history.pending_action=None;
        candidate.history_replay=Some(HistoryReplay{target,phase:0,limit});
        if self.history_reply(host,HistoryRequest::Close)?.is_none(){return Ok(false)}
        self.state=candidate;self.program=Some(program);
        Ok(true)
    }
}
impl Machine {
    fn prepare_history_preview(&mut self,row:u8,large:bool)->Result<()> {
        if !large&&self.state.history.last_hover==Some(row){return Ok(())}
        let flat=self.state.history.rows.get(usize::from(row)).context("Circus hovered history row")?.flat_index;
        ensure!(self.state.history.preview.is_none(),"Circus history preview already owned");
        let capture=self.state.native_live.freeze_rollback()?;
        let body=capture.admitted_body()?.clone();
        let mut restore=super::save_ops::RestorePlan::decode(body,super::save_ops::RestoreOrigin::Rollback{preserve_effects:false})?;
        self.prepare_restore_audio(&mut restore)?;
        self.state.history.selected=flat;
        let original=Arc::new(self.state.clone());
        self.state.history.preview=Some(Box::new(HistoryPreview{original,
            program:self.program.clone(),flat,row,large,phase:0,restore:Arc::new(restore)}));
        Ok(())
    }
    pub(super) fn poll_history_preview(&mut self,host:&mut dyn NativeHost,now:u64)->Result<Outcome>{
        let preview=self.state.history.preview.as_ref().context("Circus history preview owner")?.clone();
        if preview.phase==0 {
            //422540(1) is only the native25104 capture/declaration guard.
            //It is not an audio mute or protected-scene flag.
            self.state.native_flags.insert(25104,1);
            if !self.begin_history_replay(host,preview.flat,now,true)?{return Ok(Outcome::Pending)}
            self.state.history.preview.as_mut().context("Circus replay lost preview owner")?.phase=1;
            return Ok(Outcome::Advanced)
        }
        if preview.phase==1 {
            let result=self.step_without_history_preview(host,now,false)?;
            if self.state.history_replay.is_none(){self.state.history.preview.as_mut().unwrap().phase=2;return Ok(Outcome::Advanced)}
            return Ok(result)
        }
        if preview.phase==2 {
            if self.history_reply(host,HistoryRequest::CapturePreview{flat_index:preview.flat})?.is_none(){return Ok(Outcome::Pending)}
            self.state.history.preview.as_mut().unwrap().phase=3;
            return Ok(Outcome::Advanced)
        }
        if preview.phase==3 {
            use super::save_ops::{SaveRequest,SaveReply};
            let Some(reply)=host.save(&SaveRequest::Restore{plan:(*preview.restore).clone()})? else{return Ok(Outcome::Pending)};
            ensure!(matches!(reply,SaveReply::Restored|SaveReply::RestoredAudio(_)|SaveReply::RestoredMedia{..}),"Circus history preview restoration receipt");
            let mut original=(*preview.original).clone();
            let mut retained_owner=self.state.history.clone();
            retained_owner.pending_action=original.history.pending_action.clone();
            let mut completed=preview.clone();completed.phase=4;
            retained_owner.preview=Some(completed);
            //402BC0 backs up Buf and its mirrors, not the CWnd10344 log
            //object. ASM41FBAA calls406DB0 on that same owner: preserve its
            //target/active changes, even though the live game body is restored.
            original.history=retained_owner;
            original.dialogue.native_auto_started=None;original.dialogue.native_forward_pending=false;
            self.state=original;self.program=preview.program.clone();
            self.sync_sprite_states(host)?;
            self.sync_effect_frame(&host.effect_states()?)?;
            self.sync_comment_record(host)?;
            return Ok(Outcome::Advanced)
        }
        ensure!(preview.phase==4,"Circus history preview phase");
        if self.history_reply(host,HistoryRequest::PreviewShell)?.is_none(){return Ok(Outcome::Pending)}
        if self.history_reply(host,HistoryRequest::ShowPreview{flat_index:preview.flat,row:preview.row,large:preview.large})?.is_none(){return Ok(Outcome::Pending)}
        self.state.history.preview=None;
        self.state.history.last_hover=Some(preview.row);
        self.state.history.large_preview=preview.large.then_some(preview.flat);
        self.state.history.last_poll=Some(now);
        Ok(Outcome::WaitForInput)
    }
}
impl Machine {
    ///433E50:445A10 frees each normal/captured queue, then445030(0)
    ///calls444BC0's full numerical reset and443EA0(null). Keep the
    ///renderer1904/1912 capture declarations, which are outside that reset.
    pub(super) fn reset_history_drawables(&mut self,host:&mut dyn NativeHost)->Result<bool>{
        use crysirror_scene::layer::queued::SequenceOperation;
        let mut commands=Vec::new();
        for object in 10..64 {
            let seed=crate::native_live::NativeLiveState::sprite_reset_seed(object)?;
            commands.push(sprite_ops::SpriteCommand{seed:seed.clone(),operations:vec![
                SequenceOperation::Finish{cancel:true,lanes:None},
                SequenceOperation::Reset{values:seed.values,retained:seed.retained},
                SequenceOperation::Capture{enabled:self.state.sprite.capture,repeats:0,forever:self.state.sprite.forever},
            ]});
        }
        if !host.sprite(&sprite_ops::SpriteRequest{commands})?{return Ok(false)}
        for object in 10..64 {self.state.native_live.reset_object(object)?;}
        self.sync_sprite_states(host)?;
        self.sync_effect_frame(&host.effect_states()?)?;
        Ok(true)
    }
}
