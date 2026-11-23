// SPDX-License-Identifier: MPL-2.0
//! CVNS's six independent scene clips (+11744). The media owner supplies their
//! images and PCM clocks; the VM owns the separate load/start clock for silent clips.
use crate::vm::CmvsVm;
use anyhow::{Context, Result};
use crysirror_core::{
    path::VfsPath,
    service::{
        AssetReader, HostServices, IndexedMoviePacing, MediaAudioClock, MediaRequest,
        MediaSourceOptions, UploadedImage,
    },
    state::ClientId,
};
use std::sync::Arc;

pub(super) const COUNT: usize = 6;
const BASE: u32 = 48;
const RESTORE_BASE: u32 = BASE + COUNT as u32;

pub(in crate::adapter) struct Clip {
    source: Arc<dyn AssetReader>,
    name: Vec<u8>,
    pub(super) load_options: [u32; 2],
    frames: u32,
    rate: u32,
}

impl Clip {
    pub fn read(source: Arc<dyn AssetReader>, name: Vec<u8>) -> Result<Self> {
        let mut head = [0; 36];
        crysirror_resource::io::read_exact_at(source.as_ref(), 0, &mut head)?;
        let [frames, rate, _] = super::super::movie::timing(&head);
        Ok(Self {
            source,
            name,
            load_options: [0; 2],
            frames,
            rate,
        })
    }

    fn open(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        id: u32,
        gain: f32,
    ) -> Result<VfsPath> {
        let options = MediaSourceOptions {
            audio: true,
            looping: false,
            gain,
        };
        Ok(host.indexed_movie_reader(
            client,
            id,
            self.source.clone(),
            options,
            Default::default(),
            IndexedMoviePacing::NativeRate,
        )?)
    }
}

struct Slot {
    clip: Clip,
    image: UploadedImage,
    frozen: Option<(u64, UploadedImage)>,
    clock: u32,
    started: bool,
    has_audio: bool,
}

pub(in crate::adapter) struct Slots {
    base: u32,
    slots: [Option<Slot>; COUNT],
    preparing: Option<usize>,
}

impl Default for Slots {
    fn default() -> Self {
        Self {
            base: BASE,
            slots: Default::default(),
            preparing: None,
        }
    }
}

impl Slots {
    pub(super) fn detached(&self) -> Self {
        Self {
            base: if self.base == BASE {
                RESTORE_BASE
            } else {
                BASE
            },
            ..Default::default()
        }
    }

    pub(super) fn poll_staged(
        &mut self,
        host: &dyn HostServices,
        client: ClientId,
    ) -> Result<bool> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(slot) = slot.as_mut().filter(|s| s.image.size == [0; 2]) else {
                continue;
            };
            let id = self.base + i as u32;
            let Some(frame) = host.media_frame(client, id)? else {
                return Ok(true);
            };
            slot.image.size = frame.size();
            slot.has_audio = host.media_audio_clock(client, id)? != MediaAudioClock::NoTrack;
            slot.clock = host.now_ms() as u32;
        }
        self.preparing = None;
        Ok(false)
    }

    pub(super) fn records(&self, out: &mut Vec<u8>) -> Result<()> {
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(slot) = slot else { continue };
            let mut bytes = slot.clip.name.clone();
            bytes.push(0);
            for word in slot
                .clip
                .load_options
                .into_iter()
                .chain([u32::from(slot.started)])
            {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            crate::vm::save_slot::csp1::append(out, 0x700, &[index as u16], &bytes)?;
        }
        Ok(())
    }

    pub fn gain_targets(&self) -> impl Iterator<Item = u32> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter_map(move |(i, s)| s.as_ref().map(|_| self.base + i as u32))
    }

    pub(super) fn image(&self, index: usize) -> Option<&UploadedImage> {
        Some(&self.slots.get(index)?.as_ref()?.image)
    }

    pub(super) fn frozen(&self, index: usize) -> Option<&UploadedImage> {
        self.slots
            .get(index)?
            .as_ref()?
            .frozen
            .as_ref()
            .map(|(_, image)| image)
    }

    pub(super) fn freeze_frames(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let Some(slot) = slot else { continue };
            let Some(frame) = host.media_frame(client, self.base + index as u32)? else {
                continue;
            };
            if slot
                .frozen
                .as_ref()
                .is_some_and(|(revision, _)| *revision == frame.revision)
            {
                continue;
            }
            let image = host.publish_image_resource(
                client,
                0x4356_9400 + index as u32,
                frame.as_image(client),
            )?;
            if let Some((_, old)) = slot.frozen.replace((frame.revision, image)) {
                host.release_image_resource(client, old.path.relative())?;
            }
        }
        Ok(())
    }

    pub fn remove(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
    ) -> Result<()> {
        if self.slots[index].is_some() {
            host.media_request(
                client,
                MediaRequest::Remove {
                    id: self.base + index as u32,
                },
            )?;
            if let Some((_, image)) = self.slots[index]
                .as_mut()
                .and_then(|slot| slot.frozen.take())
            {
                host.release_image_resource(client, image.path.relative())?;
            }
            self.slots[index] = None;
        }
        Ok(())
    }

    pub fn load(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        clip: Clip,
        gain: f32,
    ) -> Result<()> {
        let id = self.base + index as u32;
        let path = clip.open(host, client, id, gain)?;
        if let Err(e) = host.media_request(client, MediaRequest::Pause { id }) {
            host.media_request(client, MediaRequest::Remove { id })?;
            return Err(e.into());
        }
        self.slots[index] = Some(Slot {
            clip,
            image: UploadedImage {
                path,
                size: [0; 2],
                opaque: true,
            },
            frozen: None,
            clock: 0,
            started: false,
            has_audio: false,
        });
        self.preparing = Some(index);
        Ok(())
    }

    pub fn poll(
        &mut self,
        host: &dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<bool> {
        let Some(index) = self.preparing else {
            return Ok(false);
        };
        let id = self.base + index as u32;
        let Some(frame) = host.media_frame(client, id)? else {
            return Ok(true);
        };
        let slot = self.slots[index]
            .as_mut()
            .context("CVNS scene movie was released during load")?;
        slot.image.size = frame.size();
        slot.has_audio = host.media_audio_clock(client, id)? != MediaAudioClock::NoTrack;
        slot.clock = host.now_ms() as u32;
        self.preparing = None;
        vm.complete_owner_call(2, Some(1))?;
        Ok(false)
    }

    pub fn start(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        gain: f32,
    ) -> Result<()> {
        let slot = self.slots[index]
            .as_mut()
            .context("CVNS start on an empty movie slot")?;
        let id = self.base + index as u32;
        if slot.started {
            slot.image.path = slot.clip.open(host, client, id, gain)?;
            // Reopening starts a new revision sequence, even for the same file.
            if let Some((_, image)) = slot.frozen.take() {
                host.release_image_resource(client, image.path.relative())?;
            }
        } else {
            host.media_request(client, MediaRequest::Resume { id })?;
        }
        slot.started = true;
        slot.clock = host.now_ms() as u32;
        Ok(())
    }

    pub fn position(
        &self,
        host: &dyn HostServices,
        client: ClientId,
        index: usize,
        lead_ms: u32,
    ) -> Result<u32> {
        let slot = self.slots[index]
            .as_ref()
            .context("CVNS clock on an empty movie slot")?;
        let time = if slot.has_audio {
            match host.media_audio_clock(client, self.base + index as u32)? {
                MediaAudioClock::Playing(ms) => (ms as u32).saturating_sub(lead_ms),
                _ => return Ok(u32::MAX),
            }
        } else {
            (host.now_ms() as u32).wrapping_sub(slot.clock)
        };
        Ok(
            if time.wrapping_mul(slot.clip.rate) / 1000 >= slot.clip.frames {
                u32::MAX
            } else {
                time
            },
        )
    }
}
