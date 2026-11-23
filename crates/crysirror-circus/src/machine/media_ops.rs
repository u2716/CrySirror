//! Original audio publication and playlist declaration policy.
use super::*;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub(super) struct PendingAudio {
    pub slot:u32,
    pub name:String,
    pub voice:bool,
    pub looping:bool,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct AudioPublication {
    pub channels:Vec<u32>,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct AudioDeclaration {
    pub name:String,
    pub voice:bool,
    pub looping:bool,
    pub voice_volume:Option<(i32,i32)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct MediaState {
    playlists: BTreeMap<i16, Vec<String>>,
    saved_music_volume: i32,
    resident_pending_music: bool,
    pending_balance:BTreeMap<u8,(i32,i32,u32)>,
    #[serde(default)]
    pub(super) effective_gains:Option<[f32;3]>,
    #[serde(skip)]
    setters:std::collections::VecDeque<AudioSetter>,
}
#[derive(Clone,Debug)]
struct AudioSetter {
    controls:MixerControls,
    force_gain:[bool;3],
    force_enable:[bool;3],
    controls_done:bool,
    music_name:Option<String>,
    asset:Option<MusicAsset>,
}
impl MediaState {
    pub(super) fn playlist(&self,backend:i16)->&[String]{self.playlists.get(&backend).map_or(&[],Vec::as_slice)}
    pub(super) fn clear_playlist(&mut self,backend:i16){self.playlists.remove(&backend);}
    pub(super) fn restore_playlist(&mut self,backend:i16,rows:Vec<String>){self.playlists.insert(backend,rows);}
}
impl MediaState {pub(super) fn restore_volume_backup(&mut self,value:i32){self.saved_music_volume=value;}}
impl MediaState {pub(super) fn pending_controls(&self)->bool {!self.pending_balance.is_empty()||!self.setters.is_empty()}}
impl MediaState {pub(super) fn clear_playlist_for_restore(&mut self){self.playlists.clear();self.resident_pending_music=false;self.pending_balance.clear();self.setters.clear();}}
impl State {
    pub(super) fn remember_audio_setter(&mut self,index:i16,old_value:i32,old_controls:MixerControls)->Result<()> {
        self.media.effective_gains.get_or_insert(old_controls.gain);
        if index==9 {return Ok(())} //40D240 writes voice preference only.
        let (computed,valid)=self.native_audio_gains(old_controls.gain)?;
        let mut gains=self.media.effective_gains.unwrap();
        let mut force_gain=[false;3];let mut force_enable=[false;3];
        for group in 0..3 {
            if index==225||index==203||index==7&&group==0||index==8&&group==1 {
                gains[group]=computed[group];force_gain[group]=valid[group];
            }
            if index==14&&group==0||index==15&&group==1||index==5&&group==2 {force_enable[group]=true;}
        }
        self.media.effective_gains=Some(gains);
        let controls=self.audio_controls()?;
        let music_name=if index==14&&old_value==0&&self.value(99,14)?!=0 {
            let name=self.native_live.retained_music_name()?;
            (!name.is_empty()).then_some(name)
        }else{None};
        self.media.setters.push_back(AudioSetter{controls,force_gain,force_enable,controls_done:false,music_name,asset:None});
        Ok(())
    }
}
impl Machine {
    pub(crate) fn audio_setter_pending(&self)->bool {!self.state.media.setters.is_empty()}
    pub(super) fn poll_audio_setters(&mut self,host:&mut dyn NativeHost)->Result<bool>{
        while let Some(mut job)=self.state.media.setters.front().cloned() {
            if !job.controls_done {
                if !host.emit(&Event::NativeMixerControls{enabled:job.controls.enabled,gain:job.controls.gain,playback_gain:job.controls.playback_gain,force_gain:job.force_gain,force_enable:job.force_enable})? {return Ok(false)}
                self.state.media.setters.front_mut().unwrap().controls_done=true;
                job.controls_done=true;
            }
            if let Some(name)=&job.music_name {
                let asset=if let Some(asset)=job.asset {asset}else {
                    let Some(asset)=host.music_by_name(name,true)? else{return Ok(false)};
                    self.state.media.setters.front_mut().unwrap().asset=Some(asset.clone());asset
                };
                if !host.emit(&Event::PlayMusic{asset:asset.clone(),looping:true})? {return Ok(false)}
                self.state.last_music=Some(asset);
            }
            self.state.media.setters.pop_front();
        }
        Ok(true)
    }
    pub(crate) fn publish_native_audio_frame(&mut self,host:&mut dyn NativeHost)->Result<bool>{
        let percent=self.state.system_ops.frame_audio;
        if percent!=0&&!host.emit(&Event::MusicLoopStart{percent})?{return Ok(false)}
        self.state.system_ops.frame_audio=0;Ok(true)
    }
    pub(super) fn publish_native_audio(&mut self, host: &mut dyn NativeHost, explicit_flush:bool) -> Result<bool> {
        if self.state.native_flags.get(&25088).copied().unwrap_or(0)!=0 {
            self.state.pending_music=None;self.state.pending_audio.clear();
        }
        if !explicit_flush&&self.state.dispatch_a2!=0 {self.state.pending_music=None;}
        if self.state.native_skipping()||self.state.native_flags.get(&15628).copied().unwrap_or(0)!=0 {
            self.state.pending_audio.retain(|pending|!pending.voice);
        }
        if let Some((asset, looping)) = &self.state.pending_music {
            let event=if self.state.media.resident_pending_music {
                Event::PlayResidentMusic{asset:asset.clone(),looping:*looping}
            }else{Event::PlayMusic{asset:asset.clone(),looping:*looping}};
            if !host.emit(&event)? {return Ok(false)}
            self.state.media.resident_pending_music=false;
            self.state.pending_music = None;
        }
        while let Some(pending) = self.state.pending_audio.first().cloned() {
            let voice_volume=if pending.voice {
                let Some(volume)=self.state.native_voice_volume(&pending.name)? else{self.state.pending_audio.remove(0);continue};
                Some(volume)
            }else{None};
            let Some(receipt)=host.publish_audio(&AudioDeclaration{name:pending.name.clone(),voice:pending.voice,looping:pending.looping,voice_volume})? else{return Ok(false)};
            if !pending.voice {
                for channel in receipt.channels {self.state.native_live.sound_loaded(channel,&pending.name,pending.looping)?;}
            }
            self.state.pending_audio.remove(0);
        }
        while let Some((&kind,&(from,to,steps)))=self.state.media.pending_balance.first_key_value(){
            if !host.emit(&Event::AudioBalance{kind,from,to,steps})? {return Ok(false)}
            self.state.media.pending_balance.remove(&kind);
        }
        Ok(true)
    }
    pub(super) fn execute_media(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>> {
        let mut outcome=Outcome::Advanced;
        match i.opcode {
            0x4c|0x4d|0x4e=>{
                if self.state.native_flags.get(&25088).copied().unwrap_or(0)==0 {
                    if !host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:30})?{return Ok(Some(CommandResult::Retry))}
                }
                self.state.dialogue.voice_source=text.to_owned();
                self.state.native_live.voice_source(i.opcode,text)?;
                let slot=if i.opcode==0x4e {9}else{8};
                self.state.pending_audio.retain(|pending|pending.slot!=slot);
                self.state.pending_audio.push(PendingAudio{slot,name:text.to_owned(),voice:true,looping:false});
            },
            0x71 if a[0]>=0=>{
                let occupied=self.state.pending_audio.iter().map(|pending|pending.slot).collect::<BTreeSet<_>>();
                if let Some(slot)=(2..=5).find(|slot|!occupied.contains(slot)) {
                    let Some(name)=host.sound_name(a[0] as u16)? else{return Ok(Some(CommandResult::Retry))};
                    self.state.pending_audio.push(PendingAudio{slot,name,voice:false,looping:a[1]!=0});
                }
            },
            0x71 if a[0]==-2=>{
                self.state.pending_audio.retain(|pending|pending.slot<8);
                self.state.dialogue.voice_source.clear();
                self.state.native_live.voice_source(0x71,"")?;
                if self.state.native_flags.get(&25088).copied().unwrap_or(0)==0 {
                    if !host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:30})?{return Ok(Some(CommandResult::Retry))}
                }
            },
            0x71 if a[0]==-1=>{
                self.state.native_live.sound_stop_declared(self.state.native_flags.get(&25088).copied().unwrap_or(0)!=0);
                self.state.pending_audio.retain(|pending|!(2..=5).contains(&pending.slot));
                if self.state.native_flags.get(&25088).copied().unwrap_or(0)==0 {
                    if !host.emit(&Event::ReleaseAudioGroup{kind:1,forced:false,steps:30})?{return Ok(Some(CommandResult::Retry))}
                }
            },
            0x07=>{self.state.wait_process=7;self.state.wait_text_idle=Some(now);outcome=Outcome::WaitUntil(now.saturating_add(1001));},
            0x10=>{self.state.halted=true;outcome=Outcome::Halted;},
            0x05 => match a[0] {
                254 => {
                    if !self.publish_native_audio(host,true)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.native_live.audio_publication_completed();
                }
                255 => {
                    let volume = self.state.media.saved_music_volume;
                    if volume != 0 {
                        self.state.assign(99, 7, volume)?;
                        self.state.media.saved_music_volume = 0;
                    }
                }
                percent => {
                    let old = self.state.value(99, 7)?;
                    self.state.media.saved_music_volume = old;
                    self.state
                        .assign(99, 7, (i32::from(percent) * old / 100).clamp(0, 100))?;
                }
            },
            0x70=>{
                // Restore the temporary ducking backup even for the same-ID
                // early return or the native255 stop command.
                if self.state.media.saved_music_volume!=0 {
                    self.state.assign(99,7,self.state.media.saved_music_volume)?;
                    self.state.media.saved_music_volume=0;
                    self.state.native_live.music_volume_backup(0);
                }
                if a[0]==255 {
                    if !host.emit(&Event::ReleaseMusic{forced:true,steps:30})?{return Ok(Some(CommandResult::Retry))}
                    self.state.pending_music=None;
                    self.state.last_music=None;
                    self.state.native_live.music(255,"")?;
                }else{
                    let Some(asset)=host.music(a[0] as u16,a[1]<2)?else{return Ok(Some(CommandResult::Retry))};
                    if !asset.name.is_empty(){self.state.assign(96,a[0],1)?;}
                    if !(a[1]==1 && self.state.current_music==a[0]) {self.state.pending_music=Some((asset.clone(),a[1]<2));}
                    self.state.last_music=Some(asset.clone());
                    self.state.native_live.music(a[0],&asset.name)?;
                }
                self.state.current_music=a[0];
            },
            0x6d => {
                //457F70 dispatches to456FA0(max20) or4580D0(max200).
                // Neither helper opens a decoder or starts a presentation.
                if let Some(backend) = self.state.video_backend {
                    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
                    ensure!(
                        !errors && bytes.len() < 82,
                        "Circus native playlist row extent"
                    );
                    let list = self.state.media.playlists.entry(backend).or_default();
                    let capacity = match backend {
                        1 => 20,
                        2 => 200,
                        _ => bail!("Circus native playlist backend"),
                    };
                    if list.len() < capacity {
                        list.push(text.into());
                    }
                }
                self.state.native_live.movie_playlist(text)?;
            }
            0x6f => {
                let looping = a[1] == 0;
                let Some(asset) = host.music(u16::try_from(a[0])?, false)? else {
                    return Ok(Some(CommandResult::Retry));
                };
                if !asset.name.is_empty(){self.state.assign(96,a[0],1)?;}
                self.state.media.resident_pending_music = true;
                self.state.pending_music = Some((asset.clone(), looping));
                if a[0] != 255 {
                    self.state.last_music = Some(asset.clone());
                }
                self.state.current_music = a[0];
                self.state.native_live.music(
                    a[0],
                    &self
                        .state
                        .last_music
                        .as_ref()
                        .map_or("", |asset| asset.name.as_str()),
                )?;
            }
            0xaf=>{
                if a[3]<3 {
                    ensure!(a[3]>=0,"Circus native pan category index");
                    self.state.media.pending_balance.insert(a[3] as u8,(100*i32::from(a[0]),100*i32::from(a[1]),u32::try_from(a[2]).context("Circus native pan duration")?));
                }
            },
            0xab if a[0] == 99 => {
                if !self.reset_comments(host,matches!(a[1],1|2).then_some(a[1]),now,true)?{return Ok(Some(CommandResult::Retry))}
                //457D90 deletes both old player objects before selecting a
                //fresh backend. Its declaration lists cannot survive this.
                self.state.media.playlists.clear();
                self.state.video_backend = matches!(a[1], 1 | 2).then_some(a[1]);
                self.state.native_live.movie_backend_selected(self.state.video_backend)?;
            }
            0xab if a[0]==-1=>{if !self.reset_comments(host,self.state.video_backend,now,false)?{return Ok(Some(CommandResult::Retry))}self.state.native_live.movie_stopped();self.state.media.playlists.clear();self.state.assign(99,186,0)?;self.state.native_flags.insert(272086,0);},
            0xab if self.state.video_backend.is_none() => {} // native unopened-player early return
            _ => return Ok(None),
        }
        if i.opcode==0x05 {
            self.state.native_live.music_volume_backup(self.state.media.saved_music_volume);
        }
        self.state.native_live.media_completed(i.opcode,a[0]);
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome,
        }))
    }
}

impl State {
    ///40DBA0/40DD40 admission and per-character volume, shared by ordinary,
    ///backlog and original-slot voice publication. None is native suppression.
    pub(crate) fn native_voice_volume(&self,name:&str)->Result<Option<(i32,i32)>> {
        let system=name.get(..12).is_some_and(|name|name.eq_ignore_ascii_case("voice\\system"));
        let suffix=name.rsplit('_').next().unwrap_or(name);
        let index=self.voice_prefixes.iter().position(|prefix| {
            suffix.get(..prefix.len()).is_some_and(|name|name.eq_ignore_ascii_case(prefix))
            ||prefix.get(..3).is_some_and(|p|p.eq_ignore_ascii_case("asu"))&&suffix.get(..3).is_some_and(|p|p.eq_ignore_ascii_case("asa"))
            ||prefix.get(..3).is_some_and(|p|p.eq_ignore_ascii_case("ktr"))&&suffix.get(..3).is_some_and(|p|p.eq_ignore_ascii_case("vkt"))
        }).unwrap_or(50) as i16;
        if !system&&self.value(95,index)?==0{return Ok(None)}
        let base=self.value(99,9)?;
        let volume=if system&&self.value(9,98)?==0{base}else{base.wrapping_mul(self.value(94,index)?)/100};
        Ok(Some((if self.value(99,203)?!=0{self.value(99,225)?}else{0},volume)))
    }
}
impl Machine {
    pub(super) fn prepare_restore_audio(&self,plan:&mut crate::native_save::RestorePlan)->Result<()> {
        let mut prepared=Vec::with_capacity(plan.audio.len());
        for mut audio in std::mem::take(&mut plan.audio) {
            if audio.kind==2 {
                let Some(volume)=self.state.native_voice_volume(&audio.name)? else{continue};
                audio.voice_volume=Some(volume);
            }
            prepared.push(audio);
        }
        plan.audio=prepared;Ok(())
    }
}

#[derive(Clone,Copy,Debug)]
pub(crate) struct NativeAudioFramePolicy {
    pub music_volume:i32,pub voice_volume:i32,pub duck_ratio:i32,pub duck_enabled:bool,
    pub master:i32,pub global_enabled:bool,pub auto_dialogue:bool,
}
impl State {
    pub(crate) fn native_audio_frame_policy(&self)->Result<NativeAudioFramePolicy> {
        Ok(NativeAudioFramePolicy{music_volume:self.value(99,7)?,voice_volume:self.value(99,9)?,duck_ratio:self.value(99,206)?,duck_enabled:self.value(99,190)?!=0,master:self.value(99,225)?,global_enabled:self.value(99,203)?!=0,auto_dialogue:self.wait_process==6&&self.wait_click&&!self.history.active()&&!self.menu_wait&&!self.dialogue.panel_wait&&!self.system_ops.gestures.modal_active()&&self.value(99,13)?!=0})
    }
}
