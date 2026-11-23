// SPDX-License-Identifier: MPL-2.0
//! Free flight of the installed point distribution, before solver contacts.
use super::Controls;
use crate::{level::Level, map::View, unity::Unity};
use anyhow::{Context, Result, ensure};
use crysirror_core::service::buttons;
use crysirror_unity::{
    managed::{Fields, Value},
    world6::world_matrix,
};
use crysirror_vfx::{
    BlendMode, EmitterSpec, InitSpec, Module, ParameterValue, RenderSpec, SpawnSpec, SystemSpec,
};
use glam::{Mat4, Quat, Vec3};

fn f(v: &Fields, key: &str) -> Result<f32> {
    Ok(v.float(key).with_context(|| format!("WaterGun {key}"))? as f32)
}
fn xyz(v: &Fields) -> Result<Vec3> {
    Ok(Vec3::new(f(v, "x")?, f(v, "y")?, f(v, "z")?))
}
pub struct Jet {
    pub system: SystemSpec,
    pub controls: Controls,
    nozzle: Mat4,
    rig: usize,
    raised: Mat4,
    /// Original gun mesh uploads placed in this scene.
    pub model_entities: Vec<(u32, crysirror_core::path::VfsPath)>,
    pub previous_motion: Option<(String, String)>,
    point: Vec3,
    axis: Vec3,
    spread: [[f32; 2]; 2],
    input: u64,
    pulse_end: Option<f64>,
    pulse_on: bool,
    aim: Quat,
    pub speed: f32,
    pub held: bool,
}
impl Jet {
    pub fn load(unity: &mut Unity, params: &Fields) -> Result<(Level, Self)> {
        let (level, controls) = Controls::load(unity, params)?;
        let component = |script: &str| {
            level
                .scene
                .nodes
                .iter()
                .enumerate()
                .find_map(|(i, n)| {
                    n.components
                        .iter()
                        .find(|c| c.script == script)
                        .map(|c| (i, &c.fields))
                })
                .with_context(|| format!("WaterGun {script}"))
        };
        let (_, solver) = component("Obi.ObiSolver")?;
        let (node, shape) = component("Obi.ObiEmitterShapeDisk")?;
        let (_, emitter) = component("Obi.ObiEmitter")?;
        let (_, scene) = component("AL.WaterGunScene")?;
        let (_, trigger) = component("WaterGunObiFFEmitKeyPress")?;
        let camera = scene
            .fields("_cameraInfo")
            .and_then(|c| c.pptr("Camera"))
            .and_then(|p| level.owner(p.path_id))
            .context("WaterGun camera")?;
        let rig = scene
            .pptr("_transWatergunMove")
            .and_then(|p| level.owner(p.path_id))
            .context("gun movement root")?;
        let parent = level.scene.nodes[rig]
            .parent
            .context("gun movement parent")?;
        let local = &level.scene.nodes[rig].xform;
        let mut position = Vec3::from_array(local.position);
        position.z = f(params, "MoveObjMax")?;
        // WaterGunMove is started with IsWaterGunInit=false; it raises X from
        // RotObjMin to RotObjMax and moves local Z from MoveObjMin to Max.
        let raised = world_matrix(&level.scene, camera).inverse()
            * world_matrix(&level.scene, parent)
            * Mat4::from_scale_rotation_translation(
                Vec3::from_array(local.scale),
                Quat::from_rotation_x(f(params, "RotObjMax")?.to_radians()),
                position,
            );
        let nozzle = world_matrix(&level.scene, rig).inverse() * world_matrix(&level.scene, node);
        let [Value::Struct(dist)] = shape.array("distribution").context("disk distribution")?
        else {
            anyhow::bail!("unsupported WaterGun disk distribution");
        };
        let point = xyz(dist.fields("position").context("emission point")?)?;
        let axis = xyz(dist.fields("direction").context("emission axis")?)?;
        let bp = emitter
            .pptr("emitterBlueprint")
            .context("fluid blueprint")?;
        let (key, id) = unity.target(&level.key, bp)?;
        let bp = unity.decode(&key, id)?;
        let capacity = u32::try_from(bp.int("capacity").context("fluid capacity")?)?;
        let resolution = f(&bp, "resolution")?;
        ensure!(
            resolution > 0.
                && f(emitter, "randomDirection")? == 0.
                && f(emitter, "inheritVelocity")? == 0.,
            "unsupported fluid birth parameters"
        );
        let diameter = 0.1 / resolution.cbrt();
        let lifetime = f(emitter, "lifespan")?;
        let color = shape.fields("color").context("shape color")?;
        let color = [
            f(color, "r")?,
            f(color, "g")?,
            f(color, "b")?,
            f(color, "a")?,
        ];
        let gravity = xyz(solver.fields("gravity").context("solver gravity")?)?;
        let angles = |name| -> Result<[f32; 2]> {
            let r = trigger.fields(name).context("gun rotation limits")?;
            Ok([f(r, "x")?, f(r, "y")?])
        };
        ensure!(
            params.int("ShootKey") == Some(32) && params.int("ShootMouseKey") == Some(2),
            "unmapped WaterGun controls"
        );
        // Separate birth speeds share one active switch; changing mode does not reset flying drops.
        let mut system = SystemSpec {
            emitters: Vec::new(),
            ..Default::default()
        };
        for (name, speed) in [
            ("laser", controls.laser_speed),
            ("pulse", controls.burst_speed),
        ] {
            system
                .parameters
                .insert(name.to_owned(), ParameterValue::Bool(false));
            let mut render = RenderSpec::default();
            if let RenderSpec::Sprite { blend, .. } = &mut render {
                *blend = BlendMode::Alpha;
            }
            let spec = EmitterSpec {
                id: name.to_owned(),
                enabled_parameter: Some(name.to_owned()),
                capacity,
                render,
                spawn: SpawnSpec {
                    rate: (speed / diameter).into(),
                    ..Default::default()
                },
                initialize: InitSpec {
                    speed: [speed; 2],
                    direction: Vec3::Z.to_array().into(),
                    lifetime: [lifetime; 2],
                    size: [diameter; 2],
                    color,
                    spread_degrees: 0.,
                    ..Default::default()
                },
                update: vec![Module::Acceleration {
                    value: gravity.to_array().into(),
                }],
                ..Default::default()
            };
            system.emitters.push(spec);
        }
        system.validate()?;
        let spread = [angles("_rotLimitX")?, angles("_rotLimitY")?];
        Ok((
            level,
            Self {
                controls,
                system,
                nozzle,
                rig,
                raised,
                model_entities: Vec::new(),
                previous_motion: None,
                point,
                axis,
                spread,
                input: buttons::MOUSE_MIDDLE | buttons::SPACE,
                pulse_end: None,
                pulse_on: false,
                aim: Quat::IDENTITY,
                speed: 0.,
                held: false,
            },
        ))
    }

    pub fn set_texture(&mut self, path: &str) {
        for e in &mut self.system.emitters {
            if let RenderSpec::Sprite { texture, .. } = &mut e.render {
                *texture = Some(path.to_owned());
            }
        }
    }

    /// Unity samples the held buttons every update; the pulsed mode starts on,
    /// alternates independent native intervals, and releases without a tail burst.
    pub fn update(&mut self, buttons: u64, now: f64, mut random: impl FnMut() -> f32) -> bool {
        let range = |[a, b]: [f32; 2], u: f32| a + (b - a) * u;
        let held = buttons & self.input != 0;
        self.held = held;
        if !held || self.controls.laser {
            self.pulse_end = None;
            self.pulse_on = false;
            self.aim = Quat::IDENTITY;
        } else if self.pulse_end.is_none_or(|end| now >= end) {
            self.pulse_on = !self.pulse_on;
            let span = if self.pulse_on {
                let [x, y] = self.spread.map(|r| range(r, random()).to_radians());
                self.aim = Quat::from_rotation_y(y) * Quat::from_rotation_x(x);
                self.controls.on_seconds
            } else {
                self.controls.off_seconds
            };
            self.pulse_end = Some(now + f64::from(range(span, random())));
        }
        let speed = if held && self.controls.laser {
            self.controls.laser_speed
        } else if self.pulse_on {
            self.controls.burst_speed
        } else {
            0.
        };
        let changed = self.speed != speed;
        if changed {
            self.system.parameters.insert(
                "laser".into(),
                ParameterValue::Bool(held && self.controls.laser),
            );
            self.system
                .parameters
                .insert("pulse".into(), ParameterValue::Bool(self.pulse_on));
            self.speed = speed;
        }
        changed
    }
    fn camera(view: &View) -> Mat4 {
        let eye = Vec3::from_array(view.eye);
        let forward = (Vec3::from_array(view.target) - eye).normalize();
        let right = forward.cross(Vec3::from_array(view.up)).normalize();
        let up = right.cross(forward);
        Mat4::from_cols(
            right.extend(0.),
            up.extend(0.),
            forward.extend(0.),
            eye.extend(1.),
        )
    }
    /// The GLB exporter mirrors X; the camera basis maps Unity XY/Z directly.
    pub fn model_pose(&self, view: &View) -> Mat4 {
        Self::camera(view) * self.raised * Mat4::from_scale(Vec3::new(-1., 1., 1.))
    }
    pub fn model(&self, level: &Level, unity: &mut Unity) -> Result<Vec<Vec<u8>>> {
        let mut rig = level.scene.subtree(self.rig);
        let root = &mut rig.nodes[0].xform;
        root.position = [0.; 3];
        root.rotation = [0., 0., 0., 1.];
        root.scale = [1.; 3];
        let (chunks, report) = crysirror_unity::world6::scene_glb(
            &rig,
            &level.key,
            &[0],
            unity,
            &Default::default(),
            "CrySirror WaterGun",
        )?;
        ensure!(
            report.errors.is_empty(),
            "WaterGun mesh: {:?}",
            report.errors
        );
        Ok(chunks)
    }
    pub fn pose(&self, view: &View) -> (Vec3, Quat) {
        let m = Self::camera(view) * self.raised * self.nozzle;
        let p = m.transform_point3(self.point);
        let d = m.transform_vector3(self.aim * self.axis).normalize();
        (p, Quat::from_rotation_arc(Vec3::Z, d))
    }
}

/// Coverage of a round particle's projected footprint, shared by both birth modes.
pub fn drop_mask() -> Vec<u8> {
    (0..32 * 32)
        .flat_map(|i| {
            let x = (i % 32) as f32 + 0.5 - 16.;
            let y = (i / 32) as f32 + 0.5 - 16.;
            let alpha = ((16. - x.hypot(y)).clamp(0., 1.) * 255.).round() as u8;
            [255, 255, 255, alpha]
        })
        .collect()
}
