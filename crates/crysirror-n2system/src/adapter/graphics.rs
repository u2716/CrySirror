// SPDX-License-Identifier: MPL-2.0
//! Native coordinates and animation policy translated onto shared paths,
//! scalar clocks, retained geometry and filters. No pixels are drawn here.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_scene::{
    animation::{CubicBezierPath, ScalarTween},
    layer::{Easing, LayerFilter, SpriteTransform},
};
use std::sync::Arc;

pub(super) struct PlainEffect {
    pub passive: bool,
    pub pending: Option<ImageJob>,
    /// Per-frame colour transform installed by DrawEffect (432770 names
    /// 436430/436560/436690/4367C0). `None` is the Plain copy (436270).
    pub color: Option<ColorEffect>,
    pub process: Option<ImageJob>,
    pub rebuild_view: bool,
}
/// Native frame-callback transforms 443C10/4435C0/4439D0/4437C0. Each reads the
/// presented frame inside the entity rectangle and leaves destination alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColorEffect {
    NegaPosi,
    Monochrome,
    Sepia,
    KitanoBlue,
}
impl ColorEffect {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "NegaPosi" => Self::NegaPosi,
            "Monochrome" => Self::Monochrome,
            "Sepia" => Self::Sepia,
            "KitanoBlue" => Self::KitanoBlue,
            _ => return None,
        })
    }
    /// Native luma is three independently truncated terms,
    /// (77*c0>>8)+(151*c1>>8)+(28*c2>>8), then optional saturated channel offsets.
    fn effects(self) -> Arc<[u8]> {
        let luma = json!({"kind":"fixed_luma","spec":{
            "coefficients":[77,151,28],"groups":[0,1,2],
            "fractional_bits":8,"amount256":256}});
        let lift = |channel: u8, write_mask: u8, add: u32| {
            let table = (0..256u32).map(|v| (v + add).min(255)).collect::<Vec<_>>();
            json!({"kind":"channel_lookup","channel":channel,"write_mask":write_mask,"table":table})
        };
        let list = match self {
            Self::NegaPosi => vec![json!({"kind":"invert"})],
            Self::Monochrome => vec![luma],
            Self::Sepia => vec![luma, lift(0, 1, 80), lift(1, 2, 20)],
            Self::KitanoBlue => vec![luma, lift(1, 6, 30)],
        };
        Arc::from(serde_json::to_vec(&list).expect("static effect list"))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MaskTrack {
    pub progress: ScalarTween,
    pub asset: String,
    pub feather: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PathTrack {
    pub segments: Vec<CubicBezierPath>,
    pub progress: ScalarTween,
}
impl N2SystemVm {
    /// AlphaMask in the native entity tree supplies the nearest ancestor's
    /// screen-space red matte. Rendering parents are flattened, so propagate
    /// this relationship explicitly without making the mask a drawable image.
    pub(super) fn update_alpha_masks(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut changes = Vec::new();
        for entity in self.entities.nodes.values() {
            if !matches!(entity.kind, Kind::Visual) {
                continue;
            }
            let mut parent = entity.parent;
            let mut stencil = None;
            let mut depth = 0;
            while let Some(id) = parent {
                ensure!(depth < 128, "N2System mask ancestry depth");
                depth += 1;
                let ancestor = self
                    .entities
                    .nodes
                    .get(&id)
                    .context("N2System dangling mask parent")?;
                if let Some(asset) = self.alpha_masks.get(&id) {
                    let layer = self.layer(host, id)?;
                    stencil = Some(crysirror_scene::layer::LayerStencil {
                        asset: asset.clone(),
                        canvas: self.config.canvas,
                        nearest: self.stencils.contains_key(&id),
                        rgba: self.rgba_masks.contains(&id),
                        outside_zero: self.stencils.contains_key(&id),
                        origin: self.stencils.get(&id).copied().unwrap_or([
                            layer["rect"][0].as_f64().context("mask x")? as f32,
                            layer["rect"][1].as_f64().context("mask y")? as f32,
                        ]),
                    });
                    break;
                }
                parent = ancestor.parent;
            }
            let layer = self.layer(host, entity.id)?;
            let mut filter: LayerFilter = layer
                .get("filter")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()?
                .unwrap_or_default();
            if filter.stencil != stencil {
                filter.stencil = stencil;
                changes.push(json!({"op":"set_filter","id":entity.id,"filter":filter}));
            }
        }
        self.submit(host, changes, vec![])
    }
    pub(super) fn sample_mask_transitions(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut changes = Vec::new();
        for (&id, track) in &self.mask_transitions {
            let layer = self.layer(host, id)?;
            let mut filter: LayerFilter = layer
                .get("filter")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()?
                .unwrap_or_default();
            let progress = track.progress.sample(host.now_ms()).clamp(0., 1.);
            if progress >= 1. {
                filter.mask = None;
            } else {
                filter.mask = Some(crysirror_scene::layer::LayerMask {
                    asset: track.asset.clone(),
                    gray_alpha: false,
                    channel: crysirror_scene::layer::MaskChannel::Red,
                    offset: 0.,
                    scale: 1.,
                    preserve_source_alpha: true,
                    source_space: false,
                    coverage: Some(crysirror_scene::layer::MaskCoverage {
                        progress,
                        feather: track.feather,
                        canvas: Some(self.config.canvas),
                    }),
                });
            }
            changes.push(json!({"op":"set_filter","id":id,"filter":filter}));
        }
        self.submit(host, changes, vec![])?;
        self.mask_transitions
            .retain(|_, t| !t.progress.complete(host.now_ms()));
        Ok(())
    }
    pub(super) fn publish_capture(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        if self.shades.contains_key(&id) {
            return self.refresh_shade_capture(host, id, image);
        }
        let uploaded = host.publish_image_resource(self.client, id, image)?;
        let path = uploaded.path.relative().to_owned();
        let paint = json!({"kind":"image","asset":path});
        if let Kind::Image { paint: stored, .. } =
            &mut self.entities.nodes.get_mut(&id).unwrap().kind
        {
            *stored = paint;
        } else {
            self.submit(
                host,
                vec![json!({"op":"set_paint","id":id,"paint":paint})],
                vec![],
            )?;
        }
        if let Some(previous) = self.captured_images.insert(id, path) {
            host.release_image_resource(self.client, &previous)?;
        }
        Ok(())
    }
    pub(super) fn capture_visual(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let effect = call.function == Builtin::CreateEffect;
        if effect && args.string(6)?.eq_ignore_ascii_case("Lens") {
            return self.native_create_lens(host, current, state, args);
        }
        if effect {
            let name = args.string(6)?;
            if let Some(kind) = particles::Kind::from_name(&name) {
                return self.native_create_particles(host, current, state, args, kind);
            }
        }
        if effect {
            ensure!(
                args.string(6)?.eq_ignore_ascii_case("Plain"),
                "N2System unknown capture effect"
            );
        }
        if state.capture.is_none() {
            state.capture = Some(host.begin_frame_capture(self.client)?);
        }
        let image = match state.capture.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(e) => bail!("N2System screen capture: {e}"),
            ImageProcessState::Ready(image) => image,
        };
        state.capture = None;
        let size = if effect {
            let w = args.integer(4)?;
            let h = args.integer(5)?;
            ensure!(
                w > 0 && h > 0 && w <= 16384 && h <= 16384,
                "N2System capture extent"
            );
            [w as u32, h as u32]
        } else {
            self.config.canvas
        };
        // The frame is rendered by the shared presenter. A UV view selects
        // Plain's source rectangle without an adapter-owned pixel operation.
        let id = if call.function == Builtin::LoadImage {
            self.create(
                host,
                current,
                &args.string(0)?,
                Kind::Image {
                    paint: json!({"kind":"group"}),
                    size,
                },
                size.map(|n| n as f32),
            )?
        } else {
            self.visual(host, current, args, json!({"kind":"group"}), size, None)?
        };
        if effect {
            let mut layer = self.layer(host, id)?;
            let x = layer["rect"][0].as_f64().context("capture x")? as f32;
            let y = layer["rect"][1].as_f64().context("capture y")? as f32;
            let [w, h] = self.config.canvas.map(|n| n as f32);
            ensure!(
                x >= 0. && y >= 0. && x + size[0] as f32 <= w && y + size[1] as f32 <= h,
                "N2System Plain source rectangle outside frame"
            );
            layer["uv"] = json!([
                x / w,
                y / h,
                (x + size[0] as f32) / w,
                (y + size[1] as f32) / h
            ]);
            self.submit(host, vec![json!({"op":"put","layer":layer})], vec![])?;
        }
        self.publish_capture(host, id, image)?;
        self.bitmap_formats.insert(
            id,
            bitmap_read::BitmapFormat {
                origin: None,
                alpha: Some(false),
            },
        );
        if effect {
            self.effects.insert(
                id,
                PlainEffect {
                    passive: false,
                    pending: None,
                    color: None,
                    process: None,
                    rebuild_view: false,
                },
            );
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn install_plain_effect(&mut self, host: &mut dyn HostServices, id: u32, color: Option<ColorEffect>) -> Result<()> {
        self.remove_ripple(host, id, true)?;
        self.lenses.remove(&id);
        self.particles.remove(&id);
        self.effects.insert(id, PlainEffect { passive: false, pending: None, color, process: None, rebuild_view: true });
        Ok(())
    }
    fn publish_plain_frame(&mut self, host: &mut dyn HostServices, id: u32, image: Arc<dyn ImageResource>) -> Result<()> {
        self.publish_capture(host, id, image)?;
        if self.effects.get(&id).is_some_and(|e| e.rebuild_view) {
            let mut layer = self.layer(host, id)?;
            let [w, h] = self.config.canvas.map(|v| v as f64);
            let x = layer["rect"][0].as_f64().context("Plain x")?;
            let y = layer["rect"][1].as_f64().context("Plain y")?;
            let size = self.entities.nodes[&id].size;
            ensure!(x >= 0. && y >= 0. && x + f64::from(size[0]) <= w && y + f64::from(size[1]) <= h, "Plain rebuild source extent");
            layer["uv"] = json!([x / w, y / h, (x + f64::from(size[0])) / w, (y + f64::from(size[1])) / h]);
            self.submit(host, vec![json!({"op":"put","layer":layer})], vec![])?;
            self.effects.get_mut(&id).unwrap().rebuild_view = false;
        }
        Ok(())
    }
    pub(super) fn update_effects(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids = self.effects.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let effect = self.effects.get_mut(&id).unwrap();
            if effect.passive {
                continue;
            }
            if let Some(job) = &mut effect.process {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(e) => bail!("N2System colour effect: {e}"),
                    ImageProcessState::Ready(image) => {
                        effect.process = None;
                        self.publish_plain_frame(host, id, image)?;
                    }
                }
            }
            let effect = self.effects.get_mut(&id).unwrap();
            if let Some(job) = &mut effect.pending {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(e) => bail!("N2System Plain capture: {e}"),
                    ImageProcessState::Ready(image) => {
                        effect.pending = None;
                        if let Some(color) = effect.color {
                            match self.images.as_ref().unwrap().begin_process(
                                ImageInput::Resource(image),
                                color.effects(),
                            ) {
                                Ok(job) => {
                                    self.effects.get_mut(&id).unwrap().process = Some(job);
                                }
                                Err(Error::Limit("image preparation task capacity")) => {
                                    // Retry with a fresh capture next frame.
                                }
                                Err(e) => return Err(e.into()),
                            }
                            continue;
                        }
                        self.publish_plain_frame(host, id, image)?;
                    }
                }
            }
            // Read only the lower painter prefix, so a moving Plain never
            // feeds its own previous output (or later overlays) back into itself.
            let layer = self.layer(host, id)?;
            let order = [layer["order"][0].as_i64().context("Plain order")? as i32,
                layer["order"][1].as_i64().context("Plain order")? as i32];
            self.effects.get_mut(&id).unwrap().pending = Some(host.begin_layer_prefix_image(
                self.client, self.config.canvas, order)?);
        }
        Ok(())
    }
    pub(super) fn sample_paths(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut changes = Vec::new();
        let mut finished = Vec::new();
        for (&id, track) in &self.paths {
            if !self.entities.nodes.contains_key(&id) {
                finished.push(id);
                continue;
            }
            let p = track.progress.sample(now).clamp(0., 1.) * track.segments.len() as f32;
            let segment = (p as usize).min(track.segments.len() - 1);
            let point = track.segments[segment].sample(p - segment as f32)?;
            let mut layer = self.layer(host, id)?;
            layer["rect"][0] = json!(point[0]);
            layer["rect"][1] = json!(point[1]);
            changes.push(json!({"op":"set_rect","id":id,"rect":layer["rect"]}));
            if track.progress.complete(now) {
                finished.push(id);
            }
        }
        self.submit(host, changes, vec![])?;
        for id in finished {
            self.paths.remove(&id);
        }
        Ok(())
    }
    pub(super) fn native_graphics(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        if call.function == Builtin::DrawTransition {
            let mut ids = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                ensure!(
                    !self.cameras.contains_key(&root),
                    "N2System camera rotation/filter channels require their native projection profile"
                );
                for id in self.entities.subtree(root) {
                    if matches!(self.entities.nodes[&id].kind, Kind::Visual) {
                        ids.insert(id);
                    }
                }
            }
            if !ids.is_empty() {
                let asset = resource::resolve(self.catalog.as_ref().unwrap(), &args.string(6)?)?;
                let from = args.value(2).number()? / 1000.;
                let to = args.value(3).number()? / 1000.;
                let feather = args.value(4).number()? / 1000.;
                ensure!(
                    [from, to, feather]
                        .iter()
                        .all(|v| v.is_finite() && (0. ..=1.).contains(v)),
                    "N2System transition amounts"
                );
                let progress = ScalarTween::new(
                    from,
                    to,
                    host.now_ms(),
                    args.time(1)?,
                    serde_json::from_value(easing(args, 5)?)?,
                )?;
                for &id in &ids {
                    ensure!(
                        !self.pages.contains_key(&id)
                            && self.entities.nodes[&id].container.is_none(),
                        "N2System mask transition requires a sprite or color plane"
                    );
                    let layer = self.layer(host, id)?;
                    ensure!(
                        layer["filter"]["mask"].is_null()
                            || !layer["filter"]["mask"]["coverage"].is_null(),
                        "N2System mask transition over an existing independent mask"
                    );
                    self.mask_transitions.insert(
                        id,
                        MaskTrack {
                            progress: progress.clone(),
                            asset: asset.clone(),
                            feather,
                        },
                    );
                }
                self.sample_mask_transitions(host)?;
                let timeout = args.anim_wait(7, args.time(1)?)?;
                if timeout > 0 {
                    state.wait = Wait::Motion {
                        ids: ids.into_iter().collect(),
                        properties: vec![],
                        until: Some(host.now_ms().saturating_add(timeout)),
                    };
                }
            }
        } else if call.function == Builtin::CreateEffect {
            return self.capture_visual(host, current, state, args, call);
        } else if call.function == Builtin::Draw {
            state.wait = Wait::Frame(self.frame);
        } else if call.function == Builtin::WaitFade {
            let timeout = args.time(1)?;
            state.wait = Wait::Motion {
                ids: self.query(current, &args.string(0)?)?,
                properties: vec!["opacity"],
                until: if timeout > 0 {
                    Some(host.now_ms().saturating_add(timeout))
                } else {
                    None
                },
            };
        } else {
            let mut changes = Vec::new();
            let mut ids = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                if call.function == Builtin::Rotate {
                    if self.cameras.contains_key(&root) { ids.insert(root); }
                    ids.extend(self.entities.animation_targets(root));
                } else if matches!(self.entities.nodes[&root].kind, Kind::Visual) {
                    ids.insert(root);
                }
            }
            for &id in &ids {
                if call.function == Builtin::Rotate
                    && self.animate_cube(host, id, args, call.function)?
                {
                    continue;
                }
                if call.function == Builtin::Rotate && self.cameras.contains_key(&id) {
                    self.rotate_camera(id, host.now_ms(), args)?;
                    continue;
                }
                let layer = self.layer(host, id)?;
                match call.function {
                    Builtin::Rotate => {
                        let duration = args.time(1)?;
                        let easing = easing(args, 5)?;
                        for (axis, (property, field)) in [
                            ("rotation_x", "rotate_x"),
                            ("rotation_y", "rotate_y"),
                            ("rotation_z", "rotate"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let base = layer["transform"][field]
                                .as_f64()
                                .context("N2System rotation component")?
                                as f32;
                            let target = match args.value(axis + 2) {
                                Value::Number(n) => n.to_radians(),
                                Value::Delta(n) => base + n.to_radians(),
                                Value::Null => base,
                                _ => bail!("N2System rotation argument"),
                            };
                            changes.push(json!({"op":"begin_motion","id":id,"track":{"property":property,"to":target,"duration":duration,"easing":easing}}));
                            self.record_blur_motion(
                                host,
                                id,
                                property,
                                target,
                                duration,
                                easing.clone(),
                            )?;
                        }
                    }
                    Builtin::BezierMove => {
                        let Value::Curve(curve) = args.value(2) else {
                            bail!("N2System Bezier control points")
                        };
                        ensure!(
                            !curve.is_empty() && curve.len() <= 4096,
                            "N2System Bezier segment capacity"
                        );
                        let size = self.entities.nodes[&id].size;
                        let current = [
                            layer["rect"][0].as_f64().unwrap() as f32,
                            layer["rect"][1].as_f64().unwrap() as f32,
                        ];
                        let mut segments = Vec::with_capacity(curve.len());
                        for points in curve {
                            let mut control = [[0.; 3]; 4];
                            for (at, (x, y)) in points.iter().enumerate() {
                                control[at][0] =
                                    self.native_position(host, id, x, current[0], size[0], 0)?;
                                control[at][1] =
                                    self.native_position(host, id, y, current[1], size[1], 1)?;
                            }
                            let segment = CubicBezierPath { control };
                            segment.validate()?;
                            segments.push(segment);
                        }
                        let easing: Easing = serde_json::from_value(easing(args, 3)?)?;
                        let track = PathTrack {
                            segments,
                            progress: ScalarTween::new(
                                0.,
                                1.,
                                host.now_ms(),
                                args.time(1)?,
                                easing,
                            )?,
                        };
                        for property in ["x", "y"] {
                            changes.push(json!({"op":"end_motion","id":id,"property":property}));
                        }
                        self.record_blur_path(id, &track)?;
                        self.paths.insert(id, track);
                    }
                    Builtin::SetVertex => {
                        self.entities.nodes.get_mut(&id).unwrap().pivot_auto = false;
                        let mut transform: SpriteTransform =
                            serde_json::from_value(layer["transform"].clone())?;
                        let size = self.entities.nodes[&id].size;
                        for (axis, old) in [transform.pivot_x, transform.pivot_y]
                            .into_iter()
                            .enumerate()
                        {
                            let value = match args.value(axis + 1) {
                                Value::Number(n) => *n,
                                Value::Delta(n) => old + n,
                                Value::Null => old,
                                v if v.is_constant("Inherit") => old,
                                v if v.is_constant("Center") || v.is_constant("Middle") => {
                                    size[axis] * 0.5
                                }
                                v if v.is_constant("Right") || v.is_constant("Bottom") => {
                                    size[axis]
                                }
                                Value::Constant(_) => 0.,
                                _ => bail!("N2System pivot coordinate"),
                            };
                            if axis == 0 {
                                transform.pivot_x = value;
                            } else {
                                transform.pivot_y = value;
                            }
                        }
                        changes.push(json!({"op":"set_transform","id":id,"transform":transform}));
                    }
                    Builtin::SetColor => {
                        let mut filter: LayerFilter = if layer["filter"].is_null() {
                            LayerFilter::default()
                        } else {
                            serde_json::from_value(layer["filter"].clone())?
                        };
                        let color = args.color(1)?;
                        filter.multiply = [color[0] as f32 / 255., color[1] as f32 / 255., color[2] as f32 / 255.];
                        changes.push(json!({"op":"set_filter","id":id,"filter":filter}));
                    }
                    _ => bail!("N2System non-graphics call"),
                }
            }
            self.submit(host, changes, vec![])?;
            if call.function == Builtin::Rotate { self.sample_cameras(host)?; }
            if call.function == Builtin::BezierMove {
                self.sample_paths(host)?;
                if args.truth(4)? {
                    state.wait = Wait::Motion {
                        ids: ids.into_iter().collect(),
                        properties: vec!["x", "y"],
                        until: None,
                    };
                }
            } else if call.function == Builtin::Rotate {
                let timeout = args.anim_wait(6, args.time(1)?)?;
                if timeout > 0 {
                    state.wait = Wait::Motion {
                        ids: ids.into_iter().collect(),
                        properties: vec!["rotation_x", "rotation_y", "rotation_z"],
                        until: Some(host.now_ms().saturating_add(timeout)),
                    };
                }
            }
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}

#[cfg(test)]
mod color_effect_tests {
    use super::ColorEffect;
    use crysirror_image::{effects::ImageEffect, ImageSurface};
    fn run(effect: ColorEffect, rgba: [u8; 4]) -> [u8; 4] {
        let mut image = ImageSurface { width: 1, height: 1, rgba: rgba.to_vec() };
        let list: Vec<ImageEffect> = serde_json::from_slice(&effect.effects()).unwrap();
        for step in &list {
            image = image.apply_effect(step).unwrap();
        }
        image.rgba.try_into().unwrap()
    }
    #[test]
    fn native_frame_callbacks() {
        // 443C10: 255-c on RGB, alpha untouched.
        assert_eq!(run(ColorEffect::NegaPosi, [10, 200, 255, 255]), [245, 55, 0, 255]);
        // 4435C0: 77*200>>8 + 151*100>>8 + 28*50>>8 = 60+58+5.
        assert_eq!(run(ColorEffect::Monochrome, [200, 100, 50, 255]), [123, 123, 123, 255]);
        // 4439D0: gray 123 -> [203,143,123]; saturation at 255.
        assert_eq!(run(ColorEffect::Sepia, [200, 100, 50, 255]), [203, 143, 123, 255]);
        assert_eq!(run(ColorEffect::Sepia, [255, 255, 255, 255]), [255, 255, 253, 255]);
        // 4437C0: gray, then G/B +30.
        assert_eq!(run(ColorEffect::KitanoBlue, [200, 100, 50, 255]), [123, 153, 153, 255]);
    }
}
