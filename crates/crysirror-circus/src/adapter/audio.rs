//! Native audio names/loop metadata mapped onto the shared decoder and mixer.
use super::*;
use crate::machine::{MixerControls, MusicAsset};
#[path="audio_duck.rs"]
mod duck;
const GROUPS: [&str; 3] = ["circus/music", "circus/sfx", "circus/voice"];

pub(super) struct AudioMetadata {
    pub loops:BTreeMap<String,[u64;2]>,
    pub sfx_prefixes:Vec<String>,
}
pub(super) fn load_music_loops(storage: &dyn ClientStorage) -> Result<AudioMetadata> {
    // The loop table is native data, not code.43C390 converts these decoded
    // frame indices into byte offsets with channels*bits_per_sample/8.
    const BUILD_SHA: &str = "178ee9a5e34356b11942213e2e32a34afe4933c90d563bf5a3d6402250f54ab7";
    let candidates = storage
        .list(Mount::Asset, "")?
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .collect::<Vec<_>>();
    ensure!(
        candidates.len() <= 32,
        "Circus native metadata executable candidate limit"
    );
    let mut remaining = crysirror_resource::pe::MAX_PE_BYTES as u64;
    for name in candidates {
        let path = VfsPath::new(Mount::Asset, &name)?;
        let metadata = storage
            .metadata(Mount::Asset, path.relative())?
            .context("Circus native metadata file missing")?;
        if metadata.is_directory {
            continue;
        }
        let size = metadata
            .size
            .context("Circus native metadata file size unavailable")?;
        ensure!(size <= remaining, "Circus native metadata read budget");
        remaining -= size;
        let reader = storage.open_read(&path)?;
        let bytes = crysirror_resource::read_all(reader.as_ref(), size)?;
        let Ok(image) = crysirror_resource::pe::PeData::parse(&bytes) else {
            continue;
        };
        let digest = image
            .sha256()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if digest != BUILD_SHA {
            continue;
        }
        let mut sfx_prefixes=Vec::new();
        for index in 0..256u64 {
            let row=image.va_slice(0x4b32b0+index*16,16)?;
            if row[0]==0 {break}
            let end=row.iter().position(|byte|*byte==0).context("Circus SFX prefix terminator")?;
            let name=std::str::from_utf8(&row[..end])?;
            ensure!(name.is_ascii()&&!name.chars().any(char::is_control),"Circus native SFX prefix");
            sfx_prefixes.push(name.to_owned());
            ensure!(index<255,"Circus native SFX prefix table limit");
        }
        let mut records = BTreeMap::new();
        for index in 0..256u64 {
            let row = image.va_slice(0x4b4ba0 + index * 24, 24)?;
            if row[0] == 0 {
                return Ok(AudioMetadata{loops:records,sfx_prefixes});
            }
            let end = row[..16]
                .iter()
                .position(|&b| b == 0)
                .context("Circus native music loop name terminator")?;
            let name = std::str::from_utf8(&row[..end])?;
            ensure!(
                name.is_ascii()
                    && !name.contains(['\\', '/', ':'])
                    && !name.chars().any(char::is_control),
                "Circus native loop resource basename"
            );
            let start = u64::from(u32::from_le_bytes(row[16..20].try_into()?));
            let end = u64::from(u32::from_le_bytes(row[20..24].try_into()?));
            ensure!(end == 0 || start < end, "Circus native loop frame interval");
            ensure!(
                records
                    .insert(name.to_ascii_uppercase(), [start, end])
                    .is_none(),
                "Circus native loop resource duplicate"
            );
        }
        bail!("Circus native music loop metadata lacks terminator");
    }
    bail!(
        "Circus native music loop profile requires a recognized original executable as read-only data"
    )
}
impl Bridge<'_> {
    pub(super) fn release_native_group(&mut self,kind:u8,forced:bool,steps:u32)->Result<bool>{
        let (first,last)=match kind{0=>(0,1),1=>(2,5),2=>(8,11),_=>bail!("Circus audio stop category")};
        if self.services.sound_stop_group.is_none(){self.services.sound_stop_group=Some((kind,forced,steps,first));}
        loop {
            let (expected,expected_forced,expected_steps,channel)=self.services.sound_stop_group.unwrap();
            ensure!((expected,expected_forced,expected_steps)==(kind,forced,steps),"Circus grouped stop continuation changed");
            if channel>last{self.services.sound_stop_group=None;return Ok(true)}
            if forced {
                if channel==0{self.cancel_native_duck()?;}
                self.host.submit_audio(self.client,AudioRequest::Stop{channel,fade_ms:0.})?;
                self.services.balance_tracks.remove(&channel);
                self.services.stopped_audio.push(channel);
            }else if !self.release_native_sound(channel,steps)?{return Ok(false)}
            self.services.sound_stop_group.as_mut().unwrap().3+=1;
        }
    }
    pub(super) fn release_native_sound(&mut self,channel:u32,steps:u32)->Result<bool> {
        use crysirror_core::service::{AudioReleaseSpec,AudioReleasePhase as Phase};
        let handle=if let Some((expected,expected_steps,handle))=self.services.sound_release_pending {
            ensure!(expected==channel&&expected_steps==steps,"Circus sound release continuation changed");handle
        }else{
            let handle=self.host.begin_audio_release(self.client,AudioReleaseSpec{source_channel:channel,tail_channel:channel,threshold_db100:-3000,target_above_db100:-3000,target_below_db100:-10000,steps,pan:None,playback_rate:None,freeze_group_gain:true,release_tail_when_source_missing:true})?;
            self.services.sound_release_pending=Some((channel,steps,handle));handle
        };
        let receipt=self.host.audio_release_info(self.client,handle)?;
        match receipt.phase {
            Phase::Pending=>Ok(false),
            Phase::Cancelled=>bail!("Circus native sound release cancelled before admission"),
            phase=>{
                self.services.sound_release_pending=None;
                if channel==0{self.cancel_native_duck()?;}
                if phase==Phase::Active {
                    self.services.sound_release_channels.insert(handle.generation,channel);
                    self.services.music_releases.push((handle,0));
                }else{
                    if receipt.source_position_frames.is_some(){self.services.stopped_audio.push(channel);}
                    self.host.release_audio_release(self.client,handle)?;
                }
                Ok(true)
            }
        }
    }
    pub(super) fn publish_native_sound(&mut self,declaration:&crate::machine::media_ops::AudioDeclaration)->Result<Option<crate::machine::media_ops::AudioPublication>> {
        use crate::machine::media_ops::AudioPublication;
        let kind=if declaration.voice {2}else{1};
        if declaration.name.is_empty()||!self.services.mixer_controls.context("Circus mixer controls missing")?.enabled[kind] {
            return Ok(Some(AudioPublication{channels:vec![]}));
        }
        if self.services.sound_starting.is_none() {
            if !declaration.voice&&self.services.sound_prefixes.is_none() {
                let Some(Prepared::MusicLoops(metadata))=self.services.prepare(Work::MusicLoops,self.client)? else{return Ok(None)};
                self.services.music_loops=Some(metadata.loops);
                self.services.sound_prefixes=Some(metadata.sfx_prefixes);
            }
            let mut selected=if declaration.name.contains("silent") {
                if declaration.voice {(8..=11).collect()}else{(2..=7).collect()}
            }else{Vec::new()};
            let channel=if declaration.voice {
                let mut free=11;
                for channel in 8..11 {if !self.native_channel_playing(channel)? {free=channel;break}}
                free
            }else if declaration.looping {2}else{
                let mut free=5;
                for channel in 3..5 {if !self.native_channel_playing(channel)? {free=channel;break}}
                free
            };
            selected.push(channel);
            if !declaration.voice {
                for channel in &mut selected {
                    if (2..=5).contains(channel) {
                        if let Some(index)=self.services.sound_prefixes.as_ref().unwrap().iter().position(|prefix|declaration.name.get(..prefix.len()).is_some_and(|name|name.eq_ignore_ascii_case(prefix))) {
                            *channel=if index==0{7}else{6};
                        }
                    }
                }
            }
            self.services.sound_starting=Some((declaration.clone(),selected,0));
        }
        loop {
            let (expected,channels,index)=self.services.sound_starting.as_ref().unwrap().clone();
            ensure!(&expected==declaration,"Circus sound preparation continuation changed");
            let Some(&channel)=channels.get(index) else {
                self.services.sound_starting=None;
                return Ok(Some(AudioPublication{channels}));
            };
            if !self.play_audio(channel,&declaration.name,declaration.voice,declaration.looping,0,None)? {return Ok(None)}
            self.services.sound_starting.as_mut().unwrap().2+=1;
        }
    }
    pub(super) fn native_channel_playing(&mut self,channel:u32)->Result<bool> {
        let Some(info)=self.host.audio_info(self.client,channel)? else{return Ok(false)};
        if (8..=11).contains(&channel) {
            let controls=self.services.mixer_controls.context("Circus mixer controls missing")?;
            if !controls.enabled[2]||controls.gain[2]==0. {return Ok(false)}
            let release=self.services.music_releases.iter().find(|(handle,_)|self.services.sound_release_channels.get(&handle.generation)==Some(&channel)).map(|(handle,_)|*handle);
            if let Some(handle)=release {
                let release=self.host.audio_release_info(self.client,handle)?;
                if release.volume_db100.is_some_and(|db|db<=-9999){return Ok(false)}
            }else if info.gain*self.services.voice_playback_gain.unwrap_or(controls.gain[2])<=0.0000113 {return Ok(false)}
        }
        Ok(info.source_playing)
    }
    pub(super) fn release_music(&mut self, forced: bool, steps: u32) -> Result<bool> {
        if forced{self.cancel_native_duck()?;}
        use crysirror_core::service::{AudioReleasePhase as Phase, AudioReleaseSpec};
        if forced {
            self.services.balance_tracks.remove(&0);self.services.balance_tracks.remove(&1);
            ensure!(
                self.services.music_release_pending.is_none(),
                "Circus pending music release ownership"
            );
            for channel in [0, 1] {
                self.host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel,
                        fade_ms: 0.,
                    },
                )?;
            }
            return Ok(true);
        }
        let handle = if let Some(handle) = self.services.music_release_pending {
            handle
        } else {
            let handle = self.host.begin_audio_release(
                self.client,
                AudioReleaseSpec {
                    source_channel: 0,
                    tail_channel: 1,
                    threshold_db100: -3000,
                    target_above_db100: -3000,
                    target_below_db100: -10000,
                    steps,
                    pan: Some(0.),
                    playback_rate: Some(1.),
                    freeze_group_gain: true,
                    release_tail_when_source_missing: true,
                },
            )?;
            self.services.music_release_pending = Some(handle);
            handle
        };
        match self.host.audio_release_info(self.client, handle)?.phase {
            Phase::Pending => Ok(false),
            Phase::Cancelled => bail!("Circus native music release cancelled before admission"),
            phase => {
                self.services.music_release_pending = None;
                self.cancel_native_duck()?;
                if phase == Phase::Active {
                    self.services.balance_tracks.remove(&0);self.services.balance_tracks.remove(&1);
                    self.host.submit_audio(self.client,AudioRequest::Balance{channel:1,command:crysirror_core::service::AudioBalanceCommand::Set{db100:0,cancel_track:true}})?;
                    self.services.music_releases.push((handle, 0));
                } else {
                    self.host.release_audio_release(self.client, handle)?;
                }
                Ok(true)
            }
        }
    }
    ///43D010 is called before the next native process, independently of PCM
    ///frames or the200ms stream-refill timer. Resource/budget retries never tick.
    pub(super) fn native_audio_update(&mut self,policy:crate::machine::media_ops::NativeAudioFramePolicy) -> Result<()> {
        use crysirror_core::service::AudioReleasePhase as Phase;
        if !self.services.audio_update_due {
            return Ok(());
        }
        self.services.audio_update_due = false;
        self.advance_native_balance()?;
        self.advance_native_duck()?;
        let mut active = Vec::new();
        for (handle, updates) in self.services.music_releases.drain(..) {
            let info = self.host.audio_release_info(self.client, handle)?;
            if matches!(info.phase, Phase::Complete | Phase::Cancelled) {
                if let Some(channel)=self.services.sound_release_channels.remove(&handle.generation) {
                    if info.phase==Phase::Complete {self.services.stopped_audio.push(channel);}
                }
                self.host.release_audio_release(self.client, handle)?;
            } else {
                let updates = updates
                    .checked_add(1)
                    .context("Circus audio update overflow")?;
                self.host
                    .advance_audio_release(self.client, handle, updates)?;
                active.push((handle, updates));
            }
        }
        self.services.music_releases = active;
        self.declare_native_duck(policy)?;
        Ok(())
    }

    pub(super) fn sync_audio_controls(&mut self, controls: MixerControls) -> Result<()> {
        self.sync_audio_controls_forced(controls,[false;3],[false;3])
    }
    pub(super) fn sync_audio_controls_forced(&mut self, mut controls: MixerControls,force_gain:[bool;3],force_enable:[bool;3]) -> Result<()> {
        if !controls.enabled[0]&&force_enable[0]{self.cancel_native_duck()?;}
        let volume_track=self.native_duck_active()?;
        //40DAD0 and ordinary preference observation update admission only.
        //Only43B4C0 setters or a newly ready43C390 buffer change live gain.
        if let Some(previous)=self.services.mixer_controls {
            for index in 0..3 {if !force_gain[index]{controls.gain[index]=previous.gain[index];}}
        }
        if self.services.mixer_controls == Some(controls)&&!force_gain.into_iter().any(|force|force)&&!force_enable.into_iter().any(|force|force) {
            return Ok(());
        }
        for index in 0..3 {
            //43ABF0(a4=0) ignores setters during the active43B3D0 ramp.
            if index==0&&volume_track {if let Some(previous)=self.services.mixer_controls{controls.gain[0]=previous.gain[0];}}
            if !(index==0&&volume_track)&&(force_gain[index] || self
                .services
                .mixer_controls
                .is_none_or(|old| old.gain[index] != controls.gain[index]))
            {
                self.host.submit_audio(
                    self.client,
                    AudioRequest::SetMixGroupGain {
                        name: GROUPS[index].into(),
                        gain: controls.gain[index],
                        fade_ms: 0.,
                    },
                )?;
                if index==2 {self.services.voice_playback_gain=Some(controls.gain[index]);}
            }
            //43C320 always stops0/8..11 when explicitly assigned0,
            //including repeated0. SFX disable changes only future admission.
            if !controls.enabled[index] && (force_enable[index] || self
                .services
                .mixer_controls
                .is_some_and(|old| old.enabled[index]))
                && index != 1
            {
                let channels = if index == 0 { 0..=0 } else { 8..=11 };
                for channel in channels {
                    self.host.submit_audio(
                        self.client,
                        AudioRequest::Stop {
                            channel,
                            fade_ms: 0.,
                        },
                    )?;
                }
            }
        }
        self.services.mixer_controls = Some(controls);
        Ok(())
    }
    pub(super) fn native_music(&mut self, id: u16, looping: bool) -> Result<Option<MusicAsset>> {
        if self.services.names.is_none() {
            let Some(Prepared::Names(names)) = self.services.prepare(Work::Names, self.client)?
            else {
                return Ok(None);
            };
            self.services.names = Some(names);
        }
        let name = self
            .services
            .names
            .as_ref()
            .unwrap()
            .music
            .get(usize::from(id)).cloned().unwrap_or_default();
        self.native_music_by_name(&name,looping)
    }
    pub(super) fn native_music_by_name(&mut self,name:&str,looping:bool)->Result<Option<MusicAsset>> {
        let name=name.to_owned();
        if name.is_empty(){return Ok(Some(MusicAsset{name,loop_start:0,loop_end:None}))}
        let mut asset = MusicAsset {
            name,
            loop_start: 0,
            loop_end: None,
        };
        if looping {
            if self.services.music_loops.is_none() {
                let Some(Prepared::MusicLoops(records)) =
                    self.services.prepare(Work::MusicLoops, self.client)?
                else {
                    return Ok(None);
                };
                self.services.sound_prefixes=Some(records.sfx_prefixes);
                self.services.music_loops = Some(records.loops);
            }
            let mut basename = asset
                .name
                .rsplit(['\\', '/'])
                .next()
                .context("Circus music resource basename")?
                .to_ascii_uppercase();
            if !basename.ends_with(".PCM"){basename.push_str(".PCM");}
            if let Some([start, end]) = self.services.music_loops.as_ref().unwrap().get(&basename) {
                asset.loop_start = *start;
                asset.loop_end = (*end != 0).then_some(*end);
            }
        }
        Ok(Some(asset))
    }
    pub(super) fn play_audio(
        &mut self,
        channel: u32,
        name: &str,
        voice: bool,
        looping: bool,
        loop_start: u64,
        loop_end: Option<u64>,
    ) -> Result<bool> {
        self.play_audio_policy(channel,name,voice,looping,loop_start,loop_end,channel!=0)
    }
    pub(super) fn play_audio_policy(
        &mut self,
        channel: u32,
        name: &str,
        voice: bool,
        looping: bool,
        loop_start: u64,
        loop_end: Option<u64>,
        resident:bool,
    ) -> Result<bool> {
        let kind = if voice {
            2
        } else if channel == 0 {
            0
        } else {
            1
        };
        if !self
            .services
            .mixer_controls
            .context("Circus native mixer controls missing")?
            .enabled[kind]
        {
            return Ok(true);
        }
        if let Some((pending_channel, pending_name, ticket)) = self.services.audio.as_ref() {
            ensure!(
                *pending_channel == channel && pending_name == name,
                "Circus audio completion ownership"
            );
            let ticket = *ticket;
            let prepared = self.host.audio_preparation(self.client, ticket)?;
            match prepared.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => bail!(
                    "Circus audio preparation: {}",
                    prepared.error.unwrap_or("unknown error".into())
                ),
                AudioPreparePhase::Ready => {
                    //43C390 applies43B630 to the entire voice pool only after
                    //the new buffer has actually been published by its owner.
                    if voice {self.native_rate_group_ready(2,self.services.voice_rate_percent)?;}
                    self.host.release_audio(self.client, ticket)?;
                    self.services.audio = None;
                    self.services.balance_tracks.remove(&channel);
                    return Ok(true);
                }
            }
        }
        let Some(Prepared::Audio(bytes, archive)) = self
            .services
            .prepare(Work::Audio(name.into(), voice), self.client)?
        else {
            return Ok(false);
        };
        if let Some(archive) = archive {
            self.services.voice_archive = Some(archive);
        }
        if channel==0 {self.cancel_native_duck()?;}
        let mut controls=self.services.mixer_controls.context("Circus PCM mixer owner missing")?;
        let voice_volume=self.services.sound_starting.as_ref().and_then(|(declaration,_,_)|declaration.voice_volume);
        let requested=if voice {voice_volume.map(|(master,volume)| {
            let level=if master!=0&&volume!=0 {5000i32.wrapping_mul(volume.wrapping_mul(master)/100)/100-5000}else{-10000};
            (-10000..=0).contains(&level).then(||10f32.powf(level as f32/2000.))
        }).unwrap_or(controls.playback_gain[kind])}else{controls.playback_gain[kind]};
        if let Some(gain)=requested {
            controls.gain[kind]=gain;
            let mut force=[false;3];force[kind]=true;
            self.sync_audio_controls_forced(controls,force,[false;3])?;
        }
        let source=EncodedAudio{bytes,frame_limit:None};
        if channel==0 {self.services.music_loop_end=loop_end;}
        let parameters=AudioParameters{bus:if voice{PcmBus::Voice}else if channel==0{PcmBus::Music}else{PcmBus::Sfx},mix_group:Some(GROUPS[kind].into()),looping,loop_start,loop_end,..Default::default()};
        let request=if resident {AudioRequest::PlayResident{channel,source,intro:None,parameters}}else{AudioRequest::Play{channel,source,intro:None,parameters}};
        self.services.resident_next_music=false;
        let ticket=self.host.begin_audio(self.client,request)?;
        self.services.audio = Some((channel, name.into(), ticket));
        Ok(false)
    }
}

impl Bridge<'_> {
    pub(super) fn native_balance(&mut self,kind:u8,from:i32,to:i32,steps:u32)->Result<()> {
        use crysirror_core::service::AudioBalanceCommand as B;
        let channels=match kind {0=>0..=1,1=>2..=7,2=>8..=11,_=>bail!("Circus balance category")};
        ensure!((-10000..=10000).contains(&from)&&(-10000..=10000).contains(&to)&&steps<=100000,"Circus balance controls exceed native DirectSound range");
        for channel in channels {
            if self.host.audio_info(self.client,channel)?.is_none(){continue}
            self.host.submit_audio(self.client,AudioRequest::Balance{channel,command:B::Set{db100:from,cancel_track:false}})?;
            if steps!=0 {
                let track=self.services.balance_generation.checked_add(1).context("Circus balance identity exhausted")?;
                self.services.balance_generation=track;
                self.host.submit_audio(self.client,AudioRequest::Balance{channel,command:B::Begin{track,target_db100:to,steps,finish_previous:true}})?;
                self.services.balance_tracks.insert(channel,(track,0));
            }
        }Ok(())
    }
    fn advance_native_balance(&mut self)->Result<()> {
        use crysirror_core::service::AudioBalanceCommand as B;
        let tracks=self.services.balance_tracks.iter().map(|(channel,state)|(*channel,*state)).collect::<Vec<_>>();
        for (channel,(track,updates)) in tracks {
            let Some(info)=self.host.audio_info(self.client,channel)?else{self.services.balance_tracks.remove(&channel);continue};
            if info.balance.track==track&&!info.balance.active&&info.balance.completed_updates>=updates&&updates!=0 {self.services.balance_tracks.remove(&channel);continue}
            let updates=updates.checked_add(1).context("Circus balance tick overflow")?;
            self.host.submit_audio(self.client,AudioRequest::Balance{channel,command:B::Advance{track,updates}})?;
            self.services.balance_tracks.insert(channel,(track,updates));
        }Ok(())
    }
}

impl Bridge<'_> {
    pub(super) fn play_native_music(&mut self,asset:&crate::machine::MusicAsset,looping:bool,resident:bool)->Result<bool>{
        if resident{self.services.resident_next_music=true;}
        if asset.name.is_empty(){return Ok(true)}
        if self.services.music_starting.is_none(){
            if self.host.audio_info(self.client,0)?.is_some_and(|info|info.source_playing){
                if !self.release_music(false,30)?{return Ok(false)}
            }
            self.services.music_starting=Some((asset.name.clone(),looping,self.services.resident_next_music));
        }
        let (name,expected_loop,resident)=self.services.music_starting.as_ref().unwrap().clone();
        ensure!(name==asset.name&&expected_loop==looping,"Circus music preparation continuation changed");
        if !self.play_audio_policy(0,&asset.name,false,looping,if resident{0}else{asset.loop_start},if resident{None}else{asset.loop_end},resident)?{return Ok(false)}
        self.services.music_starting=None;self.services.resident_next_music=false;Ok(true)
    }
}

impl Bridge<'_>{
    pub(super) fn native_music_loop_start(&mut self,percent:i16)->Result<bool>{
        let Some(info)=self.host.audio_info(self.client,0)?else{return Ok(true)};
        if percent==0{return Ok(true)}
        let length=info.source_frames.context("Circus loop-start declaration requires decoded source length")?;
        ensure!(length>0&&info.source_rate>0,"Circus loop-start source dimensions");
        let end=self.services.music_loop_end.filter(|end|*end!=0).unwrap_or(length).min(length);
        let cap=u64::try_from(u128::from(end)*100/u128::from(length))?;
        let percent=u64::try_from(percent.clamp(0,100))?.min(cap);
        //43B2C0 changes the retained loop read offset, leaving live transport
        //unchanged. Its full-duration whole-second truncation is intentional.
        let seconds=length/u64::from(info.source_rate);
        let start=u64::try_from(u128::from(seconds)*u128::from(percent)/100)?*u64::from(info.source_rate);
        if start<end {
            self.host.submit_audio(self.client,AudioRequest::SetLooped{channel:0,looping:info.looping,loop_start:Some(start)})?;
        }
        Ok(true)
    }
}

impl Bridge<'_> {
    pub(in crate::adapter) fn native_voice_rate(&mut self,percent:i32)->Result<bool>{self.native_rate_group(2,percent)}
    pub(in crate::adapter) fn native_rate_group(&mut self,kind:u8,percent:i32)->Result<bool> {
        if self.services.audio.is_some(){return Ok(false)}
        self.native_rate_group_ready(kind,percent)?;Ok(true)
    }
    fn native_rate_group_ready(&mut self,kind:u8,percent:i32)->Result<()> {
        let channels=match kind{0=>0..=1,1=>2..=7,2=>8..=11,_=>bail!("Circus native frequency category")};
        //43ACF0 deliberately uses44100 rather than the source's base rate.
        //<=100 sends DSBFREQUENCY_ORIGINAL; overflow is signed32-bit. The
        //original ignores a failed SetFrequency instead of clamping it.
        //Microsoft DirectSound Frequency enum defines0/original and100..200000:
        //https://learn.microsoft.com/en-us/previous-versions/ms811482(v=msdn.10)
        let frequency=if percent<=100{0}else{44100i32.wrapping_mul(percent)/100};
        if frequency!=0&&!(100..=200000).contains(&frequency){return Ok(())}
        for channel in channels {if let Some(info)=self.host.audio_info(self.client,channel)? {
            let rate=if frequency==0{1.}else{frequency as f32/info.source_rate as f32};
            self.host.submit_audio(self.client,AudioRequest::SetRate{channel,rate})?;
        }}Ok(())
    }
}
