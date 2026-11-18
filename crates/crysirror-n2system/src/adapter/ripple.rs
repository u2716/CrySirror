// SPDX-License-Identifier: MPL-2.0
//! DrawEffect412340 ->44A120/4571B0/432770; Ripple4358F0/435E00/436020.
//! Native per-pixel geometry and sampling live in shared radial_phase workers.
//! This module owns only native bitmap, parameter, clock and replacement policy.
use super::native_bitmap::NativeBitmap;
use super::*;
use crysirror_core::image_processing::{
    FieldWarpSpec, ImageInput, ImageJob, ImageProcessState, ImageResource, RadialPhaseField,
};
use crysirror_scene::{animation::ScalarTween, layer::LayerSpec};

struct Input {
    id: u32,
    layer: LayerSpec,
    read: Option<NativeBitmap>,
    image: Option<Arc<dyn ImageResource>>,
    task: Option<Task<Arc<RadialPhaseField>>>,
    field: Option<Arc<RadialPhaseField>>,
    clear: Option<[u8; 4]>,
    encoding_path: Option<String>,
    encoding: Option<Box<dyn crysirror_core::image_encoding::ImageEncodingInfoQuery>>,
}
pub(super) struct Preparing {
    inputs: Vec<Input>,
    pub(super) bytes: usize,
}
/// Native bitmap-effect family sharing the source/progress/restore machinery
/// (432770 names; setup 4358F0/433330, per-frame callbacks in 5176).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Proc {
    Ripple,
    /// Mosaic 433330/433630: block size in pixels at the start and end.
    Mosaic,
    /// LowWave/MiddleWave/HighWave/SuperWave 433B10/433E80: per-row sine
    /// shifts; the period constant is 64, 512, 1024 or 4096 phase steps.
    Wave(u32),
    /// SoftSplit/Split/HardSplit 4347B0/434E80: an N x N tile grid (4, 8, 12)
    /// whose tiles drift with random vertical speeds while the group slides.
    Split(u32),
    /// TurnOver 435260/435690: 32 x 32 tiles flipped column by column.
    TurnOver,
    /// Compatibility extension: safe concentric polar twist in place of the
    /// invalid4344C0 circle-state/433E80 wave-callback pairing. Amount0..1000
    /// is zero to one centre turn, tapering to zero at half the shorter edge.
    Spiral,
}
impl Proc {
    fn tiled(self) -> bool {
        matches!(self, Self::Split(_) | Self::TurnOver)
    }
    fn from_name(name: &str) -> Result<Option<Self>> {
        Ok(Some(match name {
            "Ripple" => Self::Ripple,
            "Mosaic" => Self::Mosaic,
            "LowWave" => Self::Wave(64),
            "MiddleWave" => Self::Wave(512),
            "HighWave" => Self::Wave(1024),
            "SuperWave" => Self::Wave(4096),
            "SoftSplit" => Self::Split(4),
            "Split" => Self::Split(8),
            "HardSplit" => Self::Split(12),
            "TurnOver" => Self::TurnOver,
            "Spiral" => Self::Spiral,
            _ => return Ok(None),
        }))
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct TileState {
    offset: [i32; 2],
    speed: i32,
    sign: i32,
    angle: i32,
}
pub(super) struct Ripple {
    tiles: Vec<TileState>,
    grid: [u32; 2],
    rect: Json,
    steps_at: u64,
    final_built: bool,
    proc: Proc,
    blocks: [i32; 2],
    last_block: i32,
    source: Arc<dyn ImageResource>,
    field: Option<Arc<RadialPhaseField>>,
    original: Json,
    original_alias: String,
    output: Option<String>,
    job: Option<ImageJob>,
    progress: ScalarTween,
    amounts: [u16; 2],
    phase: u32,
    last: u64,
    dirty: bool,
    clear: [u8; 4],
    reset_view: bool,
    reset_rect: Option<Json>,
}
impl Ripple {
    pub(super) fn active(&self, now: u64) -> bool {
        !self.progress.complete(now)
    }
    fn amplitude(&self, now: u64) -> u16 {
        let delta = i32::from(self.amounts[1]) - i32::from(self.amounts[0]);
        (i32::from(self.amounts[0])
            + (f64::from(delta) * f64::from(self.progress.sample(now))).trunc() as i32)
            as u16
    }
    fn bytes(&self) -> usize {
        self.field.as_ref().map_or(0, |f| f.distances.len() * 5)
    }
    /// Mosaic block edge at `now`: 433630 truncates the eased table value and
    /// moves away from the start block size toward the end block size.
    fn block(&self, now: u64) -> i32 {
        let [from, to] = self.blocks;
        let delta = f64::from((to - from).abs());
        let step = (delta * f64::from(self.progress.sample(now).clamp(0., 1.))).trunc() as i32;
        if from >= to { from - step } else { from + step }
    }
}
fn budget() -> usize {
    if cfg!(target_os = "vita") {
        24 * 1024 * 1024
    } else {
        96 * 1024 * 1024
    }
}
impl N2SystemVm {
    pub(super) fn native_draw_effect(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        ensure!(args.len() == 6, "N2System DrawEffect signature");
        let effect = args.string(2)?;
        let color = graphics::ColorEffect::from_name(&effect);
        if effect == "Plain" || color.is_some() {
            let mut targets = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? { targets.extend(self.entities.bitmap_targets(root)); }
            for id in targets {
                if matches!(self.entities.nodes[&id].kind, Kind::Visual) {
                    self.install_plain_effect(host, id, color)?;
                }
            }
            state.ripple_preparation = None;
            self.machine.complete(&mut state.thread, Value::Null)?;
            return Ok(true);
        }
        let proc = Proc::from_name(&effect)?.with_context(|| {
            format!("N2System DrawEffect {effect} has no audited effect binding")
        })?;
        let from = args.integer(3)?;
        let to = args.integer(4)?;
        ensure!(
            (0..=1000).contains(&from) && (0..=1000).contains(&to),
            "N2System {effect} amount range"
        );
        if state.ripple_preparation.is_none() {
            let mut targets = BTreeSet::new();
            for root in self.query(current, &args.string(0)?)? {
                targets.extend(self.entities.bitmap_targets(root));
            }
            let mut inputs = Vec::new();
            for id in targets {
                // Replacing the native callback freezes its last published
                // bitmap. Drop obsolete jobs before preparing the new effect.
                self.lenses.remove(&id);
                self.particles.remove(&id);
                if let Some(plain) = self.effects.get_mut(&id) {
                    plain.passive = true;
                    plain.pending = None;
                    plain.process = None;
                }
                if !matches!(self.entities.nodes[&id].kind, Kind::Visual) {
                    continue;
                }
                let mut layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
                if let Some(previous) = self.ripples.get(&id).filter(|r| r.proc.tiled()) {
                    layer.rect = serde_json::from_value(previous.rect.clone())?;
                }
                if !matches!(
                    layer.paint,
                    crysirror_scene::layer::Paint::Image { .. }
                        | crysirror_scene::layer::Paint::Solid { .. }
                ) {
                    continue;
                }
                ensure!(
                    !self.surfaces.contains_key(&id)
                        && !self.movies.contains_key(&id)
                        && !self.frame_animations.contains_key(&id)
                        && self.effects.get(&id).is_none_or(|e| e.passive),
                    "N2System Ripple requires a stable bitmap source"
                );
                if self.shades.get(&id).is_some_and(shade::Shade::pending) {
                    return Ok(false);
                }
                let existing = self.ripples.get(&id);
                let image = existing.map(|r| r.source.clone());
                let field = existing
                    .filter(|r| r.proc == proc)
                    .and_then(|r| r.field.clone());
                let read = if image.is_none() {
                    let Some(read) = self.read_native_bitmap_layer(host, id, &layer)? else {
                        return Ok(false);
                    };
                    Some(read)
                } else {
                    None
                };
                let (clear, encoding_path) = if let Some(existing) = existing {
                    (Some(existing.clear), None)
                } else if self.effects.contains_key(&id) {
                    (Some([0, 0, 0, 255]), None)
                } else {
                    let format = self.bitmap_formats.get(&id).cloned().unwrap_or_else(|| {
                        self.bitmap_format(&serde_json::to_value(&layer.paint).unwrap())
                    });
                    (
                        format
                            .alpha
                            .map(|alpha| [0, 0, 0, if alpha { 0 } else { 255 }]),
                        format.origin,
                    )
                };
                inputs.push(Input {
                    id,
                    layer,
                    read,
                    image,
                    field,
                    task: None,
                    clear,
                    encoding_path,
                    encoding: None,
                });
            }
            state.ripple_preparation = Some(Preparing { inputs, bytes: 0 });
        }
        let others = self
            .threads
            .values()
            .filter_map(|t| t.ripple_preparation.as_ref())
            .map(|p| p.bytes)
            .sum::<usize>();
        let retained = self.ripples.values().map(Ripple::bytes).sum::<usize>();
        let preparation = state.ripple_preparation.as_mut().unwrap();
        let mut ready = true;
        for input in &mut preparation.inputs {
            if !self.entities.nodes.contains_key(&input.id) {
                continue;
            }
            if input.clear.is_none() {
                if input.encoding.is_none() {
                    match self.images.as_ref().unwrap().begin_encoding_info(
                        input
                            .encoding_path
                            .as_ref()
                            .context("N2System Ripple source has no original bitmap format")?,
                    ) {
                        Ok(query) => input.encoding = Some(query),
                        Err(Error::Limit("encoded image metadata task capacity")) => {
                            ready = false;
                            continue;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                use crysirror_core::image_encoding::ImageEncodingInfoState as S;
                match input.encoding.as_mut().unwrap().poll()? {
                    S::Pending => {
                        ready = false;
                        continue;
                    }
                    S::Failed(error) => return Err(error.into()),
                    S::Ready(receipt) => {
                        ensure!(
                            receipt.client == self.client,
                            "N2System Ripple encoding owner"
                        );
                        let alpha = receipt.encoding.has_alpha_plane.context(
                            "N2System Ripple needs original bitmap alpha-plane metadata",
                        )?;
                        input.clear = Some([0, 0, 0, if alpha { 0 } else { 255 }]);
                        self.bitmap_formats.insert(
                            input.id,
                            bitmap_read::BitmapFormat {
                                origin: input.encoding_path.clone(),
                                alpha: Some(alpha),
                            },
                        );
                        if let Some(read) = &mut input.read {
                            read.set_encoding(receipt.fingerprint, alpha);
                        }
                        input.encoding = None;
                    }
                }
            }
            if input.image.is_none() {
                input.image = input.read.as_mut().unwrap().poll(
                    self.client,
                    self.pool.as_ref().unwrap(),
                    self.images.as_ref().unwrap(),
                )?;
                if input.image.is_none() {
                    ready = false;
                    continue;
                }
                input.read = None;
            }
            if matches!(proc, Proc::Ripple | Proc::Spiral) && input.field.is_none() {
                if let Some(task) = &mut input.task {
                    if let Some(field) = task.try_take()? {
                        input.field = Some(field);
                        input.task = None;
                    }
                }
                if input.field.is_none() && input.task.is_none() {
                    let size = input.image.as_ref().unwrap().size();
                    // Vec-to-Arc publication temporarily retains both arrays.
                    let bytes = size[0] as usize * size[1] as usize * 10;
                    ensure!(
                        bytes <= budget()
                            && retained + others + preparation.bytes + bytes <= budget(),
                        "N2System Ripple geometry budget"
                    );
                    let client = self.client;
                    let center = if proc == Proc::Spiral {
                        size.map(|v| (v / 2) as i32)
                    } else {
                        [
                            input.layer.transform.pivot_x.trunc() as i32,
                            input.layer.transform.pivot_y.trunc() as i32,
                        ]
                    };
                    match self.pool.as_ref().unwrap().spawn(move |cancel| {
                        crysirror_image::radial_phase::prepare(client, size, center, &|| {
                            cancel.check()
                        })
                    }) {
                        Ok(task) => {
                            input.task = Some(task);
                            preparation.bytes += bytes;
                        }
                        Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                        Err(e) => return Err(e),
                    }
                }
                ready &= input.field.is_some();
            }
        }
        if !ready {
            return Ok(false);
        }
        let preparation = state.ripple_preparation.take().unwrap();
        let now = host.now_ms();
        let progress = ScalarTween::new(
            0.,
            1.,
            now,
            args.time(1)?.max(16),
            serde_json::from_value(easing(args, 5)?)?,
        )?;
        for input in preparation.inputs {
            if !self.entities.nodes.contains_key(&input.id) {
                continue;
            }
            let image = input.image.unwrap();
            let uploaded = host.publish_image_resource(self.client, input.id, image.clone())?;
            let alias = uploaded.path.relative().to_owned();
            let mut original = serde_json::to_value(&input.layer.paint)?;
            original["kind"] = json!("image");
            original["asset"] = json!(alias);
            // Keep the existing bitmap's tint, not a malformed Solid record.
            if original.get("color").is_some() {
                original = json!({"kind":"image","asset":alias});
            }
            let old = self.ripples.remove(&input.id);
            let reset_rect = old.as_ref().is_some_and(|old| old.proc.tiled() && !proc.tiled());
            let output = old.as_ref().and_then(|old| old.output.clone());
            if let Some(old) = old {
                host.release_image_resource(self.client, &old.original_alias)?;
            }
            let [w, h] = image.size();
            // 433330: block edges are a tenth of the shorter side scaled by
            // the 0..1000 operands, truncated.
            let layer_json = serde_json::to_value(&input.layer)?;
            let rect = layer_json["rect"].clone();
            let origin = [
                rect[0].as_f64().unwrap_or(0.).trunc() as i32,
                rect[1].as_f64().unwrap_or(0.).trunc() as i32,
            ];
            let canvas = self.config.canvas;
            let edge = |amount: i64| match proc {
                Proc::Spiral => amount as i32,
                Proc::Split(_) => ((f64::from(canvas[1]) - f64::from(origin[1])) * amount as f64
                    / 1000.)
                    .trunc() as i32,
                Proc::TurnOver => (amount as f64 * 64. / 1000.).trunc() as i32,
                // 433B10: amplitude in pixels is (width>>1) scaled by 0..1000.
                Proc::Wave(_) => (f64::from(w >> 1) * amount as f64 / 1000.).trunc() as i32,
                _ => (f64::from(w.min(h)) * 0.1 * amount as f64 / 1000.).trunc() as i32,
            };
            let grid = match proc {
                Proc::Split(n) => [n, n],
                Proc::TurnOver => [32, 32],
                _ => [0, 0],
            };
            let mut seed = (u64::from(input.id) << 32) ^ now ^ 0x9e37_79b9_7f4a_7c15;
            let mut next = move || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed >> 33) as i32
            };
            let tiles = (0..grid[0] * grid[1])
                .map(|t| {
                    // 4347B0: neighbours alternate between a downward speed
                    // 2..=6 and an upward speed -4..=-12; the horizontal sign
                    // alternates with them.
                    let even = t % 2 == 0;
                    TileState {
                        offset: [0, 0],
                        speed: if even {
                            next() % 5 + 2
                        } else {
                            -4 - next() % 5
                        },
                        sign: if even { 1 } else { -1 },
                        angle: 0,
                    }
                })
                .collect::<Vec<_>>();
            self.ripples.insert(
                input.id,
                Ripple {
                    tiles,
                    grid,
                    rect,
                    steps_at: now,
                    final_built: false,
                    proc,
                    blocks: [edge(i64::from(from)), edge(i64::from(to))],
                    last_block: -1,
                    source: image,
                    field: if matches!(proc, Proc::Ripple | Proc::Spiral) {
                        Some(input.field.context("N2System Ripple geometry missing")?)
                    } else {
                        None
                    },
                    original,
                    original_alias: alias,
                    output,
                    job: None,
                    progress,
                    amounts: [
                        (f64::from(from) * 0.02).trunc() as u16,
                        (f64::from(to) * 0.02).trunc() as u16,
                    ],
                    phase: 0,
                    last: if matches!(proc, Proc::Wave(_) | Proc::Spiral) {
                        now.saturating_sub(33)
                    } else {
                        now
                    },
                    dirty: true,
                    clear: input.clear.unwrap(),
                    reset_view: input.layer.uv.is_some() && self.effects.contains_key(&input.id),
                    reset_rect: if reset_rect { Some(serde_json::to_value(input.layer.rect)?) } else { None },
                },
            );
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn remove_ripple(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        restore: bool,
    ) -> Result<()> {
        let Some(mut ripple) = self.ripples.remove(&id) else {
            return Ok(());
        };
        ripple.job = None;
        if restore {
            // Switching paint class clears the captured Plain source view;
            // the local bitmap already contains that crop and its guard pixels.
            if ripple.reset_view {
                self.submit(
                    host,
                    vec![json!({"op":"set_paint","id":id,"paint":{"kind":"group"}})],
                    vec![],
                )?;
            }
            self.submit(
                host,
                vec![json!({"op":"set_paint","id":id,"paint":ripple.original})],
                vec![],
            )?;
            if ripple.proc.tiled() || ripple.reset_rect.is_some() {
                self.submit(
                    host,
                    vec![json!({"op":"set_rect","id":id,"rect":ripple.rect})],
                    vec![],
                )?;
            }
            if let Some(old) = self
                .captured_images
                .insert(id, ripple.original_alias.clone())
            {
                host.release_image_resource(self.client, &old)?;
            }
        } else {
            host.release_image_resource(self.client, &ripple.original_alias)?;
        }
        if let Some(path) = ripple.output {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
    pub(super) fn update_ripples(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut commands = Vec::new();
        let mut release = Vec::new();
        let mut restore = Vec::new();
        let canvas = self.config.canvas;
        for (&id, ripple) in &mut self.ripples {
            let ends_at_zero = match ripple.proc {
                Proc::Ripple => ripple.amounts[1] == 0,
                // 433970: a non-zero end block keeps the final mosaic in place.
                Proc::Mosaic | Proc::Wave(_) | Proc::Split(_) | Proc::TurnOver | Proc::Spiral => {
                    ripple.blocks[1] == 0
                }
            };
            if ripple.progress.complete(now) && ends_at_zero {
                restore.push(id);
                continue;
            }
            match ripple.proc {
                Proc::Ripple => {
                    if now.saturating_sub(ripple.last) >= 32 {
                        ripple.phase = ripple.phase.wrapping_add(1);
                        ripple.last = now;
                        ripple.dirty = true;
                    }
                }
                Proc::Wave(_) => {
                    // 433E80 advances only when more than 32 ms have passed.
                    if now.saturating_sub(ripple.last) > 32 {
                        ripple.last = now;
                        ripple.dirty = true;
                    }
                }
                Proc::Spiral => {
                    // Coalesce eased integer changes, then retain the exact
                    // nonzero endpoint without continually rebuilding it.
                    if now.saturating_sub(ripple.last) > 32
                        || (ripple.progress.complete(now) && !ripple.final_built)
                    {
                        ripple.last = now;
                        let amount = ripple.block(now);
                        if amount != ripple.last_block {
                            ripple.last_block = amount;
                            ripple.dirty = true;
                        }
                        ripple.final_built = ripple.progress.complete(now);
                    }
                }
                Proc::Split(_) | Proc::TurnOver => {
                    if ripple.progress.complete(now) {
                        // 435090/4342E0 with a zero argument stop the updater and
                        // leave the last tile state displayed.
                        if !ripple.final_built {
                            ripple.final_built = true;
                            ripple.dirty = true;
                        }
                    } else {
                        let steps = (now.saturating_sub(ripple.steps_at) / 16).min(8);
                        if steps > 0 {
                            ripple.steps_at += steps * 16;
                            for _ in 0..steps {
                                if ripple.proc == Proc::TurnOver {
                                    let count = ripple.block(now).clamp(0, 64);
                                    for s in 0..count {
                                        let column = (s & 31) as usize;
                                        for row in 0..32usize {
                                            let tile = &mut ripple.tiles[row * 32 + column];
                                            tile.offset[1] += 4;
                                            tile.angle = (tile.angle + 4).min(180);
                                        }
                                    }
                                } else {
                                    for tile in &mut ripple.tiles {
                                        tile.offset[1] += tile.speed;
                                        tile.offset[0] += tile.sign * tile.speed;
                                    }
                                }
                            }
                            ripple.dirty = true;
                        }
                    }
                }
                Proc::Mosaic => {
                    let block = ripple.block(now);
                    if block != ripple.last_block {
                        ripple.last_block = block;
                        ripple.dirty = true;
                    }
                }
            }
            if let Some(job) = &mut ripple.job {
                match job.poll()? {
                    ImageProcessState::Pending => continue,
                    ImageProcessState::Failed(error) => bail!("N2System Ripple: {error}"),
                    ImageProcessState::Ready(image) => {
                        let uploaded = host.publish_image_resource(self.client, id, image)?;
                        let path = uploaded.path.relative().to_owned();
                        let mut paint = ripple.original.clone();
                        paint["asset"] = json!(path);
                        if ripple.reset_view {
                            commands
                                .push(json!({"op":"set_paint","id":id,"paint":{"kind":"group"}}));
                            ripple.reset_view = false;
                        }
                        commands.push(json!({"op":"set_paint","id":id,"paint":paint}));
                        if let Some(rect) = ripple.reset_rect.take() {
                            commands.push(json!({"op":"set_rect","id":id,"rect":rect}));
                        }
                        if ripple.proc.tiled() {
                            // The output is a full-canvas picture of the tiles.
                            commands.push(json!({"op":"set_rect","id":id,
                                "rect":[0,0,canvas[0],canvas[1]]}));
                        }
                        if let Some(old) = ripple.output.replace(path) {
                            release.push(old);
                        }
                        ripple.job = None;
                    }
                }
            }
            if ripple.dirty && ripple.job.is_none() {
                let (input, effects): (ImageInput, Arc<[u8]>) = match ripple.proc {
                    Proc::Spiral => {
                        let [w, h] = ripple.source.size();
                        (
                            ImageInput::FieldWarp {
                                source: Box::new(ImageInput::Resource(ripple.source.clone())),
                                size: [w, h],
                                spec: FieldWarpSpec::PolarTwist {
                                    field: ripple
                                        .field
                                        .clone()
                                        .context("N2System Spiral geometry")?,
                                    center: [(w / 2) as i32, (h / 2) as i32],
                                    radius: (w.min(h) / 2).max(1) as u16,
                                    strength: ripple.block(now).clamp(0, 1000) as u16,
                                    clear: ripple.clear,
                                },
                            },
                            Arc::from(&b"[]"[..]),
                        )
                    }
                    Proc::Ripple => (
                        ImageInput::FieldWarp {
                            source: Box::new(ImageInput::Resource(ripple.source.clone())),
                            size: ripple.source.size(),
                            spec: FieldWarpSpec::RadialPhase {
                                field: ripple.field.clone().context("N2System Ripple field")?,
                                phase: ripple.phase,
                                frequency: (256. / f64::from(ripple.source.size()[0]) * 20.).trunc()
                                    as u16,
                                amplitude: ripple.amplitude(now),
                                clear: ripple.clear,
                            },
                        },
                        Arc::from(&b"[]"[..]),
                    ),
                    Proc::Wave(period) => {
                        let [width, height] = ripple.source.size();
                        let amplitude = ripple.block(now);
                        let slope = ((period as i64) << 16) / i64::from(height.max(1));
                        let offsets = (0..height)
                            .map(|y| {
                                let phase = (((i64::from(y) * slope) >> 16) as u32 & 0xffff)
                                    .wrapping_add(ripple.phase);
                                let sine = ((f64::from(phase & 0xff) * std::f64::consts::PI / 128.)
                                    .sin() as f32)
                                    as f64;
                                (sine * f64::from(amplitude)).trunc() as i32
                            })
                            .collect::<Vec<_>>();
                        let _ = width;
                        ripple.phase = ripple.phase.wrapping_add(1);
                        (
                            ImageInput::Resource(ripple.source.clone()),
                            Arc::from(serde_json::to_vec(
                                &json!([{"kind":"row_shift","offsets":offsets}]),
                            )?),
                        )
                    }
                    Proc::Split(_) | Proc::TurnOver => {
                        let [w, h] = ripple.source.size();
                        let [cols, rows] = ripple.grid;
                        let (tw, th) = ((w / cols).max(1), (h / rows).max(1));
                        let origin = [
                            ripple.rect[0].as_f64().unwrap_or(0.).trunc() as i32,
                            ripple.rect[1].as_f64().unwrap_or(0.).trunc() as i32,
                        ];
                        let base = if ripple.proc.tiled() && matches!(ripple.proc, Proc::Split(_)) {
                            ripple.block(now)
                        } else {
                            0
                        };
                        let mut draws = Vec::with_capacity(ripple.tiles.len());
                        for (index, tile) in ripple.tiles.iter().enumerate() {
                            let (c, r) = ((index as u32) % cols, (index as u32) / cols);
                            let (x, y) = (c * tw, r * th);
                            let width = if c + 1 == cols { w - x } else { tw };
                            let height = if r + 1 == rows { h - y } else { th };
                            let flip = if ripple.proc == Proc::TurnOver {
                                ((f64::from(tile.angle).to_radians().cos()) * 256.).round() as i32
                            } else {
                                256
                            };
                            draws.push(json!({
                                "src":[x, y, width, height],
                                "dst":[origin[0] + x as i32 + tile.offset[0],
                                       origin[1] + y as i32 + tile.offset[1] + base],
                                "scale":[flip, 256],
                                "opacity":255
                            }));
                        }
                        let canvas = self.config.canvas;
                        (
                            ImageInput::Resource(ripple.source.clone()),
                            Arc::from(serde_json::to_vec(&json!([{
                                "kind":"tile_compose",
                                "spec":{"canvas":canvas,"tiles":draws}
                            }]))?),
                        )
                    }
                    Proc::Mosaic => {
                        let cell = ripple.last_block;
                        let effects = if cell > 0 {
                            serde_json::to_vec(&json!([{"kind":"full_block_average","cell":cell}]))?
                        } else {
                            b"[]".to_vec()
                        };
                        (
                            ImageInput::Resource(ripple.source.clone()),
                            Arc::from(effects),
                        )
                    }
                };
                match self.images.as_ref().unwrap().begin_process(input, effects) {
                    Ok(job) => {
                        ripple.job = Some(job);
                        ripple.dirty = false;
                    }
                    Err(Error::Limit("image preparation task capacity")) => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        self.submit(host, commands, vec![])?;
        for path in release {
            host.release_image_resource(self.client, &path)?;
        }
        for id in restore {
            self.remove_ripple(host, id, true)?;
        }
        Ok(())
    }
}
