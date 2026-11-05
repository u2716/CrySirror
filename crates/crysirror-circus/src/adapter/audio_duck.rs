//!433E50→43C2C0→43B3D0 temporary native music-volume policy.
//!The existing shared scalar sequencer owns the f32 dB accumulator/tolerance;
//!the shared audio mixer owns publication. No PCM or interpolation lives here.
use super::*;
use crate::machine::media_ops::NativeAudioFramePolicy;
use crysirror_scene::layer::queued::{SequenceOperation as Op,SequenceStatus};
const ACTOR:u32=0x3a06_0000;
impl Bridge<'_> {
    fn native_duck_status(&mut self)->Result<Option<SequenceStatus>> {
        if !self.services.native_duck_created{return Ok(None)}
        Ok(serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::LayerSequence(ACTOR))?)?)
    }
    pub(in crate::adapter) fn native_duck_active(&mut self)->Result<bool>{Ok(self.native_duck_status()?.is_some_and(|status|!status.complete))}
    pub(in crate::adapter) fn cancel_native_duck(&mut self)->Result<()> {
        if self.services.native_duck_created {self.submit(vec![json!({"op":"sequence","id":ACTOR,"operation":Op::Finish{cancel:true,lanes:None}})],vec![])?;}Ok(())
    }
    fn apply_native_duck_level(&mut self,level:f32)->Result<()> {
        let db=level as i32;
        //DirectSound ignores out-of-range SetVolume while the original CPU
        //accumulator continues to its endpoint. Do not clamp that accumulator.
        if !(-10000..=0).contains(&db){return Ok(())}
        let gain=10f32.powf(db as f32/2000.);
        self.host.submit_audio(self.client,AudioRequest::SetMixGroupGain{name:GROUPS[0].into(),gain,fade_ms:0.})?;
        if let Some(controls)=self.services.mixer_controls.as_mut(){controls.gain[0]=gain;}
        Ok(())
    }
    pub(super) fn advance_native_duck(&mut self)->Result<()> {
        if self.host.audio_info(self.client,0)?.is_none(){return self.cancel_native_duck()}
        if let Some(status)=self.native_duck_status()?.filter(|status|!status.complete) {
            let operation=Op::Advance{generation:status.generation,revision:status.revision.checked_add(1).context("Circus music level clock overflow")?,rate:1.,fast:false};
            self.submit(vec![json!({"op":"sequence","id":ACTOR,"operation":operation})],vec![])?;
            let status=self.native_duck_status()?.context("Circus music level owner disappeared")?;
            self.apply_native_duck_level(status.values[0])?;
        }Ok(())
    }
    pub(super) fn declare_native_duck(&mut self,policy:NativeAudioFramePolicy)->Result<()> {
        if self.host.audio_info(self.client,0)?.is_none()||self.native_duck_active()?
            ||self.services.sound_release_channels.values().any(|channel|*channel==0){return Ok(())}
        let mut voice=false;
        if policy.voice_volume!=0 {for channel in 8..=11 {if self.native_channel_playing(channel)?{voice=true;break}}}
        let volume=if voice {
            if !policy.duck_enabled||policy.duck_ratio>=100{return Ok(())}
            policy.duck_ratio.wrapping_mul(policy.music_volume)/100
        }else{
            if policy.auto_dialogue{return Ok(())}
            policy.music_volume
        };
        let target=if policy.master!=0&&volume!=0&&policy.global_enabled {
            5000i32.wrapping_mul(volume.wrapping_mul(policy.master)/100)/100-5000
        }else{-10000};
        let gain=self.services.mixer_controls.context("Circus music level mixer missing")?.gain[0];
        let current=if gain<=0.00001{-10000}else{(2000.*f64::from(gain).log10()).round().clamp(-10000.,0.) as i32};
        if current==target{return Ok(())}
        if !self.services.native_duck_created {
            self.submit(vec![json!({"op":"put","layer":{"id":ACTOR,"rect":[0,0,1,1],"visible":false,"paint":{"kind":"solid","color":[0,0,0,0]}}}),json!({"op":"begin_sequence","id":ACTOR,"spec":{"values":[current],"bindings":[]}})],vec![])?;
            self.services.native_duck_created=true;
        }
        self.submit(vec![json!({"op":"sequence","id":ACTOR,"operation":Op::Set{channels:vec![0],values:vec![current as f32],retained:true}}),json!({"op":"sequence","id":ACTOR,"operation":{"kind":"enqueue","lane":0,"track":{"kind":"linear","channels":[0],"target":[target],"ticks":30,"references":[0],"loop_origins":[0],"delta":"current","complete_first":true,"complete_zero":true,"fast":false}}})],vec![])?;
        //43B3D0 sets its active flag immediately; activation itself has no
        //increment. The next43D010/native frame performs the first addition.
        let state=self.native_duck_status()?.context("Circus music level declaration owner")?;
        self.submit(vec![json!({"op":"sequence","id":ACTOR,"operation":Op::Advance{generation:state.generation,revision:state.revision.checked_add(1).context("Circus music level activation clock")?,rate:1.,fast:false}})],vec![])?;
        Ok(())
    }
}
