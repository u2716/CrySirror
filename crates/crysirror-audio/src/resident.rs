//! Complete immutable PCM preparation on shared workers, without an SFX-only
//! clip-size limit or decoder work inside the mixer callback.
use crate::decoder::{AudioSource,SourceRead,StereoFrame};
use anyhow::{Result,ensure};
use std::sync::Arc;
pub(crate) fn decode(mut input:Box<dyn AudioSource>,cancel:&crysirror_tasks::Cancellation,storage:Option<Arc<dyn Send+Sync>>)->Result<Box<dyn AudioSource>>{
    let rate=input.sample_rate();let replay_gain=input.has_replay_gain();let channels=input.channel_count();
    if let Some(frames)=input.frame_count(){ensure!(frames<=crysirror_core::service::MAX_PCM_FRAMES as u64,"resident PCM frame budget");}
    let mut samples=Vec::new();let mut block=[[0.;2];4096];
    loop {cancel.check()?;let count=input.read(&mut block)?;ensure!(count<=block.len(),"resident decoder read extent");if count==0{break}
        ensure!(samples.len().checked_add(count).is_some_and(|count|count<=crysirror_core::service::MAX_PCM_FRAMES),"resident PCM frame budget");samples.extend_from_slice(&block[..count]);}
    cancel.check()?;let mut pcm=crysirror_core::service::PcmAudio::new(rate,samples).map_err(|e|anyhow::anyhow!("{e}"))?;
    if let Some(storage)=storage{pcm.retain_storage(storage);}
    Ok(Box::new(Resident{inner:crate::decoder::open_pcm(pcm),replay_gain,channels}))
}
struct Resident{inner:Box<dyn AudioSource>,replay_gain:bool,channels:Option<u16>}
impl AudioSource for Resident{
    fn has_replay_gain(&self)->bool{self.replay_gain}
    fn channel_count(&self)->Option<u16>{self.channels}
    fn sample_rate(&self)->u32{self.inner.sample_rate()}
    fn frame_count(&self)->Option<u64>{self.inner.frame_count()}
    fn read_position(&self)->Option<u64>{self.inner.read_position()}
    fn is_nonblocking(&self)->bool{true}
    fn read(&mut self,out:&mut [StereoFrame])->Result<usize>{self.inner.read(out)}
    fn poll_read(&mut self,out:&mut [StereoFrame])->Result<SourceRead>{self.inner.poll_read(out)}
    fn seek(&mut self,frame:u64)->Result<()>{self.inner.seek(frame)}
}
