// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CMySlider's native integer value/released-preview owner. Pointer capture and
//! admission are supplied by shared UI; this module never performs hit tests or
//! raster work. One admitted Config Calcu is distinct from an asset retry.
use crate::native_config_volume::{Preview, Profile};
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub(crate) struct SliderOwner {
    pub gaze: i32,
    captured: bool,
    frame: Option<u64>,
}
pub(crate) struct Change {
    pub id: i32,
    pub value: i32,
    pub preview: Option<Preview>,
}
impl SliderOwner {
    pub(crate) fn new(profile: &Profile, index: usize, value: i32) -> Result<Self> {
        Ok(Self {
            gaze: profile.initial_gaze(index, value)?,
            captured: false,
            frame: None,
        })
    }
    /// `captured` is the current owner-scoped UiInteraction result, not a
    /// second adapter hit calculation. The previous capture admits release's
    /// final pointer sample, as CMySlider::Calcu does before clearing dragFlag.
    pub(crate) fn calcu(
        &mut self,
        profile: &Profile,
        index: usize,
        ticket: u64,
        captured: bool,
        pointer_x: i32,
        enabled: bool,
        movie_volume: i32,
        sound_volume: i32,
    ) -> Result<Option<Change>> {
        if self.frame == Some(ticket) {
            return Ok(None);
        }
        self.frame = Some(ticket);
        let slider = profile
            .sliders
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("NNN Config slider identity"))?;
        if !enabled || !slider.exists {
            return Ok(None);
        }
        let sample = self.captured || captured;
        self.captured = captured;
        if !sample {
            return Ok(None);
        }
        let width = i64::from(slider.size[0]);
        ensure!(width > 0, "NNN Config slider source width");
        let x = (i64::from(pointer_x) - i64::from(slider.origin[0])).clamp(0, width);
        let divide = i64::from(slider.divide.max(1));
        let gaze = if slider.digital {
            x * (divide + 1) / width
        } else {
            (x * divide + width / 2) / width
        };
        self.gaze = gaze.clamp(0, divide) as i32;
        let (id, value) = profile.changed_value(index, self.gaze)?;
        Ok(Some(Change {
            id,
            value,
            preview: if captured {
                None
            } else {
                profile.preview(index, self.gaze, movie_volume, sound_volume)?
            },
        }))
    }
    /// CMySlider::Print/AppearPrint describes a source crop, not a stretched
    /// bar. The engine clips and samples the original atlas at natural size.
    pub(crate) fn fill_width(&self, profile: &Profile, index: usize) -> Result<i32> {
        let slider = profile
            .sliders
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("NNN Config slider identity"))?;
        let divide = i64::from(slider.divide.max(1));
        let width = if slider.digital {
            i64::from(slider.size[0]) * (i64::from(self.gaze) + 1) / (divide + 1)
        } else {
            i64::from(slider.size[0]) * i64::from(self.gaze) / divide
        };
        Ok(i32::try_from(width)?)
    }
    pub(crate) fn resume(&mut self) {
        self.frame = None;
        self.captured = false;
    }
}
