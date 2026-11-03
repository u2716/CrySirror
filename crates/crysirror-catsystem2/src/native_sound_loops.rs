// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original5EAEA0 cases182/218 ->5D17A0/5D1E90 ->5D1955/5D1A9C.
//! The native word is total passes:1 once,0 unlimited,other DWORDs decrement at
//! each completed pass. Shared Audio owns the actual cursor/range/count loops.
use anyhow::Result;
use crysirror_core::{service::{AudioLoopRange,AudioLoopTimebase,HostServices},state::ClientId};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub(crate) struct Loops{pub looping:bool,pub passes:Option<u32>,pub start:u64,pub end:Option<u64>}
pub(crate) fn project(count:u32,rate:u32,range:Option<AudioLoopRange>)->Result<Loops>{
 if count==1{return Ok(Loops{looping:false,passes:None,start:0,end:None});}
 let range=if let Some(range)=range{
  //5D17A0 divides original44100 coordinates only for22050, and leaves all
  //other source rates unchanged. This is explicit native policy, not a new
  //automatic resampling behavior in the shared loop catalog.
  let rate=match range.timebase{AudioLoopTimebase::ReferenceFrames{sample_rate:44100}=>if rate==22050{22050}else{44100},_=>rate};
  Some(crysirror_audio::loop_catalog::resolve_loop_frames(range,rate,crysirror_audio::loop_catalog::LoopFrameRounding::Floor)?)
 }else{None};
 Ok(Loops{looping:true,passes:(count!=0).then_some(count),start:range.map_or(0,|r|r.start_frame),end:range.and_then(|r|r.end_frame)})
}
pub(crate) fn lookup(host:&dyn HostServices,client:ClientId,asset:&str,rate:u32,count:u32)->Result<Loops>{
 let range=if count==1{None}else{host.audio_loop_metadata(client,asset)?.map(|m|m.range)};
 project(count,rate,range)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn original_counter_and_single_pass_range_override_are_exact()->Result<()>{
  let range=Some(AudioLoopRange{timebase:AudioLoopTimebase::ReferenceFrames{sample_rate:44100},start_frame:101,end_frame:Some(301)});
  assert_eq!(project(1,22050,range)?,Loops{looping:false,passes:None,start:0,end:None});
  assert_eq!(project(0,22050,range)?,Loops{looping:true,passes:None,start:50,end:Some(150)});
  assert_eq!(project(2,44100,range)?,Loops{looping:true,passes:Some(2),start:101,end:Some(301)});
  assert_eq!(project(2,48000,range)?.start,101,"original only halves22050; no implicit48000 conversion");
  assert_eq!(project(u32::MAX,44100,None)?.passes,Some(u32::MAX));Ok(())
 }
}
