//!4233A0 cases05/103: native volume restoration, index/name policy and
//!dispatch-local deferred music declarations. No decoder or PCM lives here.
use super::*;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
impl Machine {
    pub(super) fn execute_music(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>> {
        if instruction.opcode != 0x67 && !(instruction.opcode == 0x05 && a[0] != 254) {
            return Ok(None);
        }
        if instruction.opcode == 0x67 || a[0] == 255 {
            let volume = self.state.audio.deferred_music_volume;
            //Buf23575 uses zero as its original "nothing to restore" sentinel.
            if volume != 0 {
                let mut data = self.state.system_data.clone();
                ensure!(data.set_prefix_value(7, volume)?, "DC3 music volume field");
                if !self.publish_music_volume(host, &data)? { return Ok(Some(false)); }
                self.state.system_data = data;
                self.state.audio.deferred_music_volume = 0;
            }
            if instruction.opcode == 0x05 { return Ok(Some(true)); }
        } else {
            let mut data = self.state.system_data.clone();
            let previous = data.prefix_value(7)?.context("DC3 music volume preference")?;
            let scaled = i32::from(a[0]).wrapping_mul(previous) / 100;
            ensure!(data.set_prefix_value(7, scaled.clamp(0, 100))?, "DC3 music proportional volume field");
            if !self.publish_music_volume(host, &data)? { return Ok(Some(false)); }
            self.state.system_data = data;
            self.state.audio.deferred_music_volume = previous;
            return Ok(Some(true));
        }
        ensure!(a[0] >= 0, "DC3 negative native music index");
        let index = usize::try_from(a[0])?;
        let valid = match host.resource(&ResourceRequest::MusicIndex {index})? {
            None => return Ok(Some(false)),
            Some(ResourceReply::Index(index)) => index.is_some(),
            Some(_) => bail!("DC3 music directory index receipt"),
        };
        //40BC90 marks native byte360+index before same-id suppression or stop.
        if valid { self.state.system_data.set_byte(96, a[0], 1)?; }
        let looping = a[1] < 2;
        let Some(asset) = host.music(a[0] as u16, looping)? else { return Ok(Some(false)); };
        ensure!(asset.name.len() < 260 && !asset.name.contains('\0'), "DC3 music declaration extent");
        if a[0] == 255 {
            if !host.emit(&Event::StopAudio {channel: 0})? { return Ok(Some(false)); }
            self.state.audio.pending.remove(&0);
            self.state.audio.music_name.clear();
            self.state.audio.local_aux[0] = 0;
        } else {
            if a[1] != 1 || self.state.audio.music_index != i32::from(a[0]) {
                self.state.audio.pending.insert(0, PendingSound {name: asset.name.clone(), voice: false, looping});
            }
            self.state.audio.music_name = asset.name;
        }
        self.state.audio.music_index = i32::from(a[0]);
        Ok(Some(true))
    }
    fn publish_music_volume(&mut self, host: &mut dyn NativeHost, data: &crate::dc3_settings::SystemData) -> Result<bool> {
        let mix = controls(data)?;
        if !host.emit(&Event::Dc3MusicVolume {
            enabled: mix.enabled, gain: mix.gain, playback_gain: mix.playback_gain,
        })? { return Ok(false); }
        self.state.audio.mixer_initialized = true;
        Ok(true)
    }
}
