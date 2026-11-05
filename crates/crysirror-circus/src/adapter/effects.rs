//! Native effect declarations and sequence leases. Particle pools, integration,
//! atlas animation and mesh publication belong to the shared VFX worker.
use super::*;
use crysirror_vfx::{
    BlendMode, Burst, EmitterSpec, FacingMode, RenderSpec, SimulationSpace, SystemSpec,
    falling_atlas::{FallingAtlasSpec, SequenceExchange, SequenceRng, SimulationClock},
    screen::ScreenSpriteTarget,
};

const EFFECT_ENTITY: u32 = 0x3900_0000;
const EFFECT_LAYER: u32 = 0x3900_0010;
pub(super) struct NativeEffect {
    pub(super) spec: SystemSpec,
    kind: i16,
    parameter: i16,
}
impl Bridge<'_> {
    fn world(&mut self, commands: Vec<Value>) -> Result<()> {
        self.host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"world":commands}))?,
        )?;
        Ok(())
    }
    pub(super) fn effects_ready(&mut self) -> Result<bool> {
        if !self.framebuffer_ready()? {
            return Ok(false);
        }
        let Some(effect) = &self.services.effects else {
            return Ok(true);
        };
        let SimulationClock::External { tick, .. } = effect.spec.clock else {
            bail!("Circus native effect clock")
        };
        let epoch = effect
            .spec
            .sequence_exchange
            .as_ref()
            .map_or(0, |e| e.epoch);
        let bytes = self.host.query_engine(
            self.client,
            EngineQuery::VfxSequence {
                entity: EFFECT_ENTITY,
                revision: effect.spec.revision,
                completed_tick: tick,
                exchange_epoch: epoch,
            },
        )?;
        let response: Value = serde_json::from_slice(&bytes)?;
        if response["phase"] == "pending" {
            return Ok(false);
        }
        ensure!(response["phase"] == "ready", "Circus VFX completion status");
        self.services.sequence_rng =
            Some(serde_json::from_value(response["sequence_rng"].clone())?);
        Ok(true)
    }
    pub(super) fn advance_effects(&mut self, now: u64) -> Result<()> {
        let Some(effect) = &mut self.services.effects else {
            return Ok(());
        };
        let SimulationClock::External { tick, wall_time_ms } = &mut effect.spec.clock else {
            bail!("Circus native effect clock")
        };
        *tick = tick
            .checked_add(1)
            .context("Circus native effect tick overflow")?;
        *wall_time_ms = now;
        let spec = serde_json::to_value(&effect.spec)?;
        self.world(vec![json!({"op":"set_vfx","id":EFFECT_ENTITY,"vfx":spec})])
    }
    pub(super) fn falling_effect(&mut self, kind: i16, parameter: i16) -> Result<bool> {
        if matches!(kind, 7 | 16) {
            return self.falling_scene_special(kind, parameter);
        }
        if !self.effects_ready()? {
            return Ok(false);
        }
        //40EEC0 selects the pool wrapper even for the empty kind0 profile;
        //density and texture presence do not select the primary sprite back.
        self.mark_particle_wrappers(if kind==15 {&[59]}else{&[6,19,29,39,59]})?;
        if kind == 0 {
            if self.services.effect_stopping {
                self.services.effect_stopping = false;
                self.services.effect_control.clear_pool_controls();
                return Ok(true);
            }
            if let Some(previous) = &self.services.effects {
                if previous.kind == 0 {
                    self.services.effect_control.clear_pool_controls();
                    return Ok(true);
                }
                //41DAC0(0) clears descriptors but keeps the twenty native
                //sprite objects per pool. In particular rain inherits their
                //scale after a later restart; despawning loses that state.
                let mut spec = previous.spec.clone();
                for emitter in &mut spec.emitters {
                    emitter.revision = emitter
                        .revision
                        .checked_add(1)
                        .context("Circus cleared pool revision")?;
                    emitter.spawn.bursts.clear();
                    emitter.spawn.rate = 0.0.into();
                    emitter.screen = None;
                    emitter.falling_atlas = None;
                    emitter.orbit_atlas = None;
                    emitter.drifting_atlas = None;
                    if let RenderSpec::Sprite { texture, .. } = &mut emitter.render {
                        *texture = None;
                    }
                }
                let SimulationClock::External { tick, .. } = spec.clock else {
                    bail!("Circus cleared pool clock");
                };
                let epoch = spec.sequence_exchange.as_ref().map_or(Ok(1), |e| {
                    e.epoch.checked_add(1).context("Circus cleared pool epoch")
                })?;
                spec.sequence_exchange = Some(SequenceExchange {
                    epoch,
                    base_tick: tick,
                    state: self
                        .services
                        .sequence_rng
                        .clone()
                        .context("Circus cleared pool sequence")?,
                });
                self.world(vec![json!({"op":"set_vfx","id":EFFECT_ENTITY,"vfx":spec})])?;
                self.services.effects = Some(NativeEffect {
                    spec,
                    kind: 0,
                    parameter: 0,
                });
                self.services.effect_stopping = true;
                return Ok(false);
            }
            self.services.effect_control.clear_pool_controls();
            return Ok(true);
        }
        if self.services.effect_starting {
            let effect = self
                .services
                .effects
                .as_ref()
                .context("Circus pending effect declaration")?;
            ensure!(
                effect.kind == kind && effect.parameter == parameter,
                "Circus effect completion declaration changed"
            );
            self.services.effect_starting = false;
            return Ok(true);
        }
        // These three profiles bind their fixed image before constructing any
        // pool (4284B0 case40); ordinary/rain pools use the existing resource.
        let built_in = match kind {
            13 => Some((28, "system\\effyousei")),
            15 => Some((30, "system\\efflens")),
            17 => Some((28, "system\\effhotaru")),
            _ => None,
        };
        //425640 reconstructs pools from the already restored resource handles;
        //in particular a saved hotaru texture must not turn into effyousei just
        //because that reconstruction invokes the common kind13 initializer.
        if let Some((slot, name)) = built_in.filter(|_| self.services.saves.restore.is_none()) {
            if !self.emit(&Event::LoadImage {
                slot,
                name: name.into(),
            })? {
                return Ok(false);
            }
        }
        let image = self.services.images.get(&if kind == 15 {
            30
        } else if kind == 2 {
            29
        } else {
            28
        });
        let rng = self
            .services
            .sequence_rng
            .clone()
            .context("Circus process random sequence missing")?;
        let mut spec = if (parameter % 10).clamp(0, 6) == 0 {
            //41DAC0 stores the passed handle, clears its twenty descriptors,
            //then exits before any texture bind/dimension read at density0.
            //Keep native pool objects (and retained scale), consume no RNG,
            //and permit the original null texture on this empty branch.
            let mut empty = empty_pools("", rng.clone(), self.host.now_ms());
            for emitter in &mut empty.emitters {
                emitter.screen = None;
                if let RenderSpec::Sprite { texture, .. } = &mut emitter.render {
                    *texture = None;
                }
            }
            if kind == 15 {
                let mut spec = self
                    .services
                    .effects
                    .as_ref()
                    .map(|effect| effect.spec.clone())
                    .unwrap_or_else(|| empty.clone());
                spec.emitters[0] = empty.emitters.remove(0);
                spec
            } else {
                empty
            }
        } else if kind == 15 {
            let image = image.context("Circus particle texture missing")?;
            let emitter = native_orbit_emitter(
                parameter,
                image.path.relative(),
                image.size,
                self.services.effect_control.orbit_control(),
            )?;
            let mut spec = if let Some(effect) = &self.services.effects {
                effect.spec.clone()
            } else {
                empty_pools(image.path.relative(), rng.clone(), self.host.now_ms())
            };
            spec.emitters[0] = emitter;
            spec
        } else if matches!(kind, 2 | 3 | 13 | 14 | 17) {
            let image = image.context("Circus particle texture missing")?;
            drifting_spec(
                kind,
                parameter,
                image.path.relative(),
                image.size,
                rng.clone(),
                self.host.now_ms(),
            )?
        } else {
            let image = image.context("Circus particle texture missing")?;
            native_spec(
                parameter,
                image.path.relative(),
                image.size,
                rng.clone(),
                self.host.now_ms(),
            )?
        };
        for emitter in &mut spec.emitters {
            if let Some(node) = &mut emitter.falling_atlas {
                node.spawn_window = self.services.effect_control.spawn_window();
            }
            if let Some(node) = &mut emitter.drifting_atlas {
                node.spawn_window = self.services.effect_control.drift_window();
            }
            let bypass = emitter.drifting_atlas.as_ref().is_some_and(|node| {
                matches!(
                    node.motion,
                    crysirror_vfx::drifting_atlas::DriftMotion::Buoyant
                        | crysirror_vfx::drifting_atlas::DriftMotion::FallingGlow
                )
            });
            emitter.paused = !self.services.effect_control.effects_enabled() && !bypass;
        }
        let replacing = self.services.effects.is_some();
        if let Some(previous) = &self.services.effects {
            ensure!(
                previous.spec.emitters.len() == 5 && spec.emitters.len() == 5,
                "Circus native particle pool count"
            );
            for (index, emitter) in spec.emitters.iter_mut().enumerate() {
                if kind != 15 || index == 0 {
                    emitter.revision = previous.spec.emitters[index]
                        .revision
                        .checked_add(1)
                        .context("Circus particle pool revision overflow")?;
                }
            }
            let SimulationClock::External { tick, .. } = previous.spec.clock else {
                bail!("Circus particle pool clock")
            };
            let epoch = previous
                .spec
                .sequence_exchange
                .as_ref()
                .map_or(Ok(1), |exchange| {
                    exchange
                        .epoch
                        .checked_add(1)
                        .context("Circus particle pool exchange overflow")
                })?;
            spec.clock = previous.spec.clock.clone();
            spec.revision = previous.spec.revision;
            spec.sequence_rng = previous.spec.sequence_rng.clone();
            spec.sequence_exchange = Some(SequenceExchange {
                epoch,
                base_tick: tick,
                state: rng,
            });
        }
        spec.compile()?;
        self.root()?;
        if replacing {
            self.world(vec![json!({"op":"set_vfx","id":EFFECT_ENTITY,"vfx":spec})])?;
        } else {
            self.world(vec![json!({"op":"spawn","entity":{"id":EFFECT_ENTITY,"name":"Circus native particle pools","vfx":spec}})])?;
        }
        self.services.effects = Some(NativeEffect {
            spec,
            kind,
            parameter,
        });
        self.services.effect_starting = true;
        Ok(false)
    }
    pub(super) fn native_random(&mut self, modulus: u16) -> Result<Option<i32>> {
        ensure!(modulus > 0, "Circus native random modulo domain");
        if !self.effects_ready()? {
            return Ok(None);
        }
        if let Some((pending, value)) = self.services.random_exchange.take() {
            ensure!(
                pending == modulus,
                "Circus random completion instruction changed"
            );
            return Ok(Some(value));
        }
        let rng = self
            .services
            .sequence_rng
            .as_mut()
            .context("Circus process random sequence missing")?;
        let value = (rng.next_u32()? % u32::from(modulus)) as i32;
        if let Some(effect) = &mut self.services.effects {
            let SimulationClock::External { tick, .. } = effect.spec.clock else {
                bail!("Circus effect random clock")
            };
            let epoch = effect.spec.sequence_exchange.as_ref().map_or(Ok(1), |e| {
                e.epoch
                    .checked_add(1)
                    .context("Circus random epoch overflow")
            })?;
            effect.spec.sequence_exchange = Some(SequenceExchange {
                epoch,
                base_tick: tick,
                state: rng.clone(),
            });
            let spec = serde_json::to_value(&effect.spec)?;
            self.world(vec![json!({"op":"set_vfx","id":EFFECT_ENTITY,"vfx":spec})])?;
            self.services.random_exchange = Some((modulus, value));
            return Ok(None);
        }
        Ok(Some(value))
    }
}
fn native_spec(
    parameter: i16,
    texture: &str,
    extent: [u32; 2],
    rng: SequenceRng,
    now: u64,
) -> Result<SystemSpec> {
    // Native42A982 splits the two decimal density/wind components.41DAC0
    // clamps density to0..6 and constructs the original five pools.
    let tens = i32::from(parameter) / 10;
    let wind = if parameter < 100 {
        tens
    } else {
        -(i32::from(parameter) % 100 / 10)
    };
    let density = (i32::from(parameter) - 10 * tens).clamp(0, 6) as usize;
    let count = [0, 1, 3, 6, 9, 12, 20][density];
    let emitters = (0..=4)
        .rev()
        .map(|depth| {
            let mut emitter = EmitterSpec::default();
            emitter.id = format!("depth-{depth}");
            emitter.capacity = 20;
            emitter.space = SimulationSpace::Local;
            emitter.spawn.rate = 0.0.into();
            emitter.spawn.bursts = if count == 0 {
                vec![]
            } else {
                vec![Burst { time: 0., count }]
            };
            emitter.spawn.burst_looping = false;
            emitter.screen = Some(ScreenSpriteTarget {
                canvas: [1280, 720],
                layer: EFFECT_LAYER + depth,
                parent: Some(SCENE_ROOT),
                order: pool_order(depth),
                anchor: [0., 0.],
            });
            emitter.falling_atlas = Some(FallingAtlasSpec {
                canvas: [1280, 720],
                tick_hz: 60,
                x_bands: vec![0, -1, 1],
                fall_range: [2, 11],
                initial_fall_range: if density == 6 { Some([2, 5]) } else { None },
                spawn_window: None,
                fall_divisor: 1.5,
                sway_range: [28, 59],
                phase_modulus: 360,
                phase_step: 0.05,
                wind_step: wind as f32,
                rows: vec![depth, depth + 1],
                row_frames: vec![7, 7, 6, 8, 5, 8],
                cell: [28, 28],
                stride: [32, 28],
                texture_extent: extent,
                scale: 1. - 0.1 * depth as f32,
                alpha_base: 0.5,
                alpha_step: 0.1,
                alpha_count: 3,
                alpha_offset: -0.02 * depth as f32,
                atlas_interval_ms: 300,
                atlas_clock_bits: 32,
            });
            emitter.render = RenderSpec::Sprite {
                texture: Some(texture.into()),
                columns: 1,
                rows: 1,
                frames: 1,
                fps: 0.,
                soft_distance: 0.,
                blend: BlendMode::Alpha,
                facing: FacingMode::Camera,
            };
            emitter
        })
        .collect();
    let spec = SystemSpec {
        emitters,
        clock: SimulationClock::External {
            tick: 0,
            wall_time_ms: now,
        },
        sequence_rng: Some(rng),
        ..Default::default()
    };
    spec.compile()?;
    Ok(spec)
}
fn pool_order(depth: u32) -> [i32; 2] {
    //41F300 binds depth4/3/2/1/0 to wrappers6/19/29/39/59. Use the same
    //painter partitions as the ordinary native object renderer.
    let object = [59, 39, 29, 19, 6][depth as usize];
    if object < 10 {
        [0, object]
    } else {
        [1, 50 * 64 + object]
    }
}
fn drifting_spec(
    kind: i16,
    parameter: i16,
    texture: &str,
    extent: [u32; 2],
    rng: SequenceRng,
    now: u64,
) -> Result<SystemSpec> {
    let mut spec = empty_pools(texture, rng, now);
    let count = [0, 1, 3, 6, 9, 12, 20][(parameter % 10).clamp(0, 6) as usize];
    for (index, emitter) in spec.emitters.iter_mut().enumerate() {
        let depth = 4 - index as u32;
        emitter.enabled = true;
        emitter.capacity = 20;
        emitter.spawn.bursts = if count == 0 {
            vec![]
        } else {
            vec![Burst { time: 0., count }]
        };
        emitter.screen = Some(ScreenSpriteTarget {
            canvas: [1280, 720],
            layer: EFFECT_LAYER + depth,
            parent: Some(SCENE_ROOT),
            order: pool_order(depth),
            anchor: [0., 0.],
        });
        emitter.drifting_atlas = Some(crysirror_vfx::drifting_atlas::DriftingAtlasSpec {
            canvas: [1280, 720],
            texture_extent: extent,
            tick_hz: 60,
            depth,
            motion: match kind {
                2 => crysirror_vfx::drifting_atlas::DriftMotion::Streak,
                3 => crysirror_vfx::drifting_atlas::DriftMotion::FallingGlow,
                14 => crysirror_vfx::drifting_atlas::DriftMotion::RisingGlow,
                _ => crysirror_vfx::drifting_atlas::DriftMotion::Buoyant,
            },
            spawn_window: None,
        });
        emitter.render = RenderSpec::Sprite {
            texture: Some(texture.into()),
            columns: 1,
            rows: 1,
            frames: 1,
            fps: 0.,
            soft_distance: 0.,
            blend: BlendMode::Alpha,
            facing: FacingMode::Camera,
        };
    }
    spec.compile()?;
    Ok(spec)
}
fn empty_pools(texture: &str, rng: SequenceRng, now: u64) -> SystemSpec {
    let mut emitters = Vec::new();
    for depth in (0..=4).rev() {
        let mut emitter = EmitterSpec::default();
        emitter.id = format!("depth-{depth}");
        emitter.enabled = false;
        emitter.capacity = 20;
        emitter.space = SimulationSpace::Local;
        emitter.spawn.rate = 0.0.into();
        emitter.screen = Some(ScreenSpriteTarget {
            canvas: [1280, 720],
            layer: EFFECT_LAYER + depth,
            parent: Some(SCENE_ROOT),
            order: pool_order(depth),
            anchor: [0., 0.],
        });
        emitter.render = RenderSpec::Sprite {
            texture: Some(texture.into()),
            columns: 1,
            rows: 1,
            frames: 1,
            fps: 0.,
            soft_distance: 0.,
            blend: BlendMode::Alpha,
            facing: FacingMode::Camera,
        };
        emitters.push(emitter);
    }
    SystemSpec {
        emitters,
        clock: SimulationClock::External {
            tick: 0,
            wall_time_ms: now,
        },
        sequence_rng: Some(rng),
        ..Default::default()
    }
}
fn native_orbit_emitter(
    parameter: i16,
    texture: &str,
    extent: [u32; 2],
    control: crysirror_vfx::orbit_atlas::OrbitControl,
) -> Result<EmitterSpec> {
    let density = (parameter % 10).clamp(0, 6) as usize;
    let count = [0, 1, 3, 6, 9, 12, 20][density];
    let mut emitter = EmitterSpec::default();
    emitter.id = "depth-4".into();
    emitter.capacity = 20;
    emitter.space = SimulationSpace::Local;
    emitter.spawn.rate = 0.0.into();
    emitter.spawn.bursts = if count == 0 {
        vec![]
    } else {
        vec![Burst { time: 0., count }]
    };
    emitter.spawn.burst_looping = false;
    emitter.screen = Some(ScreenSpriteTarget {
        canvas: [1280, 720],
        layer: EFFECT_LAYER + 4,
        parent: Some(SCENE_ROOT),
        order: pool_order(4),
        anchor: [0.5, 0.5],
    });
    emitter.orbit_atlas = Some(crysirror_vfx::orbit_atlas::OrbitAtlasSpec {
        tick_hz: 60,
        texture_extent: extent,
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
        control,
    });
    emitter.render = RenderSpec::Sprite {
        texture: Some(texture.into()),
        columns: 1,
        rows: 1,
        frames: 1,
        fps: 0.,
        soft_distance: 0.,
        blend: BlendMode::Alpha,
        facing: FacingMode::Camera,
    };
    Ok(emitter)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_five_depth_declaration_is_one_shared_system_in_original_order() {
        let s = native_spec(5, "authored.png", [256, 256], SequenceRng::msvc(7), 42).unwrap();
        assert_eq!(s.emitters.len(), 5);
        assert_eq!(
            s.emitters
                .iter()
                .map(|e| e.spawn.bursts[0].count)
                .sum::<u32>(),
            60
        );
        assert_eq!(s.emitters[0].id, "depth-4");
        assert_eq!(s.emitters[4].id, "depth-0");
        assert_eq!(s.emitters[0].falling_atlas.as_ref().unwrap().rows, [4, 5]);
        assert_eq!(s.sequence_rng, Some(SequenceRng::msvc(7)));
        assert!(native_spec(5, "authored.png", [128, 128], SequenceRng::msvc(7), 42).is_err());
    }
}
