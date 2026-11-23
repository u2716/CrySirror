// SPDX-License-Identifier: MPL-2.0
//!44B8C0/42EA20/42EB60 raster policy. Shared captures and RectMap workers
//!own all image allocation, sampling, clipping and pixel traversal.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
pub(super) const OVERLAY: u32 = 60001;
#[derive(Default)]
pub(super) struct Raster {
    pub state: i32,
    pub family: i32,
    amplitude: i32,
    phase: u32,
    vertical_phase: u32,
    next: u64,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    pending: Option<(i32, u32, u32, bool)>,
    overlay: Option<UploadedImage>,
}
impl Raster {
    //4561A0 stores only state/family. Keep the initialized native phase and
    //amplitude from this VM, while dropping transient jobs and GPU leases.
    pub(super) fn detached_seed(&self) -> Self {
        Self {
            state: self.state,
            family: self.family,
            amplitude: self.amplitude,
            phase: self.phase,
            vertical_phase: self.vertical_phase,
            next: self.next,
            ..Default::default()
        }
    }
    pub(super) fn retain_initial_image(&mut self, image: UploadedImage) {
        self.overlay = Some(image);
    }
    pub(super) fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.overlay.as_ref().map(|i| i.path.relative().to_owned())
    }

    pub(super) fn preparing(&self) -> bool {
        self.capture.is_some() || self.process.is_some()
    }
}
//The two original65-DWORD tables at493018/492E10 are the same rounded sine
//sequence. Generate its mathematical contract, rather than embedding a binary
//data block in the adapter. Index64 is the deliberate repeated endpoint.
fn sine(index: u32) -> i32 {
    (32. + 32. * (f64::from(index) * std::f64::consts::PI / 32.).sin()).round() as i32
}
fn horizontal(amplitude: i32, phase: u32) -> RectMap {
    let mut regions: Vec<RectMapRegion> = Vec::with_capacity(300);
    let mut p = phase;
    let mut second = false;
    for y in 0..600 {
        let shift = if p >= 32 {
            -(amplitude * (32 - sine(p)) / 64 / 2)
        } else {
            amplitude * (sine(p) - 32) / 64 / 2
        };
        let (sx, dx) = if shift >= 0 { (shift, 0) } else { (0, -shift) };
        let width = 1024 - shift.abs();
        //Coalesce adjacent equal scanline maps. This changes no sampling and
        //keeps the immutable public effect recipe below its1024-region budget.
        if let Some(last) = regions.last_mut().filter(|last| {
            last.source[0] == sx
                && last.destination[0] == dx
                && last.source[1] + last.source[3] == y
        }) {
            last.source[3] += 1;
            last.destination[3] += 1;
        } else {
            regions.push(RectMapRegion {
                source: [sx, y, width, 1],
                destination: [dx, y, width, 1],
            });
        }
        if p >= 63 {
            p = 0;
            second = false;
        } else if second {
            p += 1;
            second = false;
        } else {
            second = true;
        }
    }
    RectMap {
        output_size: None,
        regions,
        clear: [0, 0, 0, 255],
        sample: RectMapSample::Floor,
        copy_alpha: false,
    }
}
fn vertical(phase: u32) -> RectMap {
    //413F00 coalesces adjacent10-pixel source stripes with an equal signed
    //stretch adjustment. The native boundary samples phase16 and phase32
    //before restarting at0; preserve that asymmetry rather than a generic sin.
    let mut p = if phase > 32 { 0 } else { phase };
    let mut stripe = 0i32;
    let mut dy = 0i32;
    let mut regions = vec![];
    while stripe < 60 {
        let sy = stripe * 10;
        let low = p < 16;
        let adjustment = if low {
            (sine(2 * p) - 32) / 16
        } else {
            (32 - sine(2 * p)) / 16
        };
        let mut source_height = 10;
        let mut height = if low {
            10 - adjustment
        } else {
            10 + adjustment
        };
        p += 1;
        stripe += 1;
        let boundary = if low { 16 } else { 32 };
        while p <= boundary && stripe < 60 {
            let next = if low {
                (sine(2 * p) - 32) / 16
            } else {
                (32 - sine(2 * p)) / 16
            };
            if adjustment != next {
                break;
            }
            source_height += 10;
            height += if low { 10 - next } else { 10 + next };
            p += 1;
            stripe += 1;
        }
        if !low && (p > 32 || stripe >= 60) {
            p = 0;
        }
        let height = height.clamp(0, 600 - dy);
        let source_height = source_height.clamp(0, 600 - sy);
        if height > 0 && source_height > 0 {
            regions.push(RectMapRegion {
                source: [0, sy, 1024, source_height],
                destination: [0, dy, 1024, height],
            });
        }
        dy += height;
    }
    RectMap {
        output_size: None,
        regions,
        clear: [0, 0, 0, 255],
        sample: RectMapSample::NearestRoundLinear,
        copy_alpha: false,
    }
}
impl SugarPotVm {
    pub(super) fn raster_command(
        &mut self,
        args: &[String],
        h: &mut dyn HostServices,
    ) -> Result<bool> {
        let selector = self.number(args, 0, 0)?;
        if self.raster.as_ref().is_some_and(Raster::preparing) {
            return Ok(false);
        }
        let raster = self.raster.get_or_insert_with(Raster::default);
        if selector == 0 {
            raster.state = 3;
        } else {
            raster.state = 1;
            if selector == 1 {
                raster.family = 0;
            } else if selector == 2 {
                raster.family = 1;
            }
        }
        if raster.state == 3 {
            //44B8C0 does not enter its33ms stop loop without sprite0 active.
            if !self.sprites.first().is_some_and(|s| s.visible) {
                raster.amplitude = 0;
                raster.phase = 0;
            } else {
                self.wait = Some(Wait::Raster {
                    remaining: 151,
                    next: h.now_ms().saturating_add(33),
                });
            }
        }
        Ok(true)
    }
    pub(super) fn raster_frame(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let Some(mut raster) = self.raster.take() else {
            return Ok(());
        };
        let result = self.poll_raster(h, &mut raster);
        self.raster = Some(raster);
        result
    }
    fn poll_raster(&mut self, h: &mut dyn HostServices, r: &mut Raster) -> Result<()> {
        if let Some(job) = &mut r.process {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot raster: {e}"),
                ImageProcessState::Ready(resource) => {
                    r.process = None;
                    let finished = r.commit_map(h.now_ms())?;
                    if finished {
                        if let Some(image) = r.overlay.take() {
                            self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
                            h.release_image_resource(self.client, image.path.relative())?;
                        }
                    } else {
                        let image =
                            h.publish_image_resource(self.client, self.next_image, resource)?;
                        self.next_image = self
                            .next_image
                            .checked_add(1)
                            .context("SugarPot raster image ID")?;
                        self.submit(h,vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[999,1],"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
                        if let Some(old) = r.overlay.replace(image) {
                            h.release_image_resource(self.client, old.path.relative())?;
                        }
                    }
                    return Ok(());
                }
            }
        }
        if let Some(job) = &mut r.capture {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(()),
                ImageProcessState::Failed(e) => bail!("SugarPot raster capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    r.capture = None;
                    self.begin_raster_map(h, r, resource)?;
                    return Ok(());
                }
            }
        }
        if r.state == 0 || h.now_ms() < r.next || self.mask_update.is_some() || h.now_ms() < self.transition_until {
            return Ok(());
        }
        r.capture = Some(h.begin_layer_prefix_image_over(
            self.client,
            [1024, 600],
            [999, 0],
            [0, 0, 0, 255],
        )?);
        Ok(())
    }
    fn begin_raster_map(
        &mut self,
        h: &mut dyn HostServices,
        r: &mut Raster,
        source: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let spec = r.prepare_map();
        r.process = Some(
            h.image_metadata(self.client)?.begin_process(
                ImageInput::Resource(source),
                serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RectMap {
                    spec,
                }])?
                .into(),
            )?,
        );
        Ok(())
    }
}
impl Raster {
    /// The map the next tick would apply, without committing it (the UD
    /// target stage is composed through 4383B0's raster boundary once).
    pub(super) fn peek_map(&mut self) -> RectMap {
        let spec = self.prepare_map();
        self.pending = None;
        spec
    }
    pub(super) fn take_overlay(&mut self) -> Option<UploadedImage> {
        self.overlay.take()
    }
    pub(super) fn prepare_map(&mut self) -> RectMap {
        let r = self;
        let mut amplitude = if r.state == 1 {
            0
        } else if r.state == 3 {
            r.amplitude - 4
        } else {
            r.amplitude
        };
        amplitude = amplitude.clamp(-2, 64);
        let mut phase = if r.state == 1 { 0 } else { r.phase };
        let vertical_phase = r.vertical_phase;
        let (spec, finished) = if r.family == 0 {
            if amplitude != 64 {
                amplitude += 2;
            }
            phase = if phase < 63 { phase + 1 } else { 0 };
            (horizontal(amplitude, phase), r.state == 3 && amplitude == 0)
        } else {
            if r.state == 3 {
                amplitude = 0;
            }
            (vertical(vertical_phase), r.state == 3 && amplitude == 0)
        };
        r.pending = Some((
            amplitude,
            phase,
            if vertical_phase > 32 {
                1
            } else {
                vertical_phase + 1
            },
            finished,
        ));
        spec
    }
    pub(super) fn commit_map(&mut self, now: u64) -> Result<bool> {
        let (amplitude, phase, vertical_phase, finished) =
            self.pending.take().context("SugarPot raster completion")?;
        self.amplitude = amplitude;
        self.phase = phase;
        self.vertical_phase = vertical_phase;
        if finished {
            self.state = 0;
        } else if self.state == 1 {
            self.state = 2;
        }
        self.next = now.saturating_add(33);
        Ok(finished)
    }
}
