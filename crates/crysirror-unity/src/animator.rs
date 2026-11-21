// SPDX-License-Identifier: MPL-2.0
//! Mecanim playback of Unity 5+ `AnimatorController` (#91) and
//! `AnimationClip` (#74) data without Unity: muscle-clip curves (streamed
//! cubic segments, dense samples and constants) bound through
//! `m_ClipBindingConstant`, controller layers/state machines with exit-time
//! transitions and cross-fades, and `Animator.Play(stateName)`.
//!
//! Generic bindings are reported as (path hash, attribute, class id,
//! script) with their sampled value; [`Binding::ugui`] maps the uGUI
//! properties to [`crate::ugui::NodeState`] fields, and [`path_hash`]
//! resolves a binding path relative to the animator's GameObject.
use crate::managed::{Fields, Value};
use crate::scene6::Scene6;
use crate::ugui::NodeState;
use crate::PPtr;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// CRC-32 (IEEE) as Unity hashes binding paths, attribute names and state
/// names (`Animator.StringToHash`).
pub fn crc32(s: &str) -> u32 {
    let mut c = !0u32;
    for &b in s.as_bytes() {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// Hash of a binding path ("" for the animator's own GameObject).
pub fn path_hash(path: &str) -> u32 {
    if path.is_empty() { 0 } else { crc32(path) }
}

fn data(v: &Value) -> Option<&Fields> {
    match v {
        Value::Struct(f) => Some(f.fields("data").unwrap_or(f)),
        _ => None,
    }
}

fn items<'a>(f: &'a Fields, name: &str) -> impl Iterator<Item = &'a Fields> {
    f.array(name).unwrap_or(&[]).iter().filter_map(data)
}

fn u32s(f: &Fields, name: &str) -> Vec<u32> {
    f.array(name).unwrap_or(&[]).iter().map(|v| if let Value::Int(i) = v { *i as u32 } else { 0 }).collect()
}

fn f32s(f: &Fields, name: &str) -> Vec<f32> {
    match f.get(name) {
        Some(Value::Floats(v)) => v.clone(),
        Some(Value::Array(a)) => a.iter().map(|v| match v { Value::Float(x) => *x as f32, Value::Int(i) => *i as f32, _ => 0. }).collect(),
        _ => Vec::new(),
    }
}

/// One generic binding of a clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub path: u32,
    pub attribute: u32,
    pub class_id: i32,
    pub script: PPtr,
    pub pptr: bool,
    /// Curves this binding consumes (3/4 for transform vectors/quaternions).
    pub curves: usize,
}

/// A uGUI property driven by an animation curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiProperty {
    Active,
    GroupAlpha,
    /// `Graphic.m_Color` component (r, g, b, a) or TMP `m_fontColor`.
    Color(usize),
    AnchoredPosition(usize),
    SizeDelta(usize),
    /// `Transform.m_LocalScale` component.
    Scale(usize),
    Fill,
}

impl Binding {
    /// The uGUI property of curve `k` (0-based within this binding).
    pub fn ugui(&self, k: usize) -> Option<UiProperty> {
        const TRANSFORM: i32 = 4;
        match (self.class_id, self.attribute) {
            (TRANSFORM, 3) => Some(UiProperty::Scale(k)),
            (TRANSFORM, _) => None,
            (1, a) if a == crc32("m_IsActive") => Some(UiProperty::Active),
            (225, a) if a == crc32("m_Alpha") => Some(UiProperty::GroupAlpha),
            (224, a) => ["m_AnchoredPosition.x", "m_AnchoredPosition.y", "m_SizeDelta.x", "m_SizeDelta.y"]
                .iter()
                .position(|n| crc32(n) == a)
                .map(|i| if i < 2 { UiProperty::AnchoredPosition(i) } else { UiProperty::SizeDelta(i - 2) }),
            (114, a) => {
                for (i, c) in ["r", "g", "b", "a"].iter().enumerate() {
                    if a == crc32(&format!("m_Color.{c}")) || a == crc32(&format!("m_fontColor.{c}")) {
                        return Some(UiProperty::Color(i));
                    }
                }
                (a == crc32("m_FillAmount")).then_some(UiProperty::Fill)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct StreamedKey {
    time: f32,
    coeff: [f32; 4],
}

/// A decoded `AnimationClip`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clip {
    pub name: String,
    pub start: f32,
    pub stop: f32,
    pub looping: bool,
    pub bindings: Vec<Binding>,
    /// Streamed curves: keys per curve in time order.
    streamed: Vec<Vec<StreamedKey>>,
    dense_frames: usize,
    dense_curves: usize,
    dense_rate: f32,
    dense_begin: f32,
    dense: Vec<f32>,
    constant: Vec<f32>,
}

impl Clip {
    pub fn decode(f: &Fields) -> Result<Self> {
        let muscle = f.fields("m_MuscleClip").context("m_MuscleClip")?;
        let clip = muscle.fields("m_Clip").and_then(|c| c.fields("data").or(Some(c))).context("m_Clip")?;
        let mut out = Clip {
            name: f.string("m_Name").unwrap_or("").to_owned(),
            start: muscle.float("m_StartTime").unwrap_or(0.) as f32,
            stop: muscle.float("m_StopTime").unwrap_or(0.) as f32,
            looping: muscle.bool("m_LoopTime").unwrap_or(false),
            ..Default::default()
        };
        if let Some(s) = clip.fields("m_StreamedClip") {
            let words = u32s(s, "data");
            let count = s.int("curveCount").unwrap_or(0) as usize + s.int("discreteCurveCount").unwrap_or(0) as usize;
            out.streamed = vec![Vec::new(); count];
            let mut i = 0;
            while i + 2 <= words.len() {
                let time = f32::from_bits(words[i]);
                let keys = words[i + 1] as usize;
                i += 2;
                for _ in 0..keys {
                    anyhow::ensure!(i + 5 <= words.len(), "streamed clip key past end");
                    let idx = words[i] as usize;
                    let coeff = [words[i + 1], words[i + 2], words[i + 3], words[i + 4]].map(f32::from_bits);
                    i += 5;
                    if let Some(c) = out.streamed.get_mut(idx) {
                        c.push(StreamedKey { time, coeff });
                    }
                }
            }
        }
        if let Some(d) = clip.fields("m_DenseClip") {
            out.dense_frames = d.int("m_FrameCount").unwrap_or(0) as usize;
            out.dense_curves = d.int("m_CurveCount").unwrap_or(0) as usize;
            out.dense_rate = d.float("m_SampleRate").unwrap_or(60.) as f32;
            out.dense_begin = d.float("m_BeginTime").unwrap_or(0.) as f32;
            out.dense = f32s(d, "m_SampleArray");
        }
        if let Some(c) = clip.fields("m_ConstantClip") {
            out.constant = f32s(c, "data");
        }
        let total = out.streamed.len() + out.dense_curves + out.constant.len();
        let mut used = 0;
        if let Some(b) = f.fields("m_ClipBindingConstant") {
            for g in b.array("genericBindings").unwrap_or(&[]) {
                let Value::Struct(g) = g else { continue };
                let class_id = g.int("typeID").unwrap_or(0) as i32;
                let attribute = g.int("attribute").unwrap_or(0) as u32;
                let curves = if class_id == 4 {
                    match attribute {
                        2 => 4,
                        1 | 3 | 4 => 3,
                        _ => 1,
                    }
                } else {
                    1
                };
                used += curves;
                out.bindings.push(Binding {
                    path: g.int("path").unwrap_or(0) as u32,
                    attribute,
                    class_id,
                    script: g.pptr("script").unwrap_or_default(),
                    pptr: g.int("isPPtrCurve").unwrap_or(0) != 0,
                    curves,
                });
            }
        }
        anyhow::ensure!(used == total, "clip {}: bindings cover {used} curves of {total}", out.name);
        Ok(out)
    }

    pub fn duration(&self) -> f32 {
        (self.stop - self.start).max(0.)
    }

    /// Every curve's value at clip time `t` (seconds from the clip start).
    pub fn sample(&self, t: f32) -> Vec<f32> {
        let t = self.start + t;
        let mut out = Vec::with_capacity(self.streamed.len() + self.dense_curves + self.constant.len());
        for keys in &self.streamed {
            let k = keys.iter().rev().find(|k| k.time <= t).or(keys.first());
            out.push(k.map_or(0., |k| {
                let dt = if k.time.is_finite() { t - k.time } else { 0. };
                ((k.coeff[0] * dt + k.coeff[1]) * dt + k.coeff[2]) * dt + k.coeff[3]
            }));
        }
        if self.dense_curves > 0 && self.dense_frames > 0 {
            let pos = ((t - self.dense_begin) * self.dense_rate).clamp(0., (self.dense_frames - 1) as f32);
            let (f0, frac) = (pos.floor() as usize, pos.fract());
            let f1 = (f0 + 1).min(self.dense_frames - 1);
            for c in 0..self.dense_curves {
                let a = self.dense.get(f0 * self.dense_curves + c).copied().unwrap_or(0.);
                let b = self.dense.get(f1 * self.dense_curves + c).copied().unwrap_or(a);
                out.push(a + (b - a) * frac);
            }
        }
        out.extend_from_slice(&self.constant);
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// Destination state index in the same state machine; out of range
    /// for exits (back to the default state).
    pub destination: u32,
    pub duration: f32,
    pub offset: f32,
    pub exit_time: f32,
    pub has_exit_time: bool,
    pub fixed_duration: bool,
    /// Parameter conditions; transitions with conditions need parameters,
    /// which [`Playback`] does not set.
    pub conditions: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub name_id: u32,
    pub path_id: u32,
    pub full_path_id: u32,
    /// Index into [`Controller::clips`] of the state's single motion.
    pub clip: Option<usize>,
    pub speed: f32,
    pub transitions: Vec<Transition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub states: Vec<State>,
    pub default_state: usize,
    pub weight: f32,
}

/// A decoded `AnimatorController`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Controller {
    pub name: String,
    pub layers: Vec<Layer>,
    pub clips: Vec<PPtr>,
    /// Hash → name table (`m_TOS`).
    pub names: HashMap<u32, String>,
}

impl Controller {
    pub fn decode(f: &Fields) -> Result<Self> {
        let c = f.fields("m_Controller").context("m_Controller")?;
        let machines: Vec<&Fields> = items(c, "m_StateMachineArray").collect();
        let mut out = Controller { name: f.string("m_Name").unwrap_or("").to_owned(), ..Default::default() };
        for (i, layer) in items(c, "m_LayerArray").enumerate() {
            let machine = machines.get(layer.int("m_StateMachineIndex").unwrap_or(0) as usize).context("layer state machine")?;
            let mut states = Vec::new();
            for s in items(machine, "m_StateConstantArray") {
                let clip = items(s, "m_BlendTreeConstantArray")
                    .flat_map(|t| items(t, "m_NodeArray").map(|n| n.int("m_ClipID").unwrap_or(-1)).collect::<Vec<_>>())
                    .find(|&id| id >= 0 && id != u32::MAX as i64)
                    .map(|id| id as usize);
                let transitions = items(s, "m_TransitionConstantArray")
                    .map(|t| Transition {
                        destination: t.int("m_DestinationState").unwrap_or(-1) as u32,
                        duration: t.float("m_TransitionDuration").unwrap_or(0.) as f32,
                        offset: t.float("m_TransitionOffset").unwrap_or(0.) as f32,
                        exit_time: t.float("m_ExitTime").unwrap_or(1.) as f32,
                        has_exit_time: t.bool("m_HasExitTime").unwrap_or(false),
                        fixed_duration: t.bool("m_HasFixedDuration").unwrap_or(true),
                        conditions: t.array("m_ConditionConstantArray").map_or(0, |a| a.len()),
                    })
                    .collect();
                states.push(State {
                    name_id: s.int("m_NameID").unwrap_or(0) as u32,
                    path_id: s.int("m_PathID").unwrap_or(0) as u32,
                    full_path_id: s.int("m_FullPathID").unwrap_or(0) as u32,
                    clip,
                    speed: s.float("m_Speed").unwrap_or(1.) as f32,
                    transitions,
                });
            }
            out.layers.push(Layer {
                states,
                default_state: machine.int("m_DefaultState").unwrap_or(0) as usize,
                weight: if i == 0 { 1. } else { layer.float("m_DefaultWeight").unwrap_or(0.) as f32 },
            });
        }
        out.clips = f.array("m_AnimationClips").unwrap_or(&[]).iter().filter_map(|v| if let Value::PPtr(p) = v { Some(*p) } else { None }).collect();
        for t in f.array("m_TOS").unwrap_or(&[]) {
            if let Value::Struct(t) = t {
                out.names.insert(t.int("first").unwrap_or(0) as u32, t.string("second").unwrap_or("").to_owned());
            }
        }
        Ok(out)
    }

    /// State of `layer` named `name` (short name, `Layer.Name` path or hash).
    pub fn find_state(&self, layer: usize, name: &str) -> Option<usize> {
        let h = name.parse::<u32>().unwrap_or_else(|_| crc32(name));
        self.layers.get(layer)?.states.iter().position(|s| s.name_id == h || s.full_path_id == h || s.path_id == h)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Cursor {
    state: usize,
    time: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct LayerPlayback {
    current: Cursor,
    /// Cross-fade source and (elapsed, duration) in seconds.
    fading: Option<(Cursor, f32, f32)>,
}

/// One `Animator`'s playback of a controller.
#[derive(Clone, Debug, PartialEq)]
pub struct Playback {
    layers: Vec<LayerPlayback>,
}

fn state_length(ctl: &Controller, clips: &dyn Fn(usize) -> Option<f32>, layer: usize, state: usize) -> f32 {
    ctl.layers[layer].states[state].clip.and_then(clips).unwrap_or(1.).max(1e-4)
}

impl Playback {
    /// Every layer in its default state at time zero (the Animator's
    /// `OnEnable`).
    pub fn new(ctl: &Controller) -> Self {
        Self {
            layers: ctl.layers.iter().map(|l| LayerPlayback { current: Cursor { state: l.default_state.min(l.states.len().saturating_sub(1)), time: 0. }, fading: None }).collect(),
        }
    }

    /// `Animator.Play(name, -1, 0)`: the first layer with the state jumps to
    /// it without a cross-fade. False when no layer has it.
    pub fn play(&mut self, ctl: &Controller, name: &str) -> bool {
        for (li, lp) in self.layers.iter_mut().enumerate() {
            if let Some(s) = ctl.find_state(li, name) {
                lp.current = Cursor { state: s, time: 0. };
                lp.fading = None;
                return true;
            }
        }
        false
    }

    /// Current state name hash of `layer`.
    pub fn state(&self, layer: usize) -> Option<usize> {
        self.layers.get(layer).map(|l| l.current.state)
    }

    /// Normalized time of `layer`'s current state.
    pub fn normalized_time(&self, ctl: &Controller, clips: &dyn Fn(usize) -> Option<f32>, layer: usize) -> f32 {
        let lp = &self.layers[layer];
        lp.current.time / state_length(ctl, clips, layer, lp.current.state)
    }

    /// Advance by `dt` seconds; `clips` gives clip durations by index.
    pub fn advance(&mut self, ctl: &Controller, clips: &dyn Fn(usize) -> Option<f32>, dt: f32) {
        for (li, lp) in self.layers.iter_mut().enumerate() {
            let layer = &ctl.layers[li];
            if layer.states.is_empty() {
                continue;
            }
            let st = &layer.states[lp.current.state];
            let len = state_length(ctl, clips, li, lp.current.state);
            let before = lp.current.time / len;
            lp.current.time += dt * st.speed;
            if let Some((from, elapsed, duration)) = &mut lp.fading {
                from.time += dt * layer.states[from.state].speed;
                *elapsed += dt;
                if *elapsed >= *duration {
                    lp.fading = None;
                }
            }
            if lp.fading.is_some() {
                continue;
            }
            let after = lp.current.time / len;
            let fire = st.transitions.iter().find(|t| t.conditions == 0 && t.has_exit_time && before < t.exit_time.max(1e-6) && after >= t.exit_time.max(1e-6));
            if let Some(t) = fire {
                let dest = if (t.destination as usize) < layer.states.len() { t.destination as usize } else { layer.default_state };
                let dest_len = state_length(ctl, clips, li, dest);
                let duration = if t.fixed_duration { t.duration } else { t.duration * len };
                let from = lp.current.clone();
                lp.current = Cursor { state: dest, time: t.offset * dest_len + (after - t.exit_time) * len };
                lp.fading = (duration > 0.).then_some((from, 0., duration));
            }
        }
    }

    /// Animated values: (clip index, binding index, curve within binding)
    /// → value, cross-fades blended linearly. `clip` returns decoded clips
    /// by controller clip index.
    pub fn sample<'a>(&self, ctl: &Controller, clip: &dyn Fn(usize) -> Option<&'a Clip>) -> Vec<(Binding, usize, f32)> {
        let mut out: Vec<(Binding, usize, f32)> = Vec::new();
        let eval = |cur: &Cursor, li: usize| -> Vec<(Binding, usize, f32)> {
            let Some(c) = ctl.layers[li].states[cur.state].clip.and_then(clip) else { return Vec::new() };
            let d = c.duration();
            let t = if c.looping && d > 0. { cur.time.rem_euclid(d) } else { cur.time.clamp(0., d) };
            let values = c.sample(t);
            let mut res = Vec::new();
            let mut at = 0;
            for b in &c.bindings {
                for k in 0..b.curves {
                    if let Some(v) = values.get(at + k) {
                        res.push((b.clone(), k, *v));
                    }
                }
                at += b.curves;
            }
            res
        };
        for (li, lp) in self.layers.iter().enumerate() {
            if ctl.layers[li].weight <= 0. || ctl.layers[li].states.is_empty() {
                continue;
            }
            let mut cur = eval(&lp.current, li);
            if let Some((from, elapsed, duration)) = &lp.fading {
                let w = (elapsed / duration).clamp(0., 1.);
                for (b, k, v) in eval(from, li) {
                    match cur.iter_mut().find(|(b2, k2, _)| b2.path == b.path && b2.attribute == b.attribute && b2.class_id == b.class_id && *k2 == k) {
                        Some(e) => e.2 = v + (e.2 - v) * w,
                        None => cur.push((b, k, v)),
                    }
                }
            }
            for e in cur {
                match out.iter_mut().find(|(b, k, _)| b.path == e.0.path && b.attribute == e.0.attribute && b.class_id == e.0.class_id && *k == e.1) {
                    Some(o) => *o = e,
                    None => out.push(e),
                }
            }
        }
        out
    }
}

/// Node of `scene` under `root` whose relative path hashes to `hash`.
pub fn resolve_path(scene: &Scene6, root: usize, hash: u32) -> Option<usize> {
    if hash == 0 {
        return Some(root);
    }
    let mut stack: Vec<(usize, String)> = scene.nodes[root].children.iter().map(|&c| (c, scene.nodes[c].name.clone())).collect();
    while let Some((n, path)) = stack.pop() {
        if crc32(&path) == hash {
            return Some(n);
        }
        for &c in &scene.nodes[n].children {
            stack.push((c, format!("{path}/{}", scene.nodes[c].name)));
        }
    }
    None
}

/// Apply sampled uGUI values under the animator node `root` onto `states`,
/// starting from the scene's serialized values for partially animated
/// vectors and colours. Unsupported bindings are returned.
pub fn apply_ugui(scene: &Scene6, root: usize, values: &[(Binding, usize, f32)], states: &mut HashMap<usize, NodeState>) -> Vec<Binding> {
    let mut unsupported = Vec::new();
    for (b, k, v) in values {
        let (Some(node), Some(prop)) = (resolve_path(scene, root, b.path), b.ugui(*k)) else {
            if !unsupported.contains(b) {
                unsupported.push(b.clone());
            }
            continue;
        };
        let n = &scene.nodes[node];
        let rect = n.xform.rect.unwrap_or_default();
        let st = states.entry(node).or_default();
        match prop {
            UiProperty::Active => st.active = Some(*v > 0.5),
            UiProperty::GroupAlpha => st.group_alpha = Some(*v),
            UiProperty::Color(i) => {
                let base = st.color.unwrap_or_else(|| graphic_color(scene, node));
                let mut c = base;
                c[i] = *v;
                st.color = Some(c);
            }
            UiProperty::AnchoredPosition(i) => {
                let mut p = st.anchored_position.unwrap_or(rect.anchored_position);
                p[i] = *v;
                st.anchored_position = Some(p);
            }
            UiProperty::SizeDelta(i) => {
                let mut p = st.size_delta.unwrap_or(rect.size_delta);
                p[i] = *v;
                st.size_delta = Some(p);
            }
            UiProperty::Scale(i) => {
                let mut p = st.scale.unwrap_or(n.xform.scale);
                p[i] = *v;
                st.scale = Some(p);
            }
            UiProperty::Fill => st.fill = Some(*v),
        }
    }
    unsupported
}

/// Serialized `m_Color` (or TMP `m_fontColor`) of the node's graphic.
fn graphic_color(scene: &Scene6, node: usize) -> [f32; 4] {
    for c in &scene.nodes[node].components {
        for name in ["m_Color", "m_fontColor"] {
            if let Some(f) = c.fields.fields(name) {
                return ["r", "g", "b", "a"].map(|k| f.float(k).unwrap_or(1.) as f32);
            }
        }
    }
    [1.; 4]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_hashes() {
        // Values from the title controller's m_TOS table.
        assert_eq!(crc32("Base Layer"), 756_556_552);
        assert_eq!(crc32("pressed_Shadow_inDefault"), 304_856_993);
        assert_eq!(crc32("Base Layer.pressed_Shadow_inDefault"), 1_585_875_023);
    }

    #[test]
    fn streamed_cubic_segments() {
        let w = |x: f32| Value::Int(i64::from(x.to_bits()));
        let words = vec![
            w(f32::MIN), Value::Int(1), Value::Int(0), w(0.), w(0.), w(0.), w(0.),
            w(0.), Value::Int(1), Value::Int(0), w(0.), w(0.), w(2.), w(1.),
            w(f32::INFINITY), Value::Int(0),
        ];
        let streamed = Fields(vec![("data".into(), Value::Array(words)), ("curveCount".into(), Value::Int(1))]);
        let clip = Fields(vec![("m_StreamedClip".into(), Value::Struct(streamed))]);
        let muscle = Fields(vec![("m_Clip".into(), Value::Struct(Fields(vec![("data".into(), Value::Struct(clip))]))), ("m_StopTime".into(), Value::Float(1.))]);
        let binding = Fields(vec![("path".into(), Value::Int(0)), ("attribute".into(), Value::Int(i64::from(crc32("m_Alpha")))), ("typeID".into(), Value::Int(225))]);
        let f = Fields(vec![
            ("m_MuscleClip".into(), Value::Struct(muscle)),
            ("m_ClipBindingConstant".into(), Value::Struct(Fields(vec![("genericBindings".into(), Value::Array(vec![Value::Struct(binding)]))]))),
        ]);
        let c = Clip::decode(&f).unwrap();
        assert_eq!(c.sample(0.5), vec![2.]);
        assert_eq!(c.bindings[0].ugui(0), Some(UiProperty::GroupAlpha));
    }
}
