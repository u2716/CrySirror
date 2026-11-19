// SPDX-License-Identifier: MPL-2.0
//! Original resident sample slots over CrySirror PCM leases and channels.

use super::sound::EffectsSettings;
use crate::vm::CmvsVm;
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{
        AssetReader, AudioParameters, AudioRequest, HostServices, PcmAudio, ResidentAudioJob,
        ResidentAudioState,
    },
    state::ClientId,
};
use std::sync::Arc;

pub(super) const SLOT_COUNT: usize = 24;
const DUPLICATES: u32 = 32;
const FIRST_CHANNEL: u32 = 0x1_0000;

struct Sample {
    pcm: PcmAudio,
    primary_gain: f32,
}

pub(super) struct PreparedGroup {
    samples: Vec<(usize, Option<ResidentAudioJob>, Option<PcmAudio>)>,
}

impl PreparedGroup {
    pub fn begin(
        host: &mut dyn HostServices,
        client: ClientId,
        sources: impl IntoIterator<Item = (usize, &'static str, Option<Arc<dyn AssetReader>>)>,
    ) -> Result<Self> {
        let mut samples = Vec::new();
        for (slot, name, source) in sources {
            let job = source
                .map(|source| host.begin_resident_audio(client, source, name, Some(8 * 1024 * 1024)))
                .transpose()?;
            samples.push((slot, job, None));
        }
        Ok(Self { samples })
    }

    pub fn poll(&mut self, client: ClientId) -> Result<bool> {
        let mut ready = true;
        for (_, job_slot, sample) in &mut self.samples {
            let Some(job) = job_slot.as_mut() else { continue };
            ensure!(job.client() == client, "CMVS foreign scene sound job");
            *sample = match job.poll()? {
                ResidentAudioState::Pending => {
                    ready = false;
                    continue;
                }
                ResidentAudioState::Ready(pcm) => Some(pcm),
                ResidentAudioState::Failed(_) => None,
            };
            *job_slot = None;
        }
        Ok(ready)
    }
}

pub(super) struct Samples {
    slots: [Option<Sample>; SLOT_COUNT],
    pending: Option<(usize, ResidentAudioJob)>,
    device_gain: u16,
}

impl Default for Samples {
    fn default() -> Self {
        Self {
            slots: Default::default(),
            pending: None,
            device_gain: 256,
        }
    }
}

impl Samples {
    pub fn replace_group(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        group: PreparedGroup,
    ) -> Result<()> {
        for (slot, _, pcm) in group.samples {
            self.release(host, client, slot)?;
            self.slots[slot] = pcm.map(|pcm| Sample { pcm, primary_gain: 1. });
        }
        Ok(())
    }

    pub fn set_device_gain(&mut self, gain: u16) {
        self.device_gain = gain;
    }

    pub fn load(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        slot: usize,
        name: &str,
        reader: Option<Arc<dyn AssetReader>>,
    ) -> Result<()> {
        ensure!(self.pending.is_none(), "CMVS sample load already pending");
        if let Some(reader) = reader {
            let job = host.begin_resident_audio(client, reader, name, Some(8 * 1024 * 1024))?;
            self.pending = Some((slot, job));
        } else {
            vm.complete_owner_call(2, Some(0))?;
        }
        Ok(())
    }

    pub fn poll(&mut self, client: ClientId, vm: &mut CmvsVm) -> Result<bool> {
        let Some((slot, job)) = self.pending.as_mut() else {
            return Ok(false);
        };
        ensure!(job.client() == client, "CMVS foreign resident audio job");
        let pcm = match job.poll()? {
            ResidentAudioState::Pending => return Ok(true),
            ResidentAudioState::Ready(pcm) => Some(pcm),
            ResidentAudioState::Failed(_) => None,
        };
        vm.complete_owner_call(2, Some(u32::from(pcm.is_some())))?;
        self.slots[*slot] = pcm.map(|pcm| Sample {
            pcm,
            primary_gain: 1.,
        });
        self.pending = None;
        Ok(true)
    }

    pub fn release(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        slot: usize,
    ) -> Result<()> {
        if self.slots[slot].is_some() {
            let base = primary_channel(slot);
            for channel in base..=base + DUPLICATES {
                host.submit_audio(
                    client,
                    AudioRequest::Stop {
                        channel,
                        fade_ms: 0.,
                    },
                )?;
            }
            self.slots[slot] = None;
        }
        Ok(())
    }

    pub fn clear(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        self.pending = None;
        for slot in 0..SLOT_COUNT {
            self.release(host, client, slot)?;
        }
        Ok(())
    }

    pub fn play(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        settings: EffectsSettings,
        slot: usize,
        volume: i32,
        looping: bool,
    ) -> Result<()> {
        if !settings.enabled {
            return Ok(());
        }
        let Some(sample) = &mut self.slots[slot] else {
            return Ok(());
        };
        let gain =
            hardware_gain(settings.level(volume), self.device_gain).unwrap_or(sample.primary_gain);
        let channel = primary_channel(slot);
        if let Some(info) = host.audio_info(client, channel)? {
            host.submit_audio(
                client,
                AudioRequest::SetLooped {
                    channel,
                    looping,
                    loop_start: None,
                },
            )?;
            host.submit_audio(
                client,
                AudioRequest::SetGain {
                    channel,
                    gain,
                    fade_ms: 0.,
                },
            )?;
            if info.paused {
                host.submit_audio(
                    client,
                    AudioRequest::Pause {
                        channel,
                        paused: false,
                        fade_ms: 0.,
                    },
                )?;
            }
        } else {
            host.submit_audio(client, play_sample(channel, &sample.pcm, gain, looping))?;
        }
        sample.primary_gain = gain;
        Ok(())
    }

    pub fn stop(&self, host: &mut dyn HostServices, client: ClientId, slot: usize) -> Result<()> {
        if self.slots[slot].is_some() {
            host.submit_audio(
                client,
                AudioRequest::Pause {
                    channel: primary_channel(slot),
                    paused: true,
                    fade_ms: 0.,
                },
            )?;
        }
        Ok(())
    }

    pub fn duplicate(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        settings: EffectsSettings,
        slot: usize,
    ) -> Result<()> {
        if !settings.enabled {
            return Ok(());
        }
        let Some(sample) = &self.slots[slot] else {
            return Ok(());
        };
        let base = primary_channel(slot);
        for channel in base + 1..=base + DUPLICATES {
            if host.audio_info(client, channel)?.is_none() {
                let gain = hardware_gain(settings.level(0), self.device_gain)
                    .unwrap_or(sample.primary_gain);
                host.submit_audio(client, play_sample(channel, &sample.pcm, gain, false))?;
                break;
            }
        }
        Ok(())
    }
}

fn primary_channel(slot: usize) -> u32 {
    FIRST_CHANNEL + slot as u32 * (DUPLICATES + 1)
}

fn play_sample(channel: u32, pcm: &PcmAudio, gain: f32, looping: bool) -> AudioRequest {
    AudioRequest::PlayPcm {
        channel,
        source: pcm.clone(),
        parameters: AudioParameters {
            gain,
            looping,
            mix_group: Some(super::sound::MIX_GROUP.into()),
            ..AudioParameters::default()
        },
    }
}

// The DirectSound buffer rejects out-of-range volume writes. Its coefficient
// is logarithmic in hundredths of a dB, with the minimum used for zero.
fn hardware_gain(level: u32, device: u16) -> Option<f32> {
    let level = level.wrapping_mul(u32::from(device)) >> 8;
    if level == 0 {
        return Some(0.);
    }
    let db = ((level as f32 / 256.).log10() * 40. * 100.) as i32;
    (-10_000..=0)
        .contains(&db)
        .then(|| 10f64.powf(f64::from(db) / 2000.) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{path::VfsPath, service::FileSystem, state::Command, Error};

    #[derive(Default)]
    struct RecordingHost(Vec<AudioRequest>);

    impl FileSystem for RecordingHost {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::NotFound)
        }

        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
    }

    impl HostServices for RecordingHost {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Ok(())
        }

        fn now_ms(&self) -> u64 {
            0
        }

        fn audio_info(
            &self,
            _: ClientId,
            _: u32,
        ) -> crysirror_core::Result<Option<crysirror_core::service::AudioPlaybackInfo>> {
            Ok(None)
        }

        fn submit_audio(
            &mut self,
            _: ClientId,
            request: AudioRequest,
        ) -> crysirror_core::Result<()> {
            self.0.push(request);
            Ok(())
        }
    }

    #[test]
    fn sample_stop_leaves_shared_pcm_available_and_release_targets_all_slot_instances() {
        let pcm = PcmAudio::new(48_000, vec![[0.5; 2]; 4]).unwrap();
        let mut slots = Samples::default();
        slots.slots[23] = Some(Sample {
            pcm: pcm.clone(),
            primary_gain: 1.,
        });
        slots.set_device_gain(128);
        let settings = EffectsSettings::default();
        let mut host = RecordingHost::default();
        let client = ClientId(4);
        slots
            .play(&mut host, client, settings, 23, 0, true)
            .unwrap();
        slots.stop(&mut host, client, 23).unwrap();
        slots.duplicate(&mut host, client, settings, 23).unwrap();
        let primary = primary_channel(23);
        let [AudioRequest::PlayPcm {
            channel,
            source,
            parameters,
        }, AudioRequest::Pause {
            channel: paused,
            paused: true,
            ..
        }, AudioRequest::PlayPcm {
            channel: duplicate,
            source: copy,
            parameters: one_shot,
        }] = &host.0[..]
        else {
            panic!("unexpected sample control sequence");
        };
        assert_eq!(
            (*channel, *paused, *duplicate),
            (primary, primary, primary + 1)
        );
        assert!(std::ptr::eq(source.frames(), copy.frames()));
        assert!(std::ptr::eq(source.frames(), pcm.frames()));
        assert!(parameters.looping && !one_shot.looping);
        assert!((parameters.gain - 0.25).abs() < 0.001);
        assert_eq!(parameters.gain, one_shot.gain);
        slots.release(&mut host, client, 23).unwrap();
        assert!(slots.slots[23].is_none());
        assert_eq!(host.0.len(), 3 + 33);
        for (channel, request) in (primary..=primary + 32).zip(&host.0[3..]) {
            assert!(
                matches!(request, AudioRequest::Stop { channel: stopped, .. } if *stopped == channel)
            );
        }
    }
}
