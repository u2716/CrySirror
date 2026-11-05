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
    /// The voice's `.lv` level table, when it has one.
    levels: Option<std::sync::Arc<Vec<f32>>>,
}
struct Load {
    channel: u32,
    name: Vec<u8>,
    group: Vec<u8>,
    worker: Option<Task<(crate::fes_live_sound::Located, Option<Vec<f32>>)>>,
    ticket: Option<u64>,
    levels: Option<std::sync::Arc<Vec<f32>>>,
}

/// 5A1E20: a table of 60 Hz floats, each averaged with its neighbours (+0x28 stays at its
/// default of 1, since 575490 never changes it). The ends stay as read.
pub(crate) fn smooth_levels(bytes: &[u8]) -> Vec<f32> {
    let raw: Vec<f32> = bytes.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    let mut out = raw.clone();
    for at in 1..raw.len().saturating_sub(1) {
        out[at] = (raw[at - 1] + raw[at] + raw[at + 1]) / 3.;
    }
    out
}

/// 5AB280: `LPD\0`, row size 32, row count, the floor, then rows {frame, mouth, .., level at +28}.
/// 5A1B90 holds each row until the next one's frame, weights it by the mouth shape (5A1A10's
/// defaults) and by loudness, then smooths as `.lv`. 5A1B30 scales the result by 5 (+8 - +4).
pub(crate) fn lpb_levels(bytes: &[u8]) -> Option<Vec<f32>> {
    const WEIGHTS: [f32; 6] = [0., 1., 0.4, 0.2, 0.8, 0.6];
    let word = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if word(0)? != 0x44_504c || word(8)? != 0x20 {
        return None;
    }
    let count = word(12)? as usize;
    let floor = -f32::from_bits(word(16)?);
    let row = |at: usize| -> Option<(u32, u32, f32)> {
        let base = 20 + at * 32;
        Some((word(base)?, word(base + 4)?, f32::from_bits(word(base + 28)?)))
    };
    if count == 0 {
        return None;
    }
    let frames = row(count - 1)?.0 as usize + 1;
    let mut raw = vec![0f32; frames];
    let mut held = 0usize;
    for (frame, slot) in raw.iter_mut().enumerate() {
        while held + 1 < count && row(held + 1)?.0 as usize <= frame {
            held += 1;
        }
        let (_, mouth, level) = row(held)?;
        let quiet = (-level).max(20.).min(floor.max(20.));
        let span = floor - 20.;
        let loud = if span != 0. { (span - (quiet - 20.)) / span } else { 0. };
        *slot = (loud * 0.4 + 0.6) * WEIGHTS.get(mouth as usize).copied().unwrap_or(0.);
    }
    let bytes: Vec<u8> = raw.iter().flat_map(|v| v.to_le_bytes()).collect();
    Some(smooth_levels(&bytes).into_iter().map(|v| v * 5.).collect())
}

/// 5EAC60: `<stem>.lv` beside the voice in the same directory or archive.
fn read_levels(storage: &dyn crysirror_core::service::ClientStorage, voice: &crysirror_core::path::VfsPath) -> Result<Option<Vec<f32>>> {
    let relative = voice.relative();
    let Some((stem, _)) = relative.rsplit_once('.') else { return Ok(None) };
    // 6383xx asks for message 220 (`.lpb`) first and falls back to 221 (`.lv`).
    let packed = crysirror_core::path::VfsPath::new(voice.mount(), &format!("{stem}.lpb"))?;
    if let Some(path) = crate::native_resource_paths::resolve(storage, &packed)? {
        if let Some(levels) = lpb_levels(&storage.read(&path)?) {
            return Ok(Some(levels));
        }
    }
    let path = crysirror_core::path::VfsPath::new(voice.mount(), &format!("{stem}.lv"))?;
    match crate::native_resource_paths::resolve(storage, &path)? {
        Some(path) => Ok(Some(smooth_levels(&storage.read(&path)?))),
        None => Ok(None),
    }
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
            let worker = permit.spawn(move |cancel| {
                let located = crate::fes_live_sound::resolve(storage.as_ref(), &owned, &|| cancel.check())?;
                cancel.check()?;
                let levels = read_levels(storage.as_ref(), &located.path)?;
                Ok((located, levels))
            })?;
            self.load = Some(Load { channel: plan.channel, name: name.to_vec(), group: label, worker: Some(worker), ticket: None, levels: None });
            return Ok(Progress::Pending);
        }
        let load = self.load.as_mut().unwrap();
        if let Some(worker) = load.worker.as_mut() {
            let (located, levels) = match worker.try_take() {
                Ok(Some(found)) => found,
                Ok(None) => return Ok(Progress::Pending),
                // 5D5330 finds no file: the original leaves the channel unset.
                Err(_) => {
                    self.load = None;
                    return Ok(Progress::Done(u32::MAX));
                }
            };
            load.worker = None;
            load.levels = levels.map(std::sync::Arc::new);
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
                self.channels.insert(load.channel, Channel { resource, name: load.name, group: load.group, percent: 100, levels: load.levels });
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
    /// The file names loaded on a channel, or on all of them, for native 903's cue tables.
    pub fn loaded_names(&self, channel: Option<u32>) -> Vec<Vec<u8>> {
        self.channels.iter().filter(|(id, _)| channel.is_none_or(|channel| **id == channel)).map(|(_, slot)| slot.name.clone()).collect()
    }
    /// The level table of the voice loaded on a channel.
    pub fn levels(&self, channel: u32) -> Option<std::sync::Arc<Vec<f32>>> {
        self.channels.get(&channel).and_then(|slot| slot.levels.clone())
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

#[cfg(test)]
mod level_tests {
    #[test]
    fn lpb_rows_hold_until_the_next_frame() {
        let mut file = 0x44_504cu32.to_le_bytes().to_vec();
        file.extend(0u32.to_le_bytes());
        file.extend(32u32.to_le_bytes());
        file.extend(2u32.to_le_bytes());
        file.extend((-40f32).to_le_bytes());
        for (frame, mouth, level) in [(0u32, 1u32, -20f32), (3, 0, -20.)] {
            let mut row = [0u8; 32];
            row[..4].copy_from_slice(&frame.to_le_bytes());
            row[4..8].copy_from_slice(&mouth.to_le_bytes());
            row[28..].copy_from_slice(&level.to_le_bytes());
            file.extend(row);
        }
        let levels = super::lpb_levels(&file).unwrap();
        assert_eq!(levels.len(), 4);
        assert_eq!(levels[0], 5.);
        assert_eq!(levels[3], 0.);
    }
    #[test]
    fn levels_average_neighbours_and_keep_the_ends() {
        let bytes: Vec<u8> = [3f32, 6., 9., 0.].iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(super::smooth_levels(&bytes), vec![3., 6., 5., 0.]);
    }
}
