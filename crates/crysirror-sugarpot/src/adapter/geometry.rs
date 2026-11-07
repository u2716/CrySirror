// SPDX-License-Identifier: MPL-2.0
//!448530 EF20..28 ->434AB0/4354C0. Native coordinates and frame counters only;
//!immutable engine RectMap/Composite jobs own resampling and allocation.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::rect_map::{RectMap, RectMapRegion, RectMapSample};
const OVERLAY: u32 = 65002;
type Resource = Arc<dyn ImageResource>;
pub(super) struct Geometry {
    target: i32,
    selector: i32,
    frame: i32,
    source: Option<Resource>,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    image: Option<UploadedImage>,
    destination: [i32; 4],
    depth: i16,
}
impl Geometry {
    pub(super) fn preparing(&self) -> bool {
        self.capture.is_some() || self.process.is_some()
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.image
            .as_ref()
            .map(|image| image.path.relative().to_owned())
    }
    pub(super) fn owns_sprite(&self, id: usize) -> bool {
        self.target == id as i32
    }
}
fn whole(selector: i32, p: i32) -> RectMapRegion {
    let x = p * 512 / 8;
    let y = p * 300 / 8;
    let (source, destination) = match selector {
        20 => ([x / 2, 0, 1024 - x, 600], [0, y, 1024, 600 - y]),
        21 => ([x / 2, 0, 1024 - x, 600], [0, 0, 1024, 600 - y]),
        22 => ([0, y / 2, 1024, 600 - y], [0, 0, 1024 - x, 600]),
        23 => ([0, y / 2, 1024, 600 - y], [x, 0, 1024 - x, 600]),
        24 => ([0, y / 2, 1024, 600 - y / 2], [0, 0, 1024, 600]),
        25 => ([0, 0, 1024, 600 - y / 2], [0, 0, 1024, 600]),
        26 => ([0, 0, 1024 - x, 600], [0, 0, 1024, 600]),
        27 => ([x / 2, 0, 1024 - x / 2, 600], [0, 0, 1024, 600]),
        _ => unreachable!(),
    };
    RectMapRegion {
        source,
        destination,
    }
}
///4354C0's pointer increments are unusual but completely initialized: each
///right strip begins one pixel earlier; the left copy starts at source X
///offset (not source Y), then advances rows only after the top clear band.
fn scanline(frame: i32) -> Vec<RectMapRegion> {
    if frame == 0 {
        return vec![RectMapRegion {
            source: [0, 0, 1024, 600],
            destination: [0, 0, 1024, 600],
        }];
    }
    let offset = frame.min(7) * (600 / 16);
    let mut regions = Vec::with_capacity((1200 - offset) as usize);
    for y in 0..600 {
        let start = 944 - y;
        regions.push(RectMapRegion {
            source: [start, y, 1024 - start, 1],
            destination: [start, y, 1024 - start, 1],
        });
        if y >= offset {
            regions.push(RectMapRegion {
                source: [offset, y - offset, start, 1],
                destination: [0, y, start, 1],
            });
        }
    }
    regions
}
///434E10 clips the original sprite first, stretches that rectangle, adjusts
///source ends with f32 ratios, clips again, then generates a new nearest table.
///The source cell selector and ATR are deliberately not consulted by this
///helper:408D40 exposes its complete original24/32-bit image.
fn single(selector: i32, p: i32, position: [i32; 2], size: [u32; 2]) -> Option<RectMapRegion> {
    let [x, y] = position;
    let left = x.max(0);
    let top = y.max(0);
    let right = (i64::from(x) + i64::from(size[0])).clamp(0, 1024) as i32;
    let bottom = (i64::from(y) + i64::from(size[1])).clamp(0, 600) as i32;
    let w = right - left;
    let h = bottom - top;
    if w <= 0 || h <= 0 {
        return None;
    }
    let ax = p * (w / 2) / 8;
    let ay = p * (h / 2) / 8;
    let [dx, dy, dw, dh] = match selector {
        20 => [left - ax, top + ay, w + p * w / 8, h - ay],
        21 => [left - ax, top, w + p * w / 8, h - ay],
        22 => [left, top - ay, w - ax, h + p * h / 8],
        23 => [left + ax, top - ay, w - ax, h + p * h / 8],
        24 => [left, top - ay, w, h + ay],
        25 => [left, top, w, h + ay],
        26 => [left, top, w + ax, h],
        27 => [left - ax, top, w + ax, h],
        _ => unreachable!(),
    };
    if dw <= 0 || dh <= 0 {
        return None;
    }
    let mut sx = (-x).max(0);
    let mut sy = (-y).max(0);
    let mut ex = sx + w;
    let mut ey = sy + h;
    let rx = (f64::from(w) / f64::from(dw)) as f32;
    let ry = (f64::from(h) / f64::from(dh)) as f32;
    if dx < 0 {
        sx += (f64::from(-dx) * f64::from(rx)) as i32;
    }
    if dy < 0 {
        sy += (f64::from(-dy) * f64::from(ry)) as i32;
    }
    if dx + dw > 1024 {
        ex -= (f64::from(dx + dw - 1024) * f64::from(rx)) as i32;
    }
    if dy + dh > 600 {
        ey -= (f64::from(dy + dh - 600) * f64::from(ry)) as i32;
    }
    let clipped = [
        dx.max(0),
        dy.max(0),
        (dx + dw).min(1024) - dx.max(0),
        (dy + dh).min(600) - dy.max(0),
    ];
    (ex > sx && ey > sy && clipped[2] > 0 && clipped[3] > 0).then_some(RectMapRegion {
        source: [sx, sy, ex - sx, ey - sy],
        destination: clipped,
    })
}
impl SugarPotVm {
    pub(super) fn begin_geometry(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        selector: i32,
    ) -> Result<bool> {
        //4354C0 has no sprite-ID branch; EF28 always processes the whole
        //original1024x600 stage, including when argument0 names one sprite.
        let target = if selector == 28 {
            -1
        } else {
            self.number(args, 0, 0)?
        };
        ensure!(
            target == -1 || (target >= 0 && (target as usize) < self.sprites.len()),
            "SugarPot EF geometry sprite ID"
        );
        for window in &mut self.windows {
            window.visibility = 2;
        }
        self.publish(h, None)?;
        let capture = if target == -1 {
            Some(h.begin_layer_prefix_image_over(
                self.client,
                [1024, 600],
                [1999, 0],
                [0, 0, 0, 255],
            )?)
        } else {
            None
        };
        self.geometry = Some(Geometry {
            target,
            selector,
            frame: self.native_effect_index,
            source: None,
            capture,
            process: None,
            image: None,
            destination: [0, 0, 1024, 600],
            depth: 24,
        });
        self.wait = Some(Wait::Geometry);
        Ok(true)
    }
    pub(super) fn geometry_frame(&mut self, h: &mut dyn HostServices, skip: bool) -> Result<bool> {
        let mut work = self
            .geometry
            .take()
            .context("SugarPot EF geometry wait without controller")?;
        let result = self.poll_geometry(h, &mut work, skip);
        if !matches!(result, Ok(true)) {
            self.geometry = Some(work);
        }
        result
    }
    fn poll_geometry(
        &mut self,
        h: &mut dyn HostServices,
        work: &mut Geometry,
        skip: bool,
    ) -> Result<bool> {
        if skip || self.settings.header[4] != 0 || work.frame >= 16 {
            if h.now_ms() < self.native_effect_next {
                return Ok(false);
            }
            self.native_effect_next = h.now_ms().saturating_add(33);
            self.native_effect_index = 0;
            //4383B0's terminal branch reconstructs the ordinary scene; EF20
            //does not write any transformed coordinate back to a saved sprite.
            self.publish(h, None)?;
            if let Some(image) = work.image.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(true);
        }
        if let Some(job) = &mut work.capture {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("SugarPot EF whole source: {error}"),
                ImageProcessState::Ready(resource) => {
                    work.source = Some(resource);
                    work.capture = None;
                }
            }
        }
        if let Some(job) = &mut work.process {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("SugarPot EF rectangle map: {error}"),
                ImageProcessState::Ready(resource) => {
                    let image = h.publish_image_resource(self.client, self.next_image, resource)?;
                    self.next_image = self
                        .next_image
                        .checked_add(1)
                        .context("SugarPot EF image ID")?;
                    let (id, order) = if work.target == -1 {
                        (OVERLAY, 1999)
                    } else {
                        (
                            work.target as u32 + 1,
                            if work.target == 0 {
                                0
                            } else {
                                100 + work.target
                            },
                        )
                    };
                    self.submit(h,vec![json!({"op":"put","layer":{"id":id,"rect":[0,0,1024,600],"viewport_clip":work.destination,
                        "order":[order,0],"blend":Self::native_image_blend(work.depth,0),"sampling":{"linear":false},
                        "paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
                    if !self.published_layers.contains(&id) {
                        self.published_layers.push(id);
                    }
                    if let Some(old) = work.image.replace(image) {
                        h.release_image_resource(self.client, old.path.relative())?;
                    }
                    work.process = None;
                    work.frame += 1;
                    self.native_effect_index = work.frame;
                    //54..61 increment the original index once per completed
                    //native pump, not by elapsed/duration or by worker polls.
                    return Ok(false);
                }
            }
        }
        if h.now_ms() < self.native_effect_next {
            return Ok(false);
        }
        self.native_effect_next = h.now_ms().saturating_add(33);
        let p = if work.frame < 8 {
            work.frame
        } else {
            16 - work.frame
        };
        let (input, regions) = if work.target == -1 {
            work.destination = [0, 0, 1024, 600];
            work.depth = 24;
            (
                ImageInput::Resource(
                    work.source
                        .as_ref()
                        .context("SugarPot EF source capture")?
                        .clone(),
                ),
                if work.selector == 28 {
                    scanline(work.frame)
                } else {
                    vec![whole(work.selector, p)]
                },
            )
        } else {
            let sprite = &self.sprites[work.target as usize];
            let Some(image) = sprite
                .image
                .as_ref()
                .filter(|_| sprite.visible && sprite.system == 0)
            else {
                work.frame += 1;
                self.native_effect_index = work.frame;
                return Ok(false);
            };
            let Some(region) = single(work.selector, p, sprite.position, image.size) else {
                work.frame += 1;
                self.native_effect_index = work.frame;
                return Ok(false);
            };
            ensure!(
                matches!(sprite.native_depth, 24 | 32),
                "SugarPot EF original image depth"
            );
            let resource = h
                .published_image_resource(self.client, image.path.relative())?
                .context("SugarPot EF source image not resident")?;
            work.destination = region.destination;
            work.depth = sprite.native_depth;
            //Keep the complete original source address space. The old
            //crop-to-canvas intermediate lost initialized endpoint samples of
            //40F970/40F9F0. Shared RectMap owns the independent destination
            //extent and bounded linear border policy.
            (ImageInput::Resource(resource), vec![region])
        };
        let map = RectMap {
            output_size: Some([1024, 600]),
            regions,
            clear: [0, 0, 0, if work.target == -1 { 255 } else { 0 }],
            sample: RectMapSample::NearestRoundLinear,
            copy_alpha: work.target != -1,
        };
        work.process = Some(
            h.image_metadata(self.client)?.begin_process(
                input,
                serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RectMap {
                    spec: map,
                }])?
                .into(),
            )?,
        );
        Ok(false)
    }
}
