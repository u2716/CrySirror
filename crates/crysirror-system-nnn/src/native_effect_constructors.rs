// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit software CAllEffect constructor/last Init profiles in native order.
//! MUGEN2 ctor0x4c3520 was read in full through static IDA. It has sixteen
//! slots and lacks137/138; later Source32 cannot silently replace that layout.
use crate::image_bridge::NativePictureViews;
use anyhow::{Result, ensure};
use crysirror_core::image_processing::PackedImageSpec;
use crysirror_image::{
    radial_noise_atlas::{RadialNoiseAtlas, RadialNoiseMath, RadialNoiseTile},
    temporal_pattern::PatternRng,
};
use crysirror_tasks::{Task, TaskPool};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorProfile {
    Mugen2Legacy,
    Source32,
}

#[derive(Clone)]
pub(crate) struct ConstructorAssets {
    /// Each loader must finish its original RGB-then-indexed fallback before
    /// admission; None means both native probes actually returned FALSE.
    pub snow: Option<NativePictureViews>,
    pub snow2: Option<NativePictureViews>,
    pub flower: Option<NativePictureViews>,
}
#[derive(Clone)]
pub(crate) struct Constructed {
    pub canvas:[u32;2],
    pub profile: ConstructorProfile,
    /// Real engine-generated brush bytes, not an acknowledged empty texture.
    /// Four32x32 steam tiles at y=0/32/64/96, two128x128 erase tiles at128/256.
    pub bath_template: PackedImageSpec,
    pub beach: crate::native_beach::BeachClass,
    pub petals: crate::native_hanabira::HanabiraClass,
    pub fireflies: crate::native_hotaru::HotaruClass,
    pub fireflies_new: Option<crate::native_hotaru_new::HotaruNewClass>,
    pub snow: crate::native_snow::SnowClass,
    pub snow2: crate::native_snow::SnowClass,
    pub flower: crate::native_flower::FlowerClass,
    pub rng_after: PatternRng,
}
pub(crate) struct Construction {
    task: Option<Task<Constructed>>,
    ready: Option<Constructed>,
    failed: Option<String>,
}
impl Construction {
    pub(crate) fn source32_software(
        pool: &TaskPool,
        canvas: [u32; 2],
        assets: ConstructorAssets,
        random: PatternRng,
    ) -> Result<Self> {
        Self::begin(ConstructorProfile::Source32, pool, canvas, assets, random)
    }
    pub(crate) fn mugen2_legacy(
        pool: &TaskPool,
        canvas: [u32; 2],
        assets: ConstructorAssets,
        random: PatternRng,
    ) -> Result<Self> {
        Self::begin(
            ConstructorProfile::Mugen2Legacy,
            pool,
            canvas,
            assets,
            random,
        )
    }
    fn begin(
        profile: ConstructorProfile,
        pool: &TaskPool,
        canvas: [u32; 2],
        assets: ConstructorAssets,
        mut random: PatternRng,
    ) -> Result<Self> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN native constructor canvas"
        );
        random.validate()?;
        let task = pool.spawn(move |cancel| {
            cancel.check()?;
            // CAllEffect allocates classes in an explicit sequence, not numeric
            // effect-ID order. BATHWINDOW precedes BEACH; after all constructors
            // a separate numeric Init loop initializes FLOWER12's 200 particles.
            let steam = RadialNoiseMath::Add {
                radius_bias: 8,
                divisor: 1,
                base: 1,
                range: 8,
                minimum: 1,
            };
            // MUGEN2 0x432130 is NOT Source32's later radial multiplication:
            // legacy signed division precedes +6+rand%12 and clamps to1.
            let erase = if profile == ConstructorProfile::Mugen2Legacy {
                RadialNoiseMath::Add {
                    radius_bias: 48,
                    divisor: 2,
                    base: 6,
                    range: 12,
                    minimum: 1,
                }
            } else {
                RadialNoiseMath::Multiply {
                    radius_bias: 64,
                    divisor: 64,
                    base: 20,
                    range: 32,
                }
            };
            let mut tiles: Vec<_> = (0..4)
                .map(|i| RadialNoiseTile {
                    rectangle: [0, i * 32, 32, 32],
                    center: [16, 16],
                    math: steam.clone(),
                })
                .collect();
            tiles.extend((0..2).map(|i| RadialNoiseTile {
                rectangle: [0, 128 + i * 128, 128, 128],
                center: [64, 64],
                math: erase.clone(),
            }));
            // The shared CPU kernel consumes every original constructor draw
            // exactly once and returns immutable packed bytes with the result.
            let bath_template = RadialNoiseAtlas {
                size: [128, 384],
                tiles,
            }
            .prepare(&mut random, &|| cancel.check())?;
            let beach = crate::native_beach::BeachClass::initialize(&mut random)?;
            cancel.check()?;
            let layers = if profile == ConstructorProfile::Source32 {
                32
            } else {
                16
            };
            let petals = crate::native_hanabira::HanabiraClass::initialize(layers, &mut random)?;
            cancel.check()?;
            let fireflies = if profile == ConstructorProfile::Source32 {
                crate::native_hotaru::HotaruClass::initialize(&mut random)?
            } else {
                crate::native_hotaru::HotaruClass::initialize_legacy(&mut random)?
            };
            let fireflies_new = if profile == ConstructorProfile::Source32 {
                Some(crate::native_hotaru_new::HotaruNewClass::initialize(
                    &mut random,
                )?)
            } else {
                None
            };
            cancel.check()?;
            // MUGEN2 SetNewSnow0x4ba470/0x4c0ae0 use one draw/axis in
            // BOTH classes. Only Source32's first SNOW expands the range.
            let snow = crate::native_snow::SnowClass::new(
                canvas,
                profile == ConstructorProfile::Source32,
                assets.snow,
                &mut random,
            )?;
            let snow2 =
                crate::native_snow::SnowClass::new(canvas, false, assets.snow2, &mut random)?;
            cancel.check()?;
            // Missing FLOWER assets still consume 1400 Source32 /1000 MUGEN2
            // Init draws. Missing SNOW returns before its InitSnow draws.
            let flower = if profile == ConstructorProfile::Source32 {
                crate::native_flower::FlowerClass::initialize(canvas, assets.flower, &mut random)?
            } else {
                crate::native_flower::FlowerClass::initialize_legacy(
                    canvas,
                    assets.flower,
                    &mut random,
                )?
            };
            cancel.check()?;
            Ok(Constructed {
                canvas,
                profile,
                bath_template,
                beach,
                petals,
                fireflies,
                fireflies_new,
                snow,
                snow2,
                flower,
                rng_after: random,
            })
        })?;
        Ok(Self {
            task: Some(task),
            ready: None,
            failed: None,
        })
    }
    pub(crate) fn poll(&mut self) -> Result<Option<Constructed>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("NNN effect construction: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self
            .task
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN effect constructor task absent"))?
            .try_take();
        match result {
            Ok(Some(ready)) => {
                self.ready = Some(ready.clone());
                self.task = None;
                Ok(Some(ready))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.failed = Some(format!("{error:#}"));
                Err(error)
            }
        }
    }
}
