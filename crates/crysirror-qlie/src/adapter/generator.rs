//! Native generator grammar/target policy. Signal algorithms belong to the
//! shared animation crate; Layer external producers own pose admission.
use super::*;
mod string;
use crysirror_core::service::EngineQuery;
use crysirror_scene::{
    animation::{ScalarOscillator, ScalarTween, SignalFade, Waveform},
    layer::{Easing, ExternalMotionStamp, MotionStatus},
};
pub(super) struct Group {
    pub layer: u32,
    stamp: Option<ExternalMotionStamp>,
    pub waves: BTreeMap<u8, Generator>,
}
impl Group {
    pub(super) fn retire_registration(&mut self) {
        self.stamp = None;
    }
}
pub(super) struct Generator {
    signal: ScalarOscillator,
    started: u64,
    amplitude: [f32; 3],
    target: i32,
    fade: u8,
}
fn mode(n: i32) -> Result<(Waveform, bool)> {
    Ok(match n {
        1 => (Waveform::Noise, false),
        2 => (Waveform::PositiveNoise, false),
        3 => (Waveform::Square, false),
        4 => (Waveform::Pulse, false),
        5 => (Waveform::Triangle, false),
        6 => (Waveform::Triangle, true),
        7 => (Waveform::Sine, false),
        8 => (Waveform::Sine, true),
        9 => (Waveform::LinearNoise, false),
        10 => (Waveform::PositiveLinearNoise, false),
        11 => (Waveform::CubicNoise, false),
        12 => (Waveform::PositiveCubicNoise, false),
        _ => bail!("unsupported QLIE generator mode {n}"),
    })
}
fn fade(n: i32) -> Result<u8> {
    ensure!((0..=8).contains(&n), "unsupported QLIE generator fade {n}");
    Ok(n as u8)
}
impl Generator {
    fn envelope(&self, elapsed: u64) -> Result<f32> {
        let time = elapsed.saturating_sub(self.signal.delay_ms);
        let duration = self.signal.duration_ms;
        if time >= duration || self.fade == 0 {
            return Ok(1.);
        }
        let reverse = matches!(self.fade, 7 | 8);
        let pair = matches!(self.fade, 3 | 6 | 7 | 8);
        let first = !pair || time.saturating_mul(2) < duration;
        let (from, to) = if pair {
            if first != reverse {
                (0., 1.)
            } else {
                (1., 0.)
            }
        } else if matches!(self.fade, 2 | 5) {
            (1., 0.)
        } else {
            (0., 1.)
        };
        // Native5CA31C uses4FCF34 (engine EaseOutQuart) first and4FCFB4
        // (EaseInQuart) second. Generic "InQuart" is not this ABI profile.
        let easing = if matches!(self.fade, 4 | 5 | 6 | 8) {
            if first {
                Easing::EaseOutQuart
            } else {
                Easing::EaseInQuart
            }
        } else {
            Easing::Linear
        };
        let (at, span) = if pair {
            let at = if first { time * 2 } else { time * 2 - duration };
            (at, duration)
        } else {
            (time, duration)
        };
        Ok(ScalarTween::new(from, to, 0, span, easing)?.sample(at))
    }
}
impl QlieVm {
    pub(super) fn generator_method(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<()> {
        let camera = matches!(self.objects[id].kind.as_str(), "cam" | "camex");
        let mut slot = 4;
        let mut generator = None;
        let mut string_waves = None;
        if op == "g" {
            let n = |i: usize, default: i32| {
                a.get(i)
                    .map(Value::integer)
                    .transpose()
                    .map(|v| v.unwrap_or(default))
            };
            ensure!(
                (2..=10).contains(&a.len()) && (0..=3).contains(&n(0, 0)?),
                "QLIE numeric generator ABI"
            );
            slot = n(0, 0)? as u8;
            if n(1, 0)? != 0 {
                let (waveform, absolute) = mode(n(1, 0)?)?;
                let duration = n(2, 0)?;
                generator = Some(Generator {
                    signal: ScalarOscillator {
                        waveform,
                        absolute,
                        period_ms: n(3, 100)?.max(1) as u64,
                        duration_ms: duration.max(1) as u64,
                        delay_ms: 0,
                        sustained: duration < 0 || n(9, 0)? != 0,
                        fade: SignalFade::None,
                        seed: self.objects[id].layer,
                    },
                    started: h.now_ms(),
                    amplitude: [n(4, 0)? as f32, n(5, 0)? as f32, n(6, 0)? as f32],
                    target: n(7, 0)?,
                    fade: fade(n(8, 0)?)?,
                });
            }
        } else {
            ensure!(a.len() <= 2, "QLIE string generator ABI");
            let source = a.first().map(Value::text).unwrap_or_default();
            let reset = a.get(1).map(Value::integer).transpose()?.unwrap_or(0) != 0;
            if !reset
                && (source.is_empty() || self.objects[id].generator.is_some())
                && self.objects[id]
                    .properties
                    .get("generator_source")
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .as_deref()
                    == Some(source.as_str())
            {
                return Ok(());
            }
            string_waves = Some(string::parse(&source, self.objects[id].layer, h.now_ms())?);
            self.objects
                .get_mut(id)
                .unwrap()
                .properties
                .insert("generator_source".into(), vec![Value::Text(source)]);
        }
        if let Some(g) = &generator {
            g.signal.validate()?;
            ensure!((0..=2).contains(&g.target), "QLIE generator target");
        }
        if self.objects[id].generator.is_none()
            && generator.is_none()
            && string_waves.as_ref().is_none_or(BTreeMap::is_empty)
        {
            return Ok(());
        }
        if self.objects[id].generator.is_none() {
            if !camera {
                let layer = self.objects[id].layer;
                self.objects.get_mut(id).unwrap().generator = Some(Group {
                    layer,
                    stamp: None,
                    waves: BTreeMap::new(),
                });
            } else {
                let layer = self.next_layer;
                self.next_layer = self
                    .next_layer
                    .checked_add(1)
                    .context("QLIE generator layer identity")?;
                let parent = self.objects[id].layer;
                let [w, height] = self.config.canvas;
                self.submit(h,json!({"layers":[{"op":"put","layer":{"id":layer,"parent":parent,"rect":[0,0,w,height],"paint":{"kind":"solid","color":[0,0,0,0]},"transform":{"pivot_x":w as f32/2.,"pivot_y":height as f32/2.}}}]}))?;
                let mut commands = Vec::new();
                for o in self
                    .objects
                    .values()
                    .filter(|o| o.created && o.layer != parent)
                {
                    let spec: serde_json::Value = serde_json::from_slice(
                        &h.query_engine(self.client, EngineQuery::Layer(o.layer))?,
                    )?;
                    if spec["parent"] == parent {
                        commands.push(json!({"op":"set_parent","id":o.layer,"parent":layer}));
                    }
                }
                if !commands.is_empty() {
                    self.submit(h, json!({"layers":commands}))?;
                }
                self.objects.get_mut(id).unwrap().generator = Some(Group {
                    layer,
                    stamp: None,
                    waves: BTreeMap::new(),
                });
            }
        }
        let group = self
            .objects
            .get_mut(id)
            .unwrap()
            .generator
            .as_mut()
            .unwrap();
        if let Some(waves) = string_waves {
            group.waves.retain(|slot, _| *slot < 4);
            group.waves.extend(waves);
        } else if let Some(generator) = generator {
            group.waves.insert(slot, generator);
        } else {
            group.waves.remove(&slot);
        }
        if group.stamp.is_none() {
            let layer = group.layer;
            self.submit(
                h,
                json!({"layers":[{"op":"begin_external_motion","id":layer,"driver":0}]}),
            )?;
            let motion: MotionStatus = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::LayerMotion(layer))?,
            )?;
            self.objects
                .get_mut(id)
                .unwrap()
                .generator
                .as_mut()
                .unwrap()
                .stamp = Some(
                motion
                    .drivers
                    .into_iter()
                    .find(|v| v.driver == 0)
                    .context("QLIE generator producer registration")?
                    .stamp,
            );
        }
        self.update_generators(h)
    }
    pub(super) fn update_generators(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let registrations: Vec<_> = self
            .objects
            .iter()
            .filter(|(_, o)| o.created && !o.preparing)
            .filter_map(|(id, o)| {
                o.generator
                    .as_ref()
                    .filter(|g| g.stamp.is_none() && !g.waves.is_empty())
                    .map(|g| (id.clone(), g.layer))
            })
            .collect();
        for (id, layer) in registrations {
            self.submit(
                h,
                json!({"layers":[{"op":"begin_external_motion","id":layer,"driver":0}]}),
            )?;
            let motion: MotionStatus = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::LayerMotion(layer))?,
            )?;
            let stamp = motion
                .drivers
                .into_iter()
                .find(|v| v.driver == 0)
                .context("QLIE replacement generator registration")?
                .stamp;
            self.objects
                .get_mut(&id)
                .unwrap()
                .generator
                .as_mut()
                .unwrap()
                .stamp = Some(stamp);
        }
        let mut commands = Vec::new();
        for o in self.objects.values_mut() {
            let Some(group) = &mut o.generator else {
                continue;
            };
            let Some(stamp) = &mut group.stamp else {
                continue;
            };
            let mut values = [[0f32; 3]; 3];
            for (&slot, g) in &group.waves {
                for axis in 0..3 {
                    let elapsed = h.now_ms().saturating_sub(g.started);
                    values[g.target as usize][axis] += g.amplitude[axis]
                        * g.envelope(elapsed)?
                        * g.signal
                            .sample(elapsed, u32::from(slot) * 3 + axis as u32)?;
                }
            }
            group
                .waves
                .retain(|_, g| !g.signal.complete(h.now_ms().saturating_sub(g.started)));
            stamp.revision = stamp
                .revision
                .checked_add(1)
                .context("QLIE generator revision")?;
            let [w, height] = self.config.canvas;
            let pose = if matches!(o.kind.as_str(), "cam" | "camex") {
                json!({"rect":[-values[0][0],-values[0][1],w,height],"transform":{"z":-values[0][2],"scale_x":1.+values[1][0]/100.,"scale_y":1.+values[1][1]/100.,"scale_z":1.+values[1][2]/100.,"rotate_x":-values[2][0].to_radians(),"rotate_y":-values[2][1].to_radians(),"rotate":-values[2][2].to_radians(),"pivot_x":w as f32/2.,"pivot_y":height as f32/2.}})
            } else {
                //5D16C4/5D1594 add generator channels before one matrix is
                //constructed. A nested sprite would scale translations and
                //multiply zooms instead. Shared offsets preserve base tracks.
                json!({"offset":{"translation":values[0],"scale":values[1].map(|v|v/100.),"rotation":values[2].map(f32::to_radians)}})
            };
            let complete = group.waves.is_empty();
            commands.push(json!({"op":"commit_external_motion","id":group.layer,"driver":0,"stamp":stamp,"complete":complete,"pose":pose}));
            if complete {
                commands.push(json!({"op":"remove_external_motion","id":group.layer,"driver":0,"generation":stamp.generation.to_string()}));
                group.stamp = None;
            }
        }
        if !commands.is_empty() {
            self.submit(h, json!({"layers":commands}))?;
        }
        Ok(())
    }
}
