//! Native derived-image policy over engine-owned immutable image jobs.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_image::{
    effects::ImageEffect,
    integer_raster::{IntegerRaster, IntegerRasterOperation as Raster, display::DisplayRaster},
};
#[derive(Clone)]
pub(super) enum GraphInput {
    Bitmap(i32),
    Frozen(Arc<dyn ImageResource>),
    Solid([u32; 2], [u8; 4]),
    Prefix([u32; 2], [i32; 2]),
}
#[derive(Clone)]
pub(super) struct GraphPlan {
    pub inputs: Vec<GraphInput>,
    pub operation: Option<Raster>,
    pub quad: Option<[u32; 4]>,
    pub format: i32,
    pub force_opaque: bool,
    pub filters: Vec<(u8, i32, u16)>,
}
impl GraphPlan {
    pub fn bitmap(id: i32, format: i32) -> Self {
        Self {
            inputs: vec![GraphInput::Bitmap(id)],
            operation: None,
            quad: None,
            format,
            force_opaque: format == 1,
            filters: Vec::new(),
        }
    }
}
enum Prepared {
    Ready(Arc<dyn ImageResource>),
    Job(ImageJob),
    Solid([u32; 2], [u8; 4]),
}
struct Work {
    inputs: Vec<Prepared>,
    operation: Option<Raster>,
    quad: Option<[u32; 4]>,
    filters: Vec<(u8, i32, u16)>,
    phase: Option<ImageJob>,
    force_opaque: bool,
    format: i32,
    stage: u8,
}
pub(super) struct ImagePaint {
    pub path: String,
    pub image: Arc<dyn ImageResource>,
}
#[derive(Default)]
pub(super) struct GraphImages {
    pub plans: BTreeMap<i32, GraphPlan>,
    pub painted: BTreeMap<i32, ImagePaint>,
    pub dirty: std::collections::BTreeSet<i32>,
    jobs: BTreeMap<i32, Work>,
    pub masks: BTreeMap<i32, Arc<dyn ImageResource>>,
    pub mode3_generation: BTreeMap<i32, (u32, u64)>,
    pub expired: std::collections::BTreeSet<i32>,
    pub retired: Vec<String>,
    pub serial: u64,
    pub revisions: BTreeMap<i32, u64>,
}
impl GraphImages {
    pub fn remove(&mut self, object: i32) {
        self.plans.remove(&object);
        if let Some(old) = self.painted.remove(&object) {
            self.retired.push(old.path);
        }
        self.jobs.remove(&object);
        self.masks.remove(&object);
        self.mode3_generation.remove(&object);
        self.expired.remove(&object);
        self.dirty.remove(&object);
    }
    pub fn plan(&mut self, object: i32, plan: GraphPlan) {
        self.plans.insert(object, plan);
        self.jobs.remove(&object);
        self.dirty.insert(object);
        self.serial = self.serial.wrapping_add(1);
        self.revisions.insert(object, self.serial);
    }
    pub fn dirty(&mut self, object: i32) {
        if self.plans.contains_key(&object) {
            self.jobs.remove(&object);
            self.dirty.insert(object);
            self.serial = self.serial.wrapping_add(1);
            self.revisions.insert(object, self.serial);
        }
    }
}
fn image_input(input: &Prepared) -> Result<ImageInput> {
    Ok(match input {
        Prepared::Ready(image) => ImageInput::Resource(image.clone()),
        Prepared::Solid(size, color) => ImageInput::Solid {
            size: *size,
            color: *color,
        },
        Prepared::Job(_) => bail!("BGI pending image input"),
    })
}
fn job_image(job: &mut ImageJob) -> Result<Option<Arc<dyn ImageResource>>> {
    Ok(match job.poll()? {
        ImageProcessState::Pending => None,
        ImageProcessState::Failed(error) => bail!("BGI graph image: {error}"),
        ImageProcessState::Ready(image) => Some(image),
    })
}
impl BgiVm {
    pub(crate) fn graph_bitmap_dependencies(
        &self,
        call: &NativeCall,
        vm: &Machine,
    ) -> Result<Vec<u32>> {
        if !super::handles(call.group, call.id) {
            return Ok(Vec::new());
        }
        let a = &call.args;
        let object = if call.group == 0x90 && matches!(call.id, 0x40..=0x4d) {
            0
        } else if call.group == 0x91 && matches!(call.id, 0x38 | 0x3d) {
            a.get(1).copied().unwrap_or(0)
        } else {
            a.first().copied().unwrap_or(0)
        };
        let mut ids = Vec::new();
        let indices: &[usize] = match (call.group, call.id) {
            (0x90, 0x58 | 0x5a) => &[3, 4],
            (0x90, 0x40 | 0x46 | 0x48 | 0x49) => &[0],
            (0x90, 0x41 | 0x4a) => &[0, 1],
            (0x90, 0x42) => &[0, 1, 2, 3],
            (0x90, 0x75) => &[3],
            (0x90, 0x66) => &[3],
            _ => &[],
        };
        for &index in indices {
            if let Some(&id) = a.get(index) {
                if (id as u32) < 0x4000 {
                    ids.push(id as u32);
                }
            }
        }
        for object in self.graph.descendants(object) {
            if let Some(plan) = self.graph.images.plans.get(&object) {
                for input in &plan.inputs {
                    if let GraphInput::Bitmap(id) = input {
                        ids.push(*id as u32);
                    }
                }
            }
            if let Some(bitmap) = self
                .graph
                .objects
                .get(&object)
                .and_then(|p| p.format_resource)
            {
                if (bitmap as u32) < 0x4000 {
                    ids.push(bitmap as u32);
                }
            }
        }
        if call.group == 0x90 && call.id == 0x44 {
            let count = usize::try_from(a[0])?;
            ensure!((2..=32).contains(&count), "BGI frame table count");
            let bytes = vm.memory(a[1] as u32, count * 4)?;
            for bytes in bytes.chunks_exact(4) {
                let id = u32::from_le_bytes(bytes.try_into().unwrap());
                if id < 0x4000 {
                    ids.push(id);
                }
            }
        }
        if let Some(frames) = self.graph.frames.get(&object) {
            ids.extend(frames.iter().map(|&(id, _)| id as u32));
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
    pub(crate) fn invalidate_graph_bitmap(&mut self, bitmap: u32) {
        let raw = self
            .sprites
            .iter()
            .filter_map(|(&id, s)| (s.bitmap == Some(bitmap)).then_some(id as i32))
            .collect::<Vec<_>>();
        for id in raw {
            self.graph.mark_visual(id);
        }
        let masks = self
            .graph
            .filters
            .iter()
            .filter_map(|(&id, f)| {
                f.mask
                    .is_some_and(|(b, _)| b as u32 == bitmap)
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        for id in masks {
            self.graph.filters.get_mut(&id).unwrap().signature = None;
        }

        let affected = self
            .graph
            .images
            .plans
            .iter()
            .filter_map(|(&object, plan)| {
                plan.inputs
                    .iter()
                    .any(|input| matches!(input,GraphInput::Bitmap(id)if *id as u32==bitmap))
                    .then_some(object)
            })
            .collect::<Vec<_>>();
        let mut affected = affected;
        affected.extend(self.graph.frames.iter().filter_map(|(&object, frames)| {
            frames
                .iter()
                .any(|&(id, _)| id as u32 == bitmap)
                .then_some(object)
        }));
        affected.sort_unstable();
        affected.dedup();
        for object in affected {
            // CDspObjMap retains a copied tile cache. Atlas pixel changes do
            // not repaint it until the native invalidation/viewport operation.
            if !self.graph.maps.contains_key(&object) {
                self.graph.images.dirty(object);
            }
        }
    }
    pub(crate) fn release_graph_retired(&mut self, host: &mut dyn HostServices) -> Result<()> {
        for path in std::mem::take(&mut self.graph.images.retired) {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
    pub(crate) fn graph_tick(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.release_graph_retired(host)?;
        let objects = self.graph.images.dirty.iter().copied().collect::<Vec<_>>();
        let mut changed = false;
        for object in objects {
            changed |= self.refresh_graph_image(host, object)?;
        }
        if changed || self.graph.needs_publish {
            self.publish_graph(host)?;
        }
        self.refresh_native_filters(host)?;
        Ok(())
    }
    pub(super) fn set_graph_plan(&mut self, object: i32, mut plan: GraphPlan) {
        plan.filters = self
            .graph
            .objects
            .get(&object)
            .map(|p| {
                (0..16)
                    .filter_map(|index| p.properties.get(&(0x1100 | index)).copied())
                    .filter_map(|(packed, color)| {
                        let kind = ((packed as u32 >> 8) & 255) as u8;
                        (kind != 0).then_some((kind, color, (packed as u32 >> 16) as u16))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.graph.images.plan(object, plan);
    }
    fn expire_graph_image(&mut self, host: &mut dyn HostServices, object: i32) -> Result<bool> {
        self.hide_graph_object(host, object)?;
        let painted = self.graph.images.painted.remove(&object);
        if let Some(painted) = painted {
            self.graph.images.retired.push(painted.path);
        }
        self.graph.images.jobs.remove(&object);
        self.graph.images.dirty.remove(&object);
        self.graph.images.expired.insert(object);
        self.release_graph_retired(host)?;
        Ok(true)
    }
    pub(super) fn refresh_graph_image(
        &mut self,
        host: &mut dyn HostServices,
        object: i32,
    ) -> Result<bool> {
        if let Some(&(bitmap, generation)) = self.graph.images.mode3_generation.get(&object) {
            if self
                .bitmaps
                .get(&bitmap)
                .is_none_or(|b| b.generation != generation)
            {
                return self.expire_graph_image(host, object);
            }
        }
        if self.graph.frames.get(&object).is_some_and(|frames| {
            frames.iter().any(|&(id, generation)| {
                self.bitmaps
                    .get(&(id as u32))
                    .is_none_or(|bitmap| bitmap.generation != generation)
            })
        }) {
            return self.expire_graph_image(host, object);
        }
        if self.graph.filters.contains_key(&object) {
            let cut = self.graph.tree.chain_position(object).unwrap_or(0);
            if self.graph.images.dirty.iter().any(|id| {
                *id != object
                    && self
                        .graph
                        .tree
                        .chain_position(*id)
                        .is_some_and(|position| position < cut)
            }) {
                return Ok(false);
            }
        }
        if !self.graph.images.dirty.contains(&object) {
            return Ok(false);
        }
        let mut work = if let Some(work) = self.graph.images.jobs.remove(&object) {
            work
        } else {
            let Some(plan) = self.graph.images.plans.get(&object).cloned() else {
                self.graph.images.dirty.remove(&object);
                return Ok(false);
            };
            let mut inputs = Vec::new();
            for input in plan.inputs {
                inputs.push(match input {
                    GraphInput::Frozen(image) => Prepared::Ready(image),
                    GraphInput::Solid(size, color) => Prepared::Solid(size, color),
                    GraphInput::Prefix(size, cut) => {
                        self.publish_graph(host)?;
                        Prepared::Job(host.begin_layer_prefix_image_over(
                            self.client,
                            size,
                            cut,
                            [0, 0, 0, 255],
                        )?)
                    }
                    GraphInput::Bitmap(id) => {
                        let (_, bitmap) = self.bitmap(id)?;
                        if let Some(image) = &bitmap.image {
                            Prepared::Ready(image.clone())
                        } else {
                            Prepared::Job(host.begin_canvas_image(self.client, id as u32)?)
                        }
                    }
                });
            }
            Work {
                inputs,
                operation: plan.operation,
                quad: plan.quad,
                filters: plan.filters,
                phase: None,
                force_opaque: plan.force_opaque,
                format: plan.format,
                stage: 0,
            }
        };
        loop {
            let current = if let Some(job) = &mut work.phase {
                let Some(image) = job_image(job)? else {
                    self.graph.images.jobs.insert(object, work);
                    return Ok(false);
                };
                work.phase = None;
                image
            } else {
                for input in &mut work.inputs {
                    if let Prepared::Job(job) = input {
                        let Some(image) = job_image(job)? else {
                            self.graph.images.jobs.insert(object, work);
                            return Ok(false);
                        };
                        *input = Prepared::Ready(image);
                    }
                }
                if let Some([x, y, w, h]) = work.quad.take() {
                    ensure!(work.inputs.len() == 4, "BGI four-cell backdrop image count");
                    let rects = [
                        ([x, y, w - x, h - y], [0, 0]),
                        ([0, y, x, h - y], [w - x, 0]),
                        ([x, 0, w - x, y], [0, h - y]),
                        ([0, 0, x, y], [w - x, h - y]),
                    ];
                    let mut layers = Vec::new();
                    for (index, (rect, destination)) in rects.into_iter().enumerate() {
                        layers.push(crysirror_core::image_processing::ImageCompositeLayer {
                            input: Box::new(image_input(&work.inputs[index])?),
                            source: rect.map(|n| n as i32),
                            destination: destination.map(|n| n as i32),
                            clip: None,
                            opacity: 255,
                            math: crysirror_core::image_processing::ImageCompositeMath::Copy,
                        });
                    }
                    work.phase = Some(host.image_metadata(self.client)?.begin_process(
                        ImageInput::Composite {
                            base: Box::new(ImageInput::Solid {
                                size: [w, h],
                                color: [0; 4],
                            }),
                            layers,
                        },
                        Arc::from(b"[]".as_slice()),
                    )?);
                    work.stage = 1;
                    continue;
                }
                if matches!(
                    &work.operation,
                    Some(Raster::Display {
                        spec: DisplayRaster::ThresholdMask { .. }
                    })
                ) {
                    if let Some(Prepared::Ready(mask)) = work.inputs.get(1) {
                        self.graph.images.masks.insert(object, mask.clone());
                        if let Some(plan) = self.graph.images.plans.get_mut(&object) {
                            plan.inputs[1] = GraphInput::Frozen(mask.clone());
                        }
                    }
                }
                if let Some(operation) = work.operation.take() {
                    ensure!(work.inputs.len() == 2, "BGI graph pair input count");
                    let spec = IntegerRaster {
                        destination_format: work.format as u32,
                        source_format: work.format as u32,
                        raster: operation,
                    };
                    let effects =
                        Arc::from(serde_json::to_vec(&vec![ImageEffect::IntegerRaster {
                            spec,
                        }])?);
                    work.phase = Some(host.image_metadata(self.client)?.begin_process(
                        ImageInput::Raster {
                            destination: Box::new(image_input(&work.inputs[0])?),
                            source: Box::new(image_input(&work.inputs[1])?),
                        },
                        effects,
                    )?);
                    work.stage = 1;
                    continue;
                }
                match &work.inputs[0] {
                    Prepared::Ready(image) => image.clone(),
                    Prepared::Solid(size, color) => {
                        work.phase = Some(host.image_metadata(self.client)?.begin_process(
                            ImageInput::Solid {
                                size: *size,
                                color: *color,
                            },
                            Arc::from(b"[]".as_slice()),
                        )?);
                        work.stage = 1;
                        continue;
                    }
                    _ => unreachable!(),
                }
            };
            if work.stage <= 1 && !work.filters.is_empty() {
                let spec = IntegerRaster {
                    destination_format: 2,
                    source_format: 2,
                    raster: Raster::Display {
                        spec: DisplayRaster::ColorFilters {
                            filters: std::mem::take(&mut work.filters),
                        },
                    },
                };
                let effects = Arc::from(serde_json::to_vec(&vec![ImageEffect::IntegerRaster {
                    spec,
                }])?);
                work.phase = Some(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Raster {
                        destination: Box::new(ImageInput::Resource(current.clone())),
                        source: Box::new(ImageInput::Resource(current)),
                    },
                    effects,
                )?);
                work.stage = 2;
                continue;
            }
            if work.force_opaque && !current.opaque() {
                work.phase = Some(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Resource(current),
                    Arc::from(serde_json::to_vec(&vec![ImageEffect::OverrideAlpha {
                        alpha: 255,
                    }])?),
                )?);
                work.force_opaque = false;
                work.stage = 3;
                continue;
            }
            let uploaded = host.publish_image_resource(
                self.client,
                0xd0000000 ^ (object as u32),
                current.clone(),
            )?;
            if let Some(old) = self.graph.images.painted.insert(
                object,
                ImagePaint {
                    path: uploaded.path.relative().to_string(),
                    image: current,
                },
            ) {
                self.graph.images.retired.push(old.path);
            }
            self.release_graph_retired(host)?;
            self.graph.images.dirty.remove(&object);
            self.graph.mark_visual(object);
            return Ok(true);
        }
    }
    pub(crate) fn graph_alpha(&mut self, object: i32, value: i32) -> Result<()> {
        let objects = self.graph.descendants(object);
        let mut backdrops = Vec::new();
        for object in objects {
            let p = self.graph.objects.entry(object).or_default();
            if let Some(background) = p.background {
                let changed = p.alpha_parameter() != value;
                p.set_alpha_parameter(value);
                if changed && !matches!(background.class, BackClass::BackN | BackClass::BackS) {
                    backdrops.push(object);
                }
                continue;
            }
            let mode = p
                .named_properties
                .get("target-object-mode")
                .copied()
                .unwrap_or(0);
            if mode == 1 && p.background.is_none() {
                let selector = p
                    .named_properties
                    .get("mode1-selector")
                    .copied()
                    .unwrap_or(0);
                if matches!(selector, -1 | 0 | 2 | 3) {
                    p.set_alpha_parameter(value);
                }
                if matches!(selector, 1 | 2) {
                    p.named_properties.insert("mode1-amount".into(), value);
                    if let Some(plan) = self.graph.images.plans.get_mut(&object) {
                        if let Some(Raster::Display {
                            spec: DisplayRaster::Crossfade { amount },
                        }) = &mut plan.operation
                        {
                            *amount = value.clamp(0, 256);
                        }
                    }
                    self.graph.images.dirty(object);
                }
            } else {
                // Ordinary alpha is a retained Layer gate. In particular a
                // mode-3 mask depends on the fixed parameter, not base alpha.
                p.set_alpha_parameter(value);
            }
        }
        for object in backdrops {
            self.rebuild_backdrop_plan(object)?;
        }
        Ok(())
    }
}
