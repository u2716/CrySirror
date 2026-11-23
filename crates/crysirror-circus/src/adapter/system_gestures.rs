//!433680 native right-button gesture policy. Shared Ui owns owner eligibility
//!and pointer observations; shared Layers render the original Group7 paints.
use super::*;
use crate::machine::gesture_ops::{GestureReply,GestureRequest,PrefixInput};
use crate::machine::history_ops::{HistoryAction,HistoryReply,HistoryRequest,HistoryRow};
use crysirror_core::service::buttons;
const ROOT:u32=0x38e0_0000;
#[derive(Default)]
pub(super) struct NativeGestures {
    down_since:Option<u64>,
    threshold:i32,
    eligible:bool,
    tracking:bool,
    origin:Option<[i32;2]>,
    overlay:bool,
    question:bool,
    action:Option<i32>,
    revision:Option<u64>,
    prefix_revision:Option<u64>,
    prefix_consumed:u64,
    control_since:Option<u64>,
}
impl NativeGestures {
    pub(super) fn observe_stock(&mut self,down:u64,pressed:u64,time_ms:u64,enabled:bool) {
        for (key,since) in [(buttons::CTRL,&mut self.control_since),(buttons::MOUSE_RIGHT,&mut self.down_since)] {
            if !enabled||down&key==0 {*since=None;}
            else if since.is_none()||pressed&key!=0 {*since=Some(time_ms);}
        }
    }
    pub(super) fn finish_input_frame(&mut self) {self.prefix_consumed=0;}
    pub(in crate::adapter) fn confirming(&self)->bool{self.question}
    pub(super) fn waiting(&self)->bool{self.tracking||self.question}
}
impl Bridge<'_> {
    ///Native442EC0's state2 is clock driven even without pointer motion.
    ///Once tracking starts, only actual input or existing animation wakes it.
    pub(in crate::adapter) fn native_gesture_deadline(&self)->Option<u64> {
        let state=&self.services.system.gestures;
        if !state.eligible||state.tracking||state.action.is_some()||state.question{return None}
        let start=state.down_since?;
        let delay=200i32.max(state.threshold.wrapping_mul(300)) as u64;
        Some(start.saturating_add(delay).saturating_add(1))
    }
    fn gesture_overlay(&mut self,group:Option<&Group>,position:[i32;2])->Result<()> {
        if self.services.system.gestures.overlay {self.submit(vec![json!({"op":"remove","id":ROOT})],vec![])?;self.services.system.gestures.overlay=false;}
        let Some(group)=group else{return Ok(())};
        self.root()?;
        let mut layers=vec![json!({"op":"put","layer":{"id":ROOT,"parent":SCENE_ROOT,"rect":[position[0],position[1],1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[1000,0]}})];
        for (index,part) in group.images.iter().enumerate(){
            let id=ROOT+1+index as u32;
            self.image_layer(&mut layers,id,part,100)?;
            if let Some(command)=layers.iter_mut().rev().find(|command|command["layer"]["id"]==id){command["layer"]["parent"]=json!(ROOT);command["layer"]["order"]=json!([1000,id&0xffff]);command["layer"]["visible"]=json!(true);}
        }
        self.text_layers(&mut layers,ROOT,ROOT+0xf000,&group.texts,1000)?;
        for (ordinal,button) in group.buttons.iter().enumerate(){
            for (index,part) in button.images.iter().enumerate(){
                let id=ROOT+0x1000+ordinal as u32*128+index as u32;
                self.image_layer(&mut layers,id,part,100)?;
                if let Some(command)=layers.iter_mut().rev().find(|command|command["layer"]["id"]==id){command["layer"]["parent"]=json!(ROOT);command["layer"]["order"]=json!([1000,id&0xffff]);command["layer"]["visible"]=json!(crate::machine::menu_ops::image_visible(part.state,99));}
            }
            self.text_layers(&mut layers,ROOT,ROOT+0x8000+ordinal as u32*128,&button.texts,1000)?;
        }
        self.submit(layers,vec![])?;self.services.system.gestures.overlay=true;Ok(())
    }
    pub(super) fn native_gesture(&mut self,request:GestureRequest)->Result<Option<SystemReply>> {
        let reply=match request {
            GestureRequest::Acknowledge=>{
                self.sample_callback_releases()?;
                self.services.system.gestures.prefix_revision=self.services.system.callback_input_revision;
                self.services.system.gestures.prefix_consumed=u64::MAX;
                self.services.system.gestures.action=None;
                //442B00 clears transient release stock; it retains a genuine
                //held state2 rather than inventing a release for another UI.
                self.services.system.callback_releases=0;
                self.services.system.callback_buttons_pressed=0;
                GestureReply::Done
            }
            GestureRequest::PrefixInput=>{
                self.sample_callback_releases()?;
                let input=self.host.input_observation(self.client)?;
                ensure!(input.client==self.client,"Circus foreign433AA0 input");
                let enabled=self.services.system.callback_input_enabled;
                let down=self.services.system.callback_buttons_down;
                let state=&mut self.services.system.gestures;
                if state.prefix_revision!=Some(input.revision) {
                    state.prefix_revision=Some(input.revision);state.prefix_consumed=0;
                }
                let pressed=if enabled{self.services.system.callback_buttons_pressed&!state.prefix_consumed}else{0};
                let released=self.services.system.callback_releases&!state.prefix_consumed;
                let elapsed=state.control_since.map_or(0,|start|(self.host.now_ms() as u32).wrapping_sub(start as u32));
                GestureReply::PrefixInput(PrefixInput{revision:input.revision,enabled,pressed,released,down,
                    control_trigger:pressed&buttons::CTRL!=0||down&buttons::CTRL!=0&&elapsed>200,
                    control_stock:(down|released)&buttons::CTRL!=0})
            }
            GestureRequest::ConsumeInput{mask}=>{
                self.sample_callback_releases()?;
                self.services.system.callback_releases&=!mask;
                self.services.system.callback_buttons_pressed&=!mask;
                self.services.system.gestures.prefix_revision=self.services.system.callback_input_revision;
                self.services.system.gestures.prefix_consumed|=mask;
                GestureReply::Done
            }
            GestureRequest::DialogueObjects=>GestureReply::Objects((10..59u16)
                .filter(|object|!self.services.effect_control.particle_wrapper(*object)&&!self.services.primary.paused.contains(object)).collect()),
            GestureRequest::Confirm{kind,template,warp_pointer}=>{
                let row=HistoryRow{flat_index:-1,body:String::new(),speaker:String::new(),voice:None,scene:String::new()};
                let Some(HistoryReply::Ready)=self.native_history(&HistoryRequest::Confirm{kind,row,template,warp_pointer})? else{return Ok(None)};
                self.services.system.gestures.question=true;GestureReply::Done
            }
            GestureRequest::PollConfirmation=>{
                let Some(reply)=self.native_history(&HistoryRequest::Poll{active:true,can_open:false,right_opens:false})? else{return Ok(None)};
                match reply {
                    HistoryReply::Input{action:Some(HistoryAction::Confirm(accepted)),..}=>{self.services.system.gestures.question=false;GestureReply::Confirmed(accepted)},
                    HistoryReply::Input{action:Some(HistoryAction::ToggleConfirmation),..}=>GestureReply::ToggleConfirmation,
                    HistoryReply::Input{..}=>GestureReply::Holding{began:false},
                    _=>bail!("Circus native gesture question receipt"),
                }
            }
            GestureRequest::Poll{enabled,can_track,threshold,actions,overlay}=>{
                self.services.system.gestures.threshold=threshold;
                self.services.system.gestures.eligible=enabled&&can_track;
                if let Some(action)=self.services.system.gestures.action {return Ok(Some(SystemReply::Gesture(GestureReply::Action(action))))}
                self.sample_callback_releases()?;
                let input=self.host.input_observation(self.client)?;
                ensure!(input.client==self.client,"Circus foreign right gesture observation");
                let down=self.services.system.callback_buttons_down&buttons::MOUSE_RIGHT!=0;
                let now=self.host.now_ms();
                //Press timestamps came from the revision-locked Core buffer
                //before native frame admission, not this later gesture poll.
                if !enabled {return Ok(Some(SystemReply::Gesture(GestureReply::Idle)))}
                if !can_track||input.focused==Some(false) {
                    self.services.system.gestures.tracking=false;
                    //442EC0 owns the physical held clock independently of
                    //whether this VM process currently permits433680 tracking.
                    self.gesture_overlay(None,[0,0])?;
                    return Ok(Some(SystemReply::Gesture(GestureReply::Idle)))
                }
                let elapsed=self.services.system.gestures.down_since.map(|start|(now as u32).wrapping_sub(start as u32) as i32).unwrap_or(0);
                let long=down&&elapsed>200&&elapsed>threshold.wrapping_mul(300);
                let tracking=self.services.system.gestures.tracking;
                let point=input.pointer;
                if long {
                    let began=!tracking;
                    if began {
                        if let Some(group)=overlay.as_ref() {
                            let point=point.context("Circus native gesture pointer unavailable")?;
                            self.gesture_overlay(Some(group),[point[0].wrapping_sub(90),point[1].wrapping_sub(103)])?;
                            self.services.system.gestures.origin=Some(point);
                        }
                        self.services.system.gestures.tracking=true;
                    }
                    self.services.system.gestures.revision=Some(input.revision);
                    GestureReply::Holding{began}
                }else if tracking {
                    let origin=self.services.system.gestures.origin.context("Circus native gesture origin has no Group7 initialization provenance")?;
                    let point=point.context("Circus native gesture release pointer unavailable")?;
                    let dx=origin[0].wrapping_sub(point[0]);let dy=origin[1].wrapping_sub(point[1]);
                    //433680 tests down, left, up, right in that order; both
                    //the64px threshold and100px orthogonal bound are strict.
                    let action=if dy< -64&&dx.wrapping_abs()<100{actions[0]}
                        else if dx>64&&dy.wrapping_abs()<100{actions[1]}
                        else if dy>64&&dx.wrapping_abs()<100{actions[2]}
                        else if dx< -64&&dy.wrapping_abs()<100{actions[3]}else{-1};
                    self.services.system.gestures.tracking=false;
                    self.services.system.gestures.action=Some(action);
                    if !down{self.services.system.gestures.down_since=None;}
                    self.gesture_overlay(None,[0,0])?;
                    self.services.system.callback_releases&=!buttons::MOUSE_RIGHT;
                    GestureReply::Action(action)
                }else{
                    if !down{self.services.system.gestures.down_since=None;}
                    GestureReply::Idle
                }
            }
        };
        Ok(Some(SystemReply::Gesture(reply)))
    }
}
