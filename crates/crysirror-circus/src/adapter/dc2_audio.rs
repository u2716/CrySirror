//! Per-channel native gain/pan are attached to shared Audio preparation. The
//! source remains paused until its original dB balance has been applied.
use super::*;
use crate::dc2_audio::Request;
#[derive(Default)]
pub(super) struct DirectAudio {
    pending: Option<(Request, u64)>,
    stream_path: Option<(Request, Receiver<std::result::Result<VfsPath, String>>)>,
}
impl DirectAudio {
    pub(super) fn pending(&self) -> bool {
        self.pending.is_some() || self.stream_path.is_some()
    }
}
impl Bridge<'_> {
    pub(in crate::adapter) fn dc2_music_lookup(
        &mut self,
        id: u16,
    ) -> Result<Option<crate::dc2_audio::MusicLookup>> {
        use crate::dc2_audio::MusicLookup;
        if self.services.dc2_names.is_none() {
            let Some(Prepared::Names(names)) = self.services.prepare(Work::Names, self.client)?
            else {
                return Ok(None);
            };
            self.services.names = Some(names);
        }
        let names = self
            .services
            .dc2_names
            .as_ref()
            .context("DC2 native music directory owner")?;
        let Some(row) = names.music.get(usize::from(id)) else {
            return Ok(Some(MusicLookup::Missing));
        };
        //426D40 reads only name0..127 and signed attenuation188..191.
        //The remaining record bytes describe gallery labels, not loop points.
        let attenuation = i32::from_le_bytes(row.bytes()[188..192].try_into()?);
        Ok(Some(MusicLookup::Found {
            name: row.name()?,
            attenuation_db100: attenuation.wrapping_mul(-100),
        }))
    }
    ///427070's SE directory row: name0..127, signed attenuation128..131.
    pub(in crate::adapter) fn dc2_sound_lookup(
        &mut self,
        id: u16,
    ) -> Result<Option<crate::dc2_audio::MusicLookup>> {
        use crate::dc2_audio::MusicLookup;
        if self.services.dc2_names.is_none() {
            let Some(Prepared::Names(names)) = self.services.prepare(Work::Names, self.client)?
            else {
                return Ok(None);
            };
            self.services.names = Some(names);
        }
        let names = self.services.dc2_names.as_ref().context("DC2 native SE directory owner")?;
        let Some(row) = names.sfx.get(usize::from(id)) else {
            return Ok(Some(MusicLookup::Missing));
        };
        let attenuation = i32::from_le_bytes(row.bytes()[128..132].try_into()?);
        Ok(Some(MusicLookup::Found { name: row.name()?, attenuation_db100: attenuation.wrapping_mul(-100) }))
    }
    pub(in crate::adapter) fn close_dc2_audio(&mut self) -> Result<()> {
        self.services.dc2_audio.stream_path = None;
        if let Some((_, ticket)) = self.services.dc2_audio.pending.take() {
            self.host.release_audio(self.client, ticket)?;
        }
        for channel in [0, 2, 3, 8] {
            self.host.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
        }
        Ok(())
    }
    pub(in crate::adapter) fn dc2_direct_audio(&mut self, request: &Request) -> Result<bool> {
        ensure!(
            self.services.native_profile == Some(crate::Profile::Dc2PlusCommunication),
            "DC2 direct audio profile"
        );
        ensure!(
            matches!(request.channel, 0 | 2 | 3 | 8)
                && (-10000..=0).contains(&request.gain_db100)
                && (-10000..=10000).contains(&request.balance_db100),
            "DC2 direct audio native channel/gain/balance"
        );
        if let Some((expected, ticket)) = &self.services.dc2_audio.pending {
            ensure!(expected == request, "DC2 audio preparation receipt owner");
            let ticket = *ticket;
            let prepared = self.host.audio_preparation(self.client, ticket)?;
            match prepared.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => bail!(
                    "DC2 shared audio preparation: {}",
                    prepared.error.unwrap_or_else(|| "unknown error".into())
                ),
                AudioPreparePhase::Ready => {
                    self.host.submit_audio(
                        self.client,
                        AudioRequest::Balance {
                            channel: request.channel,
                            command: crysirror_core::service::AudioBalanceCommand::Set {
                                db100: request.balance_db100,
                                cancel_track: true,
                            },
                        },
                    )?;
                    self.host.submit_audio(
                        self.client,
                        AudioRequest::Pause {
                            channel: request.channel,
                            paused: false,
                            fade_ms: 0.,
                        },
                    )?;
                    self.host.release_audio(self.client, ticket)?;
                    self.services.dc2_audio.pending = None;
                    return Ok(true);
                }
            }
        }
        let parameters = AudioParameters {
            gain: 10f32.powf(request.gain_db100 as f32 / 2000.),
            bus: if request.channel == 0 {
                PcmBus::Music
            } else if request.voice {
                PcmBus::Voice
            } else {
                PcmBus::Sfx
            },
            looping: request.looping,
            paused: true,
            ..Default::default()
        };
        if request.streamed {
            ensure!(
                request.channel == 0 && !request.voice,
                "DC2 streamed native music category"
            );
            if self.services.dc2_audio.stream_path.is_none() {
                let storage = self
                    .services
                    .storage
                    .as_ref()
                    .context("DC2 streamed audio storage")?
                    .clone();
                let name = request.name.clone();
                let (sender, receiver) = mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .name("circus-dc2-music-path".into())
                    .spawn(move || {
                        let result =
                            dc2_audio_files::loose_path(storage.as_ref(), &name.replace('\\', "/"));
                        let _ = sender.send(result.map_err(|error| format!("{error:#}")));
                    })?;
                self.services.dc2_audio.stream_path = Some((request.clone(), receiver));
                return Ok(false);
            }
            let (expected, receiver) = self.services.dc2_audio.stream_path.as_ref().unwrap();
            ensure!(expected == request, "DC2 streamed path completion owner");
            let path = match receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(false),
                Err(TryRecvError::Disconnected) => bail!("DC2 streamed path worker disconnected"),
                Ok(path) => path.map_err(anyhow::Error::msg)?,
            };
            self.services.dc2_audio.stream_path = None;
            let ticket = self.host.begin_audio(
                self.client,
                AudioRequest::PlayAsset {
                    channel: 0,
                    source: crysirror_core::service::AudioAsset {
                        path,
                        frame_limit: None,
                    },
                    intro: None,
                    parameters,
                },
            )?;
            self.services.dc2_audio.pending = Some((request.clone(), ticket));
            return Ok(false);
        }
        let Some(Prepared::Audio(bytes, archive)) = self.services.prepare(
            Work::Audio(request.name.clone(), request.voice),
            self.client,
        )?
        else {
            return Ok(false);
        };
        if let Some(archive) = archive {
            self.services.voice_archive = Some(archive)
        }
        let source = EncodedAudio {
            bytes,
            frame_limit: None,
        };
        let ticket = self.host.begin_audio(
            self.client,
            AudioRequest::PlayResident {
                channel: request.channel,
                source,
                intro: None,
                parameters,
            },
        )?;
        self.services.dc2_audio.pending = Some((request.clone(), ticket));
        Ok(false)
    }
}
