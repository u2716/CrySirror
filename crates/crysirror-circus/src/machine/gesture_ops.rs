//!433680 and432F90 native gesture/action policy. Source input and retained
//!Group12 confirmation reuse shared services; no second UI/input engine lives here.
use super::*;
use super::system_ops::{SystemReply,SystemRequest};
#[path="input_prefix.rs"]
mod prefix;
pub(super) use prefix::PrefixState;

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum GestureRequest {
    Poll{enabled:bool,can_track:bool,threshold:i32,actions:[i32;4],overlay:Option<Group>},
    Acknowledge,
    PrefixInput,
    ConsumeInput{mask:u64},
    DialogueObjects,
    Confirm{kind:i16,template:Option<Group>,warp_pointer:bool},
    PollConfirmation,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum GestureReply {Idle,Holding{began:bool},Action(i32),Done,Confirmed(bool),ToggleConfirmation,PrefixInput(PrefixInput),Objects(Vec<u16>)}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct PrefixInput {pub revision:u64,pub enabled:bool,pub pressed:u64,pub released:u64,pub down:u64,pub control_trigger:bool,pub control_stock:bool}
#[derive(Clone,Debug,Default)]
pub(super) struct GestureState {
    completed:u64,
    direct_action:bool,
    pending:Option<i32>,phase:u8,question_kind:Option<i16>,question_ready:bool,
    confirmed:Option<bool>,menu_backup:Option<(i32,i32)>,
    ///427E20 explicitly initializes CWnd23912 to0. It is only replaced by
    ///432EE0 when a2=2/3 actually opens a confirmation, not on bypass.
    saved_window_gate:i32,
}
impl GestureState {
    ///432EE0 changes the native process to89 only after the Group12 owner
    ///exists. Preference-bypassed confirmations remain the caller's process6.
    pub(super) fn modal_active(&self)->bool {self.question_ready}
}
impl Machine {
    pub(super) fn native_action_pending(&self)->bool {self.state.system_ops.gestures.pending.is_some()}
    pub(super) fn native_system_action(&mut self,host:&mut dyn NativeHost,action:i32,now:u64)->Result<bool> {
        if let Some(pending)=self.state.system_ops.gestures.pending {
            ensure!(pending==action,"Circus native system action continuation");
        }else{self.state.system_ops.gestures.pending=Some(action);self.state.system_ops.gestures.direct_action=true;}
        self.poll_native_gestures(host,now)
    }
    pub(super) fn gesture_request(&mut self,host:&mut dyn NativeHost,request:GestureRequest)->Result<Option<GestureReply>> {
        match host.system(SystemRequest::Gesture(request))? {
            Some(SystemReply::Gesture(reply))=>Ok(Some(reply)),None=>Ok(None),
            _=>bail!("Circus native gesture receipt"),
        }
    }
    ///Poll before history/global callbacks: a captured right release belongs
    ///to433680 until its action and confirmation have acknowledged it.
    pub(crate) fn poll_native_gestures(&mut self,host:&mut dyn NativeHost,now:u64)->Result<bool> {
        if self.state.system_ops.gestures.pending.is_none() {
            let request=GestureRequest::Poll{
                enabled:self.state.value(99,201)?!=0&&self.state.native_flags.get(&11480).copied().unwrap_or(0)==0,
                can_track:self.state.wait_click&&self.state.wait_process==6&&!self.state.history.active()&&!self.state.movie.active(),
                threshold:self.state.value(99,194)?,
                actions:[self.state.value(99,197)?,self.state.value(99,196)?,self.state.value(99,199)?,self.state.value(99,198)?],
                overlay:self.state.groups.get(&7).cloned(),
            };
            let Some(reply)=self.gesture_request(host,request)? else{return Ok(false)};
            match reply {
                GestureReply::Idle=>return Ok(true),
                GestureReply::Holding{began}=>{
                    if began&&(self.state.native_skipping()||self.state.value(99,13)?!=0) {
                        self.state.native_flags.insert(11480,0);
                        self.state.assign(99,195,0)?;self.state.native_flags.insert(11484,0);
                        self.state.assign(99,13,0)?;
                    }
                    return Ok(false)
                }
                GestureReply::Action(action)=>{self.state.system_ops.gestures.pending=Some(action);},
                _=>bail!("Circus right gesture poll response"),
            }
        }
        let action=self.state.system_ops.gestures.pending.unwrap();
        if self.state.system_ops.gestures.phase==0 {
            if (0..=12).contains(&action) {
                if let Some((scene,start,end))=&self.state.read_run {
                    let Some(index)=host.scene_index(scene)? else{return Ok(false)};
                    if index>=0&&*start!=0 {self.state.system_data.mark_read(index as usize,*start,*end)?;}
                }
            }
            self.state.system_ops.gestures.phase=1;
        }
        let mut complete=true;
        match action {
            0|1 if self.state.system_ops.gestures.direct_action=>{
                //432F90 differs from433680's two special mesmenu actions.
                let value=i32::from(self.state.value(99,195)?==0);
                self.state.assign(99,195,value)?;
                if action==1 {self.state.assign(99,30,0)?;}
                self.state.assign(99,13,0)?;
            }
            0|1=>{
                if self.state.system_ops.gestures.phase==1 {
                    if action==0 {self.state.assign(99,30,1)?;}
                    let skip=i32::from(self.state.native_flags.get(&11484).copied().unwrap_or(0)!=self.state.value(99,195)?);
                    let auto=self.state.value(99,13)?;
                    self.state.system_ops.gestures.menu_backup=Some((skip,auto));
                    self.state.native_live.menu_pause_all(true);
                    self.state.system_ops.gestures.phase=2;
                }
                if !self.native_menu_entry(host,"mesmenu",now)? {return Ok(false)}
                self.state.native_live.menu_pause_all(false);
                let (skip,auto)=self.state.system_ops.gestures.menu_backup.take().context("Circus gesture menu backup")?;
                self.state.assign(99,195,skip)?;self.state.native_flags.insert(11484,skip);
                self.state.assign(99,13,auto)?;
                let value=self.state.value(99,195)?;
                self.state.assign(99,195,i32::from(value==0))?;
                if action==1 {self.state.assign(99,30,0)?;}
                self.state.assign(99,13,0)?;
            }
            2=>{
                let value=i32::from(self.state.value(99,13)?==0);self.state.assign(99,13,value)?;
                if value!=0 {self.state.dialogue.native_auto_started=Some(0);}
                self.state.assign(99,195,0)?;
            }
            3=>{complete=self.open_native_history(host,now,false)?;}
            4|5=>{
                if self.state.value(9,7)?==0 {complete=self.native_menu_entry(host,if action==4{"save"}else{"load"},now)?;}
            }
            6|10=>{
                if action==10 {self.state.assign(100,54,5)?;}
                complete=self.native_menu_entry(host,"config",now)?;
            }
            7|8|9|11=>{
                let guarded=matches!(action,7|8)&&(self.state.value(9,7)?!=0
                    ||self.state.native_skipping()&&self.state.native_flags.get(&66510).copied().unwrap_or(0)==0);
                if !guarded&&(action!=8||host.save_exists(2)?) {
                    if self.state.system_ops.gestures.question_kind.is_none() {
                        let kind=if action==9 {
                            match host.system(SystemRequest::NativeWindowMode)? {
                                Some(SystemReply::NativeWindowMode(mode))=>if mode!=0{2}else{3},
                                None=>return Ok(false),_=>bail!("Circus gesture native window mode receipt"),
                            }
                        }else if action==11{4}else{action as i16};
                        self.state.system_ops.gestures.question_kind=Some(kind);
                    }
                    let kind=self.state.system_ops.gestures.question_kind.unwrap();
                    if self.state.system_ops.gestures.confirmed.is_none() {
                        if self.state.value(93,kind)?!=0 {self.state.system_ops.gestures.confirmed=Some(true);}
                        else {
                            if !self.state.system_ops.gestures.question_ready {
                                if matches!(action,7|8)&&self.state.system_ops.gestures.phase==1 {
                                    if host.publish_audio(&media_ops::AudioDeclaration{name:"se\\201".into(),voice:false,looping:false,voice_volume:None})?.is_none(){return Ok(false)}
                                    self.state.system_ops.gestures.phase=2;
                                }
                                let mut template=self.state.groups.get(&12).cloned();
                                if let Some(group)=template.as_mut(){for button in &mut group.buttons{button.check_index=kind;}self.resolve_group_values(host,group)?;}
                                if !matches!(self.gesture_request(host,GestureRequest::Confirm{kind,template,warp_pointer:self.state.value(99,205)?!=0})?,Some(GestureReply::Done)){return Ok(false)}
                                if matches!(kind,2|3) {
                                    let Some(SystemReply::WindowTransitionFlag(value))=host.system(SystemRequest::WindowTransitionFlag(None))? else{return Ok(false)};
                                    self.state.system_ops.gestures.saved_window_gate=value;
                                    if host.system(SystemRequest::WindowTransitionFlag(Some(0)))?.is_none(){return Ok(false)}
                                }
                                self.state.system_ops.gestures.question_ready=true;
                            }
                            match self.gesture_request(host,GestureRequest::PollConfirmation)? {
                                Some(GestureReply::Confirmed(value))=>self.state.system_ops.gestures.confirmed=Some(value),
                                Some(GestureReply::ToggleConfirmation)=>{let value=self.state.value(93,kind)?;self.state.assign(93,kind,i32::from(value==0))?;return Ok(false)},
                                Some(GestureReply::Holding{..})|None=>return Ok(false),
                                _=>bail!("Circus gesture confirmation input receipt"),
                            }
                        }
                    }
                    if self.state.system_ops.gestures.confirmed==Some(true) {
                        match action {
                            7|8=>complete=self.native_quick_slot(host,action==7)?,
                            9=>{
                                let mode=if kind==2{0}else{1};
                                complete=matches!(host.system(SystemRequest::NativeFullscreenMode(mode))?,Some(SystemReply::Done));
                            }
                            11=>complete=self.native_menu_entry(host,"gotitle",now)?,
                            _=>unreachable!(),
                        }
                    }
                    if complete&&matches!(kind,2|3)&&host.system(SystemRequest::WindowTransitionFlag(Some(self.state.system_ops.gestures.saved_window_gate)))?.is_none(){return Ok(false)}
                }
            }
            12=>{complete=matches!(host.system(SystemRequest::VoiceRate(self.state.value(99,185)?.wrapping_add(100)))?,Some(SystemReply::Done));}
            _=>{}, //432F90 explicitly ignores every unsigned action>12.
        }
        if !complete{return Ok(false)}
        if !matches!(self.gesture_request(host,GestureRequest::Acknowledge)?,Some(GestureReply::Done)){return Ok(false)}
        let saved_window_gate=self.state.system_ops.gestures.saved_window_gate;
        let completed=self.state.system_ops.gestures.completed.wrapping_add(1);
        self.state.system_ops.gestures=GestureState{saved_window_gate,completed,..Default::default()};
        Ok(true)
    }
}
