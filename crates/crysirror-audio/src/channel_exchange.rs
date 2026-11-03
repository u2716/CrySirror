// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Atomic client-scoped logical channel exchange. No decoder is reopened, no
//! frame is consumed and no live voice/DSP/automation record is recreated.
use crate::channels::ChannelAudio;
use anyhow::{ensure,Result};
impl ChannelAudio {
    /// Swap allocated voices, moving one if the other channel is empty. All
    /// dependencies are inspected without polling workers or mutating actors.
    /// Ready receipts remain valid observations of their earlier publication.
    pub fn exchange_channels(&mut self,client:u32,left:u32,right:u32)->Result<()> {
        ensure!(client!=0,"audio channel exchange client zero is reserved");
        self.mixer.idle()?;
        ensure!(!self.jobs.exchange_reserved(client,left,right),"audio channel exchange has unpublished preparation intent");
        ensure!(!self.seeks.exchange_reserved(client,left,right),"audio channel exchange has pending seek");
        ensure!(!self.releases.exchange_reserved(client,left,right),"audio channel exchange has pending or active retirement");
        let a=self.voices.get(&(client,left)).copied();
        let b=self.voices.get(&(client,right)).copied();
        ensure!(a.is_none_or(|h|h.client==client)&&b.is_none_or(|h|h.client==client),"audio channel exchange foreign voice ownership");
        if left==right{return Ok(());}
        self.voices.remove(&(client,left));self.voices.remove(&(client,right));
        if let Some(voice)=b{self.voices.insert((client,left),voice);}
        if let Some(voice)=a{self.voices.insert((client,right),voice);}
        Ok(())
    }
}
#[cfg(test)]mod tests{
 use super::*;
 use crate::{channels::prepare,decoder::{AudioSource,StereoFrame}};
 use crysirror_core::{service::{AudioAsset,AudioParameters,AudioRequest,AudioReleaseSpec,ClientStorage,EncodedAudio},path::{Mount,VfsPath},Error};
 use std::sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}};
 struct Samples{at:u64,seeks:Arc<AtomicUsize>,ready:Option<Arc<AtomicBool>>}
 impl AudioSource for Samples{
  fn sample_rate(&self)->u32{48000}fn frame_count(&self)->Option<u64>{Some(10000)}
  fn read(&mut self,out:&mut[StereoFrame])->Result<usize>{let n=out.len().min((10000-self.at)as usize);for p in &mut out[..n]{*p=[(self.at+1)as f32/10000.;2];self.at+=1;}Ok(n)}
  fn seek(&mut self,frame:u64)->Result<()>{self.seeks.fetch_add(1,Ordering::SeqCst);self.at=frame;Ok(())}
  fn is_nonblocking(&self)->bool{self.ready.is_some()}
  fn poll_seek(&mut self,frame:u64)->Result<bool>{self.seeks.fetch_add(1,Ordering::SeqCst);if self.ready.as_ref().is_none_or(|r|r.load(Ordering::SeqCst)){self.at=frame;Ok(true)}else{Ok(false)}}
 }
 fn play(a:&mut ChannelAudio,channel:u32,ready:Option<Arc<AtomicBool>>)->Result<Arc<AtomicUsize>>{
  let seeks=Arc::new(AtomicUsize::new(0));let p=prepare(Box::new(Samples{at:0,seeks:seeks.clone(),ready}),None,AudioParameters{rate:1.25,looping:true,..Default::default()})?;a.publish(7,channel,p)?;Ok(seeks)
 }
 fn render(a:&mut ChannelAudio,frames:usize)->Result<Vec<StereoFrame>>{let mut p=vec![[0.;2];frames];a.mixer.render(&mut p)?;Ok(p)}
 fn setup()->Result<(ChannelAudio,Vec<Arc<AtomicUsize>>)>{let mut a=ChannelAudio::default();let counts=vec![play(&mut a,3,None)?,play(&mut a,7,None)?];render(&mut a,7)?;
  a.submit(7,AudioRequest::SetGain{channel:3,gain:0.25,fade_ms:5.})?;a.submit(7,AudioRequest::Pause{channel:7,paused:true,fade_ms:0.})?;Ok((a,counts))}
 #[test]fn exact_handles_fractional_cursor_pause_gain_ramp_and_pcm_survive_exchange()->Result<()>{
  let(mut a,counts)=setup()?;let(mut reference,_)=setup()?;
  let ha=a.voices[&(7,3)];let hb=a.voices[&(7,7)];let ia=a.playback_info(7,3)?.unwrap();let ib=a.playback_info(7,7)?.unwrap();
  let reads=counts.iter().map(|x|x.load(Ordering::SeqCst)).collect::<Vec<_>>();
  a.submit(7,AudioRequest::ExchangeChannels{left:3,right:7})?;
  assert_eq!(a.voices[&(7,3)],hb);assert_eq!(a.voices[&(7,7)],ha);assert_eq!(a.playback_info(7,3)?,Some(ib));assert_eq!(a.playback_info(7,7)?,Some(ia));
  assert_eq!(counts.iter().map(|x|x.load(Ordering::SeqCst)).collect::<Vec<_>>(),reads,"exchange must not seek or reopen a source");
  assert_eq!(render(&mut a,17)?,render(&mut reference,17)?,"map exchange must not change running source/DSP/gain trajectory");
  a.exchange_channels(7,7,99)?;assert!(!a.voices.contains_key(&(7,7)));assert_eq!(a.voices[&(7,99)],ha);
  a.exchange_channels(8,3,99)?;assert_eq!(a.voices[&(7,99)],ha,"same numeric channel in another client cannot touch this voice");
  let before=a.voices.clone();assert!(a.exchange_channels(0,3,99).is_err());assert_eq!(a.voices,before);a.exchange_channels(7,99,99)?;assert_eq!(a.voices,before);Ok(())
 }
 struct NoFiles(Arc<AtomicUsize>);
 impl ClientStorage for NoFiles{
  fn remove(&self,_:&crysirror_core::path::VfsPath)->crysirror_core::Result<()>{Err(crysirror_core::Error::NotFound)}
  fn list(&self,_:crysirror_core::path::Mount,_:&str)->crysirror_core::Result<Vec<String>>{Err(crysirror_core::Error::NotFound)}
  fn read(&self,_:&VfsPath)->crysirror_core::Result<Vec<u8>>{self.0.fetch_add(1,Ordering::SeqCst);Err(Error::NotFound)}
  fn write_atomic(&self,_:&VfsPath,_:&[u8])->crysirror_core::Result<()>{Err(Error::ReadOnly)}
 }
 #[test]fn pending_preparation_is_rejected_without_polling_io_or_changing_intent()->Result<()>{
  let mut a=ChannelAudio::default();play(&mut a,3,None)?;let before=a.voices.clone();let reads=Arc::new(AtomicUsize::new(0));
  a.configure_preparation(crysirror_tasks::TaskPool::new("exchange-test",1,1)?,true);
  let ticket=a.begin_preparation(7,AudioRequest::PlayAsset{channel:3,source:AudioAsset{path:VfsPath::new(Mount::Asset,"missing.wav")?,frame_limit:None},intro:None,parameters:Default::default()},Arc::new(NoFiles(reads.clone())))?;
  assert!(a.exchange_channels(7,3,7).unwrap_err().to_string().contains("unpublished"));assert_eq!(a.voices,before);assert_eq!(reads.load(Ordering::SeqCst),0,"exchange is a read-only dependency check, not a worker poll");
  a.release_preparation(7,ticket)?;a.exchange_channels(7,3,7)?;assert_eq!(reads.load(Ordering::SeqCst),0);Ok(())
 }
 #[test]fn pending_seek_rejects_atomically_but_ready_receipt_does_not_pin_channel_name()->Result<()>{
  let mut a=ChannelAudio::default();let ready=Arc::new(AtomicBool::new(false));let polls=play(&mut a,3,Some(ready.clone()))?;
  let ticket=a.begin_channel_seek(7,3,50)?;let before=a.voices.clone();let count=polls.load(Ordering::SeqCst);
  assert!(a.exchange_channels(7,3,7).unwrap_err().to_string().contains("pending seek"));assert_eq!(a.voices,before);assert_eq!(polls.load(Ordering::SeqCst),count);
  ready.store(true,Ordering::SeqCst);assert_eq!(a.channel_seek_preparation(7,ticket)?.phase,crysirror_core::service::AudioPreparePhase::Ready);
  a.exchange_channels(7,3,7)?;assert_eq!(a.playback_info(7,7)?.unwrap().position_frames,50);
  assert_eq!(a.channel_seek_preparation(7,ticket)?.channel,3,"terminal receipt still describes its earlier publication");a.release_channel_seek(7,ticket)?;Ok(())
 }
 #[test]fn active_retirement_and_render_lease_reject_before_mapping_changes()->Result<()>{
  let mut a=ChannelAudio::default();play(&mut a,3,None)?;
  let release=a.begin_audio_release(7,AudioReleaseSpec{source_channel:3,tail_channel:7,threshold_db100:-3000,target_above_db100:-3000,target_below_db100:-10000,steps:30,pan:None,playback_rate:None,freeze_group_gain:true,release_tail_when_source_missing:false})?;
  let before=a.voices.clone();assert!(a.exchange_channels(7,7,9).unwrap_err().to_string().contains("retirement"));assert_eq!(a.voices,before);
  a.advance_audio_release(7,release,100)?;a.release_audio_release(7,release)?;
  play(&mut a,3,None)?;let before=a.voices.clone();let worker=a.mixer.lease_render(987)?;
  assert!(a.exchange_channels(7,3,9).is_err());assert_eq!(a.voices,before);assert!(a.mixer.finish_render(987,worker));a.exchange_channels(7,3,9)?;Ok(())
 }
 fn wave()->Vec<u8>{let mut b=b"RIFF".to_vec();b.extend(40u32.to_le_bytes());b.extend(b"WAVEfmt ");b.extend(16u32.to_le_bytes());b.extend(1u16.to_le_bytes());b.extend(1u16.to_le_bytes());b.extend(48000u32.to_le_bytes());b.extend(96000u32.to_le_bytes());b.extend(2u16.to_le_bytes());b.extend(16u16.to_le_bytes());b.extend(b"data");b.extend(4u32.to_le_bytes());b.extend([0,64,0,64]);b}
 #[test]fn ready_preparation_receipt_survives_voice_exchange_without_republication()->Result<()>{
  let mut a=ChannelAudio::default();let ticket=a.begin_preparation(7,AudioRequest::Play{channel:3,source:EncodedAudio{bytes:wave().into(),frame_limit:None},intro:None,parameters:AudioParameters{looping:true,..Default::default()}},Arc::new(NoFiles(Arc::new(AtomicUsize::new(0)))))?;
  assert_eq!(a.preparation(7,ticket)?.phase,crysirror_core::service::AudioPreparePhase::Ready);let h=a.voices[&(7,3)];
  a.exchange_channels(7,3,7)?;assert_eq!(a.preparation(7,ticket)?.channel,3);assert_eq!(a.voices[&(7,7)],h);assert!(!a.voices.contains_key(&(7,3)));a.release_preparation(7,ticket)?;assert_eq!(a.voices[&(7,7)],h);Ok(())
 }
}
