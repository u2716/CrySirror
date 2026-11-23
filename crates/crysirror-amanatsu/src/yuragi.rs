// SPDX-License-Identifier: MPL-2.0
//! The serialized HScene.YuragiCtrl and its two independent clocks.
use anyhow::{Context, Result, ensure};
use crysirror_unity::managed::Fields;

pub(crate) struct Yuragi {
    interval: [f32; 2],
    duration: [f32; 2],
    minimum_step: f32,
    curve: Fields,
    change_after: f32,
    change_elapsed: f32,
    travel_time: f32,
    travel_elapsed: f32,
    moving: bool,
    increasing: bool,
    endpoints: [f32; 2],
    pub(crate) value: f32,
}

impl Yuragi {
    pub(crate) fn new(fields: &Fields, random: &mut impl FnMut() -> f32) -> Result<Self> {
        let number = |key| {
            fields
                .float(key)
                .map(|v| v as f32)
                .with_context(|| format!("YuragiCtrl.{key}"))
        };
        let interval = [
            number("_changeAutoMotionTimeMin")?,
            number("_changeAutoMotionTimeMax")?,
        ];
        let duration = [
            number("_changeMotionTimeMin")?,
            number("_changeMotionTimeMax")?,
        ];
        let minimum_step = number("_changeMotionMinRate")?;
        let curve = fields
            .fields("_changeMotionCurve")
            .context("YuragiCtrl curve")?
            .clone();
        ensure!(
            interval[0] >= 0.
                && interval[1] >= interval[0]
                && duration[0] > 0.
                && duration[1] >= duration[0]
                && (0.0..=1.0).contains(&minimum_step),
            "invalid YuragiCtrl timing"
        );
        ensure!(
            crysirror_unity::particles::curve(&curve, 0.).is_some(),
            "empty YuragiCtrl curve"
        );
        let mut state = Self {
            interval,
            duration,
            minimum_step,
            curve,
            change_after: 0.,
            change_elapsed: 0.,
            travel_time: 0.,
            travel_elapsed: 0.,
            moving: false,
            increasing: true,
            endpoints: [0.; 2],
            value: 0.,
        };
        state.reset(random);
        Ok(state)
    }

    pub(crate) fn reset(&mut self, random: &mut impl FnMut() -> f32) {
        self.value = 0.;
        self.travel_elapsed = 0.;
        self.change_after = range(self.interval, random);
        self.change_elapsed = 0.;
    }

    /// ChangeTimer is called by the weak-loop controller. Process runs each
    /// frame, so an in-progress interpolation continues outside that state.
    pub(crate) fn advance(&mut self, dt: f32, weak_loop: bool, random: &mut impl FnMut() -> f32) {
        if weak_loop {
            self.change_elapsed = (self.change_elapsed + dt).min(self.change_after);
            if !self.moving && self.change_elapsed >= self.change_after {
                self.change_after = range(self.interval, random);
                self.change_elapsed = 0.;
                let remaining = if self.increasing {
                    1. - self.value
                } else {
                    self.value
                };
                let step = if self.minimum_step < remaining {
                    range([self.minimum_step, remaining], random)
                } else {
                    remaining
                };
                let target = if self.increasing {
                    self.value + step
                } else {
                    self.value - step
                };
                self.endpoints = [self.value, target];
                if target >= 1. {
                    self.increasing = false;
                }
                if target <= 0. {
                    self.increasing = true;
                }
                self.travel_time = range(self.duration, random);
                self.travel_elapsed = 0.;
                self.moving = true;
            }
        }
        if self.moving {
            self.travel_elapsed = (self.travel_elapsed + dt).min(self.travel_time);
            let position = crysirror_unity::particles::curve(
                &self.curve,
                self.travel_elapsed / self.travel_time,
            )
            .unwrap()
            .clamp(0., 1.);
            self.value = self.endpoints[0] + (self.endpoints[1] - self.endpoints[0]) * position;
            if position >= 1. {
                self.moving = false;
            }
        }
    }
}

fn range([min, max]: [f32; 2], random: &mut impl FnMut() -> f32) -> f32 {
    min + (max - min) * random()
}

/// One actor's exported variants that the native base controller blends
/// through Motion. Both sources run on the same normalized cycle.
pub(crate) struct Actor {
    pub(crate) entities: Vec<u32>,
    pairs: std::collections::HashMap<String, [(String, f32); 2]>,
    phase: f32,
}

impl Actor {
    pub(crate) fn bind(
        unity: &mut crate::unity::Unity,
        base: &crate::h::AssetInfo,
        size: &str,
        lengths: &std::collections::HashMap<String, f32>,
        entities: Vec<u32>,
    ) -> Result<Self> {
        use crysirror_unity::animator::Controller;
        let mut pairs = std::collections::HashMap::new();
        if !base.asset.is_empty() {
            let (file, id) = unity
                .bundle_asset(&format!("lib/{}", base.bundle), &base.asset, 91)?
                .context("posture base controller")?;
            let fields = unity.decode(&file, id)?;
            let controller = Controller::decode(&fields)?;
            let mut names = std::collections::HashMap::new();
            for slots in Controller::linear_clip_pairs(&fields, "Motion") {
                let mut sources = Vec::new();
                for slot in slots {
                    if !names.contains_key(&slot) {
                        let p = *controller.clips.get(slot).context("blend source clip")?;
                        let (clip_file, id) = unity.target(&file, p)?;
                        let data = unity.decode(&clip_file, id)?;
                        names.insert(
                            slot,
                            data.string("m_Name")
                                .context("blend source name")?
                                .to_owned(),
                        );
                    }
                    let name = &names[&slot];
                    let name = match name.rsplit_once('_') {
                        Some((state, suffix @ ("S" | "M" | "L"))) if suffix == size => state,
                        Some((_, "S" | "M" | "L")) => break,
                        _ => name,
                    };
                    let Some(seconds) = lengths.get(name).copied().filter(|s| *s > 0.) else {
                        break;
                    };
                    sources.push((name.to_owned(), seconds));
                }
                if let Ok(pair) = <[(String, f32); 2]>::try_from(sources) {
                    pairs.insert(pair[0].0.clone(), pair.clone());
                    pairs.insert(pair[1].0.clone(), pair);
                }
            }
        }
        Ok(Self {
            entities,
            pairs,
            phase: 0.,
        })
    }

    pub(crate) fn reset_phase(&mut self) {
        self.phase = 0.;
    }

    pub(crate) fn has_motion(&self, name: &str) -> bool {
        self.pairs.contains_key(name)
    }

    pub(crate) fn layers(
        &mut self,
        motion: &str,
        weight: f32,
        dt: f32,
    ) -> Option<serde_json::Value> {
        let [a, b] = self.pairs.get(motion)?;
        let duration = (1. - weight) * a.1 + weight * b.1;
        self.phase = (self.phase + dt / duration).rem_euclid(1.);
        let layer = |(clip, seconds): &(String, f32), amount: f32| {
            serde_json::json!({
                "clip": clip, "time": self.phase * seconds, "speed": seconds / duration, "weight": amount,
                "target_weight": amount, "looping": true
            })
        };
        Some(serde_json::json!([layer(a, 1. - weight), layer(b, weight)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_unity::managed::Value;
    #[test]
    fn exported_layers_share_phase_and_keep_pose_weights() {
        let sources = [("WLoop1".to_string(), 2.), ("WLoop2".to_string(), 4.)];
        let mut actor = Actor {
            entities: vec![1],
            pairs: [("WLoop1".to_owned(), sources)].into(),
            phase: 0.,
        };
        let layers = actor.layers("WLoop1", 0.25, 0.625).unwrap();
        assert_eq!(layers[0]["weight"], 0.75);
        assert_eq!(layers[1]["weight"], 0.25);
        assert_eq!(layers[0]["time"], 0.5);
        assert_eq!(layers[1]["time"], 1.);
        assert!(actor.layers("Idle", 0.9, 1.).is_none());
    }

    #[test]
    fn weak_loop_arms_the_clock_and_the_curve_runs_to_both_endpoints() {
        let key = |t: f64| {
            Value::Struct(Fields(vec![
                ("time".into(), Value::Float(t)),
                ("value".into(), Value::Float(t)),
                ("inSlope".into(), Value::Float(0.)),
                ("outSlope".into(), Value::Float(0.)),
            ]))
        };
        let curve = Fields(vec![(
            "m_Curve".into(),
            Value::Array(vec![key(0.), key(1.)]),
        )]);
        let mut fields = Fields(vec![("_changeMotionCurve".into(), Value::Struct(curve))]);
        for (key, value) in [
            ("_changeAutoMotionTimeMin", 4.),
            ("_changeAutoMotionTimeMax", 4.),
            ("_changeMotionTimeMin", 2.),
            ("_changeMotionTimeMax", 2.),
            ("_changeMotionMinRate", 0.1),
        ] {
            fields.0.push((key.into(), Value::Float(value)));
        }
        let mut y = Yuragi::new(&fields, &mut || 1.).unwrap();
        y.advance(10., false, &mut || 1.);
        y.advance(3.5, true, &mut || 1.);
        assert_eq!(y.value, 0.);
        y.advance(0.5, true, &mut || 1.);
        assert_eq!(y.value, 0.15625);
        y.advance(1.5, false, &mut || 1.);
        assert_eq!(y.value, 1.);
        y.advance(4., true, &mut || 1.);
        assert_eq!(y.value, 0.);
    }
}
