// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original Config Print commands over shared immutable image graphs. Native
//! owners retain their candidate button clocks until the complete frame is
//! ready. No private framebuffer, bitmap loop or image algorithm is used.
use crate::{
    image_bridge::NativePictureViews,
    native_title::{NativeButtonSpec, NativeButtonState},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    fixed_scanlines::{FixedScanlines, ScanlineMath},
};
use std::{collections::VecDeque, sync::Arc};
#[derive(Clone)]
pub(crate) enum Draw {
    AppearanceButton {
        paint: crate::native_config_layout::appearance::Paint,
        spec: NativeButtonSpec,
        state: NativeButtonState,
        picture: NativePictureViews,
    },
    Button {
        spec: NativeButtonSpec,
        state: NativeButtonState,
        picture: NativePictureViews,
    },
    Slider {
        picture: NativePictureViews,
        origin: [i32; 2],
        width: i32,
        height: i32,
        percent: i32,
        bad: bool,
        transparent: bool,
    },
}
fn input(draw: Draw, canvas: [u32; 2], base: ImageInput) -> Result<Option<ImageInput>> {
    match draw {
        Draw::AppearanceButton {
            paint,
            spec,
            state,
            picture,
        } => {
            use crate::native_config_layout::appearance::Paint;
            match paint {
                Paint::Normal => {
                    crate::native_button_image::image_input(&spec, &state, &picture, canvas, base)
                }
                Paint::Copy {
                    source,
                    destination,
                    percent,
                } => crate::native_anime_parts_image::regions_image_input(
                    [source; 2],
                    crate::native_anime_parts::PartPrint {
                        destination,
                        paint: if percent == 100 {
                            crate::native_anime_parts::PartPaint::Masked { cell: 0 }
                        } else {
                            crate::native_anime_parts::PartPaint::Translucent { cell: 0, percent }
                        },
                    },
                    &picture,
                    canvas,
                    base,
                ),
                Paint::Stretch {
                    source,
                    destination,
                    percent,
                } => {
                    let Some(plan) = crate::native_stretch_blit::plan(
                        picture.raw.size(),
                        canvas,
                        source,
                        destination,
                        picture.has_mask,
                        true,
                        percent,
                    )?
                    else {
                        return Ok(None);
                    };
                    let math = match plan.math {
                        crate::native_stretch_blit::StretchMath::Copy => ScanlineMath::Stored {
                            math: ByteRasterMath::CopyRgb,
                        },
                        crate::native_stretch_blit::StretchMath::CoverageSeparate {
                            weight,
                            opaque_copy,
                        } => ScanlineMath::CoverageSeparate {
                            weight,
                            opaque_copy,
                        },
                        crate::native_stretch_blit::StretchMath::ConstantSeparate { weights } => {
                            ScanlineMath::ConstantSeparate { weights }
                        }
                    };
                    let spec = FixedScanlines::rectangle(
                        plan.destination,
                        plan.source_q16,
                        plan.source_step_q16,
                        math,
                        plan.corner_rgb_key,
                        None,
                    )?;
                    Ok(Some(ImageInput::Process {
                        input: Box::new(ImageInput::Raster {
                            destination: Box::new(base),
                            source: Box::new(ImageInput::Resource(picture.raw.clone())),
                        }),
                        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                            spec: ByteRaster::FixedScanlines { spec },
                        }])?
                        .into(),
                    }))
                }
            }
        }
        Draw::Button {
            spec,
            state,
            picture,
        } => crate::native_button_image::image_input(&spec, &state, &picture, canvas, base),
        Draw::Slider {
            picture,
            origin,
            width,
            height,
            percent,
            bad,
            transparent,
        } => {
            if width <= 0 || height <= 0 {
                return Ok(None);
            }
            ensure!(
                (0..=100).contains(&percent),
                "NNN Config appearance percentage"
            );
            let percent = if bad { percent / 2 } else { percent };
            // AppearPrint's normal branch unconditionally calls TransBlt3,
            // even when VolumeTransFlag is0. Only bad+nontransparent chooses
            // TransBlt0; do not replace both with a modern opacity blend.
            let math = if !bad || transparent {
                if !picture.has_mask || percent <= 0 {
                    return Ok(None);
                }
                ScanlineMath::Stored {
                    math: ByteRasterMath::MaskPiecewiseWrapping256 {
                        weight: (percent * 256 / 100) as u16,
                    },
                }
            } else {
                // Complete software32 transLucentBlt0::Print uses pmullw,
                // paddw then a SINGLE shift. Win64's broken src increment
                // is not the selected profile. Source coverage is ignored.
                let weight = (percent * 256 / 100) as u16;
                ScanlineMath::ConstantSummed {
                    weights: [weight, 256 - weight],
                }
            };
            let Some((source, at)) = crysirror_image::coverage_bounds::intersect_source_window(
                [0, 0, width, height],
                origin,
                picture.source_clip,
            )?
            else {
                return Ok(None);
            };
            let Some((source, at)) = crysirror_core::region_map::clip_region_copy(
                Some(picture.raw.size()),
                canvas,
                source.map(i64::from),
                at.map(i64::from),
                None,
            )?
            else {
                return Ok(None);
            };
            let spec = FixedScanlines::rectangle(
                [
                    at[0] as i32,
                    at[1] as i32,
                    source[2] as i32,
                    source[3] as i32,
                ],
                [source[0] * 65536, source[1] * 65536],
                [65536, 65536],
                math,
                false,
                None,
            )?;
            Ok(Some(ImageInput::Process {
                input: Box::new(ImageInput::Raster {
                    destination: Box::new(base),
                    source: Box::new(ImageInput::Resource(picture.raw.clone())),
                }),
                effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                    spec: ByteRaster::FixedScanlines { spec },
                }])?
                .into(),
            }))
        }
    }
}
pub(crate) struct FrameWork {
    client: ClientId,
    size: [u32; 2],
    current: Arc<dyn ImageResource>,
    commands: VecDeque<Draw>,
    job: Option<ImageJob>,
    finished: bool,
}
impl FrameWork {
    pub(crate) fn begin(
        client: ClientId,
        base: Arc<dyn ImageResource>,
        commands: Vec<Draw>,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(
            base.client() == client && commands.len() <= 4096,
            "NNN Config frame ownership/budget"
        );
        let mut work = Self {
            client,
            size: base.size(),
            current: base,
            commands: commands.into(),
            job: None,
            finished: false,
        };
        work.admit(host)?;
        Ok(work)
    }
    fn admit(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut graph = ImageInput::Resource(self.current.clone());
        // Source draws retain their original ordering. Materialize small
        // batches through the shared service instead of exceeding graph depth.
        // Each Draw adds two graph levels; twelve stay under the depth bound.
        for _ in 0..12 {
            let Some(command) = self.commands.pop_front() else {
                break;
            };
            if let Some(next) = input(command, self.size, graph.clone())? {
                graph = next;
            }
        }
        self.job = Some(
            host.image_metadata(self.client)?
                .begin_process(graph, b"[]".as_slice().into())?,
        );
        Ok(())
    }
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<Arc<dyn ImageResource>>> {
        if self.finished {
            return Ok(Some(self.current.clone()));
        }
        match self
            .job
            .as_mut()
            .context("NNN Config admitted image job absent")?
            .poll()?
        {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => anyhow::bail!("NNN Config shared Print: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.size,
                    "NNN Config Print result scope"
                );
                self.current = image;
                self.job = None;
                if self.commands.is_empty() {
                    self.finished = true;
                    Ok(Some(self.current.clone()))
                } else {
                    self.admit(host)?;
                    Ok(None)
                }
            }
        }
    }
}
