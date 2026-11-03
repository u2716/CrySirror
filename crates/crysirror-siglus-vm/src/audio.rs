// SPDX-License-Identifier: MPL-2.0
//! Native routing declarations over client-scoped shared Audio gain groups.
//! No sample storage, decoder, device, DSP or private audio clock lives here.
use anyhow::{Result, ensure};
use crysirror_core::service::AudioRequest;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy)]
pub(crate) enum TrackKind {
    Bgm,
    Koe,
    Se,
    Mov,
    GlobalPcm,
}
impl TrackKind {
    pub(crate) fn group(self) -> &'static str {
        match self {
            Self::Bgm => "siglus.system.bgm",
            Self::Koe => "siglus.system.koe",
            Self::Se => "siglus.system.se",
            Self::Mov => "siglus.system.mov",
            Self::GlobalPcm => "siglus.system.pcm",
        }
    }
}
pub(crate) fn pcmch_group(slot: usize) -> String {
    format!("siglus.system.pcmch.{slot}")
}
#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct AudioHub {
    #[serde(skip)]
    gains: Arc<BTreeMap<String, u8>>,
    #[serde(skip)]
    pub(crate) pending: Vec<AudioRequest>,
    #[serde(skip)]
    published: Arc<BTreeMap<String, u8>>,
}
impl AudioHub {
    fn set(&mut self, name: String, gain: u8) -> Result<()> {
        if self.gains.get(&name) != Some(&gain) {
            Arc::make_mut(&mut self.gains).insert(name, gain);
        }
        Ok(())
    }
    pub(crate) fn activate(&mut self, name: String, used: bool) -> Result<()> {
        if !used && !self.published.contains_key(&name) {
            return Ok(());
        }
        let gain = *self.gains.get(&name).unwrap_or(&255);
        if self.published.get(&name) != Some(&gain) {
            ensure!(
                self.pending.len() < 1024,
                "native system audio routing batch limit"
            );
            self.pending.push(AudioRequest::SetMixGroupGain {
                name: name.clone(),
                gain: f32::from(gain) / 255.,
                fade_ms: 0.,
            });
            Arc::make_mut(&mut self.published).insert(name, gain);
        }
        Ok(())
    }
    pub(crate) fn set_track_master_volume_raw(&mut self, kind: TrackKind, value: u8) -> Result<()> {
        self.set(kind.group().into(), value)
    }
    pub(crate) fn set_slot_system_volume_raw(&mut self, slot: usize, value: u8) -> Result<()> {
        ensure!(slot < 256, "native system audio slot limit");
        self.set(pcmch_group(slot), value)
    }
    pub(crate) fn invalidate(&mut self) {
        self.gains = Arc::default();
        self.pending.clear();
        self.published = Arc::default();
    }
}
pub use crate::bgm_bridge::BgmEngine;
pub use crate::koe_bridge::KoeEngine;
pub use crate::sfx_bridge::{PcmEngine, SeEngine};
