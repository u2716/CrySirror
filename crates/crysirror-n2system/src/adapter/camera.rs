// SPDX-License-Identifier: MPL-2.0
//! Trial40E7A0/448A30 creates a non-drawing, inherited camera record;
//! 411570/448C10/451860 moves its three integer channels. 492AC0 uses
//! current camera Z as the perspective numerator, a 1000-unit sprite plane,
//! canvas-centred projection and signed16 screen vertices. SetFov457A50
//! applies only to cube records, never to this 2D camera.
use super::*;
use crysirror_scene::animation::ScalarTween;

#[derive(Clone)]
pub(super) struct Camera {
    position: [f32; 3],
    motion: Option<[ScalarTween; 3]>,
    rotation: [f32; 3],
    angular_motion: Option<[ScalarTween; 3]>,
    published: Option<([f32; 16], [f32; 3], f32)>,
}
impl Camera {
    fn sample(&self, now: u64) -> [f32; 3] {
        self.motion.as_ref().map_or(self.position, |tracks| {
            tracks.each_ref().map(|track| track.sample(now).trunc())
        })
    }
    pub(super) fn active(&self, now: u64) -> bool {
        self.motion
            .as_ref()
            .is_some_and(|tracks| tracks.iter().any(|t| !t.complete(now)))
            || self
                .angular_motion
                .as_ref()
                .is_some_and(|tracks| tracks.iter().any(|t| !t.complete(now)))
    }
    pub(super) fn active_channels(&self, now: u64, properties: &[&str]) -> bool {
        let angular =
            properties.is_empty() || properties.iter().any(|p| p.starts_with("rotation_"));
        let linear =
            properties.is_empty() || properties.iter().any(|p| matches!(*p, "x" | "y" | "z"));
        (angular
            && self
                .angular_motion
                .as_ref()
                .is_some_and(|ts| ts.iter().any(|t| !t.complete(now))))
            || (linear
                && self
                    .motion
                    .as_ref()
                    .is_some_and(|ts| ts.iter().any(|t| !t.complete(now))))
    }
    fn angles(&self, now: u64) -> [f32; 3] {
        self.angular_motion
            .as_ref()
            .map_or(self.rotation, |tracks| {
                tracks.each_ref().map(|track| track.sample(now).trunc())
            })
    }
    fn matrix(&self, now: u64) -> glam::Mat4 {
        let p = self.sample(now);
        // 492AC0 subtracts camera Euler channels before sprite rotation;
        // an inverse view rotation would give different combined-axis poses.
        glam::Mat4::from_translation(glam::vec3(-p[0], -p[1], 1000.))
    }
}
pub(super) fn destination(value: &Value, current: f32) -> Result<f32> {
    let (relative, text) = match value {
        Value::Delta(n) => (true, n.to_string()),
        other => {
            let s = other.string()?;
            (s.starts_with('@'), s.trim_start_matches('@').to_owned())
        }
    };
    // Native atoi accepts an integer prefix and treats absent/NULL as zero.
    let text = text.trim_start();
    let sign = usize::from(text.starts_with('-') || text.starts_with('+'));
    let end = sign + text[sign..].bytes().take_while(u8::is_ascii_digit).count();
    let n = if end == sign {
        0
    } else {
        text[..end]
            .parse::<i32>()
            .context("N2System camera integer range")?
    };
    let result = if relative {
        current + n as f32
    } else {
        n as f32
    };
    ensure!(
        result.is_finite() && result.abs() <= 1_000_000.,
        "N2System camera coordinate bounds"
    );
    Ok(result)
}
impl N2SystemVm {
    pub(super) fn native_camera(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let now = host.now_ms();
        if call.function == Builtin::CreateCamera {
            ensure!(args.len() == 4, "N2System CreateCamera signature");
            let position = [
                destination(args.value(1), 0.)?,
                destination(args.value(2), 0.)?,
                destination(args.value(3), 0.)?,
            ];
            let camera = Camera {
                position,
                motion: None,
                rotation: [0.; 3],
                angular_motion: None,
                published: None,
            };
            // A native camera is not an ordinary sprite container: keep it
            // as an ownership group so children inherit the shared lens once.
            let id = self.create(
                host,
                current,
                &args.string(0)?,
                Kind::Name,
                self.config.canvas.map(|v| v as f32),
            )?;
            self.cameras.insert(id, camera);
            self.sample_cameras(host)?;
        } else {
            ensure!(
                matches!(call.function, Builtin::MoveCamera | Builtin::MoveCube) && args.len() == 7,
                "N2System MoveCamera signature"
            );
            let duration = args.time(1)?;
            let easing = serde_json::from_value(easing(args, 5)?)?;
            let mut updates = Vec::new();
            for id in self.query(current, &args.string(0)?)? {
                let Some(camera) = self.cameras.get(&id) else {
                    continue;
                };
                let start = camera.sample(now);
                let end = [
                    destination(args.value(2), start[0])?,
                    destination(args.value(3), start[1])?,
                    destination(args.value(4), start[2])?,
                ];
                let camera = Camera {
                    position: end,
                    rotation: camera.rotation,
                    angular_motion: camera.angular_motion.clone(),
                    published: camera.published,
                    motion: Some([
                        ScalarTween::new(start[0], end[0], now, duration, easing)?,
                        ScalarTween::new(start[1], end[1], now, duration, easing)?,
                        ScalarTween::new(start[2], end[2], now, duration, easing)?,
                    ]),
                };
                updates.push((id, camera));
            }
            let mut ids: Vec<_> = updates.iter().map(|(id, _)| *id).collect();
            for id in self.query(current, &args.string(0)?)? {
                if let Some(cube) = self.cubes.get_mut(&id) {
                    cube.move_to(now, args, duration, easing)?;
                    ids.push(id);
                }
            }
            self.sample_cubes(host)?;
            self.cameras.extend(updates);
            self.sample_cameras(host)?;
            let timeout = args.anim_wait(6, duration)?;
            if timeout > 0 {
                state.wait = Wait::Actions {
                    ids,
                    until: Some(now.saturating_add(timeout)),
                };
            }
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn sample_cameras(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        // Native camera pointers select the nearest owner, rather than
        // accumulating each enclosing camera. Cancel the enclosing matrix
        // while preserving its logical visibility/ownership group.
        let mut commands = Vec::new();
        let mut published = Vec::new();
        for (&id, camera) in &self.cameras {
            let mut parent = self.entities.nodes[&id].parent;
            let mut inherited = glam::Mat4::IDENTITY;
            let mut depth = 0;
            while let Some(ancestor) = parent {
                ensure!(depth < 128, "N2System camera ownership depth");
                depth += 1;
                if let Some(outer) = self.cameras.get(&ancestor) {
                    inherited = outer.matrix(now);
                    break;
                }
                parent = self.entities.nodes[&ancestor].parent;
            }
            let local = inherited.inverse() * camera.matrix(now);
            let p = camera.sample(now);
            let key = (
                local.to_cols_array(),
                camera.angles(now).map(|v| -v.to_radians()),
                p[2],
            );
            if camera.published == Some(key) {
                continue;
            }
            published.push((id, key));
            commands.push(json!({"op":"set_transform","id":id,"transform":{
                "local_matrix":local.to_cols_array(),
                "canvas_projection":{"model_rotation_offset":camera.angles(now).map(|v| -v.to_radians()),
                    "plane":p[2],"near":0.1,"far":1000000.,
                    "affine_uv":true,"signed_16_xy":true}
            }}));
        }
        self.submit(host, commands, vec![])?;
        for (id, key) in published {
            self.cameras.get_mut(&id).unwrap().published = Some(key);
        }
        for camera in self.cameras.values_mut() {
            if camera
                .motion
                .as_ref()
                .is_some_and(|tracks| tracks.iter().all(|t| t.complete(now)))
            {
                camera.motion = None;
            }
            if camera
                .angular_motion
                .as_ref()
                .is_some_and(|tracks| tracks.iter().all(|t| t.complete(now)))
            {
                camera.angular_motion = None;
            }
        }
        Ok(())
    }
    pub(super) fn rotate_camera(&mut self, id: u32, now: u64, args: &Arguments<'_>) -> Result<()> {
        let duration = args.time(1)?;
        let curve = serde_json::from_value(easing(args, 5)?)?;
        let camera = self
            .cameras
            .get_mut(&id)
            .context("N2System camera rotation owner")?;
        let start = camera.angles(now);
        let mut end = start;
        for axis in 0..3 {
            end[axis] = match args.value(axis + 2) {
                Value::Null => start[axis],
                other => destination(other, start[axis])?,
            };
        }
        camera.rotation = end;
        camera.angular_motion = Some([
            ScalarTween::new(start[0], end[0], now, duration, curve)?,
            ScalarTween::new(start[1], end[1], now, duration, curve)?,
            ScalarTween::new(start[2], end[2], now, duration, curve)?,
        ]);
        Ok(())
    }
}
