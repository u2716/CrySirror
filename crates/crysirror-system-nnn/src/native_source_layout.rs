// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Published-source storage capacity differs from GameCallBack::layerExpand.
//! allEffect.h owns32 physical pictures/EFFECTs; source SetLayerScript and
//! SetEffectScript admit only configured script slots (default16). Reserved
//! BG30 and post-text31 stay compile-time slots, not script_slots-2/-1.
use crate::config::Names;
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceLayout {
    pub script_slots: u16,
}
impl SourceLayout {
    pub(crate) const PHYSICAL_SLOTS: u16 = 32;
    pub(crate) const POST_TEXT: i32 = 31;
    pub(crate) const BACKGROUND: i32 = 30;
    pub(crate) fn read(setup: &Names) -> Result<Self> {
        let slots = setup.integer(b"layerExpand")?.unwrap_or(16);
        ensure!(
            (1..=16).contains(&slots) || slots == 32,
            "NNN source layerExpand cannot fit the declared native16-short variable-control status"
        );
        // Source's17..31 branch writes past STATUS.varcontrollayer[16].
        // Reject that undefined profile instead of overwriting adjacent saves.
        Ok(Self {
            script_slots: slots as u16,
        })
    }
    pub(crate) fn admits_script(self, slot: i32) -> bool {
        (0..i32::from(self.script_slots)).contains(&slot)
    }
    pub(crate) fn picture_index(slot: i32) -> i32 {
        slot.clamp(0, 31)
    }
    pub(crate) fn background(slot: i32) -> bool {
        slot < 4 || slot == Self::BACKGROUND
    }
    /// STATUS's16 signed shorts pack BOTH halves as signed bytes ONLY when
    /// layerExpand is32. Other legal profiles retain complete signed words.
    pub(crate) fn decode_layer_variables(self, saved: [i16; 16]) -> [i32; 32] {
        let mut out = [0; 32];
        if self.script_slots == 32 {
            for i in 0..16 {
                out[i] = (saved[i] as u8 as i8) as i32;
                out[i + 16] = ((saved[i] >> 8) as u8 as i8) as i32;
            }
        } else {
            let n = self.script_slots as usize;
            for i in 0..n {
                out[i] = i32::from(saved[i]);
            }
        }
        out
    }
    pub(crate) fn encode_layer_variables(self, live: &[i32; 32], retained: [i16; 16]) -> [i16; 16] {
        let mut out = retained;
        if self.script_slots == 32 {
            for i in 0..16 {
                let pair = (live[i] & 255) | ((live[i + 16].wrapping_shl(8)) & 0xff00);
                out[i] = pair as i16;
            }
        } else {
            let n = self.script_slots as usize;
            for i in 0..n {
                out[i] = live[i] as i16;
            }
        }
        out
    }
}
