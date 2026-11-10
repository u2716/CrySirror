// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original534110/452b10 miniature policy and native BGRX serialization. All
//! sampling/arithmetic is an engine BoxReduce job, never an adapter rasterizer.
use crate::{config::Names, native_system_file::SystemStamp};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState},
    service::{CivilTimeQuery, CivilTimeState, HostServices},
    state::ClientId,
};
use crysirror_image::{
    box_reduce::{BoxAlpha, BoxAnchors, BoxArithmetic, BoxReduce},
    effects::ImageEffect,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ThumbnailPolicy {
    canvas: [u32; 2],
    reduce: BoxReduce,
}
pub(crate) struct NativeThumbnail {
    pub size: [u32; 2],
    /// Row-major little-endian XRGB words (B,G,R,0), not engine alpha bytes.
    pub bgrx: Vec<u8>,
}
impl ThumbnailPolicy {
    pub(crate) fn from_image(
        &self,
        client: ClientId,
        pool: TaskPool,
        host: &mut dyn HostServices,
        image: Arc<dyn crysirror_core::image_processing::ImageResource>,
    ) -> Result<ThumbnailJob> {
        ensure!(
            image.client() == client && image.size() == self.canvas,
            "NNN saved menu background scope"
        );
        let effects = serde_json::to_vec(&vec![ImageEffect::BoxReduce {
            spec: self.reduce.clone(),
        }])?;
        let reduce = host
            .image_metadata(client)?
            .begin_process(ImageInput::Resource(image), effects.into())?;
        Ok(ThumbnailJob {
            client,
            pool,
            policy: self.clone(),
            capture: None,
            reduce: Some(reduce),
            encode: None,
            civil: Some(host.begin_civil_time(client)?),
            stamp: None,
            thumbnail: None,
        })
    }
    pub(crate) fn prepare(setup: &Names, canvas: [u32; 2]) -> Result<Self> {
        // Actual542180 initializes6/132/100, then derives aligned dimensions
        // before explicit overrides.452a20 selects fit exactly as below.
        let factor = setup.integer(b"miniCGReduce")?.unwrap_or(6);
        let mut size = [132i32, 100];
        if factor > 0 {
            size = [
                (i32::try_from(canvas[0])? / factor) & !3,
                i32::try_from(canvas[1])? / factor,
            ];
            if factor == 6 && canvas == [800, 600] {
                size[0] = 132;
            }
        }
        size[0] = setup.integer(b"miniCGSizeX")?.unwrap_or(size[0]);
        size[1] = setup.integer(b"miniCGSizeY")?.unwrap_or(size[1]);
        let size = [
            u32::try_from(size[0]).context("NNN negative MINI width")?,
            u32::try_from(size[1]).context("NNN negative MINI height")?,
        ];
        ensure!(size.iter().all(|v| *v > 0), "NNN zero MINI dimensions");
        let dv = factor.max(1) as u32;
        let special = factor == 6 && canvas == [800, 600] && size == [132, 100];
        let fit = [canvas[0] / dv, canvas[1] / dv] == size || special;
        let reduce = if fit {
            // Original division-by-zero/out-of-bounds domains have no defined
            // image. Refuse them instead of invoking undefined native RAM reads.
            ensure!(
                (1..=1024).contains(&factor),
                "NNN fit MINI reduction factor"
            );
            let factor = factor as u32;
            let extent = [
                size[0]
                    .checked_mul(factor)
                    .context("NNN MINI extent overflow")?,
                size[1]
                    .checked_mul(factor)
                    .context("NNN MINI extent overflow")?,
            ];
            ensure!(
                extent[0] <= canvas[0] && extent[1] <= canvas[1],
                "NNN centered MINI exceeds source"
            );
            BoxReduce {
                size,
                source: Some([
                    (canvas[0] - extent[0]) / 2,
                    (canvas[1] - extent[1]) / 2,
                    extent[0],
                    extent[1],
                ]),
                window: [factor as u16; 2],
                anchors: BoxAnchors::Grid {
                    stride: [factor; 2],
                },
                arithmetic: BoxArithmetic::SignedWordHigh {
                    multiplier: (65536 / (factor * factor)) as u16,
                },
                alpha: BoxAlpha::Opaque,
            }
        } else {
            let window = [(canvas[0] / size[0]).max(1), (canvas[1] / size[1]).max(1)];
            BoxReduce {
                size,
                source: None,
                window: [window[0].try_into()?, window[1].try_into()?],
                anchors: BoxAnchors::Proportional,
                arithmetic: BoxArithmetic::Mean,
                alpha: BoxAlpha::Opaque,
            }
        };
        reduce.output_size(canvas)?;
        Ok(Self { canvas, reduce })
    }
    pub(crate) fn begin(
        &self,
        client: ClientId,
        pool: TaskPool,
        host: &mut dyn HostServices,
    ) -> Result<ThumbnailJob> {
        // Actual534110 first Erase()s the current native mode. This immutable
        // prefix includes its retained windows/text; later public no-message/
        // HsaveMask extensions are absent from the reviewed legacy helper.
        let capture = host.begin_layer_prefix_image_over(
            client,
            self.canvas,
            [i32::MAX, i32::MAX],
            [0, 0, 0, 255],
        )?;
        let civil = host.begin_civil_time(client)?;
        Ok(ThumbnailJob {
            client,
            pool,
            policy: self.clone(),
            capture: Some(capture),
            reduce: None,
            encode: None,
            civil: Some(civil),
            stamp: None,
            thumbnail: None,
        })
    }
}
pub(crate) struct ThumbnailJob {
    client: ClientId,
    pool: TaskPool,
    policy: ThumbnailPolicy,
    capture: Option<ImageJob>,
    reduce: Option<ImageJob>,
    encode: Option<Task<NativeThumbnail>>,
    civil: Option<Box<dyn CivilTimeQuery>>,
    stamp: Option<SystemStamp>,
    thumbnail: Option<NativeThumbnail>,
}
impl ThumbnailJob {
    pub(crate) fn poll(
        &mut self,
        host: &mut dyn HostServices,
    ) -> Result<Option<(NativeThumbnail, SystemStamp)>> {
        if let Some(civil) = self.civil.as_mut() {
            match civil.poll()? {
                CivilTimeState::Pending => {}
                CivilTimeState::Failed(reason) => anyhow::bail!("NNN MINI civil time: {reason}"),
                CivilTimeState::Ready(time) => {
                    self.stamp = Some(SystemStamp::from_civil(time, self.client)?);
                    self.civil = None;
                }
            }
        }
        if let Some(capture) = self.capture.as_mut() {
            match capture.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(reason) => anyhow::bail!("NNN MINI capture: {reason}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == self.policy.canvas,
                        "NNN MINI capture scope"
                    );
                    let effects = serde_json::to_vec(&vec![ImageEffect::BoxReduce {
                        spec: self.policy.reduce.clone(),
                    }])?;
                    self.reduce = Some(
                        host.image_metadata(self.client)?
                            .begin_process(ImageInput::Resource(image), effects.into())?,
                    );
                    self.capture = None;
                    return Ok(None);
                }
            }
        }
        if let Some(reduce) = self.reduce.as_mut() {
            match reduce.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(reason) => anyhow::bail!("NNN MINI reduction: {reason}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == self.policy.reduce.size,
                        "NNN MINI output scope"
                    );
                    let size = image.size();
                    let rgba: Arc<[u8]> = image.rgba();
                    self.encode = Some(self.pool.spawn(move |cancel| {
                        ensure!(
                            rgba.len() == size[0] as usize * size[1] as usize * 4,
                            "NNN MINI byte layout"
                        );
                        let mut bgrx = vec![0; rgba.len()];
                        for row in 0..size[1] as usize {
                            cancel.check()?;
                            let begin = row * size[0] as usize * 4;
                            for at in (begin..begin + size[0] as usize * 4).step_by(4) {
                                bgrx[at..at + 4].copy_from_slice(&[
                                    rgba[at + 2],
                                    rgba[at + 1],
                                    rgba[at],
                                    0,
                                ]);
                            }
                        }
                        Ok(NativeThumbnail { size, bgrx })
                    })?);
                    self.reduce = None;
                    return Ok(None);
                }
            }
        }
        if let Some(encode) = self.encode.as_mut() {
            let Some(thumbnail) = encode.try_take()? else {
                return Ok(None);
            };
            self.thumbnail = Some(thumbnail);
            self.encode = None;
        }
        if self.stamp.is_some() && self.thumbnail.is_some() {
            return Ok(Some((
                self.thumbnail.take().unwrap(),
                self.stamp.take().unwrap(),
            )));
        }
        Ok(None)
    }
}
