// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original76BAF0/76C000/770200/7702F0/7792A0/779A80/779AC0
//! SOUND argument and flag policy. No decoder/mixer or playback success here.
use crate::{fes_object_bank::Bank,native_class_state::{SoundState,State},native_interpreter::Interpreter,native_values::StringVariables};
use anyhow::{ensure,Context,Result};
pub(crate) fn supports(method:&[u8])->bool{
 matches!(method,b"load"|b"play"|b"channel"|b"stype"|b"loop"|b"pause"|b"crossfade")
}
fn word(vm:&mut Interpreter,args:&[Vec<u8>],at:usize,default:u32)->Result<u32>{
 args.get(at).map_or(Ok(default),|value|vm.evaluate(value))
}
fn mutate(sound:&mut SoundState,method:&[u8],args:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables)->Result<()> {
 match method {
  b"load"=>{
   let filename=if let Some(name)=args.first(){strings.expand(name,259)?}else{Vec::new()};
   ensure!(filename.len()<260&&!filename.contains(&0),"CatSystem2 SOUND load filename extent");
   //The second numeric field is consumed even though SOUND does not use its
   //IMAGE frame value. Crossfade words follow it; default duration is60.
   let _unused_frame=word(vm,args,1,0)?;
   let crossfade=word(vm,args,2,0)?;
   let duration=word(vm,args,3,60)?;
   sound.filename=filename;sound.crossfade=[crossfade,duration];sound.load_dirty=true;
  },
  b"play"=>{
   sound.playing=true;sound.paused=false;sound.stop_requested=false;sound.play_dirty=true;
   if let Some(value)=args.first(){let value=vm.evaluate(value)?;if sound.passes!=value{sound.passes=value;sound.loop_dirty=true;}}
   sound.play_style=Some(word(vm,args,1,1)?);
   //The original evaluates this expression and passes it to74C710. It is not
   //an invented playback priority or loop count.
   sound.force_play=word(vm,args,2,0)?;
  },
  b"channel"=>sound.channel=word(vm,args,0,0)? as i32,
  b"stype"=>sound.subtype=word(vm,args,0,0)?, //word78: normal vs system-play preference
  b"loop"=>if let Some(value)=args.first(){let value=vm.evaluate(value)?;if sound.passes!=value{sound.passes=value;sound.loop_dirty=true;}},
  b"pause"=>{sound.paused=true;sound.pause_dirty=true;},
  b"crossfade"=>sound.crossfade=[word(vm,args,0,0)?,word(vm,args,1,0)?],
  _=>anyhow::bail!("CatSystem2 unknown SOUND method"),
 }Ok(())
}
pub(crate) fn apply(bank:&mut Bank,target:&[u8],arguments:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables)->Result<()> {
 let method=arguments.first().context("CatSystem2 SOUND method missing")?;
 ensure!(supports(method),"CatSystem2 unreviewed SOUND command");
 let mut staged=vm.clone();
 let reference=crate::fes_references::evaluate(target,&mut staged,strings,&|name|bank.count(name))?;
 //These three exact natives return immediately for the all-array sentinel,
 //before consuming parameters; do not manufacture calls to every member.
 if reference.index==-1&&matches!(method.as_slice(),b"play"|b"loop"|b"pause"){
  *vm=staged;return Ok(());
 }
 let count=bank.count(&reference.name).context("CatSystem2 SOUND target name missing")?;
 let ids=if reference.index==-1{(0..count).map(|index|bank.lookup(&crate::fes_references::IndexedName{name:reference.name.clone(),index:index as i32}).context("CatSystem2 SOUND array member missing")).collect::<Result<Vec<_>>>()?}
 else{vec![bank.lookup(&reference).context("CatSystem2 SOUND target outside array")?]};
 let mut updated=Vec::with_capacity(ids.len());
 //channel/stype/crossfade parse their values before the native member loop.
 //load consumes a fresh parameter sequence per member, as76BAF0 does.
 let mut once:Option<[u32;2]>=None;
 for id in ids {
  let State::Sound(original)=&bank.objects[&id].state else{anyhow::bail!("CatSystem2 SOUND command on unsupported native class")};
  let mut sound=original.clone();
  if matches!(method.as_slice(),b"channel"|b"stype"|b"crossfade") {
   if let Some(values)=once {match method.as_slice(){b"channel"=>sound.channel=values[0]as i32,b"stype"=>sound.subtype=values[0],_=>sound.crossfade=values}}
   else{mutate(&mut sound,method,&arguments[1..],&mut staged,strings)?;once=Some(match method.as_slice(){b"channel"=>[sound.channel as u32,0],b"stype"=>[sound.subtype,0],_=>sound.crossfade});}
  }else{mutate(&mut sound,method,&arguments[1..],&mut staged,strings)?;}
  updated.push((id,State::Sound(sound)));
 }
 for(id,state)in updated{bank.objects.get_mut(&id).unwrap().state=state;}
 *vm=staged;Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 fn words(args:&[&[u8]])->Vec<Vec<u8>>{args.iter().map(|s|s.to_vec()).collect()}
 #[test]fn load_play_loop_pause_crossfade_keep_native_flags_and_expression_side_effects()->Result<()>{
  let mut sound=SoundState::default();let mut vm=Interpreter::new(1024)?;let mut strings=StringVariables::default();strings.assign(9,b"voice_01".to_vec())?;
  mutate(&mut sound,b"load",&words(&[b"$str9",b"++$1",b"++$2"]),&mut vm,&strings)?;
  assert_eq!(sound.filename,b"voice_01");assert_eq!(sound.crossfade,[1,60]);assert!(sound.load_dirty);assert_eq!(vm.flag(1),1);assert!(!sound.playing);
  mutate(&mut sound,b"play",&words(&[b"1",b"3",b"++$2",b"++$3"]),&mut vm,&strings)?;
  assert!(sound.playing&&sound.play_dirty&&sound.loop_dirty);assert_eq!(sound.play_style,Some(3));assert_eq!(sound.force_play,2);assert_eq!(vm.flag(3),0,"extra arguments ignored");
  mutate(&mut sound,b"pause",&words(&[b"++$3"]),&mut vm,&strings)?;
  assert!(sound.paused&&sound.pause_dirty&&sound.playing);assert_eq!(vm.flag(3),0);
  mutate(&mut sound,b"play",&[],&mut vm,&strings)?;assert!(!sound.paused);assert_eq!(sound.play_style,Some(1));assert_eq!(sound.passes,1,"missing loop keeps native previous mode");
  mutate(&mut sound,b"crossfade",&words(&[b"2"]),&mut vm,&strings)?;assert_eq!(sound.crossfade,[2,0]);
  mutate(&mut sound,b"loop",&words(&[b"0"]),&mut vm,&strings)?;assert_eq!(sound.passes,0);Ok(())
 }
}
