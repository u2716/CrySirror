// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonPrintCG's difference-navigation and Print-owned countdown.
//! This policy never captures a composed screen: the original old source is
//! a reloaded first tile, or the last frame when the old CG was animated.
use crate::config::Names;
use anyhow::{Result, ensure};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DifferenceProfile {
    pub selector: i32,
    pub frames: i32,
}
impl DifferenceProfile {
    pub(crate) fn read(table: &Names) -> Result<Self> {
        let selector = table.integer(b"diffFadeType")?.unwrap_or(0);
        // The source only reads diffFadeTime for a positive selector. A
        // negative nonzero selector therefore reaches the direct-copy branch
        // with its constructor time1; it is not an unknown wipe lookup.
        let frames = if selector > 0 {
            table.integer(b"diffFadeTime")?.unwrap_or(1)
        } else {
            1
        };
        ensure!(
            (-864000..=864000).contains(&frames),
            "NNN CG difference time budget"
        );
        ensure!(
            selector <= 0 || crate::native_simple_wipe::supported(selector - 1),
            "NNN CG difference wipe selector {selector}"
        );
        Ok(Self { selector, frames })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DifferencePrint {
    Normal,
    /// CPicture::Blt(0,0,0,0,screenW,screenH,FALSE): caller retains physical
    /// source/CharaClip clipping, leaves uncovered destination pixels intact.
    CopyOpaque,
    /// CGameCallBack::PrintSimpleWipe on raw opaque screen-sized source
    /// views. Old CG is not a screenshot of an interrupted blended frame.
    Wipe {
        kind: i32,
        count: u32,
        duration: u32,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OldDifferenceImage {
    FirstTile,
    LastAnimationFrame,
}
#[derive(Clone, Debug)]
pub(crate) struct DifferenceState {
    pub profile: DifferenceProfile,
    pub enabled: bool,
    pub count: i32,
}
impl DifferenceState {
    pub(crate) fn new(profile: DifferenceProfile) -> Self {
        Self {
            profile,
            enabled: false,
            count: 0,
        }
    }
    /// CCommonPrintCG::Calcu checks the new CG's animation flag, not the
    /// previous one. Return the raw resource which LoadCG(TRUE,old) reloads.
    pub(crate) fn navigate(
        &mut self,
        same_block: bool,
        new_animation: bool,
        changed: bool,
        old_animation: bool,
    ) -> Option<OldDifferenceImage> {
        self.enabled = same_block && !new_animation;
        if changed && self.enabled && self.profile.selector != 0 {
            self.count = 0;
            Some(if old_animation {
                OldDifferenceImage::LastAnimationFrame
            } else {
                OldDifferenceImage::FirstTile
            })
        } else {
            None
        }
    }
    pub(crate) fn forces_redraw(&self) -> bool {
        self.enabled && self.profile.selector != 0 && self.count < self.profile.frames
    }
    pub(crate) fn plan(&self, column: u32) -> DifferencePrint {
        // The original checks i==0 inside EVERY tile row, not i==0&&j==0.
        // Each first-column wipe writes at screen origin with no scroll offset;
        // subsequent tiles retain ordinary shifted character/background blits.
        if column != 0 || !self.enabled || self.profile.selector == 0 {
            return DifferencePrint::Normal;
        }
        if self.profile.frames <= 1 {
            DifferencePrint::CopyOpaque
        } else {
            DifferencePrint::Wipe {
                kind: self.profile.selector - 1,
                count: self.count.max(0).min(self.profile.frames) as u32,
                duration: self.profile.frames as u32,
            }
        }
    }
    /// Called once after an admitted native Print is published, even if that
    /// Print had no dirty image region. Worker pumps and Calcu do not call it.
    pub(crate) fn printed(&mut self) {
        if self.enabled && self.profile.selector != 0 && self.count < self.profile.frames {
            self.count = self.count.wrapping_add(1);
        }
        // Source does not clear sabunFlag at the endpoint or synthesize an
        // extra final redraw. Another dirty/scroll frame may render countMax.
    }
}
