// SPDX-License-Identifier: MPL-2.0
//! Trial40DE30/447510/493040/45B3E0: six immutable square faces, 500-unit
//! half extent, inverse scale/translation/quaternion view and LH perspective.
//! Native face strips and pose policy only; clipping and GPU mesh composition
//! are reusable scene operations. No D3D/GPU handles enter the VM.
use super::*;
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_scene::{
    animation::ScalarTween,
    layer::Easing,
    projected_mesh::{MeshFrame, Part, Vertex},
};
use glam::{Mat4, Vec3};

struct Face {
    read: native_bitmap::NativeBitmap,
    image: Option<Arc<dyn ImageResource>>,
    opaque: Option<ImageJob>,
}
pub(super) struct Preparing {
    faces: Vec<Face>,
}
pub(super) struct Cube {
    mesh: Arc<MeshFrame>,
    pose: [f32; 9],
    tracks: BTreeMap<usize, ScalarTween>,
    fov: f32,
    published: Option<([f32; 9], f32)>,
}
impl Cube {
    fn sample(&self, now: u64) -> [f32; 9] {
        core::array::from_fn(|i| {
            self.tracks
                .get(&i)
                .map_or(self.pose[i], |t| t.sample(now).trunc())
        })
    }
    pub(super) fn active(&self, now: u64, properties: &[&str]) -> bool {
        self.tracks.iter().any(|(&i, t)| {
            !t.complete(now)
                && (properties.is_empty()
                    || properties.contains(
                        &[
                            "x",
                            "y",
                            "z",
                            "rotation_x",
                            "rotation_y",
                            "rotation_z",
                            "scale_x",
                            "scale_y",
                            "scale_z",
                        ][i],
                    ))
        })
    }
    pub(super) fn move_to(
        &mut self,
        now: u64,
        args: &Arguments<'_>,
        duration: u64,
        easing: Easing,
    ) -> Result<()> {
        let start = self.sample(now);
        for i in 0..3 {
            let end = camera::destination(args.value(i + 2), start[i])?;
            self.tracks
                .insert(i, ScalarTween::new(start[i], end, now, duration, easing)?);
        }
        Ok(())
    }
    fn command(&self, id: u32, now: u64) -> Result<Json> {
        let p = self.sample(now);
        ensure!(
            p[6..].iter().all(|v| v.abs() >= 1.),
            "N2System singular cube scale"
        );
        let camera = Mat4::from_rotation_z(p[5].to_radians())
            * Mat4::from_rotation_y(p[4].to_radians())
            * Mat4::from_rotation_x(p[3].to_radians())
            * Mat4::from_translation(Vec3::new(p[0], p[1], p[2]))
            * Mat4::from_scale(Vec3::new(p[6], p[7], p[8]) / 1000.);
        let perspective = Mat4::perspective_lh(self.fov.to_radians(), 16. / 9., 0.1, 2000.);
        let frame = self
            .mesh
            .project_homogeneous(perspective * camera.inverse(), true)?;
        // Original cube raster is an opaque canvas-sized bitmap, including
        // black clear pixels. Shared snapshot rendering applies outer native
        // sprite/filter/opacity after the cube's independent view projection.
        let [w, h] = self.mesh.size;
        Ok(
            json!({"op":"set_paint","id":id,"paint":{"kind":"snapshot","size":[w as u32,h as u32],"layers":[
                {"id":1,"rect":[0,0,w,h],"paint":{"kind":"solid","color":[0,0,0,255]}},
                {"id":2,"rect":[0,0,w,h],"paint":{"kind":"projected_mesh","frame":frame}}
            ]}}),
        )
    }
}
impl N2SystemVm {
    pub(super) fn native_cube(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        if call.function == Builtin::SetFov {
            ensure!(args.len() == 2, "N2System SetFov signature");
            let degrees = args.integer(1)?;
            ensure!(
                (1..180).contains(&degrees),
                "N2System invalid cube field of view"
            );
            for id in self.query(current, &args.string(0)?)? {
                if let Some(cube) = self.cubes.get_mut(&id) {
                    cube.fov = degrees as f32;
                }
            }
            self.sample_cubes(host)?;
        } else {
            ensure!(args.len() == 8, "N2System CreateCube signature");
            if state.cube_preparation.is_none() {
                let mut faces = Vec::with_capacity(6);
                for i in 0..6 {
                    let Some(read) =
                        self.read_native_bitmap(host, current, &args.string(i + 2)?)?
                    else {
                        return Ok(false);
                    };
                    faces.push(Face {
                        read,
                        image: None,
                        opaque: None,
                    });
                }
                state.cube_preparation = Some(Preparing { faces });
            }
            let preparation = state.cube_preparation.as_mut().unwrap();
            let mut ready = true;
            for face in &mut preparation.faces {
                if face.image.is_none() {
                    if let Some(job) = &mut face.opaque {
                        match job.poll()? {
                            ImageProcessState::Pending => {
                                ready = false;
                                continue;
                            }
                            ImageProcessState::Failed(error) => {
                                bail!("N2System cube face: {error}")
                            }
                            ImageProcessState::Ready(image) => face.image = Some(image),
                        }
                        face.opaque = None;
                    } else if let Some(image) = face.read.poll(
                        self.client,
                        self.pool.as_ref().unwrap(),
                        self.images.as_ref().unwrap(),
                    )? {
                        if image.opaque() {
                            face.image = Some(image);
                        } else {
                            let effects = Arc::from(serde_json::to_vec(
                                &json!([{"kind":"channel_lookup","channel":3,"write_mask":8,"table":vec![255u8;256]}]),
                            )?);
                            match self
                                .images
                                .as_ref()
                                .unwrap()
                                .begin_process(ImageInput::Resource(image), effects)
                            {
                                Ok(job) => face.opaque = Some(job),
                                Err(Error::Limit("image preparation task capacity")) => {}
                                Err(e) => return Err(e.into()),
                            }
                        }
                    }
                }
                ready &= face.image.is_some();
            }
            if !ready {
                return Ok(false);
            }
            let preparation = state.cube_preparation.take().unwrap();
            let size = preparation.faces[0].image.as_ref().unwrap().size();
            // 40DE30 explicitly releases all inputs and creates nothing when
            // any dimensions disagree or a face is not square.
            if size[0] != size[1]
                || preparation
                    .faces
                    .iter()
                    .any(|f| f.image.as_ref().unwrap().size() != size)
            {
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            let bytes = u64::from(size[0]) * u64::from(size[1]) * 4 * 6;
            ensure!(
                bytes
                    <= if cfg!(target_os = "vita") {
                        24 * 1024 * 1024
                    } else {
                        96 * 1024 * 1024
                    },
                "N2System cube face budget"
            );
            let extent = self.config.canvas;
            let id = self.create(
                host,
                current,
                &args.string(0)?,
                Kind::Visual,
                extent.map(|v| v as f32),
            )?;
            let mut paths = Vec::with_capacity(6);
            for face in preparation.faces {
                let result = host.publish_image_resource(self.client, id, face.image.unwrap());
                match result {
                    Ok(receipt) => paths.push(receipt.path.relative().to_owned()),
                    Err(error) => {
                        for path in paths {
                            host.release_image_resource(self.client, &path)?;
                        }
                        self.destroy(host, id)?;
                        return Err(error.into());
                    }
                }
            }
            self.animation_image_leases.insert(id, paths.clone());
            let h = 500.;
            let n = -h;
            let positions = [
                [[n, h, n], [n, n, n], [h, h, n], [h, n, n]],
                [[h, h, h], [h, n, h], [n, h, h], [n, n, h]],
                [[h, h, n], [h, n, n], [h, h, h], [h, n, h]],
                [[n, h, h], [n, n, h], [n, h, n], [n, n, n]],
                [[n, h, h], [n, h, n], [h, h, h], [h, h, n]],
                [[n, n, n], [n, n, h], [h, n, n], [h, n, h]],
            ];
            let inset = 0.5 / size[0] as f32;
            let end = 1. - inset;
            let uv = [[inset, inset], [inset, end], [end, inset], [end, end]];
            let parts = positions
                .into_iter()
                .zip(paths)
                .map(|(face, path)| Part {
                    vertices: face
                        .into_iter()
                        .zip(uv)
                        .map(|(v, uv)| Vertex {
                            position: [v[0], v[1], v[2], 1.],
                            uv,
                            color: [1.; 4],
                        })
                        .collect(),
                    indices: vec![0, 1, 2, 2, 1, 3],
                    texture: Some(path),
                    blend: None,
                    add: [0.; 3],
                    linear: true,
                })
                .collect();
            let cube = Cube {
                mesh: Arc::new(MeshFrame {
                    size: extent.map(|v| v as f32),
                    parts,
                }),
                pose: [0., 0., 0., 0., 0., 0., 1000., 1000., 1000.],
                tracks: BTreeMap::new(),
                fov: 60.,
                published: None,
            };
            let parent = self.entities.render_parent(id)?;
            self.submit(host,vec![json!({"op":"put","layer":{"id":id,"parent":parent,"order":[args.priority(1)?,0],"rect":[0,0,extent[0],extent[1]],"paint":{"kind":"group"}}})],vec![])?;
            self.cubes.insert(id, cube);
            self.sample_cubes(host)?;
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn animate_cube(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        args: &Arguments<'_>,
        function: Builtin,
    ) -> Result<bool> {
        let Some(cube) = self.cubes.get_mut(&id) else {
            return Ok(false);
        };
        let now = host.now_ms();
        let start = cube.sample(now);
        let rotate = function == Builtin::Rotate;
        let curve = serde_json::from_value(easing(args, if rotate { 5 } else { 4 })?)?;
        for axis in 0..if rotate { 3 } else { 2 } {
            let index = if rotate { 3 } else { 6 } + axis;
            let end = match args.value(axis + 2) {
                Value::Null => start[index],
                Value::Delta(n) => start[index] + n,
                _ => args.integer(axis + 2)? as f32,
            };
            ensure!(
                end.is_finite() && end.abs() <= 1_000_000.,
                "N2System cube pose range"
            );
            if !rotate {
                ensure!(
                    end.abs() >= 1. && start[index].signum() == end.signum(),
                    "N2System singular animated cube scale"
                );
            }
            cube.tracks.insert(
                index,
                ScalarTween::new(start[index], end, now, args.time(1)?, curve)?,
            );
        }
        self.sample_cubes(host)?;
        Ok(true)
    }
    pub(super) fn sample_cubes(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut commands = Vec::new();
        for (&id, cube) in &mut self.cubes {
            let pose = cube.sample(now);
            if cube.published != Some((pose, cube.fov)) {
                commands.push(cube.command(id, now)?);
                cube.published = Some((pose, cube.fov));
            }
            cube.pose = pose;
            cube.tracks.retain(|_, track| !track.complete(now));
        }
        self.submit(host, commands, vec![])
    }
}
