//! DC3WY4233A0's eleven local audio records. They are retained across async
//! retries and published at the original dispatch fence, in native order.
//! CrySirror owns decoding, resident/streaming PCM, mixing and playback pools.
use super::*;
use crate::machine::media_ops::AudioDeclaration;
#[path = "dc3_music_policy.rs"]
mod music;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingSound {
    pub name: String,
    pub voice: bool,
    pub looping: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioState {
    pub pending: BTreeMap<u8, PendingSound>,
    pub last_voice: String,
    pub mixer_initialized: bool,
    pub publications: BTreeMap<u8, Vec<u32>>,
    ///Native live4900..4929: three ten-dword envelope declarations.
    pub envelopes: Vec<i32>,
    ///Fourth dword at272 of each stack-local276-byte sound declaration.
    pub local_aux: Vec<i32>,
    pub music_name: String,
    pub music_index: i32,
    #[serde(default)]
    pub deferred_music_volume: i32,
    ///432690: native SFX channel k=1..4 records its file name at
    ///Buf+19728+32k when it starts; only 432490 (a stop) clears it.
    #[serde(default)]
    pub channel_names: [String; 4],
    ///431870 on native channel 1: Buf4972 = loop flag.
    #[serde(default)]
    pub channel_one_loop: bool,
}
impl Default for AudioState {
    fn default() -> Self {
        Self {
            pending: BTreeMap::new(),
            last_voice: String::new(),
            mixer_initialized: false,
            publications: BTreeMap::new(),
            envelopes: vec![0; 30],
            local_aux: vec![0; 11],
            music_name: String::new(),
            music_index: 0,
            deferred_music_volume: 0,
            channel_names: Default::default(),
            channel_one_loop: false,
        }
    }
}
impl AudioState {
    pub(super) fn reset_envelopes(&mut self) {
        self.envelopes = vec![0; 30];
        for base in [0, 10, 20] {
            for field in [4, 7, 8] {
                self.envelopes[base + field] = 100;
            }
        }
    }
}
impl Machine {
    pub(super) fn execute_audio(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>> {
        if let Some(result) = self.execute_music(instruction, a, host)? {
            return Ok(Some(result));
        }
        match instruction.opcode {
            0x05 if a[0] == 254 => return self.publish_audio(host).map(Some),
            0x65 => {
                ensure!(
                    self.state.audio.local_aux.len() == 11,
                    "DC3WY audio declaration auxiliary extent"
                );
                let (slot, value) = match a[0] {
                    0 => (0, 0),
                    1 => (1, 1),
                    value => (7, i32::from(value)),
                };
                self.state.audio.local_aux[slot] = value;
            }
            0x67 if a[0] == 255 => {
                //41DE40(0) stops the sole native music channel. The
                //original end-of-case write deliberately leaves index255.
                if !host.emit(&Event::StopAudio { channel: 0 })? {
                    return Ok(Some(false));
                }
                self.state.audio.pending.remove(&0);
                self.state.audio.music_name.clear();
                self.state.audio.music_index = 255;
                *self
                    .state
                    .audio
                    .local_aux
                    .get_mut(0)
                    .context("DC3WY music auxiliary extent")? = 0;
            }
            0x45 | 0x46 => {
                ensure!(
                    text.len() < 260 && !text.contains('\0'),
                    "DC3WY voice declaration extent"
                );
                if !host.emit(&Event::ReleaseAudioGroup {
                    kind: 2,
                    forced: true,
                    steps: 0,
                })? {
                    return Ok(Some(false));
                }
                self.state.audio.pending.insert(
                    7,
                    PendingSound {
                        name: text.to_owned(),
                        voice: true,
                        looping: false,
                    },
                );
                self.state.audio.last_voice = text.to_owned();
            }
            0x68 if a[0] >= 0 => {
                let Some(slot) = (1..=4).find(|slot| !self.state.audio.pending.contains_key(slot))
                else {
                    return Ok(Some(true));
                };
                let Some(name) = host.sound_name(a[0] as u16)? else {
                    return Ok(Some(false));
                };
                ensure!(
                    name.len() < 260 && !name.contains('\0'),
                    "DC3WY SFX declaration extent"
                );
                self.state.audio.pending.insert(
                    slot,
                    PendingSound {
                        name,
                        voice: false,
                        looping: a[1] != 0,
                    },
                );
            }
            //4233A0 case 0x12 (dispatch, not replay): 41DDF0(0) ->
            //4324F0(0) -> 431DC0(0), the 60-tick music release to -10000,
            //then Buf19720 name and 4938 index are cleared and the local
            //music record v401[0..276] is zeroed. 4324F0's this[139]
            //deferral (a queued replacement) is the adapter's music queue.
            0x12 => {
                if !host.emit(&Event::ReleaseAudioGroup { kind: 0, forced: false, steps: 60 })? {
                    return Ok(Some(false));
                }
                self.state.audio.pending.remove(&0);
                self.state.audio.music_name.clear();
                self.state.audio.music_index = 0;
                *self.state.audio.local_aux.get_mut(0).context("DC3WY music auxiliary extent")? = 0;
            }
            0x68 if a[0] == -2 => {
                if !host.emit(&Event::ReleaseAudioGroup {
                    kind: 2,
                    forced: true,
                    steps: 0,
                })? {
                    return Ok(Some(false));
                }
                self.state.audio.pending.retain(|slot, _| *slot < 7);
                self.state.audio.last_voice.clear();
            }
            0x68 if a[0] == -1 => {
                //41DDF0/4324F0/431DC0 releases native1..4 to-10000db100
                //over60 render ticks. The shared Audio release queue owns
                //all sample/envelope processing and completion.
                if !host.emit(&Event::ReleaseAudioGroup {
                    kind: 1,
                    forced: false,
                    steps: 60,
                })? {
                    return Ok(Some(false));
                }
                self.state
                    .audio
                    .pending
                    .retain(|slot, _| !(1..=4).contains(slot));
                //432490 clears the names when the 60-tick release stops
                //each channel; the projection takes the stopped state.
                self.state.audio.channel_names = Default::default();
                self.state.audio.channel_one_loop = false;
            }
            _ => return Ok(None),
        }
        Ok(Some(true))
    }
    pub(crate) fn publish_audio(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let controls = controls(&self.state.system_data)?;
        if !self.state.audio.mixer_initialized {
            if !host.emit(&Event::MixerControls {
                enabled: controls.enabled,
                gain: controls.gain,
                playback_gain: controls.playback_gain,
            })? {
                return Ok(false);
            }
            self.state.audio.mixer_initialized = true;
        }
        while let Some((&slot, sound)) = self.state.audio.pending.first_key_value() {
            let sound = sound.clone();
            if slot == 0 {
                ensure!(self.state.audio.local_aux.first().copied().unwrap_or(0) == 0,
                    "DC3 native music start-position byte geometry is not connected");
                let Some(asset) = host.music_by_name(&sound.name, sound.looping)? else {return Ok(false);};
                if !host.emit(&Event::PlayMusic {asset, looping: sound.looping})? {return Ok(false);}
                self.state.audio.publications.insert(0, if controls.enabled[0] && !sound.name.is_empty() {vec![0]} else {vec![]});
                self.state.audio.pending.remove(&0);
                continue;
            }
            let voice_volume = if sound.voice {
                let (enabled, percent) = self
                    .state
                    .system_data
                    .voice_policy(&sound.name, self.state.source_count)?;
                //40CD20's disabled character/fallback is an original early
                //return before loading a buffer, not a silent decoder ACK.
                if !enabled {
                    self.state.audio.pending.remove(&slot);
                    continue;
                }
                let master = if self.state.system_data.prefix_value(203)?.unwrap_or(0) != 0 {
                    self.state.system_data.prefix_value(225)?.unwrap_or(0)
                } else {
                    0
                };
                Some((master, percent))
            } else {
                None
            };
            let declaration_name = sound.name.clone();
            let declaration_looping = sound.looping;
            let declaration = AudioDeclaration {
                name: sound.name,
                voice: sound.voice,
                looping: sound.looping,
                voice_volume,
            };
            let Some(publication) = host.publish_audio(&declaration)? else {
                return Ok(false);
            };
            if !sound.voice {
                //Adapter channel c is native channel c-1 (music 0, SFX 1..4).
                for channel in &publication.channels {
                    if let Some(native) = channel.checked_sub(1).filter(|native| (1..=4).contains(native)) {
                        self.state.audio.channel_names[native as usize - 1] = declaration_name.clone();
                        if native == 1 {
                            self.state.audio.channel_one_loop = declaration_looping;
                        }
                    }
                }
            }
            self.state
                .audio
                .publications
                .insert(slot, publication.channels);
            self.state.audio.pending.remove(&slot);
        }
        Ok(true)
    }
}
pub(super) fn controls(
    data: &crate::dc3_settings::SystemData,
) -> Result<crate::machine::MixerControls> {
    let get = |selector| -> Result<i32> {
        data.prefix_value(selector)?
            .context("DC3WY audio preference selector")
    };
    let enabled = [get(14)? != 0, get(15)? != 0, get(5)? != 0];
    let master = get(225)?;
    let global = get(203)? != 0;
    let mut gain = [0.; 3];
    for (index, volume) in [get(7)?, get(8)?, get(9)?].into_iter().enumerate() {
        //431F50 passes this signed db100 curve to DirectSound. Pixel/sample
        //processing remains in the shared mixer; this is native value policy.
        let db = if global && master != 0 && volume != 0 {
            5000i32.wrapping_mul(volume.wrapping_mul(master) / 100) / 100 - 5000
        } else {
            -10000
        };
        ensure!(
            (-10000..=0).contains(&db),
            "DC3WY native mixer gain outside DirectSound range"
        );
        gain[index] = 10f32.powf(db as f32 / 2000.);
    }
    Ok(crate::machine::MixerControls {
        enabled,
        gain,
        playback_gain: gain.map(Some),
    })
}
