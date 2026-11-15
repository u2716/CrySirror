// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! ANIMATION137 native selection, shared class cache and previous-effect policy.
//! File loading and pixels stay in the resource/picture and shared image jobs.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnimationConfig {
    pub enabled: bool,
    pub buffer_max: i32,
    pub depth: i32,
    /// CP932 native basename prefix; default contains the original directory.
    pub tag: Vec<u8>,
}
impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_max: 60,
            depth: 32,
            tag: b"bg\\bganime".to_vec(),
        }
    }
}
impl AnimationConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.tag.len() <= 254 && !self.tag.contains(&0),
            "ANIMATION native prefix capacity"
        );
        if self.enabled {
            ensure!(
                (1..=4096).contains(&self.buffer_max),
                "ANIMATION buffer count outside engine budget"
            );
        }
        Ok(())
    }
    pub(crate) fn set_tag(&mut self, name: &[u8]) {
        // Native strlen/memcpy truncates by bytes, without extending a CP932
        // character at the boundary. Path decoding later rejects a torn name.
        self.tag = name
            .iter()
            .copied()
            .take_while(|n| *n != 0)
            .take(254)
            .collect();
    }
    pub(crate) fn indexed(&self) -> bool {
        self.depth != 32
    }
    pub(crate) fn buffer_index(&self, value: i32) -> Result<Option<i32>> {
        if !self.enabled {
            return Ok(None);
        }
        self.validate()?;
        Ok(Some(value.min(self.buffer_max - 1).max(0)))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CacheSlot {
    pub filename: Vec<u8>,
    pub load_ok: bool,
    pub indexed: bool,
    pub mask_flag: bool,
}
impl CacheSlot {
    pub(crate) fn blank(indexed: bool) -> Self {
        // Sized CPicture constructors mark loadOk TRUE but leave pixels/masks
        // uninitialized. Runtime views must materialize bounded zero planes
        // for an unread initial buffer, never copy uninitialized memory.
        Self {
            filename: Vec::new(),
            load_ok: true,
            indexed,
            mask_flag: !indexed,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct AnimationClass {
    pub config: AnimationConfig,
    pub last_anime: i32,
    /// Cache identities are shared across every137 EFFECT, as are the actual
    /// engine image views owned by the resource sidecar keyed by this index.
    pub slots: BTreeMap<i32, CacheSlot>,
    pub old: Option<CacheSlot>,
}
pub(crate) fn parameters(values: &[i32], previous_effect: i32, last_anime: i32) -> [i32; 17] {
    let mut p = [0; 17];
    for i in 0..17 {
        p[i] = values.get(i).copied().unwrap_or(match i {
            0 => 20,
            1 => 4,
            3 | 4 => 1,
            5 => 5,
            _ => 0,
        });
    }
    // Omitted8..16 were uninitialized, except the explicitly overwritten work
    // words below. Define zero and preserve supplied/unmodified saved words.
    p[1] = p[1].max(1);
    p[4] = p[4].max(1);
    p[11] = 0;
    p[13] = 0;
    if previous_effect == 137 {
        p[15] = last_anime;
        p[13] = 1;
    }
    p[14] = p[2];
    p
}
pub(crate) fn validate(p: &[i32; 17]) -> Result<()> {
    ensure!(
        p[1] > 0 && p[4] > 0,
        "ANIMATION original divisor/pattern words"
    );
    Ok(())
}
pub(crate) fn count_is_max(p: &[i32; 17], count: &mut i32, frames: i32) -> bool {
    if p[7] == 0 && p[3] != 0 && frames >= 99999 {
        *count = 0;
    }
    false
}
pub(crate) fn calcu(p: &[i32; 17], previous_effect: &mut i32) {
    if p[7] != 0 {
        *previous_effect = 0;
    }
}
#[derive(Clone, Debug)]
pub(crate) struct FrameSelection {
    pub slot: i32,
    pub anime: i32,
    pub filename: Vec<u8>,
    pub caching: bool,
    pub old_filename: Option<Vec<u8>>,
    /// Native percent for old/current during an admitted transition.
    pub transition: Option<[i32; 2]>,
}
impl AnimationClass {
    pub(crate) fn select(
        &self,
        p: &mut [i32; 17],
        count: i32,
        anime_off: i32,
    ) -> Result<Option<FrameSelection>> {
        if !self.config.enabled {
            return Ok(None);
        }
        self.config.validate()?;
        validate(p)?;
        let mut n = count / p[4].max(1);
        if p[3] != 0 {
            n %= p[1];
        } else if n >= p[1] {
            n = p[1].wrapping_sub(1);
        }
        let caching = p[7] != 0;
        if caching {
            n = p[11];
            p[11] = p[11].wrapping_add(1) % p[1];
        }
        if anime_off != 0 {
            n = 0;
        }
        if n < 0 {
            n = 0;
        }
        let anime = p[2].wrapping_add(n);
        let slot = n.wrapping_add(p[6]).max(0).min(self.config.buffer_max - 1);
        let mut filename = self.config.tag.clone();
        filename.extend_from_slice(anime.to_string().as_bytes());
        ensure!(
            filename.len() < 1024,
            "ANIMATION original filename capacity"
        );
        let (old_filename, transition) = if p[13] != 0 && p[5] > 0 && count < p[5] && !caching {
            let mut name = self.config.tag.clone();
            name.extend_from_slice(p[15].to_string().as_bytes());
            let old = 100i32
                .wrapping_sub(count.wrapping_mul(100) / p[5])
                .clamp(0, 100);
            (Some(name), Some([old, 100 - old]))
        } else {
            (None, None)
        };
        Ok(Some(FrameSelection {
            slot,
            anime,
            filename,
            caching,
            old_filename,
            transition,
        }))
    }
    /// CPicture::LoadDWQ's same-name early return precedes its loadOk test.
    /// Thus a repeated failed name reports TRUE while the retained picture is
    /// still loadOk FALSE. The caller must distinguish both pieces of state.
    pub(crate) fn begin_load(&mut self, slot: i32, name: &[u8]) -> Result<bool> {
        self.config.validate()?;
        ensure!(
            (0..self.config.buffer_max).contains(&slot),
            "ANIMATION selected buffer identity"
        );
        let entry = self
            .slots
            .entry(slot)
            .or_insert_with(|| CacheSlot::blank(self.config.indexed()));
        if entry.filename == name {
            return Ok(true);
        }
        entry.filename = name.to_vec();
        entry.indexed = self.config.indexed();
        entry.mask_flag = false;
        // This is a Read action; load_ok is finalized when its worker returns.
        Ok(false)
    }
    pub(crate) fn finish_load(&mut self, slot: i32, loaded: bool, has_mask: bool) -> Result<()> {
        let entry = self
            .slots
            .get_mut(&slot)
            .ok_or_else(|| anyhow::anyhow!("ANIMATION worker has no native buffer"))?;
        entry.load_ok = loaded;
        if loaded {
            entry.mask_flag = has_mask;
        }
        Ok(())
    }
    pub(crate) fn loaded_primary(&mut self, selection: &FrameSelection) {
        if !selection.caching {
            self.last_anime = selection.anime;
        }
    }
}
