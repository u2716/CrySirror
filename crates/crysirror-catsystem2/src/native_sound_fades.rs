// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!5EAEA0/5EA3F0 original native duration/action policy. Scalar recurrence and
//!gain conversion/transport are shared Scene/Audio APIs, with no PCM processing.
use anyhow::{ensure,Result};
use crysirror_core::{service::{AudioRequest,HostServices,InputObservation},state::ClientId};
use std::collections::BTreeMap;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub(crate) enum Action {Pause,Stop,Release}
#[derive(Clone,Copy,Debug)]struct Fade {current:f32,remaining:f32,last_tick:u32,action:Action}
struct Phase {observation:InputObservation,ids:Vec<u32>,next:usize}
#[derive(Default)]pub(crate) struct Fades {active:BTreeMap<u32,Fade>,phase:Option<Phase>,last_revision:Option<u64>,completed:Vec<(u32,Action)>}
pub(crate) fn gain(percent:u32)->Result<f32>{
 let db=crate::native_sound_policy::device_attenuation(percent,100,-10000,0)?;
 Ok(crysirror_audio::levels::gain_from_legacy_db100(db)?)
}
impl Fades {
 pub fn active(&self)->bool{!self.active.is_empty()||self.phase.is_some()||!self.completed.is_empty()}
 pub fn completed(&self)->Option<(u32,Action)>{self.completed.last().copied()}
 pub fn acknowledge(&mut self){self.completed.pop();}
 pub fn releasing(&self,channel:u32)->bool{self.active.get(&channel).is_some_and(|fade|fade.action==Action::Release)}
 pub fn cancel(&mut self,channel:u32){self.active.remove(&channel);}
 pub fn start(&mut self,channel:u32,percent:u32,tick:u32,action:Action){
  //181 repeated on an already releasing slot does not reset its deadline.
  if action==Action::Release&&self.releasing(channel){return;}
  self.active.insert(channel,Fade{current:percent as f32,remaining:10.,last_tick:tick,action});
 }
 ///Return actual due actions; caller retires its resource authority only after
 ///the shared transport action succeeds. Partial retries never repeat a step.
 pub fn advance(&mut self,host:&mut dyn HostServices,client:ClientId,observation:InputObservation)->Result<()>{
  ensure!(observation.client==client,"CatSystem2 native sound fade foreign observation");
  if self.last_revision==Some(observation.revision){return Ok(());}
  if let Some(phase)=&self.phase{ensure!(phase.observation==observation,"CatSystem2 sound fade observation changed while pending");}
  else{self.phase=Some(Phase{observation,ids:self.active.keys().copied().collect(),next:0});}
  let tick=(u128::from(observation.time_ms)*60/1000)as u32;
  loop {
   let phase=self.phase.as_mut().unwrap();
   let Some(channel)=phase.ids.get(phase.next).copied()else{self.last_revision=Some(observation.revision);self.phase=None;return Ok(());};
   if let Some(fade)=self.active.get(&channel).copied(){
    if fade.remaining<=0.{
     //Target arrival was the PREVIOUS update. Do not collapse this into the
     //sample that reached zero or manufacture an action from declared duration.
     match fade.action{Action::Pause=>{if host.audio_info(client,channel)?.is_some(){host.submit_audio(client,AudioRequest::Pause{channel,paused:true,fade_ms:0.})?;}},Action::Stop|Action::Release=>host.submit_audio(client,AudioRequest::Stop{channel,fade_ms:0.})?,}
     self.active.remove(&channel);self.completed.push((channel,fade.action));
    }else{
     let elapsed=tick.wrapping_sub(fade.last_tick)as f32;
     let (current,remaining)=crysirror_scene::animation::step_remaining_time_scalar(fade.current,0.,fade.remaining,elapsed)?;
     if host.audio_info(client,channel)?.is_some(){host.submit_audio(client,AudioRequest::SetGain{channel,gain:gain(current as u32)?,fade_ms:0.})?;}
     self.active.insert(channel,Fade{current,remaining,last_tick:if remaining>0.{tick}else{fade.last_tick},action:fade.action});
    }
   }
   self.phase.as_mut().unwrap().next+=1;
  }
 }
}
#[cfg(test)]mod tests{
 use super::*;
 use crysirror_core::service::{AudioParameters,PcmAudio};
 fn input(revision:u64,time_ms:u64)->InputObservation{InputObservation{client:ClientId(1),revision,time_ms,buttons_down:0,buttons_pressed:0,pointer:None,pointer_inside:None,focused:None}}
 #[test]fn original_sound_remaining_updates_reach_real_pcm_then_pause_on_following_owner_update()->Result<()>{
  let root=tempfile::tempdir()?;
  let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native sound remaining recurrence","content_revision":"1","assets":"game","canvas":[2,2],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"fixture"}]}))?;
  let mut runtime=crysirror_runtime::Runtime::with_asset_root(project,root.path(),&root.path().join("state"),Default::default())?;
  let client=ClientId(1);runtime.host.submit_audio(client,AudioRequest::PlayPcm{channel:7,source:PcmAudio::new(48000,vec![[0.5;2];1024])?,parameters:AudioParameters{looping:true,..Default::default()}})?;
  let mut fades=Fades::default();fades.start(7,100,60,Action::Pause);
  fades.advance(&mut runtime.host,client,input(1,1017))?;
  let gain=runtime.host.audio_info(client,7)?.unwrap().gain;
  assert!((gain-crysirror_audio::levels::gain_from_legacy_db100(-184)?).abs()<1e-7,"first original step is88percent, not90 or amplitude0.9");
  runtime.host.channel_audio.render(1)?;let pcm=runtime.host.channel_audio.take_output();assert_eq!(pcm.len(),48);for p in pcm{assert!((p[0]-0.5*gain).abs()<1e-6);}
  let state=fades.active[&7];fades.advance(&mut runtime.host,client,input(1,1017))?;assert_eq!(fades.active[&7].current,state.current,"same owner revision cannot advance twice");
  //Elapsed9 now reaches target, but the native terminal transport action must
  //wait for another update even though the whole duration has elapsed.
  fades.advance(&mut runtime.host,client,input(2,1167))?;
  assert_eq!(runtime.host.audio_info(client,7)?.unwrap().gain,0.);assert!(!runtime.host.audio_info(client,7)?.unwrap().paused);assert!(fades.completed().is_none());
  fades.advance(&mut runtime.host,client,input(3,1184))?;
  assert!(runtime.host.audio_info(client,7)?.unwrap().paused);assert_eq!(fades.completed(),Some((7,Action::Pause)));
  fades.acknowledge();assert!(!fades.active());
  assert!(fades.advance(&mut runtime.host,ClientId(2),input(4,1200)).is_err());
  Ok(())
 }
 #[test]fn release_repetition_does_not_reset_deadline_and_cancel_does_not_fake_completion(){
  let mut fades=Fades::default();fades.start(2,50,10,Action::Release);fades.start(2,100,12,Action::Release);
  assert_eq!(fades.active[&2].last_tick,10);assert_eq!(fades.active[&2].current,50.);assert!(fades.releasing(2));
  fades.cancel(2);assert!(!fades.active());assert!(fades.completed().is_none());
 }
}
