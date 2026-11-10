// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native effect arguments and lifecycle. Pixels and persistent pattern state
//! belong to engine services; this module retains only native policy.
use anyhow::{Result, ensure};
use crysirror_core::service::PatternResource;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SunaParameters {
    pub frames: i32,
    pub transition: bool,
    pub end_percent: i32,
    pub start_percent: i32,
    pub easing: i32,
    pub subframe: i32,
    pub rect: [u32; 4],
}
impl SunaParameters {
    pub fn parse(values: &[i32], size: [u32; 2]) -> Result<Self> {
        ensure!(values.len() <= 224 && size.iter().all(|n| (1..=16384).contains(n)),
            "SUNA argument/canvas bounds");
        // Deliberate repair of native undefined memory: SetParam assigns
        // para[3]=para[6] before initializing the absent X parameter. Initialize
        // the defaults first, so omitted start-percent deterministically uses
        // the same default X=0. Source consumes only the first ten words; its
        // InitParameter itself passes sixteen, so trailing words are ignored.
        let get = |i: usize, default: i32| values.get(i).copied().unwrap_or(default);
        let x = get(6, 0).max(0).min(size[0] as i32 - 1) as u32;
        let y = get(7, 0).max(0).min(size[1] as i32 - 1) as u32;
        let w = get(8, size[0] as i32).max(0) as u32;
        let h = get(9, size[1] as i32).max(0) as u32;
        // Original software effect aligns its region after viewport clamping.
        let rect = [
            x & !7,
            y & !1,
            w.min(size[0] - x) & !7,
            h.min(size[1] - y) & !1,
        ];
        Ok(Self {
            frames: get(0, 0),
            transition: get(1, 0) != 0,
            end_percent: get(2, 100),
            start_percent: get(3, 0),
            easing: get(4, 0),
            subframe: get(5, 0),
            rect,
        })
    }
}
#[derive(Clone, Debug)]
pub struct SunaState {
    pub parameters: SunaParameters,
    pub resource: Arc<dyn PatternResource>,
    pub count: i32,
}
impl SunaState {
    pub(crate) fn validate_curve(&self) -> Result<()> {
        let p = &self.parameters;
        if p.transition {
            p.start_percent
                .checked_mul(256)
                .ok_or_else(|| anyhow::anyhow!("SUNA native percent overflow profile"))?;
            p.end_percent
                .checked_mul(256)
                .ok_or_else(|| anyhow::anyhow!("SUNA native percent overflow profile"))?;
            crate::native_curve::validate(p.frames, p.subframe, p.easing)?;
        }
        Ok(())
    }
    pub fn opacity256(&self) -> Result<u16> {
        self.opacity256_with(None)
    }
    pub(crate) fn opacity256_with_random(
        &self,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<u16> {
        self.opacity256_with(Some(random))
    }
    fn opacity256_with(
        &self,
        random: Option<&mut crysirror_image::temporal_pattern::PatternRng>,
    ) -> Result<u16> {
        let p = &self.parameters;
        if !p.transition {
            return Ok(256);
        }
        let start = p
            .start_percent
            .checked_mul(256)
            .ok_or_else(|| anyhow::anyhow!("SUNA native percent overflow profile"))?
            / 100;
        let end = p
            .end_percent
            .checked_mul(256)
            .ok_or_else(|| anyhow::anyhow!("SUNA native percent overflow profile"))?
            / 100;
        let value = if let Some(random) = random {
            crate::native_curve::value_with_random(
                self.count, p.frames, start, end, p.subframe, p.easing, random,
            )?
        } else {
            crate::native_curve::value(self.count, p.frames, start, end, p.subframe, p.easing)?
        };
        Ok(value.clamp(0, 256) as u16)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FadeInState {
    pub frames: i32,
    pub color: [i32; 3],
    pub easing: i32,
    pub subframe: i32,
    pub count: i32,
    /// Native11 becomes command54 at CountIsMax; native10 instead ends its fade.
    pub fade_out: bool,
}
impl FadeInState {
    pub fn parse(values: &[i32]) -> Result<Self> {
        ensure!(values.len() <= 6, "FADEIN native argument count");
        let r = values.get(1).copied().unwrap_or(0);
        let g = values.get(2).copied().unwrap_or(r);
        let b = values.get(3).copied().unwrap_or(g);
        Ok(Self {
            frames: values.first().copied().unwrap_or(20),
            color: [r, g, b],
            easing: values.get(4).copied().unwrap_or(0),
            subframe: values.get(5).copied().unwrap_or(0),
            count: 0,
            fade_out: false,
        })
    }
    pub fn parse_out(values: &[i32]) -> Result<Self> {
        let mut state = Self::parse(values)?;
        state.fade_out = true;
        Ok(state)
    }
    pub fn command(&self) -> i32 {
        if self.fade_out {
            if self.count >= self.frames { 54 } else { 11 }
        } else {
            10
        }
    }
    /// Original linear curve and x86 Q15 colour weights. All per-pixel table
    /// application remains the shared engine ImageEffect, including worker
    /// cancellation. The old high X byte is not engine display transparency.
    pub fn linear_image_effect(&self) -> Result<crysirror_image::effects::ImageEffect> {
        ensure!(
            self.easing == 0,
            "linear FADEIN helper requires a linear native curve"
        );
        self.image_effect_with(None)
    }
    pub(crate) fn validate_curve(&self) -> Result<()> {
        crate::native_curve::validate(self.frames, self.subframe, self.easing)
    }
    pub(crate) fn image_effect_with_random(
        &self,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<crysirror_image::effects::ImageEffect> {
        self.image_effect_with(Some(random))
    }
    fn image_effect_with(
        &self,
        random: Option<&mut crysirror_image::temporal_pattern::PatternRng>,
    ) -> Result<crysirror_image::effects::ImageEffect> {
        if self.command() == 54 {
            // Fill54 masks each original parameter independently. Its -1
            // bytes do not inherit another channel as the fade kernel does.
            let spec = crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed(
                self.color.map(|value| value as u8),
                0,
                1,
                0,
            )?;
            return Ok(crysirror_image::effects::ImageEffect::RgbLookup { spec });
        }
        let progress = if let Some(random) = random {
            crate::native_curve::value_with_random(
                self.count,
                self.frames,
                0,
                100,
                self.subframe,
                self.easing,
                random,
            )?
        } else {
            crate::native_curve::value(self.count, self.frames, 0, 100, self.subframe, self.easing)?
        };
        let progress = progress.clamp(0, 100) as u32;
        let percent = if self.fade_out {
            100 - progress
        } else {
            progress
        };
        let [r, mut g, mut b] = self.color;
        if g == -1 {
            g = r
        }
        if b == -1 {
            b = g
        }
        let packed = (r as u32).wrapping_shl(16) | (g as u32).wrapping_shl(8) | b as u32;
        let color = [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8];
        let spec = crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed(
            color,
            percent * 32767 / 100,
            (100 - percent) * 32767 / 100,
            15,
        )?;
        Ok(crysirror_image::effects::ImageEffect::RgbLookup { spec })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerGreyState {
    pub frames: i32,
    pub easing: i32,
    pub subframe: i32,
    pub end_percent: i32,
    pub start_percent: i32,
    pub count: i32,
    /// LAYERSEPIA128 target colour (para5..7); `None` is LAYERGREY129.
    pub sepia: Option<[i32; 3]>,
}
impl LayerGreyState {
    /// Legacy LAYERSEPIA128: Set4A7500 copies at most eight arguments with
    /// default colour107,74,43; Print4A7630 uses the LAYERGREY curve and the
    /// MMX kernel4D7EF0 ((y*c)>>8 per channel before the ps256 mix).
    pub fn parse_sepia(values: &[i32]) -> Result<Self> {
        let values = &values[..values.len().min(8)];
        let mut state = Self::parse(&values[..values.len().min(5)])?;
        let color = [
            values.get(5).copied().unwrap_or(107),
            values.get(6).copied().unwrap_or(74),
            values.get(7).copied().unwrap_or(43),
        ];
        ensure!(
            color.iter().all(|c| (0..=255).contains(c)),
            "LAYERSEPIA native byte colour profile"
        );
        state.sepia = Some(color);
        Ok(state)
    }
    pub fn command(&self) -> i32 {
        if self.sepia.is_some() { 128 } else { 129 }
    }
    pub fn parameters(&self) -> Vec<i32> {
        let mut p = vec![self.frames, self.easing, self.subframe, self.end_percent, self.start_percent];
        if let Some(color) = self.sepia {
            p.extend(color);
        }
        p
    }
    pub fn parse(values: &[i32]) -> Result<Self> {
        ensure!(values.len() <= 5, "LAYERGREY native argument count");
        let state = Self {
            frames: values.first().copied().unwrap_or(20),
            easing: values.get(1).copied().unwrap_or(0),
            subframe: values.get(2).copied().unwrap_or(0),
            end_percent: values.get(3).copied().unwrap_or(100),
            start_percent: values.get(4).copied().unwrap_or(0),
            count: 0,
            sepia: None,
        };
        ensure!(
            [state.start_percent, state.end_percent]
                .iter()
                .all(|p| (-10000..=10000).contains(p)),
            "LAYERGREY native finite percent profile"
        );
        Ok(state)
    }
    /// Legacy Nega31 /4a7270 uses signed MMX high products around128.
    /// Native scalar policy maps exactly to a shared integer colour matrix;
    /// no private LUT builder or pixel traversal is retained in this VM.
    pub(crate) fn nega_image_effect_with_random(
        &self,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<crysirror_image::effects::ImageEffect> {
        let endpoint = |percent: i32| -> Result<i32> {
            Ok(percent
                .checked_mul(-508)
                .ok_or_else(|| anyhow::anyhow!("NNN NEGA endpoint overflow"))?
                / 100
                + 254)
        };
        let coefficient = crate::native_curve::value_with_random(
            self.count,
            self.frames,
            endpoint(self.start_percent)?,
            endpoint(self.end_percent)?,
            self.subframe,
            self.easing,
            random,
        )?
        .clamp(-254, 254);
        Ok(crysirror_image::effects::ImageEffect::IntegerColorMatrix {
            spec: crysirror_image::integer_color::ColorMatrixFixed {
                matrix: [
                    [coefficient, 0, 0],
                    [0, coefficient, 0],
                    [0, 0, coefficient],
                ],
                offset: [128 * (256 - coefficient); 3],
                divisor: 256,
            },
        })
    }
    pub fn linear_image_effect(&self) -> Result<crysirror_image::effects::ImageEffect> {
        ensure!(
            self.easing == 0,
            "LAYERGREY native non-linear curve requires shared curve mapping"
        );
        self.image_effect_with(None)
    }
    pub(crate) fn validate_curve(&self) -> Result<()> {
        crate::native_curve::validate(self.frames, self.subframe, self.easing)
    }
    pub(crate) fn image_effect_with_random(
        &self,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<crysirror_image::effects::ImageEffect> {
        self.image_effect_with(Some(random))
    }
    fn image_effect_with(
        &self,
        random: Option<&mut crysirror_image::temporal_pattern::PatternRng>,
    ) -> Result<crysirror_image::effects::ImageEffect> {
        // Original calcu produces an integer first (biased +0.5, truncating
        // toward zero), then integer /100. Do not interpolate an already
        // quantized /100 endpoint: the two orders differ for small changes.
        let start = self.start_percent * 256;
        let end = self.end_percent * 256;
        let value = if let Some(random) = random {
            crate::native_curve::value_with_random(
                self.count,
                self.frames,
                start,
                end,
                self.subframe,
                self.easing,
                random,
            )?
        } else {
            crate::native_curve::value(
                self.count,
                self.frames,
                start,
                end,
                self.subframe,
                self.easing,
            )?
        };
        let amount = (value / 100).clamp(0, 256) as u16;
        Ok(crysirror_image::effects::ImageEffect::FixedLuma {
            spec: crysirror_image::fixed_luma::FixedLumaSpec {
                coefficients: [76, 150, 29],
                groups: [1, 0, 0],
                fractional_bits: 8,
                amount256: amount,
                tint: self.sepia.map(|color| color.map(|c| c as u8)),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fadeout_switches_from_q15_inheritance_to_raw_fill_bytes_at_count_max() {
        let input = crysirror_image::ImageSurface::filled(1, 1, [200, 100, 50, 91]).unwrap();
        let mut state = FadeInState::parse_out(&[2, 10, -1, -1]).unwrap();
        assert_eq!(state.command(), 11);
        state.count = 1;
        assert_eq!(
            input
                .apply_effect(&state.linear_image_effect().unwrap())
                .unwrap()
                .rgba,
            [103, 53, 28, 91]
        );
        state.count = 2;
        assert_eq!(state.command(), 54);
        assert_eq!(
            input
                .apply_effect(&state.linear_image_effect().unwrap())
                .unwrap()
                .rgba,
            [10, 255, 255, 91]
        );
    }
    #[test]
    fn legacy_fade_uses_common_q15_tables_and_opaque_display_contract() {
        let mut state = FadeInState::parse(&[]).unwrap();
        assert_eq!(state.frames, 20);
        state.count = 20;
        let image = crysirror_image::ImageSurface {
            width: 1,
            height: 1,
            rgba: vec![255, 101, 1, 255],
        };
        let result = image
            .apply_effect(&state.linear_image_effect().unwrap())
            .unwrap();
        assert_eq!(result.rgba, [254, 100, 0, 255]);
        state.count = 0;
        assert_eq!(
            image
                .apply_effect(&state.linear_image_effect().unwrap())
                .unwrap()
                .rgba,
            [0, 0, 0, 255]
        );
        state.easing = 1;
        assert!(state.linear_image_effect().is_err());
    }
}

#[cfg(test)]
mod default_tests {
    use super::*;
    #[test]
    fn native_suna_omitted_defaults_are_initialized_and_trailing_words_are_ignored() {
        let empty=SunaParameters::parse(&[],[800,600]).unwrap();
        assert_eq!((empty.frames,empty.transition,empty.start_percent,empty.end_percent,empty.rect),
            (0,false,0,100,[0,0,800,600]));
        let three=SunaParameters::parse(&[20,1,75],[800,600]).unwrap();
        assert_eq!((three.frames,three.transition,three.start_percent,three.end_percent),(20,true,0,75));
        let mut values=vec![20,1,75,12,0,0,9,3,100,50];
        let defined=SunaParameters::parse(&values,[800,600]).unwrap();
        values.extend([i32::MAX;6]);
        assert_eq!(SunaParameters::parse(&values,[800,600]).unwrap(),defined);
        assert_eq!(defined.rect,[8,2,96,50]);
        assert!(SunaParameters::parse(&[],[0,600]).is_err());
    }
}
