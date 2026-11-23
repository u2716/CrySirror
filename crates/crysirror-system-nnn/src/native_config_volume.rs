// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig's ten native slider families, ranges and released-preview
//! selection. Coordinates are VM data; shared UI owns capture and shared image
//! services own bar/knob rendering. No slider raster/hit algorithm lives here.
//! Source: commonConfig.cpp tables104..174, constructor340..505/846..1007/
//!1210..1380, Init1863..1959, Calcu2994..3063, PlayExtSe2/GetSoundName3863..4004.
use crate::config::Names;
use anyhow::{Result, ensure};
const TAGS: [&str; 10] = [
    "messageSpeed",
    "music",
    "sound",
    "voice",
    "movie",
    "soundvoice",
    "windowPercent",
    "autoSpeed",
    "total",
    "scriptse",
];
pub(crate) const PARAMETERS: [i32; 10] = [105, 100, 101, 102, 103, 104, 106, 107, 108, 109];
pub(crate) const SWITCHES: [i32; 10] = [0, 200, 201, 202, 203, 204, 0, 0, 208, 209];
#[derive(Clone, Debug)]
pub(crate) struct Slider {
    pub exists: bool,
    pub page: i32,
    pub origin: [i32; 2],
    pub size: [i32; 2],
    pub bar: Vec<u8>,
    pub transparent: bool,
    pub digital: bool,
    pub divide: i32,
    pub real_minimum: i32,
    pub real_maximum: i32,
    pub preview_start: i32,
    pub preview_splits: i32,
    pub preview_voice: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct Profile {
    pub sliders: Vec<Slider>,
    pub previews: bool,
    pub bar_voice_is_system_voice: bool,
    pub use_config_voice: bool,
    pub same_voice_buffer: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Preview {
    /// Keep the source's zero-based argument unchanged; PlayButtonVoice itself
    /// rejects0. It must not be silently changed to number+1 by the caller.
    NumberedButtonVoice(i32),
    ConfigCatalogue {
        index: i32,
        voice: bool,
        delta: i32,
        same_buffer: bool,
    },
}
fn mapped(index: usize) -> bool {
    (1..=5).contains(&index) || index == 9
}
impl Profile {
    /// `all_exist` is the MUGEN2 executable constructor (0x516670), which
    /// defaults every `%sVolumeExistFlag` to 1; the published source defaults
    /// only the first six.
    pub(crate) fn read(table: &Names, xaudio: bool, all_exist: bool) -> Result<Self> {
        let n = |name: &str, default: i32| -> Result<i32> {
            Ok(table.integer(name.as_bytes())?.unwrap_or(default))
        };
        let count = n("volumeNumber", 6)?;
        ensure!(
            (1..=10).contains(&count),
            "NNN native Config volume-name table bounds"
        );
        let mut minimum = n("volumeMin", 80)?;
        let mut maximum = n("volumeMax", 100)?;
        if xaudio {
            minimum = n("volumeMinXAudio2", minimum)?;
            maximum = n("volumeMaxXAudio2", maximum)?;
            // Source reads the Win10 keys inside the XAudio2 branch without
            // another OS test. Backend/profile choice must remain explicit.
            minimum = n("volumeMinWin10", minimum)?;
            maximum = n("volumeMaxWin10", maximum)?;
        }
        let previews = n("extSeFlag", 0)? != 0;
        let mut size = [n("volumeSizeX", 338)?, n("volumeSizeY", 19)?];
        let mut sliders = Vec::new();
        for index in 0..count as usize {
            let tag = TAGS[index];
            let field = |suffix: &str, default: i32| n(&format!("{tag}{suffix}"), default);
            let exists = field("VolumeExistFlag", i32::from(all_exist || index < 6))? != 0;
            if exists {
                // Dimensions are intentionally inherited from the last
                // EXISTING slider, not reset to global defaults each time.
                size[0] = n(&format!("volumeSizeX{}", index + 1), size[0])?;
                size[1] = n(&format!("volumeSizeY{}", index + 1), size[1])?;
            }
            let divide = field("VolumeDevide", if matches!(index, 0 | 7) { 4 } else { 100 })?;
            ensure!(
                (-1_000_000..=1_000_000).contains(&divide),
                "NNN Config slider step budget"
            );
            if exists && mapped(index) {
                ensure!(divide > 0, "NNN native mapped slider divisor is zero");
            }
            let (preview_start, preview_splits, preview_voice) = if previews {
                let split_default = [3, 0, 0, 9, 0, 0, 0, 3].get(index).copied().unwrap_or(0);
                let voice_default = [1, 0, 0, 0, 0, 0, 0, 1].get(index).copied().unwrap_or(0);
                // The original default arrays have only8 entries although the
                // volume-name table has10. Define their undefined tail as0.
                let split = field("SeBunkatsu", split_default)?;
                ensure!(
                    (0..=65536).contains(&split),
                    "NNN Config preview split budget"
                );
                let initial_voice = if index == 0 {
                    n("messageIsVoice", voice_default)?
                } else {
                    voice_default
                };
                let initial_start = if index == 0 {
                    n("messageSeNumber", 0)?
                } else {
                    0
                };
                (
                    field("SeNumber", initial_start)?.wrapping_sub(1),
                    split,
                    field("IsVoice", initial_voice)? != 0,
                )
            } else {
                (-1, 0, false)
            };
            let origin = [
                field("VolumePrintX", 0)?,
                field("VolumePrintY", (index as i32).wrapping_mul(size[1]))?,
            ];
            let bar = table
                .parameter(format!("{tag}BarFileName").as_bytes())
                .unwrap_or(b"ta_opt_bar")
                .to_vec();
            ensure!(
                bar.len() <= 254 && !bar.contains(&0),
                "NNN Config bar name bounds"
            );
            if exists {
                ensure!(
                    size.iter().all(|v| (1..=16384).contains(v))
                        && origin.iter().all(|v| v.unsigned_abs() <= 1_000_000),
                    "NNN Config slider geometry"
                );
            }
            sliders.push(Slider {
                exists,
                page: field("VolumePrintPage", 1)?,
                origin,
                size,
                bar,
                transparent: field("VolumeTransFlag", 1)? != 0,
                digital: field("VolumeDigitalFlag", 0)? != 0,
                divide,
                real_minimum: field("RealVolumeMin", minimum)?,
                real_maximum: field("RealVolumeMax", maximum)?,
                preview_start,
                preview_splits,
                preview_voice,
            });
        }
        Ok(Self {
            sliders,
            previews,
            bar_voice_is_system_voice: n("barVoiceIsSystemVoice", 0)? != 0,
            use_config_voice: n("useConfigSeVoice", 0)? != 0,
            same_voice_buffer: n("voiceSameBufferFlag", 0)? != 0,
        })
    }
    pub(crate) fn initial_gaze(&self, index: usize, value: i32) -> Result<i32> {
        let slider = self
            .sliders
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("NNN Config slider index"))?;
        if index == 6 {
            return Ok(100i32.wrapping_sub(value));
        }
        if !mapped(index) {
            return Ok(value);
        }
        let mut v = value;
        // Source's ordered two comparisons also define reversed endpoints.
        if v < slider.real_minimum {
            v = slider.real_minimum;
        }
        if v > slider.real_maximum {
            v = slider.real_maximum;
        }
        let difference = slider.real_maximum.wrapping_sub(slider.real_minimum).max(1);
        v = v
            .wrapping_sub(slider.real_minimum)
            .wrapping_mul(slider.divide)
            / difference;
        Ok(v.max(0).min(slider.divide))
    }
    pub(crate) fn changed_value(&self, index: usize, gaze: i32) -> Result<(i32, i32)> {
        let slider = self
            .sliders
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("NNN Config slider index"))?;
        ensure!(
            slider.exists && (0..=slider.divide.max(1)).contains(&gaze),
            "NNN shared Config slider value"
        );
        let value = if index == 6 {
            100i32.wrapping_sub(gaze)
        } else if mapped(index) {
            ensure!(slider.divide != 0, "NNN mapped slider division by zero");
            slider
                .real_minimum
                .wrapping_add(
                    slider
                        .real_maximum
                        .wrapping_sub(slider.real_minimum)
                        .wrapping_mul(gaze)
                        / slider.divide,
                )
                .clamp(0, 100)
        } else {
            gaze
        };
        Ok((PARAMETERS[index], value))
    }
    pub(crate) fn preview(
        &self,
        index: usize,
        gaze: i32,
        movie_volume: i32,
        sound_volume: i32,
    ) -> Result<Option<Preview>> {
        let slider = self
            .sliders
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("NNN Config preview index"))?;
        if !self.previews || slider.preview_start == -1 {
            return Ok(None);
        }
        let mut number = slider.preview_start;
        if slider.preview_splits > 0 {
            if gaze >= slider.divide {
                number = number.wrapping_add(slider.preview_splits + 1);
            } else if gaze > 0 {
                let part = (slider.divide / slider.preview_splits).max(1);
                let delta = gaze.wrapping_add(part - 1) / part;
                number = number.wrapping_add(delta.min(slider.preview_splits));
            }
        }
        if self.bar_voice_is_system_voice && slider.preview_voice {
            return Ok(Some(Preview::NumberedButtonVoice(number)));
        }
        ensure!(
            number >= 0,
            "NNN Config preview would index before the catalogue"
        );
        // PlayExtSe2 compares md to EXT_SE_MOVIE(3), not the actual movie
        // slider's index4. Preserve the resulting delta on index3 previews.
        let delta = if !slider.preview_voice && index == 3 {
            movie_volume.wrapping_sub(sound_volume)
        } else {
            0
        };
        Ok(Some(Preview::ConfigCatalogue {
            index: number,
            voice: slider.preview_voice,
            delta,
            same_buffer: self.same_voice_buffer,
        }))
    }
    pub(crate) fn source_check_volume_exists(&self, id: i32) -> bool {
        // The base source repeats SOUNDVOICE twice, ultimately selecting5;
        // SOUNDVOLUME101 is missing and returnsFALSE, not slider2's flag.
        let index = match id {
            105 => 0,
            100 => 1,
            104 => 5,
            102 => 3,
            103 => 4,
            106 => 6,
            107 => 7,
            108 => 8,
            109 => 9,
            _ => return false,
        };
        self.sliders.get(index).is_some_and(|s| s.exists)
    }
}
