// SPDX-License-Identifier: MPL-2.0
//! V1 effect commands can address the loose voice bank instead of the WAV bank.
use anyhow::{Result, bail, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioParameters, AudioPcm16Gain, AudioRequest, HostServices, PcmBus, ResidentAudioJob,
        ResidentAudioState,
    },
    state::ClientId,
};
use crysirror_resource::{DecodeContext, DecodedFile, FileFormat, exhibit_pcm::ExhibitPcm};
use crysirror_tasks::{Task, TaskPool};

pub(super) const CHANNEL: u32 = 2;
pub(super) struct Voice {
    source: Source,
    pub id: i32,
    pub wait: bool,
    stereo: [u32; 2],
}
enum Source {
    Decoding(Task<Option<DecodedFile>>),
    Preparing(ResidentAudioJob),
}
impl Voice {
    pub fn begin(
        h: &mut dyn HostServices,
        worker: &TaskPool,
        c: ClientId,
        id: i32,
        wait: bool,
        stereo: [u32; 2],
    ) -> Result<Self> {
        let storage = h.storage(c)?;
        let path = VfsPath::new(Mount::Asset, &format!("res/s/v/{:02}/{id:05}.v", id / 1000))?;
        let task = worker.spawn(move |cancel| {
            cancel.check()?;
            if storage.metadata(Mount::Asset, path.relative())?.is_none() {
                return Ok(None);
            }
            let input = storage.open_read(&path)?;
            let ctx = DecodeContext::new(c).with_cancellation(cancel);
            Ok(Some(ExhibitPcm.decode(path.relative(), input, &ctx)?))
        })?;
        Ok(Self {
            source: Source::Decoding(task),
            id,
            wait,
            stereo,
        })
    }
    pub fn poll(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        if let Source::Decoding(task) = &mut self.source {
            let Some(file) = task.try_take()? else {
                return Ok(false);
            };
            // playVoice checks for existence before touching the current voice.
            let Some(file) = file else { return Ok(true) };
            self.source = Source::Preparing(h.begin_resident_audio(
                c,
                file.reader,
                "exhibit-voice.wav",
                None,
            )?);
        }
        if let Source::Preparing(job) = &mut self.source {
            ensure!(job.client() == c, "foreign ExHibit voice preparation");
            match job.poll()? {
                ResidentAudioState::Pending => return Ok(false),
                ResidentAudioState::Failed(reason) => {
                    bail!("ExHibit voice preparation failed: {reason:?}")
                }
                ResidentAudioState::Ready(pcm) => {
                    h.submit_audio(
                        c,
                        AudioRequest::PlayPcm {
                            channel: CHANNEL,
                            source: pcm,
                            parameters: AudioParameters {
                                bus: PcmBus::Voice,
                                pcm16_gain: Some(AudioPcm16Gain {
                                    stereo: self.stereo,
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        },
                    )?;
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::{PcmAudio, ResidentAudioProcess};

    struct Prepared(ClientId);
    impl ResidentAudioProcess for Prepared {
        fn client(&self) -> ClientId {
            self.0
        }
        fn poll(&mut self) -> crysirror_core::Result<ResidentAudioState> {
            Ok(ResidentAudioState::Ready(PcmAudio::new(
                44100,
                vec![[0.25, -0.25]],
            )?))
        }
    }

    #[test]
    fn native_voice_is_published_on_its_bus_with_scoped_pcm_and_stereo_gain() -> Result<()> {
        let client = ClientId(7);
        let mut host = crate::sound::tests::Host::default();
        let mut voice = Voice {
            source: Source::Preparing(Box::new(Prepared(client))),
            id: 42,
            wait: false,
            stereo: [64, 128],
        };
        assert!(voice.poll(&mut host, ClientId(8)).is_err());
        assert!(host.requests.is_empty());
        assert!(voice.poll(&mut host, client)?);
        let [
            AudioRequest::PlayPcm {
                channel,
                source,
                parameters,
            },
        ] = host.requests.as_slice()
        else {
            bail!("expected a resident voice request");
        };
        assert_eq!(*channel, CHANNEL);
        assert!(matches!(parameters.bus, PcmBus::Voice));
        assert_eq!(parameters.pcm16_gain.unwrap().stereo, [64, 128]);
        assert_eq!(source.frames(), &[[0.25, -0.25]]);
        Ok(())
    }
}
