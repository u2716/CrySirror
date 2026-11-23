//! Ordered composition state. Commands participate in the same global stable
//! order as drawings, then affect subsequent geometry across layer subtrees.
//! Logical commands survive snapshots; the transient stack is rebuilt per frame.
use super::*;
use glam::Mat4;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Reset,
    Push { state: State },
    Pop,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct State {
    pub translation: [f32; 3],
    pub scale: [f32; 3],
    pub rotation: f32,
    pub pivot: [f32; 3],
    pub tint: [u8; 4],
    pub add: [u8; 3],
    pub blend: Option<Blend>,
    pub linear: Option<bool>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            translation: [0.; 3],
            scale: [1.; 3],
            rotation: 0.,
            pivot: [0.; 3],
            tint: [255; 4],
            add: [0; 3],
            blend: None,
            linear: None,
        }
    }
}
impl Command {
    pub fn validate(&self) -> Result<()> {
        if let Self::Push { state } = self {
            ensure!(
                state
                    .translation
                    .iter()
                    .chain(&state.scale)
                    .chain(&state.pivot)
                    .chain(std::iter::once(&state.rotation))
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                "ordered composition state bounds"
            );
        }
        Ok(())
    }
}
impl State {
    fn accumulate(&self, next: &Self) -> Self {
        Self {
            translation: std::array::from_fn(|i| self.translation[i] + next.translation[i]),
            scale: std::array::from_fn(|i| self.scale[i] * next.scale[i]),
            rotation: self.rotation + next.rotation,
            pivot: next.pivot,
            tint: std::array::from_fn(|i| {
                (u16::from(self.tint[i]) * u16::from(next.tint[i]) / 255) as u8
            }),
            add: std::array::from_fn(|i| self.add[i].saturating_add(next.add[i])),
            blend: next.blend.or(self.blend),
            linear: next.linear.or(self.linear),
        }
    }
    fn matrix(&self) -> Mat4 {
        let pivot = glam::Vec3::from_array(self.pivot);
        // Channel accumulation is explicit: sum translations/angles, multiply
        // axis scales. Scale follows rotation, not recursive parent matrices.
        Mat4::from_translation(pivot + glam::Vec3::from_array(self.translation))
            * Mat4::from_scale(glam::Vec3::from_array(self.scale))
            * Mat4::from_rotation_z(self.rotation)
            * Mat4::from_translation(-pivot)
    }
    fn apply(&self, p: &mut Primitive) {
        p.opacity = (u16::from(p.opacity) * u16::from(self.tint[3]) / 255) as u8;
        let filter = p.filter.get_or_insert_with(LayerFilter::default);
        for axis in 0..3 {
            filter.multiply[axis] *= f32::from(self.tint[axis]) / 255.;
            filter.add[axis] = (filter.add[axis] + f32::from(self.add[axis]) / 255.).min(1.);
        }
        if let Some(blend) = self.blend {
            p.blend = blend;
        }
        if let Some(linear) = self.linear {
            p.sampling.linear = linear;
        }
        if let Paint::ProjectedMesh { frame, .. } = &mut p.paint {
            if self.blend.is_some() || self.linear.is_some() {
                for part in &mut Arc::make_mut(frame).parts {
                    if let Some(blend) = self.blend {
                        part.blend = Some(blend);
                    }
                    if let Some(linear) = self.linear {
                        part.linear = linear;
                    }
                }
            }
        }
    }
}
pub(super) struct Pending {
    pub primitive: Primitive,
    pub world: [[f32; 3]; 4],
    pub camera: SpriteTransform,
    pub mesh: Option<crate::legacy_mesh::WorldFrame>,
    pub ordinary_2d: bool,
    pub geometry_order: Option<projection::GeometryOrder>,
}
pub(super) fn resolve(mut pending: Vec<Pending>, canvas: [u32; 2]) -> Result<Vec<Primitive>> {
    let mut expanded = Vec::with_capacity(pending.len());
    for mut item in pending.drain(..) {
        if let Some(order) = item.geometry_order {
            if let Some(mesh) = item.mesh.take() {
                for (key, mesh) in mesh.ordered_faces(order)? {
                    ensure!(
                        expanded.len() < 1_000_000,
                        "ordered composition primitive budget"
                    );
                    let mut primitive = item.primitive.clone();
                    primitive.order[1] = primitive.order[1]
                        .checked_add(key)
                        .context("geometry order addition overflow")?;
                    expanded.push(Pending {
                        primitive,
                        world: item.world,
                        camera: item.camera.clone(),
                        mesh: Some(mesh),
                        ordinary_2d: item.ordinary_2d,
                        geometry_order: None,
                    });
                }
                continue;
            }
            item.primitive.order[1] = item.primitive.order[1]
                .checked_add(order.key(item.world)?)
                .context("geometry order addition overflow")?;
        }
        expanded.push(item);
    }
    pending = expanded;
    pending.sort_by(|a, b| compare_primitives(&a.primitive, &b.primitive));
    let mut stacks: BTreeMap<u32, Vec<State>> = BTreeMap::new();
    let mut output = Vec::with_capacity(pending.len());
    for mut item in pending {
        let stack = stacks.entry(item.primitive.owner).or_default();
        if let Paint::OrderedState { command } = &item.primitive.paint {
            match command {
                Command::Reset => stack.clear(),
                //Native56B4B0 explicitly keeps the initial state when empty.
                Command::Pop => {
                    stack.pop();
                }
                Command::Push { state } => {
                    ensure!(stack.len() < 64, "ordered composition stack budget");
                    let mut incoming = state.clone();
                    incoming.tint[3] = (u16::from(incoming.tint[3])
                        * u16::from(item.primitive.opacity)
                        / 255) as u8;
                    if let Some(filter) = &item.primitive.filter {
                        for axis in 0..3 {
                            incoming.tint[axis] =
                                (f32::from(incoming.tint[axis]) * filter.multiply[axis])
                                    .clamp(0., 255.) as u8;
                        }
                    }
                    let state = stack
                        .last()
                        .map_or_else(|| incoming.clone(), |old| old.accumulate(&incoming));
                    ensure!(
                        state.matrix().is_finite(),
                        "ordered composition matrix overflow"
                    );
                    stack.push(state);
                }
            }
            continue;
        }
        let state = stack.last();
        let matrix = state.map_or(Mat4::IDENTITY, State::matrix);
        if let Some(mut mesh) = item.mesh {
            if state.is_some() {
                mesh.transform(matrix)?;
            }
            let frame = mesh.project(&item.camera, canvas)?;
            item.primitive.paint = Paint::ProjectedMesh {
                frame: Arc::new(frame),
                tint: [255; 4],
            };
            item.primitive.quad = [
                [0., 0., 0., 1.],
                [canvas[0] as f32, 0., 0., 1.],
                [canvas[0] as f32, canvas[1] as f32, 0., 1.],
                [0., canvas[1] as f32, 0., 1.],
            ];
        } else if state.is_some() {
            let mut visible = true;
            for i in 0..4 {
                let point = matrix.transform_point3(glam::Vec3::from_array(item.world[i]));
                let Some(p) = projection::project_world_point_unclipped(
                    &item.camera,
                    point.to_array(),
                    canvas.map(|v| v as f32),
                ) else {
                    visible = false;
                    break;
                };
                item.primitive.quad[i] = [
                    p.x,
                    p.y,
                    if item.ordinary_2d && state.is_none() {
                        0.
                    } else {
                        p.depth
                    },
                    p.clip_w,
                ];
            }
            if !visible {
                continue;
            }
        }
        if let Some(state) = state {
            state.apply(&mut item.primitive);
        }
        if let Some(lens)=item.camera.canvas_projection {
            let points=item.world.map(|p|matrix.transform_point3(glam::Vec3::from_array(p)).to_array());
            if !matches!(item.primitive.paint,Paint::ProjectedMesh{..})&&!lens.admits(points) {
                match item.primitive.coverage {
                    super::coverage::Operation::None=>continue,
                    super::coverage::Operation::Write=>item.primitive.clip=Some([0.;4]),
                    super::coverage::Operation::Reset{..}=>{},
                }
            }
        }
        if item.primitive.opacity == 0 && item.primitive.coverage==super::coverage::Operation::None
            && !super::writes_at_zero_opacity(&item.primitive.paint,item.primitive.blend) {
            continue;
        }
        if let Some(clip) = item.primitive.clip.filter(|_|item.primitive.coverage==super::coverage::Operation::None) {
            if item.primitive.quad.iter().all(|p| p[3] > 0.)
                && matches!(
                    &item.primitive.paint,
                    Paint::Solid { .. }
                        | Paint::Image { .. }
                        | Paint::Snapshot { .. }
                        | Paint::Text { .. }
                )
            {
                let q = &item.primitive.quad;
                let min_x = q.iter().map(|v| v[0]).fold(f32::INFINITY, f32::min);
                let max_x = q.iter().map(|v| v[0]).fold(f32::NEG_INFINITY, f32::max);
                let min_y = q.iter().map(|v| v[1]).fold(f32::INFINITY, f32::min);
                let max_y = q.iter().map(|v| v[1]).fold(f32::NEG_INFINITY, f32::max);
                if max_x <= clip[0]
                    || max_y <= clip[1]
                    || min_x >= clip[0] + clip[2]
                    || min_y >= clip[1] + clip[3]
                {
                    continue;
                }
            }
        }
        output.push(item.primitive);
    }
    Ok(output)
}
