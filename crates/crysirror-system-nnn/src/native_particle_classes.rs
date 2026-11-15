// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Transaction-copyable class owners for natural effects. The original saves
//! contain EFFECT parameters, not these shared per-class particle arrays. A
//! restore replays only each class's actual InitParameter hook in slot order.
use crate::{
    native_constructor_preparation::Ready,
    native_effect_constructors::{Constructed, ConstructorProfile},
};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{ImageInput, ImageResource};
use crysirror_image::temporal_pattern::PatternRng;
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct Classes {
    pub hahen:Arc<crate::native_hahen::HahenClass>,
    pub profile: ConstructorProfile,
    pub beach: Arc<crate::native_beach::BeachClass>,
    pub petals: Arc<crate::native_hanabira::HanabiraClass>,
    pub fireflies: Arc<crate::native_hotaru::HotaruClass>,
    pub fireflies_new: Option<Arc<crate::native_hotaru_new::HotaruNewClass>>,
    pub snow: Arc<crate::native_snow::SnowClass>,
    pub snow2: Arc<crate::native_snow::SnowClass>,
    pub flower: Arc<crate::native_flower::FlowerClass>,
    pub bath: crate::native_bath_window::BathWindowClass,
    pub brush_atlas: Arc<dyn ImageResource>,
    pub palette: crate::native_png_palette::PngPalette,
    pub indexed: [Option<crate::native_indexed_source::Decoded>; 4],
    pub kira: Option<crate::image_bridge::NativePictureViews>,
    flower_printed: bool,
    snow_printed: [bool; 2],
}
impl Classes {
    /// Constructor state and the global RNG candidate are separate returns:
    /// caller installs both only after all image dependencies are ready.
    pub(crate) fn from_ready(ready: Ready) -> Result<(Self, PatternRng)> {
        let canvas=ready.classes.canvas;
        let Constructed {
            profile,
            beach,
            petals,
            fireflies,
            fireflies_new,
            snow,
            snow2,
            flower,
            rng_after,
            ..
        } = ready.classes;
        ensure!(
            ready.brush_atlas.size() == [128, 384],
            "NNN particle brush atlas shape"
        );
        Ok((
            Self {
                hahen:Arc::new(crate::native_hahen::HahenClass::new(canvas)?),
                profile,
                beach: Arc::new(beach),
                petals: Arc::new(petals),
                fireflies: Arc::new(fireflies),
                fireflies_new: fireflies_new.map(Arc::new),
                snow: Arc::new(snow),
                snow2: Arc::new(snow2),
                flower: Arc::new(flower),
                bath: Default::default(),
                brush_atlas: ready.brush_atlas,
                palette: ready.assets.palette,
                indexed: ready.assets.indexed,
                kira: ready.assets.kira,
                flower_printed: false,
                snow_printed: [false; 2],
            },
            rng_after,
        ))
    }
    /// CAllEffect::BeginPrint calls each class once. Do not reset between two
    /// effect slots of the same class or when polling a pending image job.
    pub(crate) fn begin_print(&mut self) {
        self.flower_printed = false;
        self.snow_printed = [false; 2];
    }
    pub(crate) fn print_flower(
        &mut self,
        p: &[i32; 4],
        canvas: [u32; 2],
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<ImageInput>> {
        if !self.flower_printed {
            Arc::make_mut(&mut self.flower).advance_print(canvas, random)?;
            self.flower_printed = true;
        }
        self.flower.image_input(p, base)
    }
    pub(crate) fn print_snow(
        &mut self,
        second: bool,
        p: &[i32; 3],
        canvas: [u32; 2],
        base: ImageInput,
        random: &mut PatternRng,
    ) -> Result<Option<ImageInput>> {
        let index = usize::from(second);
        let snow = if second {
            &mut self.snow2
        } else {
            &mut self.snow
        };
        if !self.snow_printed[index] {
            Arc::make_mut(snow).advance_print(canvas, random)?;
            self.snow_printed[index] = true;
        }
        snow.image_input(p, base)
    }
    /// InitParameter delegates to SetParam for these classes only. FLOWER,
    /// BEACH, CLOUD and BATHWINDOW inherit an empty hook and keep class state.
    pub(crate) fn set(
        &mut self,
        command: i32,
        parameters: &[i32],
        slot: i32,
        canvas: [u32; 2],
        random: &mut PatternRng,
    ) -> Result<()> {
        match command {
            14 => Arc::make_mut(&mut self.snow).reset(canvas, random)?,
            57 => Arc::make_mut(&mut self.snow2).reset(canvas, random)?,
            70 => {
                let p: [i32; 24] = parameters
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("NNN HANABIRA restore words"))?;
                Arc::make_mut(&mut self.petals).set(&p, slot, random)?;
            }
            71 => {
                let p: [i32; 20] = parameters
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("NNN HOTARU restore words"))?;
                Arc::make_mut(&mut self.fireflies).set(&p, random)?;
            }
            138 => {
                ensure!(
                    self.profile == ConstructorProfile::Source32,
                    "NNN HOTARUNEW legacy class absence"
                );
                let p: [i32; 32] = parameters
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("NNN HOTARUNEW restore words"))?;
                Arc::make_mut(
                    self.fireflies_new
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("NNN HOTARUNEW constructor absent"))?,
                )
                .set(&p, random)?;
            }
            12 | 82 | 83 | 97 => (),
            _ => anyhow::bail!("NNN natural class restore command {command}"),
        }
        Ok(())
    }
    pub(crate) fn restore_setter(
        &mut self,
        command: i32,
        p: &[i32],
        slot: i32,
        canvas: [u32; 2],
        random: &mut PatternRng,
    ) -> Result<()> {
        if command == 138 {
            let raw: [i32; 32] = p
                .try_into()
                .map_err(|_| anyhow::anyhow!("NNN HOTARUNEW raw parameter span"))?;
            let parameters = crate::native_hotaru_new::restored(&raw, canvas);
            self.set(command, &parameters, slot, canvas, random)
        } else {
            self.set(command, p, slot, canvas, random)
        }
    }
}
