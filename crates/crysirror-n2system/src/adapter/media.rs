// SPDX-License-Identifier: MPL-2.0
//! Native media identities over engine decoder resources and transport clocks.
//! Core media argument/wait policy: NitroSharp legacy 36701328 (MIT).
use super::*;
use crysirror_core::service::{AudioResourceHandle, AudioResourceInfo, MediaPhase, MediaRequest};

pub(super) struct SoundResource {
    pub rate_tween: Option<crysirror_scene::animation::ScalarTween>,
    pub echo: Option<crysirror_core::service::StereoEchoSpec>,
    pub stereo_effect: Option<crysirror_core::service::StereoEffectSpec>,
    pub handle: Option<AudioResourceHandle>,
    pub ticket: Option<u64>,
    pub info: Option<AudioResourceInfo>,
    pub pan: f32,
    pub rate: f32,
    pub position_ms: u64,
    pub loop_region: Option<[u64; 2]>,
}
impl Default for SoundResource {
    fn default() -> Self {
        Self {
            rate_tween: None,
            echo: None,
            stereo_effect: None,
            handle: None,
            ticket: None,
            info: None,
            pan: 0.5,
            rate: 1.,
            position_ms: 0,
            loop_region: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Movie {
    pub source: String,
    pub looping: bool,
    pub gain: f32,
    pub surface: String,
}
impl N2SystemVm {
    pub(super) fn sound_ready(&mut self, host: &mut dyn HostServices, id: u32) -> Result<bool> {
        let Kind::Sound { source, .. } = &self
            .entities
            .nodes
            .get(&id)
            .context("N2System sound identity missing")?
            .kind
        else {
            bail!("N2System non-sound preparation")
        };
        let sound = self
            .sound_resources
            .get_mut(&id)
            .context("N2System missing sound resource")?;
        if sound.handle.is_none() && sound.ticket.is_none() {
            sound.ticket = Some(host.begin_audio_resource(
                self.client,
                AudioAsset {
                    path: VfsPath::new(Mount::Asset, source)?,
                    frame_limit: None,
                },
                false,
            )?);
        }
        if let Some(ticket) = sound.ticket {
            let preparation = host.audio_resource_preparation(self.client, ticket)?;
            match preparation.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => bail!(
                    "N2System sound preparation: {}",
                    preparation.error.unwrap_or_default()
                ),
                AudioPreparePhase::Ready => {
                    let handle = preparation
                        .resource
                        .context("N2System missing prepared sound")?;
                    let info = preparation
                        .info
                        .context("N2System missing sound metadata")?;
                    if let Some(old) = sound.handle.replace(handle) {
                        host.release_audio_resource(self.client, old)?;
                    }
                    sound.info = Some(info);
                    sound.ticket = None;
                    host.release_audio_resource_preparation(self.client, ticket)?;
                }
            }
        }
        Ok(true)
    }
    pub(super) fn play_sound(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut Execution,
        id: u32,
    ) -> Result<()> {
        let Kind::Sound {
            kind,
            gain,
            looping,
            playing,
            ..
        } = &mut self
            .entities
            .nodes
            .get_mut(&id)
            .context("N2System sound entity missing")?
            .kind
        else {
            bail!("N2System Play on non-sound")
        };
        let resource = self
            .sound_resources
            .get(&id)
            .context("N2System sound not prepared")?;
        let info = resource
            .info
            .context("N2System sound metadata not prepared")?;
        let paused_for_effect = resource.echo.is_some() || resource.stereo_effect.is_some();
        let parameters = AudioParameters {
            paused: paused_for_effect,
            gain: *gain,
            looping: *looping,
            rate: if resource.rate > 0. {
                resource.rate
            } else {
                1.
            },
            transport_active: resource.rate > 0.,
            pan: resource.pan,
            start_ms: resource.position_ms,
            mix_group: Some(format!("n2system-{kind}")),
            loop_start: resource
                .loop_region
                .map_or(0, |r| r[0].saturating_mul(info.source_rate.into()) / 1000),
            loop_end: resource
                .loop_region
                .map(|r| r[1].saturating_mul(info.source_rate.into()) / 1000),
            ..Default::default()
        };
        let ticket = host.begin_audio(
            self.client,
            AudioRequest::PlayResource {
                channel: id,
                resource: resource.handle.context("N2System sound handle missing")?,
                parameters,
            },
        )?;
        state.audio.push(ticket);
        if paused_for_effect { state.dsp_paused.insert(ticket); }
        *playing = true;
        Ok(())
    }
    pub(super) fn connect_sound_effects(&self, host: &mut dyn HostServices, channel: u32, paused_for_effect: bool) -> Result<()> {
        let Some(sound) = self.sound_resources.get(&channel) else {
            host.submit_audio(self.client, AudioRequest::Stop {channel,fade_ms:0.})?;
            return Ok(());
        };
        if sound.echo.is_some() || sound.stereo_effect.is_some() {
            host.submit_audio(self.client,AudioRequest::SetEcho {channel,effect:sound.echo})?;
            host.submit_audio(self.client,AudioRequest::SetStereoEffect {channel,effect:sound.stereo_effect})?;
        }
        // A clear/replacement while decoding may have removed the DSP; the
        // voice must still resume if this particular play was parked for it.
        if paused_for_effect {
            host.submit_audio(self.client,AudioRequest::Pause {channel,paused:false,fade_ms:0.})?;
        }
        Ok(())
    }
    pub(super) fn native_media(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let mut value = Value::Null;
        match call.function {
            Builtin::MoviePlay => return self.native_modal_movie(host, current, state, args),
            Builtin::SetFrequency => {
                let duration = args.time(1)?;
                let rate = args.integer(2)? as f32 / 1000.;
                ensure!((0. ..=32.).contains(&rate), "N2System source speed range");
                let curve: crysirror_scene::layer::Easing =
                    serde_json::from_value(easing(args, 3)?)?;
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(resource) = self.sound_resources.get_mut(&id) {
                        resource.rate_tween = Some(crysirror_scene::animation::ScalarTween::new(
                            resource.rate,
                            rate,
                            host.now_ms(),
                            duration,
                            curve,
                        )?);
                    }
                }
                self.sample_audio_rates(host)?;
            }
            Builtin::SoundEffect => {
                // The shipped NSS declares DirectSound's Echo argument order:
                // wet/dry percent, feedback percent, left/right milliseconds.
                let kind = args.string(1)?.to_ascii_uppercase();
                let echo = if kind == "ECHO" {
                    Some(crysirror_core::service::StereoEchoSpec {
                        wet: args.integer(2)? as f32 / 100., feedback: args.integer(3)? as f32 / 100.,
                        delay_ms: [args.time(4)? as f32, args.time(5)? as f32],
                        cross_feedback: if args.len() > 6 { args.truth(6)? } else { false },
                    })
                } else { None };
                // Trial 414580/454470/482730: Chorus takes wet, depth,
                // feedback and delay; sine frequency 1.3 Hz and phase +90°.
                // 482630 fixes WavesReverb to 0 dB gains, 1000 ms, HF .001.
                let effect = match kind.as_str() {
                    "CHORUS" => Some(crysirror_core::service::StereoEffectSpec::Chorus {
                        wet: args.integer(2)? as f32 / 100., depth: args.integer(3)? as f32 / 100.,
                        feedback: args.integer(4)? as f32 / 100., delay_ms: args.time(5)? as f32,
                        frequency_hz: 1.3, phase_degrees: 90.,
                    }),
                    "WAVESREVERB" => Some(crysirror_core::service::StereoEffectSpec::Reverb {
                        input_gain: 1., wet: 1., decay_seconds: 1., damping: 0.999,
                    }),
                    // Native 454470/482C50 removes the effect for other names.
                    _ => None,
                };
                if let Some(spec) = echo { spec.validate()?; }
                if let Some(spec) = effect { spec.validate()?; }
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(resource) = self.sound_resources.get_mut(&id) {
                        resource.echo = echo;
                        resource.stereo_effect = effect;
                        if host.audio_info(self.client, id)?.is_some() {
                            host.submit_audio(self.client, AudioRequest::SetEcho { channel: id, effect: echo })?;
                            host.submit_audio(self.client, AudioRequest::SetStereoEffect { channel: id, effect })?;
                        }
                    }
                }
            }
            Builtin::CreateSound => {
                // AudioEntity(77a93db4) retains the source identity and opens
                // playback lazily. Do not snapshot every BGM at startup.
                let source = resource::resolve(self.catalog.as_ref().unwrap(), &args.string(2)?)?;
                let kind = args.constant(1)?;
                ensure!(
                    matches!(kind, "BGM" | "SE" | "Voice"),
                    "N2System sound bus {kind}"
                );
                let id = self.create(
                    host,
                    current,
                    &args.string(0)?,
                    Kind::Sound {
                        source,
                        kind: kind.into(),
                        gain: 1.,
                        looping: false,
                        playing: false,
                    },
                    [0.; 2],
                )?;
                self.sound_resources.insert(id, SoundResource::default());
            }
            Builtin::CreateMovie => {
                let id = if let Some(id) = state.movie_init {
                    id
                } else {
                    let source =
                        resource::resolve(self.catalog.as_ref().unwrap(), &args.string(6)?)?;
                    let alpha = args.truth(5)?;
                    let looping = args.truth(4)?;
                    let gain = if args.len() > 7 {
                        args.integer(7)? as f32 / 1000.
                    } else {
                        1.
                    };
                    let id = self.create(host, current, &args.string(0)?, Kind::Visual, [1.; 2])?;
                    let request = if alpha {
                        MediaRequest::PlayAlpha { id, asset: source.clone(), looping, audio: true, gain }
                    } else {
                        MediaRequest::Play { id, asset: source.clone(), looping, audio: true, gain }
                    };
                    let surface = host.media_request(self.client, request)?.relative().to_owned();
                    self.movies.insert(
                        id,
                        Movie {
                            source,
                            looping,
                            gain,
                            surface,
                        },
                    );
                    state.movie_init = Some(id);
                    id
                };
                let info = host.media_info(self.client, id)?;
                ensure!(
                    info.phase != MediaPhase::Failed,
                    "N2System video decoder failed"
                );
                let Some(size) = info.size else {
                    return Ok(false);
                };
                self.entities.nodes.get_mut(&id).unwrap().size = size.map(|n| n as f32);
                let parent = self.entities.render_parent(id)?;
                let x = self.native_position(host, id, args.value(2), 0., size[0] as f32, 0)?;
                let y = self.native_position(host, id, args.value(3), 0., size[1] as f32, 1)?;
                self.submit(host,vec![json!({"op":"put","layer":{"id":id,"parent":parent,"order":[args.priority(1)?,0],"rect":[x,y,size[0],size[1]],"paint":{"kind":"image","asset":self.movies[&id].surface}}})],vec![])?;
                state.movie_init = None;
            }
            Builtin::SetLoopPoint => {
                let range = [args.time(1)?, args.time(2)?];
                // The Windows NSS startup registers short tracks with(0,0),
                // denoting the whole source rather than an empty frame range.
                ensure!(
                    range == [0, 0] || range[1] > range[0],
                    "N2System loop region"
                );
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(resource) = self.sound_resources.get_mut(&id) {
                        resource.loop_region = (range != [0, 0]).then_some(range);
                        if let Kind::Sound {
                            playing, looping, ..
                        } = &mut self.entities.nodes.get_mut(&id).unwrap().kind
                        {
                            *looping = true;
                            if *playing {
                                let rate =
                                    resource.info.context("N2System loop metadata")?.source_rate
                                        as u64;
                                host.submit_audio(
                                    self.client,
                                    AudioRequest::SetLoopSchedule {
                                        channel: id,
                                        start: range[0].saturating_mul(rate) / 1000,
                                        end: (range[1] != 0)
                                            .then(|| range[1].saturating_mul(rate) / 1000),
                                        passes: None,
                                    },
                                )?;
                            }
                        }
                    }
                }
            }
            Builtin::SetPan => {
                let duration = args.time(1)?;
                let pan = (args.integer(2)? as f32 / 100. + 1.) * 0.5;
                ensure!((0. ..=1.).contains(&pan), "N2System pan range");
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(resource) = self.sound_resources.get_mut(&id) {
                        resource.pan = pan;
                        if host.audio_info(self.client, id)?.is_some() {
                            host.submit_audio(
                                self.client,
                                AudioRequest::SetPan {
                                    channel: id,
                                    pan,
                                    fade_ms: duration as f32,
                                },
                            )?;
                        }
                    }
                }
            }
            Builtin::SetStream => {
                if !state.seeks.is_empty() {
                    let mut pending = Vec::new();
                    for ticket in std::mem::take(&mut state.seeks) {
                        let p = host.audio_seek_preparation(self.client, ticket)?;
                        match p.phase {
                            AudioPreparePhase::Pending => pending.push(ticket),
                            AudioPreparePhase::Ready => {
                                host.release_audio_seek(self.client, ticket)?
                            }
                            AudioPreparePhase::Failed => {
                                bail!("N2System stream seek: {}", p.error.unwrap_or_default())
                            }
                        }
                    }
                    state.seeks = pending;
                    if !state.seeks.is_empty() {
                        return Ok(false);
                    }
                } else {
                    let position = args.time(1)?;
                    for id in self.query(current, &args.string(0)?)? {
                        if let Some(resource) = self.sound_resources.get_mut(&id) {
                            resource.position_ms = position;
                            if host.audio_info(self.client, id)?.is_some() {
                                let rate = resource
                                    .info
                                    .context("N2System stream metadata")?
                                    .source_rate as u64;
                                state.seeks.push(host.begin_audio_seek(
                                    self.client,
                                    id,
                                    position.saturating_mul(rate) / 1000,
                                )?);
                            }
                        } else if self.movies.contains_key(&id) {
                            host.media_request(
                                self.client,
                                MediaRequest::Seek {
                                    id,
                                    position_ms: position,
                                },
                            )?;
                        }
                    }
                    if !state.seeks.is_empty() {
                        return Ok(false);
                    }
                }
            }
            Builtin::DurationTime | Builtin::PassageTime | Builtin::RemainTime => {
                let mut answer = 0u64;
                for id in self.query(current, &args.string(0)?)? {
                    if self.sound_resources.contains_key(&id) {
                        if !self.sound_ready(host, id)? {
                            return Ok(false);
                        }
                        let resource = &self.sound_resources[&id];
                        let info = resource.info.unwrap();
                        if info.source_frames.is_none() {
                            let Kind::Sound { source, .. } = &self.entities.nodes[&id].kind else {
                                unreachable!()
                            };
                            let ticket = host.begin_audio_resource(
                                self.client,
                                AudioAsset {
                                    path: VfsPath::new(Mount::Asset, source)?,
                                    frame_limit: None,
                                },
                                true,
                            )?;
                            self.sound_resources.get_mut(&id).unwrap().ticket = Some(ticket);
                            return Ok(false);
                        }
                        let duration = info.source_frames.unwrap().saturating_mul(1000)
                            / u64::from(info.source_rate);
                        let position = host
                            .audio_info(self.client, id)?
                            .map_or(resource.position_ms, |p| {
                                p.position_frames.saturating_mul(1000) / u64::from(p.source_rate)
                            });
                        answer = match call.function {
                            Builtin::DurationTime => duration,
                            Builtin::PassageTime => position,
                            _ => duration.saturating_sub(position),
                        };
                    } else if self.movies.contains_key(&id) {
                        let info = host.media_info(self.client, id)?;
                        if info.phase == MediaPhase::Loading {
                            return Ok(false);
                        }
                        let duration = info
                            .duration_ms
                            .context("N2System video duration is unavailable")?;
                        answer = match call.function {
                            Builtin::DurationTime => duration,
                            Builtin::PassageTime => info.position_ms,
                            _ => duration.saturating_sub(info.position_ms),
                        };
                    }
                    if answer > 0 {
                        break;
                    }
                }
                value = Value::Number(answer as f32);
            }
            _ => bail!("N2System non-media call"),
        }
        self.machine.complete(&mut state.thread, value)?;
        Ok(true)
    }
    pub(super) fn sample_audio_rates(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        for (&id, resource) in &mut self.sound_resources {
            if let Some(track) = resource.rate_tween {
                resource.rate = track.sample(now);
                if host.audio_info(self.client, id)?.is_some() {
                    if resource.rate > 0. {
                        host.submit_audio(
                            self.client,
                            AudioRequest::SetRate {
                                channel: id,
                                rate: resource.rate.max(1. / 4096.),
                            },
                        )?;
                        host.submit_audio(
                            self.client,
                            AudioRequest::SetTransportActive {
                                channel: id,
                                active: true,
                            },
                        )?;
                    } else {
                        host.submit_audio(
                            self.client,
                            AudioRequest::SetTransportActive {
                                channel: id,
                                active: false,
                            },
                        )?;
                    }
                }
                if track.complete(now) {
                    resource.rate_tween = None;
                }
            }
        }
        Ok(())
    }
}
