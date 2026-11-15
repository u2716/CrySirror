// SPDX-License-Identifier: MPL-2.0
//! Platform render tiers for the console ports.
//!
//! Pure policy over the shared 3D extraction ([`PbrScene`]). It never receives
//! VN layers, sprites, text, transitions, composite filters, puppets, movies
//! or audio, so those paths cannot be altered here: VN composition always runs
//! at the full output resolution with the shared algorithms on every tier.
//!
//! Each advanced 3D feature either stays native, is reduced, is substituted by
//! the closest technique the tier can execute, or is disabled with a reason.
//! Every non-native outcome is reported; nothing is dropped silently. The
//! desktop tier is the identity transform.
#![forbid(unsafe_code)]

pub mod derived;

use crysirror_console::Platform;
use crysirror_scene::components::{IrradianceVolumeSpec, LightShape};
use crysirror_scene::view::{Environment, PbrScene};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Desktop,
    SwitchDocked,
    SwitchHandheld,
    Vita,
}
impl Tier {
    pub const fn for_console(platform: Platform, docked: bool) -> Self {
        match platform {
            Platform::PsVita => Self::Vita,
            Platform::Switch if docked => Self::SwitchDocked,
            Platform::Switch => Self::SwitchHandheld,
        }
    }
    pub const fn is_switch(self) -> bool {
        matches!(self, Self::SwitchDocked | Self::SwitchHandheld)
    }
    /// Initial budgets. Console values are planning limits derived from the
    /// native APIs' capabilities, not measured frame costs.
    pub const fn budget(self) -> Budget {
        match self {
            Self::Desktop => Budget {
                output: None,
                scene_3d: None,
                compute: true,
                ray_queries: true,
                msaa: 4,
                cascades: 4,
                shadow_map: 2048,
                max_shadow_distance: f32::INFINITY,
                local_shadow_tiles: 64,
                max_local_lights: 4096,
                reflection_probes: 8,
                probe_width: 256,
                prefilter_width: 1024,
                prefilter_mips: 7,
                max_volume_probes: IrradianceVolumeSpec::MAX_PROBES,
                max_volume_updates: 256,
                volume_visibility: true,
                ssr_steps: 256,
                ssao_divisor: 1,
                bloom_levels: 6,
                particle_scale: (1, 1),
                max_particles: 200_000,
                gpu_particles: true,
                depth_texture: true,
                max_texture: 16384,
                max_joints: 256,
            },
            Self::SwitchDocked | Self::SwitchHandheld => Budget {
                output: Some(if matches!(self, Self::SwitchDocked) {
                    [1920, 1080]
                } else {
                    [1280, 720]
                }),
                scene_3d: Some(if matches!(self, Self::SwitchDocked) {
                    [1600, 900]
                } else {
                    [1152, 648]
                }),
                compute: true,
                ray_queries: false,
                msaa: 1,
                cascades: 3,
                shadow_map: 1024,
                max_shadow_distance: 80.,
                local_shadow_tiles: 24,
                max_local_lights: 256,
                reflection_probes: 4,
                probe_width: 128,
                prefilter_width: 512,
                prefilter_mips: 6,
                max_volume_probes: 512,
                max_volume_updates: 2,
                volume_visibility: true,
                ssr_steps: 24,
                ssao_divisor: 2,
                bloom_levels: 5,
                particle_scale: (1, 2),
                max_particles: 32_768,
                gpu_particles: true,
                depth_texture: true,
                max_texture: 4096,
                max_joints: 256,
            },
            Self::Vita => Budget {
                output: Some([960, 544]),
                scene_3d: Some([720, 408]),
                compute: false,
                ray_queries: false,
                msaa: 4,
                cascades: 2,
                shadow_map: 512,
                max_shadow_distance: 30.,
                local_shadow_tiles: 0,
                max_local_lights: 16,
                reflection_probes: 2,
                probe_width: 64,
                prefilter_width: 256,
                prefilter_mips: 5,
                max_volume_probes: 128,
                max_volume_updates: 1,
                volume_visibility: false,
                ssr_steps: 0,
                ssao_divisor: 0,
                bloom_levels: 3,
                particle_scale: (1, 4),
                max_particles: 2048,
                gpu_particles: false,
                depth_texture: false,
                max_texture: 1024,
                max_joints: 64,
            },
        }
    }
}

/// Executor limits for one tier. `None` resolutions mean the host's output.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Budget {
    /// VN/UI composition resolution; never reduced below the output.
    pub output: Option<[u32; 2]>,
    /// Offscreen 3D scene target, upscaled into the VN composition.
    pub scene_3d: Option<[u32; 2]>,
    pub compute: bool,
    pub ray_queries: bool,
    pub msaa: u32,
    pub cascades: u32,
    pub shadow_map: u32,
    pub max_shadow_distance: f32,
    /// 512² local-light shadow tiles (a point light uses six).
    pub local_shadow_tiles: u32,
    pub max_local_lights: u32,
    pub reflection_probes: u32,
    pub probe_width: u32,
    pub prefilter_width: u32,
    pub prefilter_mips: u32,
    pub max_volume_probes: u32,
    pub max_volume_updates: u32,
    pub volume_visibility: bool,
    /// 0: no screen-space reflection pass.
    pub ssr_steps: u32,
    /// 0: no SSAO pass; otherwise the resolution divisor.
    pub ssao_divisor: u32,
    pub bloom_levels: u32,
    pub particle_scale: (u32, u32),
    pub max_particles: u32,
    pub gpu_particles: bool,
    /// Opaque depth readable by later passes (soft particles, depth fade).
    pub depth_texture: bool,
    pub max_texture: u32,
    pub max_joints: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    SceneResolution,
    RayTracedGi,
    RayTracedReflections,
    ScreenSpaceReflections,
    VirtualShadows,
    CascadedShadows,
    GeometryLod,
    TemporalAa,
    AmbientOcclusion,
    Bloom,
    AutoExposure,
    HeightFog,
    Environment,
    IrradianceVolume,
    ReflectionProbes,
    LocalLights,
    LocalShadows,
    Particles,
    GpuVfx,
    SoftParticles,
}

/// The technique a tier executes instead of an unavailable feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Technique {
    /// Raster six-face irradiance volume capture.
    IrradianceVolumeRaster,
    /// SH9 environment irradiance (the shared fallback term).
    EnvironmentSh9,
    ScreenSpaceReflections,
    /// Prefiltered environment plus box-projected reflection probes.
    ReflectionProbes,
    CascadedShadows,
    /// Native multisampling instead of temporal reconstruction.
    Msaa,
    /// Material occlusion textures only.
    MaterialOcclusion,
    /// Fragment log-luminance mip reduction instead of a compute metering pass.
    FragmentLuminanceReduction,
    /// Deterministic CPU particle simulation.
    CpuParticles,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum Outcome {
    Native,
    Reduced { detail: String },
    Substituted { by: Technique, detail: String },
    Disabled { reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Decision {
    pub feature: Feature,
    #[serde(flatten)]
    pub outcome: Outcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reduced {
    pub scene: PbrScene,
    pub budget: Budget,
    /// Sorted by feature; one entry per feature present in the input.
    pub decisions: Vec<Decision>,
}
impl Reduced {
    pub fn decision(&self, feature: Feature) -> Option<&Outcome> {
        self.decisions
            .iter()
            .find(|d| d.feature == feature)
            .map(|d| &d.outcome)
    }
}

struct Log(Vec<Decision>);
impl Log {
    fn push(&mut self, feature: Feature, outcome: Outcome) {
        self.0.push(Decision { feature, outcome });
    }
    fn native(&mut self, feature: Feature) {
        self.push(feature, Outcome::Native);
    }
    fn reduced(&mut self, feature: Feature, detail: impl Into<String>) {
        self.push(feature, Outcome::Reduced { detail: detail.into() });
    }
    fn substituted(&mut self, feature: Feature, by: Technique, detail: impl Into<String>) {
        self.push(feature, Outcome::Substituted { by, detail: detail.into() });
    }
    fn disabled(&mut self, feature: Feature, reason: impl Into<String>) {
        self.push(feature, Outcome::Disabled { reason: reason.into() });
    }
}

/// Reduce one extracted 3D frame for `tier`. Deterministic for equal input.
/// The result's environment still passes [`Environment::validate`] when the
/// input did.
pub fn reduce(scene: &PbrScene, tier: Tier) -> Reduced {
    let budget = tier.budget();
    let mut out = scene.clone();
    let mut log = Log(Vec::new());
    if tier == Tier::Desktop {
        record_desktop(scene, &mut log);
    } else {
        if let Some(s) = budget.scene_3d {
            log.reduced(
                Feature::SceneResolution,
                format!("3D target {}x{}, VN composition at full output", s[0], s[1]),
            );
        }
        reduce_gi(&mut out, &budget, &mut log);
        reduce_environment(&mut out.environment, &budget, &mut log);
        reduce_probes(&mut out, &budget, &mut log);
        reduce_lights(&mut out, &budget, &mut log);
        reduce_particles(&mut out, &budget, &mut log);
    }
    log.0.sort_by_key(|d| d.feature);
    Reduced {
        scene: out,
        budget,
        decisions: log.0,
    }
}

fn record_desktop(s: &PbrScene, log: &mut Log) {
    let e = &s.environment;
    let present = [
        (Feature::RayTracedGi, e.ray_traced_gi.is_some()),
        (
            Feature::RayTracedReflections,
            e.reflections.as_ref().is_some_and(|r| r.ray_traced),
        ),
        (Feature::ScreenSpaceReflections, e.reflections.is_some()),
        (Feature::VirtualShadows, e.virtual_shadows.is_some()),
        (
            Feature::CascadedShadows,
            e.sun.as_ref().is_some_and(|s| s.shadows) && e.virtual_shadows.is_none(),
        ),
        (Feature::GeometryLod, e.geometry_lod.is_some()),
        (Feature::TemporalAa, e.temporal_aa),
        (Feature::AmbientOcclusion, e.ambient_occlusion > 0.),
        (Feature::Bloom, e.bloom > 0.),
        (Feature::AutoExposure, e.auto_exposure.is_some()),
        (Feature::HeightFog, e.fog.is_some()),
        (Feature::IrradianceVolume, !s.volumes.is_empty()),
        (Feature::ReflectionProbes, !s.probes.is_empty()),
        (Feature::LocalLights, !s.lights.is_empty()),
        (Feature::LocalShadows, s.lights.iter().any(|l| l.shadows)),
        (Feature::Particles, !s.emitters.is_empty()),
        (
            Feature::GpuVfx,
            s.vfx.as_ref().is_some_and(|v| !v.gpu_emitters.is_empty()),
        ),
    ];
    for (feature, on) in present {
        if on {
            log.native(feature);
        }
    }
}

fn reduce_gi(s: &mut PbrScene, b: &Budget, log: &mut Log) {
    let rt = s.environment.ray_traced_gi.take();
    for v in &mut s.volumes {
        v.spec.ray_traced &= b.ray_queries;
    }
    if let Some(gi) = &rt {
        if s.volumes.is_empty() {
            log.substituted(
                Feature::RayTracedGi,
                Technique::EnvironmentSh9,
                "no irradiance volume authored; the shared SH9 fallback carries indirect diffuse",
            );
        } else {
            // Per-pixel RT irradiance replaces the volume term scaled by
            // its intensity; the raster volume carries the same quantity.
            for v in &mut s.volumes {
                v.spec.intensity = (v.spec.intensity * gi.intensity).min(16.);
            }
            log.substituted(
                Feature::RayTracedGi,
                Technique::IrradianceVolumeRaster,
                format!("volume irradiance scaled by the RT intensity {}", gi.intensity),
            );
        }
    }
    if s.volumes.is_empty() {
        return;
    }
    let mut notes = Vec::new();
    for v in &mut s.volumes {
        let spec = &mut v.spec;
        let before = spec.counts;
        while spec.counts.iter().product::<u32>() > b.max_volume_probes {
            let axis = (0..3).max_by_key(|&i| (spec.counts[i], 2 - i)).unwrap();
            if spec.counts[axis] <= 2 {
                break;
            }
            spec.counts[axis] -= 1;
        }
        if spec.counts != before {
            notes.push(format!("probe grid {before:?} -> {:?}", spec.counts));
            spec.revision = spec.revision.wrapping_add(1);
        }
        let rate = spec.probes_per_frame.min(b.max_volume_updates).max(1);
        if rate != spec.probes_per_frame {
            notes.push(format!("{rate} probe update(s) per frame"));
            spec.probes_per_frame = rate;
        }
        spec.idle_probes_per_frame = spec.idle_probes_per_frame.min(spec.probes_per_frame);
        if spec.visibility && !b.volume_visibility {
            spec.visibility = false;
            notes.push("no Chebyshev visibility".into());
        }
    }
    if !b.compute {
        notes.push("sampled per draw at the object bounds centre".into());
    }
    if notes.is_empty() {
        log.native(Feature::IrradianceVolume);
    } else {
        notes.dedup();
        log.reduced(Feature::IrradianceVolume, notes.join("; "));
    }
}

fn reduce_environment(e: &mut Environment, b: &Budget, log: &mut Log) {
    if let Some(r) = e.reflections.take() {
        if b.ssr_steps == 0 {
            if r.ray_traced {
                log.substituted(
                    Feature::RayTracedReflections,
                    Technique::ReflectionProbes,
                    "no ray queries or screen-space pass",
                );
            }
            log.substituted(
                Feature::ScreenSpaceReflections,
                Technique::ReflectionProbes,
                "prefiltered environment and box-projected probes",
            );
        } else {
            let mut r = r;
            if r.ray_traced {
                r.ray_traced = false;
                r.ray_samples = 1;
                log.substituted(
                    Feature::RayTracedReflections,
                    Technique::ScreenSpaceReflections,
                    "glossy rays replaced by the depth march",
                );
            }
            if r.steps > b.ssr_steps {
                // Longer strides skip thin occluders; widen the depth
                // tolerance to one stride so hits are not lost.
                let stride = r.max_distance / b.ssr_steps as f32;
                r.thickness = r.thickness.max(stride).min(100.);
                log.reduced(
                    Feature::ScreenSpaceReflections,
                    format!("{} -> {} steps, thickness {}", r.steps, b.ssr_steps, r.thickness),
                );
                r.steps = b.ssr_steps;
            } else {
                log.native(Feature::ScreenSpaceReflections);
            }
            e.reflections = Some(r);
        }
    }
    let sun_shadows = e.sun.as_ref().is_some_and(|s| s.shadows);
    if e.virtual_shadows.take().is_some() {
        log.substituted(
            Feature::VirtualShadows,
            Technique::CascadedShadows,
            format!("{} cascade(s) of {}^2", b.cascades, b.shadow_map),
        );
    }
    if sun_shadows {
        let d = e.shadow_distance.min(b.max_shadow_distance);
        log.reduced(
            Feature::CascadedShadows,
            format!("{} x {}^2 over {d} m", b.cascades, b.shadow_map),
        );
        e.shadow_distance = d;
    }
    if let Some(l) = &mut e.geometry_lod {
        if l.gpu && !b.compute {
            l.gpu = false;
            l.pixel_error = (l.pixel_error * 2.).clamp(2., 1000.);
            log.reduced(
                Feature::GeometryLod,
                format!("CPU cluster selection, {} px error", l.pixel_error),
            );
        } else {
            log.native(Feature::GeometryLod);
        }
    }
    if e.temporal_aa {
        if b.compute {
            log.native(Feature::TemporalAa);
        } else {
            e.temporal_aa = false;
            log.substituted(
                Feature::TemporalAa,
                Technique::Msaa,
                format!("{}x MSAA", b.msaa),
            );
        }
    }
    if e.ambient_occlusion > 0. {
        if b.ssao_divisor == 0 {
            e.ambient_occlusion = 0.;
            log.substituted(
                Feature::AmbientOcclusion,
                Technique::MaterialOcclusion,
                "no depth prepass on this tier",
            );
        } else {
            log.reduced(
                Feature::AmbientOcclusion,
                format!("1/{} resolution", b.ssao_divisor),
            );
        }
    }
    if e.bloom > 0. {
        log.reduced(Feature::Bloom, format!("{} levels", b.bloom_levels));
    }
    if e.auto_exposure.is_some() {
        if b.compute {
            log.native(Feature::AutoExposure);
        } else {
            log.substituted(
                Feature::AutoExposure,
                Technique::FragmentLuminanceReduction,
                "same center-weighted log2 metering and adaptation",
            );
        }
    }
    if e.fog.is_some() {
        log.native(Feature::HeightFog);
    }
    if e.hdr.is_some() {
        log.reduced(
            Feature::Environment,
            format!("prefiltered {} wide, {} mips", b.prefilter_width, b.prefilter_mips),
        );
    }
}

fn camera_distance2(s: &PbrScene, p: [f32; 3]) -> f32 {
    let e = s.camera.eye;
    (0..3).map(|i| (p[i] - e[i]) * (p[i] - e[i])).sum()
}

fn reduce_probes(s: &mut PbrScene, b: &Budget, log: &mut Log) {
    if s.probes.is_empty() {
        return;
    }
    let before = s.probes.len();
    let mut order: Vec<_> = s
        .probes
        .iter()
        .map(|p| (camera_distance2(s, p.position), p.key))
        .collect();
    order.sort_by(|a, c| a.0.total_cmp(&c.0).then(a.1.cmp(&c.1)));
    let keep: Vec<u64> = order
        .iter()
        .take(b.reflection_probes as usize)
        .map(|k| k.1)
        .collect();
    s.probes.retain(|p| keep.contains(&p.key));
    log.reduced(
        Feature::ReflectionProbes,
        format!(
            "{} of {before} nearest probe(s), {} px captures",
            s.probes.len(),
            b.probe_width
        ),
    );
}

/// Camera-relative importance: lights whose range sphere contains or nears
/// the camera come first, ties broken by key for stability.
fn light_order(s: &PbrScene) -> Vec<(f32, u64)> {
    let mut v: Vec<_> = s
        .lights
        .iter()
        .map(|l| (camera_distance2(s, l.position).sqrt() - l.range, l.key))
        .collect();
    v.sort_by(|a, c| a.0.total_cmp(&c.0).then(a.1.cmp(&c.1)));
    v
}

fn reduce_lights(s: &mut PbrScene, b: &Budget, log: &mut Log) {
    if s.lights.is_empty() {
        return;
    }
    let order = light_order(s);
    let before = s.lights.len();
    if before > b.max_local_lights as usize {
        let keep: Vec<u64> = order
            .iter()
            .take(b.max_local_lights as usize)
            .map(|k| k.1)
            .collect();
        s.lights.retain(|l| keep.contains(&l.key));
        log.reduced(
            Feature::LocalLights,
            format!("{} of {before} most camera-relevant lights", s.lights.len()),
        );
    } else if b.compute {
        log.native(Feature::LocalLights);
    } else {
        log.reduced(Feature::LocalLights, "forward, four per draw selected on the CPU");
    }
    let shadowed = s.lights.iter().filter(|l| l.shadows).count();
    if shadowed > 0 {
        let mut tiles = b.local_shadow_tiles;
        let mut kept = 0;
        for (_, key) in &order {
            let Some(l) = s.lights.iter_mut().find(|l| l.key == *key && l.shadows) else {
                continue;
            };
            let need = match l.shape {
                LightShape::Point => 6,
                LightShape::Spot { .. } | LightShape::Rect { .. } => 1,
                LightShape::Directional | LightShape::Ambient => 0,
            };
            if need <= tiles {
                tiles -= need;
                kept += 1;
            } else {
                l.shadows = false;
            }
        }
        if kept == shadowed {
            log.native(Feature::LocalShadows);
        } else if kept == 0 {
            log.disabled(
                Feature::LocalShadows,
                format!("{} shadow tiles; sun shadows only", b.local_shadow_tiles),
            );
        } else {
            log.reduced(
                Feature::LocalShadows,
                format!("{kept} of {shadowed} shadowed lights within {} tiles", b.local_shadow_tiles),
            );
        }
    }
}

fn reduce_particles(s: &mut PbrScene, b: &Budget, log: &mut Log) {
    if s.vfx.as_ref().is_some_and(|v| !v.gpu_emitters.is_empty()) {
        if b.gpu_particles {
            log.native(Feature::GpuVfx);
        } else {
            // Simulation (and its events) is engine state; only execution moves.
            log.substituted(
                Feature::GpuVfx,
                Technique::CpuParticles,
                "GPU emitters run on the deterministic CPU simulation",
            );
        }
    }
    if s.vfx.is_some() && !b.depth_texture {
        log.disabled(
            Feature::SoftParticles,
            "no opaque depth texture; VFX keep flipbook, radial coverage and fog",
        );
    }
    if s.emitters.is_empty() {
        return;
    }
    let (num, den) = b.particle_scale;
    let scale = |v: u32| ((v as u64 * num as u64 / den as u64) as u32).max(1);
    let mut total: u64 = 0;
    for e in &mut s.emitters {
        e.spec.max = scale(e.spec.max);
        e.spec.burst = if e.spec.burst == 0 { 0 } else { scale(e.spec.burst) };
        e.spec.rate *= num as f32 / den as f32;
        total += e.spec.max as u64;
    }
    if total > b.max_particles as u64 {
        let k = b.max_particles as f64 / total as f64;
        for e in &mut s.emitters {
            let f = k as f32;
            e.spec.max = ((e.spec.max as f64 * k) as u32).max(1);
            e.spec.burst = ((e.spec.burst as f64 * k) as u32).min(e.spec.max);
            e.spec.rate *= f;
        }
    }
    if (num, den) == (1, 1) && total <= b.max_particles as u64 {
        log.native(Feature::Particles);
    } else {
        log.reduced(
            Feature::Particles,
            format!(
                "emission x{num}/{den}, {} particles max{}",
                b.max_particles,
                if b.gpu_particles { "" } else { ", CPU simulated" }
            ),
        );
    }
}

#[cfg(test)]
mod tests;
