// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!662640/6285C0 SOUND resource and transport consumer. Only original lookup,
//!channel/preference/flag policy lives here; all decoding/mixing/PCM is shared.
use crate::{native_class_state::State,native_sound_policy::{SoundPolicy,SoundSlot,sound_slot_plan},fes_object_bank::Bank};
use anyhow::{ensure,Context,Result};
use crysirror_core::{path::VfsPath,service::{AudioAsset,AudioParameters,AudioPreparePhase,AudioRequest,AudioResourceHandle,AudioResourceInfo,ClientStorage,HostServices,InputObservation,PcmBus},state::ClientId};
use crysirror_tasks::{Task,TaskPool};
use std::collections::BTreeMap;
#[derive(Clone)]struct Slot {owner:u32,object:u32,path:String,loop_key:String,resource:AudioResourceHandle,info:AudioResourceInfo,grouped:bool,percent:u32,pan:i32}
pub(crate) struct Located{pub path:VfsPath,pub loop_key:String}
struct Load {id:u32,channel:u32,worker:Option<Task<Located>>,path:Option<Located>,ticket:Option<u64>,ready:Option<(AudioResourceHandle,AudioResourceInfo)>}
struct Frame {pc:u32,observation:InputObservation,ids:Vec<u32>,next:usize,load:Option<Load>,play:Option<(u32,u64)>,controls:bool}
#[derive(Default)]pub(crate) struct Sounds {slots:BTreeMap<u32,Slot>,frames:BTreeMap<u32,Frame>,fades:crate::native_sound_fades::Fades,last_tick:Option<u32>}
pub(crate) fn resolve(storage:&dyn ClientStorage,name:&[u8],check:&dyn Fn()->Result<()>)->Result<Located>{
 check()?;
 let startup=crate::native_settings::optional_asset(storage,"config/startup.xml")?.context("CatSystem2 sound startup missing")?;
 let catalog=crate::native_resource_paths::Catalog::parse(&startup)?;
 let paths=if crate::loop_policy::SoundPaths::has_config(&startup)?{
  let config=crate::loop_policy::SoundPaths::config_name(&startup)?;
  //SOUNDCONFIG can itself be stored in the normal config namespace.
  let mut sound=None;
  for path in catalog.candidates(storage,"config",config.rsplit('/').next().unwrap_or(""),check)? {
   if let Some(path)=crate::native_resource_paths::resolve(storage,&path)?{sound=Some(crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(),1024*1024)?);break;}
  }
  crate::loop_policy::SoundPaths::parse(&startup,&sound.context("CatSystem2 sound prefix configuration missing")?)?
 }else{crate::loop_policy::SoundPaths::legacy(&startup)?};
 let logical=paths.resolve(std::str::from_utf8(name)?)?;
 for path in catalog.candidates(storage,"sound",&logical,check)?{
  check()?;if let Some(path)=crate::native_resource_paths::resolve(storage,&path)?{return Ok(Located{path,loop_key:logical.to_ascii_lowercase()});}
 }
 anyhow::bail!("CatSystem2 native SOUND resource unavailable: {}",String::from_utf8_lossy(name))
}
impl Sounds {
 pub fn diagnostics(&self)->serde_json::Value{
  serde_json::json!({"representation":"engine_audio_resources_and_original_channel_policy","slots":self.slots.iter().map(|(channel,slot)|serde_json::json!({"channel":channel,"owner":slot.owner,"object":slot.object,"asset":slot.path,"loop_key":slot.loop_key,"resource_client":slot.resource.client,"resource_id":slot.resource.id,"source_rate":slot.info.source_rate,"source_frames":slot.info.source_frames,"grouped":slot.grouped,"release_pending":self.fades.releasing(*channel)})).collect::<Vec<_>>(),"native_control_updates_pending":self.needs_updates()})
 }
 #[cfg(test)]pub fn resource(&self,channel:u32)->Option<AudioResourceHandle>{self.slots.get(&channel).map(|slot|slot.resource)}
 pub fn needs_updates(&self)->bool{self.fades.active()}
 fn retire_slot(&mut self,host:&mut dyn HostServices,client:ClientId,channel:u32)->Result<()>{
  if let Some(slot)=self.slots.get(&channel){
   ensure!(slot.resource.client==client.0,"CatSystem2 foreign native sound slot");
   host.submit_audio(client,AudioRequest::Stop{channel,fade_ms:0.})?;
   host.release_audio_resource(client,slot.resource)?;
   self.slots.remove(&channel);self.fades.cancel(channel);
  }Ok(())
 }
 fn action(&mut self,host:&mut dyn HostServices,client:ClientId,channel:u32,action:crate::native_sound_fades::Action)->Result<()>{
  let Some(slot)=self.slots.get(&channel)else{return Ok(());};
  if slot.grouped {
   self.fades.start(channel,slot.percent,self.last_tick.context("CatSystem2 sound control has no owner clock")?,action);
  }else{match action{
   crate::native_sound_fades::Action::Release=>self.retire_slot(host,client,channel)?,
   crate::native_sound_fades::Action::Stop=>host.submit_audio(client,AudioRequest::Stop{channel,fade_ms:0.})?,
   crate::native_sound_fades::Action::Pause=>{if host.audio_info(client,channel)?.is_some(){host.submit_audio(client,AudioRequest::Pause{channel,paused:true,fade_ms:0.})?;}},
  }}Ok(())
 }
 fn complete_actions(&mut self,host:&mut dyn HostServices,client:ClientId)->Result<()>{
  while let Some((channel,action))=self.fades.completed(){
   if action==crate::native_sound_fades::Action::Release{self.retire_slot(host,client,channel)?;}
   else if let Some(slot)=self.slots.get(&channel){if host.audio_info(client,channel)?.is_some(){host.submit_audio(client,AudioRequest::SetGain{channel,gain:crate::native_sound_fades::gain(slot.percent)?,fade_ms:0.})?;}}
   self.fades.acknowledge();
  }Ok(())
 }
 pub fn observe(&mut self,host:&mut dyn HostServices,client:ClientId,observation:InputObservation)->Result<()>{
  ensure!(observation.client==client,"CatSystem2 foreign sound-manager observation");
  self.last_tick=Some((u128::from(observation.time_ms)*60/1000)as u32);
  self.complete_actions(host,client)?;self.fades.advance(host,client,observation)?;self.complete_actions(host,client)
 }
 pub fn reap(&mut self,host:&mut dyn HostServices,client:ClientId,tasks:&crate::task_policy::NativeTaskPolicy)->Result<()>{
  let retired=self.slots.iter().filter(|(_,slot)|tasks.fes(slot.owner).is_err()).map(|(id,_)|*id).collect::<Vec<_>>();
  for channel in retired{self.action(host,client,channel,crate::native_sound_fades::Action::Release)?;}Ok(())
 }
 pub fn advance(&mut self,host:&mut dyn HostServices,client:ClientId,pool:&TaskPool,owner:u32,pc:u32,observation:InputObservation,bank:&mut Bank,policy:Option<&SoundPolicy>,strings:&crate::native_values::StringVariables)->Result<bool>{
  ensure!(observation.client==client&&bank.owner==owner,"CatSystem2 SOUND frame owner mismatch");
  self.observe(host,client,observation)?;
  let ids=bank.ordered_ids().into_iter().filter(|id|matches!(bank.objects[id].state,State::Sound(_))).collect::<Vec<_>>();
  if ids.is_empty(){return Ok(true);}
  self.frames.entry(owner).or_insert_with(||Frame{pc,observation,ids,next:0,load:None,play:None,controls:false});
  {
   let frame=&self.frames[&owner];ensure!(frame.pc==pc&&frame.observation==observation,"CatSystem2 SOUND frame original PC/input changed");
  }
  loop{
   let Some(id)=self.frames[&owner].ids.get(self.frames[&owner].next).copied()else{self.frames.remove(&owner);return Ok(true);};
   let object=bank.objects.get_mut(&id).context("CatSystem2 SOUND frame object lost")?;
   let State::Sound(sound)=&mut object.state else{anyhow::bail!("CatSystem2 SOUND class changed while pending")};
   let policy=if sound.filename.is_empty(){policy}else{Some(policy.context("CatSystem2 extended SOUND preferences unavailable")?)};
   ensure!(sound.crossfade[0]==0,"CatSystem2 paired/crossfade SOUND resource lifecycle requires original dual-slot publication");
   if sound.load_dirty {
    if sound.filename.is_empty(){
     if sound.channel>=0{self.action(host,client,sound.channel as u32,crate::native_sound_fades::Action::Release)?;}
     sound.load_dirty=false;
    }else if !sound.group.is_empty()&&policy.unwrap().sound_group(&sound.group).is_none(){
     //5EC038 case 205 leaves the slot at -1 for a group soundconf.xml never
     //declared; 200's channel check then ignores every later transport call.
     if sound.channel>=0{self.action(host,client,sound.channel as u32,crate::native_sound_fades::Action::Release)?;}
     sound.channel=-1;
     sound.load_dirty=false;
    }else{
     if self.frames[&owner].load.is_none(){
      let permit=match pool.reserve(){Ok(p)=>p,Err(e)if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(false),Err(e)=>return Err(e)};
      if sound.channel>=0{self.action(host,client,sound.channel as u32,crate::native_sound_fades::Action::Release)?;}
      let channel=if sound.group.is_empty(){u32::try_from(sound.channel).context("CatSystem2 ungrouped SOUND has no original channel")?}else{
       let group=policy.unwrap().sound_group(&sound.group).context("CatSystem2 SOUNDGROUP name missing")?;
       let mut slots=[SoundSlot::default();64];for (channel,slot)in &self.slots{ensure!(*channel<64,"CatSystem2 native SOUND channel extent");slots[*channel as usize].resource=Some(slot.resource);slots[*channel as usize].releasing=self.fades.releasing(*channel);}
       sound_slot_plan(client,group.minimum,group.maximum,&slots)?.channel
      };
      ensure!(channel<64,"CatSystem2 native SOUND channel64 failed-load profile");
      let name=strings.expand(&sound.filename,259)?;let storage=host.storage(client)?;
      let worker=permit.spawn(move|cancel|resolve(storage.as_ref(),&name,&||cancel.check()))?;
      self.frames.get_mut(&owner).unwrap().load=Some(Load{id,channel,worker:Some(worker),path:None,ticket:None,ready:None});
      return Ok(false);
     }
     let load=self.frames.get_mut(&owner).unwrap().load.as_mut().unwrap();ensure!(load.id==id,"CatSystem2 SOUND pending resource target changed");
     if load.path.is_none(){let Some(path)=load.worker.as_mut().unwrap().try_take()?else{return Ok(false);};load.path=Some(path);load.worker=None;}
     if load.ticket.is_none(){load.ticket=Some(host.begin_audio_resource(client,AudioAsset{path:load.path.as_ref().unwrap().path.clone(),frame_limit:None},false)?);return Ok(false);}
     if load.ready.is_none(){
      let result=host.audio_resource_preparation(client,load.ticket.unwrap())?;
      match result.phase{AudioPreparePhase::Pending=>return Ok(false),AudioPreparePhase::Failed=>anyhow::bail!("CatSystem2 shared SOUND resource: {}",result.error.unwrap_or_default()),AudioPreparePhase::Ready=>{
       let resource=result.resource.context("CatSystem2 SOUND preparation no resource")?;
       ensure!(resource.client==client.0,"CatSystem2 foreign SOUND resource completion");
       let info=result.info.context("CatSystem2 SOUND preparation no metadata")?;
       ensure!(!info.has_replay_gain,"CatSystem2 ReplayGain-tagged SOUND requires reviewed original decoder metadata policy");
       load.ready=Some((resource,info));
      }}
     }
     let (channel,path,loop_key,resource,info,ticket)={let load=self.frames[&owner].load.as_ref().unwrap();let (resource,info)=load.ready.unwrap();(load.channel,load.path.as_ref().unwrap().path.relative().to_string(),load.path.as_ref().unwrap().loop_key.clone(),resource,info,load.ticket.unwrap())};
     self.retire_slot(host,client,channel)?;
     //Resource preparation receipt release does not retire its published lease.
     host.release_audio_resource_preparation(client,ticket)?;
     self.slots.insert(channel,Slot{owner,object:id,path,loop_key,resource,info,grouped:!sound.group.is_empty(),percent:u32::MAX,pan:i32::MIN});
     sound.channel=channel as i32;sound.load_dirty=false;
     self.frames.get_mut(&owner).unwrap().load=None;
    }
   }
   let channel=u32::try_from(sound.channel).ok();
   let slot=channel.filter(|_|!sound.filename.is_empty()).and_then(|channel|self.slots.get(&channel)).cloned();
   if let (Some(channel),Some(slot))=(channel,slot){
    ensure!(slot.resource.client==client.0,"CatSystem2 foreign SOUND channel");
    let policy=policy.context("CatSystem2 SOUND preferences missing")?;
    let allowed=policy.permits_play(&sound.filename,sound.subtype!=0,sound.passes,sound.force_play)?;
    let percent=policy.playback_percent(&sound.filename,&sound.group,sound.kind!=0,sound.pose.volume)?;
    let gain=crate::native_sound_fades::gain(percent)?;
    if !self.frames[&owner].controls{
     if (slot.percent!=percent||slot.pan!=sound.pose.pan)&&host.audio_info(client,channel)?.is_some() {
      host.submit_audio(client,AudioRequest::SetGain{channel,gain,fade_ms:0.})?;
      host.submit_audio(client,crate::native_sound_policy::non_spatial_pan_request(channel,sound.pose.pan,-10000,0)?)?;
     }
     {let slot=self.slots.get_mut(&channel).unwrap();slot.percent=percent;slot.pan=sound.pose.pan;}
     if sound.pause_dirty&&sound.playing&&allowed&&object.enabled{
      self.action(host,client,channel,crate::native_sound_fades::Action::Pause)?;
     }
     sound.pause_dirty=false;
     if sound.play_dirty {
      if sound.playing&&allowed&&object.enabled {
       ensure!(!self.fades.releasing(channel),"CatSystem2 play on a releasing original sound slot requires original paired ownership policy");
       self.fades.cancel(channel);
       let loops=crate::native_sound_loops::lookup(host,client,&slot.loop_key,slot.info.source_rate,sound.passes)?;
       let parameters=AudioParameters{gain,looping:loops.looping,loop_passes:loops.passes,loop_start:loops.start,loop_end:loops.end,bus:if sound.group==b"bgm"{PcmBus::Music}else if sound.group==b"voice"{PcmBus::Voice}else{PcmBus::Sfx},..Default::default()};
       //The original count1 is one-shot/full range;0 means unbounded and
       //other DWORD counts include the current pass, using explicit LoopPoint.
       let ticket=host.begin_audio(client,AudioRequest::PlayResource{channel,resource:slot.resource,parameters})?;
       self.frames.get_mut(&owner).unwrap().play=Some((id,ticket));
      }else if sound.stop_requested||sound.playing{
       self.action(host,client,channel,crate::native_sound_fades::Action::Stop)?;
      }
      if sound.playing{sound.first_play_ms.get_or_insert(observation.time_ms as u32);}
     }
     self.frames.get_mut(&owner).unwrap().controls=true;
    }
    if let Some((target,ticket))=self.frames[&owner].play{
     ensure!(target==id,"CatSystem2 SOUND playback completion target changed");
     let result=host.audio_preparation(client,ticket)?;ensure!(result.channel==channel,"CatSystem2 foreign SOUND playback completion");
     match result.phase{AudioPreparePhase::Pending=>return Ok(false),AudioPreparePhase::Failed=>anyhow::bail!("CatSystem2 shared SOUND playback: {}",result.error.unwrap_or_default()),AudioPreparePhase::Ready=>{
      host.release_audio(client,ticket)?;self.frames.get_mut(&owner).unwrap().play=None;
      //Play replaces the voice, so independent stereo attenuation is applied
      //to the newly admitted voice, never only to its retired predecessor.
      host.submit_audio(client,crate::native_sound_policy::non_spatial_pan_request(channel,sound.pose.pan,-10000,0)?)?;
     }}
    }
    if sound.loop_dirty&&sound.playing&&host.audio_info(client,channel)?.is_some(){
     let loops=crate::native_sound_loops::lookup(host,client,&slot.loop_key,slot.info.source_rate,sound.passes)?;
     if loops.looping{host.submit_audio(client,AudioRequest::SetLoopSchedule{channel,start:loops.start,end:loops.end,passes:loops.passes})?;}
     else{host.submit_audio(client,AudioRequest::SetLooped{channel,looping:false,loop_start:None})?;}
    }
    sound.loop_dirty=false;sound.play_dirty=false;
    //6285C0 observes actual source status, not a successful preparation ticket.
    if sound.playing&&allowed&&!host.audio_playing(client,channel)?{sound.playing=false;sound.stop_requested=true;sound.play_dirty=true;}
   }else if sound.playing {
    let active=if let Some(channel)=channel{host.audio_playing(client,channel)?}else{false};
    if !active{sound.playing=false;sound.stop_requested=true;sound.play_dirty=true;}
   }
   let frame=self.frames.get_mut(&owner).unwrap();frame.next+=1;frame.controls=false;
  }
 }
}
