//! Explicit-clock elliptical atlas particles with a shared integer sequence.
//! Birth parameters are data; updates own no VM, texture loader, or renderer.
use crate::{Particle, falling_atlas::SequenceRng};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbitControl {
    /// Logical source origin, captured independently by every particle birth.
    pub origin: [f32; 2],
    /// Initial angle in degrees.
    pub angle: f32,
    /// Optional stopping angle; None leaves a static birth angle.
    pub target: Option<f32>,
    /// Signed divisor for per-tick angular displacement. Zero disables motion.
    pub ticks: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn spec() -> OrbitAtlasSpec {
        OrbitAtlasSpec {
            tick_hz: 60,
            texture_extent: [256, 64],
            cells: vec![[140, 0, 180, 40], [180, 0, 200, 20], [200, 0, 208, 8]],
            radius_stride: 64,
            radius_base: 8,
            radius_span: 20,
            scale_percent: [300, 699],
            alpha_steps: 10,
            alpha_divisor: 100.,
            radians_per_degree: 0.01745327777777778,
            position_anchor: [0.5, 0.5],
            signed_tolerance: true,
            control: OrbitControl {
                origin: [320., 180.],
                angle: 0.,
                target: Some(90.),
                ticks: 10,
            },
        }
    }
    fn particle() -> Particle {
        crate::spawn_particle(
            &crate::EmitterSpec::default()
                .compile(&Default::default())
                .unwrap(),
            0,
            glam::Mat4::IDENTITY.to_cols_array_2d(),
        )
        .unwrap()
    }
    #[test]
    fn source_trace_uses_five_sequential_draws_and_per_particle_atlas_rectangles() {
        let node = CompiledOrbitAtlas::compile(spec()).unwrap();
        let mut rng = SequenceRng::msvc(1);
        let mut particle = particle();
        let state = node.initialize(&mut particle, 0, &mut rng).unwrap();
        assert_eq!(state.radius, [9., 15.]);
        assert_eq!(state.cell, [180, 0, 200, 20]);
        assert_eq!(particle.position, [330., 205., 0.]);
        assert!((particle.size[0] - 126.8).abs() < 0.0001);
        assert_eq!(particle.color[3], 0.09);
        assert_eq!(node.uv(&state), [180. / 256., 0., 200. / 256., 20. / 64.]);
        assert_eq!(rng.next_u32().unwrap(), 15724);
    }
    #[test]
    fn signed_tolerance_preserves_forward_early_stop_and_backward_overshoot() {
        let mut node = CompiledOrbitAtlas::compile(spec()).unwrap();
        let mut rng = SequenceRng::msvc(3);
        let mut particle = particle();
        let mut state = node.initialize(&mut particle, 0, &mut rng).unwrap();
        for _ in 0..20 {
            node.step(&mut particle, &mut state);
        }
        assert_eq!(state.angle, 81.);
        assert_eq!(state.delta, 0.);
        node.update_control(OrbitControl {
            origin: [0.; 2],
            angle: 90.,
            target: Some(0.),
            ticks: 10,
        })
        .unwrap();
        let mut state = node.initialize(&mut particle, 0, &mut rng).unwrap();
        for _ in 0..11 {
            node.step(&mut particle, &mut state);
        }
        assert_eq!(state.angle, -9.);
        assert_eq!(state.delta, -9.);
    }
    #[test]
    fn live_control_only_affects_future_births_and_restored_state_keeps_the_same_orbit() {
        let mut node = CompiledOrbitAtlas::compile(spec()).unwrap();
        let mut rng = SequenceRng::msvc(9);
        let mut p = particle();
        let mut state = node.initialize(&mut p, 4, &mut rng).unwrap();
        let original = (p.clone(), state.clone(), rng.clone());
        node.update_control(OrbitControl {
            origin: [999.; 2],
            angle: -45.,
            target: None,
            ticks: 0,
        })
        .unwrap();
        assert_eq!((p.clone(), state.clone(), rng), original);
        let encoded = serde_json::to_vec(&(p.clone(), state.clone())).unwrap();
        let (mut other, mut resumed): (Particle, OrbitAtlasState) =
            serde_json::from_slice(&encoded).unwrap();
        for _ in 0..4 {
            assert_eq!(
                node.step(&mut p, &mut state),
                node.step(&mut other, &mut resumed)
            );
            assert_eq!(p, other);
            assert_eq!(state, resumed);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbitAtlasSpec {
    pub tick_hz: u32,
    pub texture_extent: [u32; 2],
    /// Ordered source rectangles [left,top,right,bottom].
    pub cells: Vec<[u32; 4]>,
    /// Radius base is index*stride+base, independently sampled for both axes.
    pub radius_stride: u32,
    pub radius_base: u32,
    pub radius_span: u32,
    /// Inclusive random scale range in percent.
    pub scale_percent: [u32; 2],
    /// Alpha is (random%alpha_steps)/alpha_divisor.
    pub alpha_steps: u32,
    pub alpha_divisor: f64,
    pub radians_per_degree: f64,
    /// Add this fraction of the unscaled cell to the published position.
    pub position_anchor: [f32; 2],
    /// Preserve signed step tolerance, including backward overshoot, when true.
    pub signed_tolerance: bool,
    pub control: OrbitControl,
}
impl OrbitControl {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.origin
                .iter()
                .chain([&self.angle])
                .chain(self.target.iter())
                .all(|value| value.is_finite() && value.abs() <= 1_000_000.),
            "VFX orbit control domain"
        );
        Ok(())
    }
}
impl OrbitAtlasSpec {
    pub fn validate(&self) -> Result<()> {
        self.control.validate()?;
        ensure!(
            (1..=240).contains(&self.tick_hz)
                && self
                    .texture_extent
                    .iter()
                    .all(|value| (1..=16384).contains(value))
                && !self.cells.is_empty()
                && self.cells.len() <= 256,
            "VFX orbit clock/atlas domain"
        );
        for cell in &self.cells {
            ensure!(
                cell[0] < cell[2]
                    && cell[1] < cell[3]
                    && cell[2] <= self.texture_extent[0]
                    && cell[3] <= self.texture_extent[1],
                "VFX orbit cell extent"
            );
        }
        ensure!(
            self.radius_span > 0
                && self.radius_span <= 65536
                && self.radius_stride <= 65536
                && self.radius_base <= 1_000_000
                && self.scale_percent[0] > 0
                && self.scale_percent[0] <= self.scale_percent[1]
                && self.scale_percent[1] <= 1600,
            "VFX orbit birth ranges"
        );
        ensure!(
            (1..=65536).contains(&self.alpha_steps)
                && self.alpha_divisor.is_finite()
                && self.alpha_divisor > 0.
                && f64::from(self.alpha_steps - 1) / self.alpha_divisor <= 1.
                && self.radians_per_degree.is_finite()
                && self.radians_per_degree > 0.
                && self.radians_per_degree <= 1.
                && self
                    .position_anchor
                    .iter()
                    .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "VFX orbit alpha/angle/anchor"
        );
        Ok(())
    }
    pub fn validate_capacity(&self, capacity: u32) -> Result<()> {
        ensure!(
            u64::from(self.radius_stride) * u64::from(capacity.saturating_sub(1))
                + u64::from(self.radius_base)
                + u64::from(self.radius_span)
                <= 1_000_000,
            "VFX orbit radius budget"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitAtlasState {
    pub origin: [f32; 2],
    pub angle: f32,
    pub target: f32,
    pub delta: f32,
    pub radius: [f32; 2],
    pub cell: [u32; 4],
}
#[derive(Clone, Debug)]
pub struct CompiledOrbitAtlas {
    spec: OrbitAtlasSpec,
}
impl CompiledOrbitAtlas {
    pub fn compile(spec: OrbitAtlasSpec) -> Result<Self> {
        spec.validate()?;
        Ok(Self { spec })
    }
    pub fn update_control(&mut self, control: OrbitControl) -> Result<()> {
        control.validate()?;
        self.spec.control = control;
        Ok(())
    }
    pub fn initialize(
        &self,
        particle: &mut Particle,
        index: u32,
        rng: &mut SequenceRng,
    ) -> Result<OrbitAtlasState> {
        let spec = &self.spec;
        let base = index
            .checked_mul(spec.radius_stride)
            .and_then(|value| value.checked_add(spec.radius_base))
            .context("VFX orbit radius overflow")?;
        let x = base + rng.next_u32()? % spec.radius_span;
        let y = base + rng.next_u32()? % spec.radius_span;
        let scale = spec.scale_percent[0]
            + rng.next_u32()? % (spec.scale_percent[1] - spec.scale_percent[0] + 1);
        let cell = spec.cells[rng.next_u32()? as usize % spec.cells.len()];
        let alpha = (f64::from(rng.next_u32()? % spec.alpha_steps) / spec.alpha_divisor) as f32;
        let delta = spec
            .control
            .target
            .filter(|_| spec.control.ticks != 0)
            .map_or(0., |target| {
                ((f64::from(target) - f64::from(spec.control.angle))
                    / f64::from(spec.control.ticks)) as f32
            });
        let state = OrbitAtlasState {
            origin: spec.control.origin,
            angle: spec.control.angle,
            target: spec.control.target.unwrap_or(spec.control.angle),
            delta,
            radius: [x as f32, y as f32],
            cell,
        };
        particle.position = self.position(&state);
        particle.previous_position = particle.position;
        particle.velocity = [0.; 3];
        particle.size = [
            (cell[2] - cell[0]) as f32 * scale as f32 / 100.,
            (cell[3] - cell[1]) as f32 * scale as f32 / 100.,
        ];
        particle.color[3] = alpha;
        particle.age = 0.;
        particle.lifetime = f32::MAX;
        Ok(state)
    }
    fn position(&self, state: &OrbitAtlasState) -> [f32; 3] {
        let angle = (f64::from(state.angle) * self.spec.radians_per_degree) as f32;
        let sine = f64::from(angle).sin() as f32;
        let cosine = f64::from(angle).cos() as f32;
        let x = (f64::from(sine) * f64::from(state.radius[0])) as f32 + state.origin[0];
        let y = (f64::from(cosine) * f64::from(state.radius[1])) as f32 + state.origin[1];
        [
            x + (state.cell[2] - state.cell[0]) as f32 * self.spec.position_anchor[0],
            y + (state.cell[3] - state.cell[1]) as f32 * self.spec.position_anchor[1],
            0.,
        ]
    }
    pub fn step(&self, particle: &mut Particle, state: &mut OrbitAtlasState) -> [f32; 4] {
        particle.previous_position = particle.position;
        if state.delta != 0. {
            state.angle += state.delta;
            let tolerance = if self.spec.signed_tolerance {
                state.delta
            } else {
                state.delta.abs()
            };
            if state.angle >= state.target - tolerance && state.angle <= state.target + tolerance {
                state.delta = 0.;
            }
            particle.position = self.position(state);
        }
        particle.age += 1. / self.spec.tick_hz as f32;
        self.uv(state)
    }
    pub fn uv(&self, state: &OrbitAtlasState) -> [f32; 4] {
        [
            state.cell[0] as f32 / self.spec.texture_extent[0] as f32,
            state.cell[1] as f32 / self.spec.texture_extent[1] as f32,
            state.cell[2] as f32 / self.spec.texture_extent[0] as f32,
            state.cell[3] as f32 / self.spec.texture_extent[1] as f32,
        ]
    }
}
