// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS sound natives over the original sound task (5EAEA0, messages 181..208). Only its
//! channel and group policy lives here; decoding, mixing and fades are shared audio.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{AudioAsset, AudioParameters, AudioPreparePhase, AudioRequest, AudioResourceHandle, HostServices, PcmBus},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::collections::BTreeMap;

struct Channel {
    resource: AudioResourceHandle,
    name: Vec<u8>,
    group: Vec<u8>,
    percent: u32,
}
struct Load {
    channel: u32,
    name: Vec<u8>,
    group: Vec<u8>,
    worker: Option<Task<crate::fes_live_sound::Located>>,
    ticket: Option<u64>,
}
#[derive(Default)]
pub(crate) struct KcsSound {
    channels: BTreeMap<u32, Channel>,
    load: Option<Load>,
    play: Option<(u32, u64)>,
}
pub(crate) enum Progress<T> {
    Pending,
    Done(T),
}
impl KcsSound {
    fn retire(&mut self, host: &mut dyn HostServices, client: ClientId, channel: u32) -> Result<()> {
        if let Some(slot) = self.channels.remove(&channel) {
            host.submit_audio(client, AudioRequest::Stop { channel, fade_ms: 0. })?;
            host.release_audio_resource(client, slot.resource)?;
        }
        Ok(())
    }
    /// 645330/6454B0 → messages 205..208 → 5ED080: pick a group channel and load a file.
    /// Answers the channel, or -1 when the group is undeclared or the file is absent.
    #[allow(clippy::too_many_arguments)]
    pub fn load(&mut self, host: &mut dyn HostServices, client: ClientId, pool: &TaskPool, policy: Option<&crate::native_sound_policy::SoundPolicy>, group_id: u32, name: &[u8]) -> Result<Progress<u32>> {
        if self.load.is_none() {
            let Some(policy) = policy else { return Ok(Progress::Done(u32::MAX)) };
            let Some(group) = policy.sound_group_by_id(group_id) else { return Ok(Progress::Done(u32::MAX)) };
            let mut slots = [crate::native_sound_policy::SoundSlot::default(); 64];
            for (channel, slot) in &self.channels {
                if *channel < 64 {
                    slots[*channel as usize].resource = Some(slot.resource);
                    slots[*channel as usize].auto_release = !host.audio_playing(client, *channel)?;
                }
            }
            let plan = crate::native_sound_policy::sound_slot_plan(client, group.minimum, group.maximum, &slots)?;
            let label = group.label.clone();
            let permit = match pool.reserve() {
                Ok(permit) => permit,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(Progress::Pending),
                Err(e) => return Err(e),
            };
            let storage = host.storage(client)?;
            let owned = name.to_vec();
            let worker = permit.spawn(move |cancel| crate::fes_live_sound::resolve(storage.as_ref(), &owned, &|| cancel.check()))?;
            self.load = Some(Load { channel: plan.channel, name: name.to_vec(), group: label, worker: Some(worker), ticket: None });
            return Ok(Progress::Pending);
        }
        let load = self.load.as_mut().unwrap();
        if let Some(worker) = load.worker.as_mut() {
            let located = match worker.try_take() {
                Ok(Some(located)) => located,
                Ok(None) => return Ok(Progress::Pending),
                // 5D5330 finds no file: the original leaves the channel unset.
                Err(_) => {
                    self.load = None;
                    return Ok(Progress::Done(u32::MAX));
                }
            };
            load.worker = None;
            load.ticket = Some(host.begin_audio_resource(client, AudioAsset { path: located.path, frame_limit: None }, false)?);
            return Ok(Progress::Pending);
        }
        let ticket = load.ticket.context("CatSystem2 KCS sound ticket lost")?;
        let result = host.audio_resource_preparation(client, ticket)?;
        match result.phase {
            AudioPreparePhase::Pending => Ok(Progress::Pending),
            AudioPreparePhase::Failed => anyhow::bail!("CatSystem2 KCS sound resource: {}", result.error.unwrap_or_default()),
            AudioPreparePhase::Ready => {
                let resource = result.resource.context("CatSystem2 KCS sound preparation has no resource")?;
                let load = self.load.take().unwrap();
                host.release_audio_resource_preparation(client, ticket)?;
                self.retire(host, client, load.channel)?;
                self.channels.insert(load.channel, Channel { resource, name: load.name, group: load.group, percent: 100 });
                Ok(Progress::Done(load.channel))
            }
        }
    }
    /// Message 181: start a loaded channel at its current percent.
    pub fn play(&mut self, host: &mut dyn HostServices, client: ClientId, policy: Option<&crate::native_sound_policy::SoundPolicy>, channel: u32) -> Result<Progress<()>> {
        if let Some((target, ticket)) = self.play {
            ensure!(target == channel, "CatSystem2 KCS sound play target changed");
            let result = host.audio_preparation(client, ticket)?;
            return match result.phase {
                AudioPreparePhase::Pending => Ok(Progress::Pending),
                AudioPreparePhase::Failed => anyhow::bail!("CatSystem2 KCS sound playback: {}", result.error.unwrap_or_default()),
                AudioPreparePhase::Ready => {
                    host.release_audio(client, ticket)?;
                    self.play = None;
                    Ok(Progress::Done(()))
                }
            };
        }
        let Some(slot) = self.channels.get(&channel) else { return Ok(Progress::Done(())) };
        let percent = match policy {
            Some(policy) => policy.playback_percent(&slot.name, &slot.group, false, slot.percent)?,
            None => slot.percent,
        };
        let gain = crate::native_sound_fades::gain(percent)?;
        let bus = match slot.group.as_slice() {
            b"bgm" => PcmBus::Music,
            b"voice" | b"bgv" => PcmBus::Voice,
            _ => PcmBus::Sfx,
        };
        let ticket = host.begin_audio(client, AudioRequest::PlayResource { channel, resource: slot.resource, parameters: AudioParameters { gain, bus, ..Default::default() } })?;
        self.play = Some((channel, ticket));
        Ok(Progress::Pending)
    }
    pub fn stop(&mut self, host: &mut dyn HostServices, client: ClientId, channel: u32, fade_ms: f32) -> Result<()> {
        if self.channels.contains_key(&channel) && host.audio_info(client, channel)?.is_some() {
            host.submit_audio(client, AudioRequest::Stop { channel, fade_ms })?;
        }
        Ok(())
    }
    pub fn playing(&self, host: &mut dyn HostServices, client: ClientId, channel: u32) -> Result<bool> {
        Ok(self.channels.contains_key(&channel) && host.audio_playing(client, channel)?)
    }
    pub fn volume(&mut self, host: &mut dyn HostServices, client: ClientId, policy: Option<&crate::native_sound_policy::SoundPolicy>, channel: u32, percent: u32, fade_ms: f32) -> Result<()> {
        let Some(slot) = self.channels.get_mut(&channel) else { return Ok(()) };
        slot.percent = percent;
        let effective = match policy {
            Some(policy) => policy.playback_percent(&slot.name, &slot.group, false, percent)?,
            None => percent,
        };
        if host.audio_info(client, channel)?.is_some() {
            host.submit_audio(client, AudioRequest::SetGain { channel, gain: crate::native_sound_fades::gain(effective)?, fade_ms })?;
        }
        Ok(())
    }
}
