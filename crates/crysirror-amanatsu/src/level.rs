// SPDX-License-Identifier: MPL-2.0
//! A loaded build scene: its hierarchy, script-driven uGUI state, Animator
//! playback ([`crysirror_unity::animator`]), `Selectable.interactable`
//! overrides and the `Manager.Scene` fade canvas.
use crate::unity::Unity;
use anyhow::{Context, Result};
use crysirror_unity::{
    animator::{self, Clip, Controller, Playback},
    events::{self, Argument, PersistentCall},
    scene6::{Scene6, ANIMATOR},
    ugui::NodeState,
    PPtr,
};
use std::{collections::HashMap, sync::Arc};

pub struct Animated {
    pub node: usize,
    pub controller: Arc<Controller>,
    pub clips: Vec<Option<Arc<Clip>>>,
    pub playback: Playback,
}

pub struct Level {
    /// File key (`levelN`) and build scene name.
    pub key: String,
    pub name: String,
    pub scene: Scene6,
    /// Script-driven state (active, CanvasGroup flags, colours).
    pub states: HashMap<usize, NodeState>,
    pub animators: Vec<Animated>,
    /// `Selectable.interactable` changed at runtime.
    pub interactable: HashMap<usize, bool>,
    pub log: Vec<String>,
}

impl Level {
    pub fn load(unity: &mut Unity, key: &str, name: &str) -> Result<Self> {
        let scene = unity.scene(key)?;
        let mut animators = Vec::new();
        let mut log = Vec::new();
        for (i, n) in scene.nodes.iter().enumerate() {
            for c in n.components.iter().filter(|c| c.class_id == ANIMATOR) {
                let p = c.fields.pptr("m_Controller").unwrap_or_default();
                if p.is_null() {
                    continue;
                }
                match load_controller(unity, key, p) {
                    Ok((controller, clips)) => {
                        let playback = Playback::new(&controller);
                        animators.push(Animated { node: i, controller, clips, playback });
                    }
                    Err(e) => log.push(format!("{}: Animator controller: {e:#}", scene.path(i))),
                }
            }
        }
        Ok(Self { key: key.to_owned(), name: name.to_owned(), scene, states: HashMap::new(), animators, interactable: HashMap::new(), log })
    }

    pub fn find(&self, path: &str) -> Option<usize> {
        self.scene.find(path)
    }

    /// Node owning a component or GameObject path id.
    pub fn owner(&self, path_id: i64) -> Option<usize> {
        self.scene.by_game_object.get(&path_id).copied().or_else(|| self.scene.nodes.iter().position(|n| n.transform == path_id || n.components.iter().any(|c| c.path_id == path_id)))
    }

    pub fn set_active(&mut self, node: usize, on: bool) {
        self.states.entry(node).or_default().active = Some(on);
    }

    pub fn is_active(&self, node: usize) -> bool {
        self.states.get(&node).and_then(|s| s.active).unwrap_or(self.scene.nodes[node].active)
    }

    /// `CanvasGroup.interactable`.
    pub fn set_group_interactable(&mut self, node: usize, on: bool) {
        self.states.entry(node).or_default().interactable = Some(on);
    }

    /// `CanvasGroup.blocksRaycasts`.
    pub fn set_blocks_raycasts(&mut self, node: usize, on: bool) {
        self.states.entry(node).or_default().blocks_raycasts = Some(on);
    }

    /// `Selectable.interactable` of the node's Button/Toggle.
    pub fn selectable_interactable(&self, node: usize) -> bool {
        self.interactable.get(&node).copied().unwrap_or_else(|| {
            self.selectable(node).map_or(false, |c| c.fields.int("m_Interactable").unwrap_or(1) != 0)
        })
    }

    fn selectable(&self, node: usize) -> Option<&crysirror_unity::scene6::Component> {
        self.scene.nodes[node].components.iter().find(|c| matches!(c.script.as_str(), "UnityEngine.UI.Button" | "UnityEngine.UI.Toggle"))
    }

    /// The Button handling a click on `node` (itself or an ancestor).
    pub fn click_handler(&self, mut node: usize) -> Option<usize> {
        loop {
            if self.scene.nodes[node].components.iter().any(|c| c.script == "UnityEngine.UI.Button") {
                return Some(node);
            }
            node = self.scene.nodes[node].parent?;
        }
    }

    /// `Button.m_OnClick` persistent calls.
    pub fn on_click(&self, node: usize) -> Vec<PersistentCall> {
        self.scene.nodes[node]
            .components
            .iter()
            .find(|c| c.script == "UnityEngine.UI.Button")
            .and_then(|c| c.fields.fields("m_OnClick"))
            .map(events::persistent_calls)
            .unwrap_or_default()
    }

    /// `Animator.Play(state)` on the node's Animator.
    pub fn play(&mut self, node: usize, state: &str) -> bool {
        match self.animators.iter_mut().find(|a| a.node == node) {
            Some(a) => a.playback.play(&a.controller, state),
            None => false,
        }
    }

    /// Run the built-in targets of persistent calls; script targets are
    /// returned in order for the scene's ported methods.
    pub fn invoke(&mut self, calls: &[PersistentCall]) -> Vec<PersistentCall> {
        let mut script = Vec::new();
        for call in calls {
            let target = self.owner(call.target.path_id);
            match (call.type_name.as_str(), call.method.as_str(), &call.argument, target) {
                ("UnityEngine.Animator", "Play", Argument::String(s), Some(n)) => {
                    if !self.play(n, s) {
                        self.log.push(format!("Animator.Play({s}): no such state on {}", self.scene.path(n)));
                    }
                }
                ("UnityEngine.GameObject", "SetActive", Argument::Bool(b), Some(n)) => self.set_active(n, *b),
                ("UnityEngine.UI.Toggle" | "UnityEngine.UI.Button" | "UnityEngine.UI.Selectable", "set_interactable", Argument::Bool(b), Some(n)) => {
                    self.interactable.insert(n, *b);
                }
                (t, _, _, _) if t.starts_with("UnityEngine.") => {
                    self.log.push(format!("unported persistent call {t}.{}", call.method));
                }
                _ => script.push(call.clone()),
            }
        }
        script
    }

    pub fn advance(&mut self, dt: f32) {
        for a in &mut self.animators {
            let d = |i: usize| a.clips.get(i)?.as_ref().map(|c| c.duration());
            a.playback.advance(&a.controller, &d, dt);
        }
    }

    /// Script state with every Animator's current values applied.
    pub fn frame_states(&mut self) -> HashMap<usize, NodeState> {
        let mut out = self.states.clone();
        for a in &self.animators {
            if !self.is_active(a.node) {
                continue;
            }
            let values = a.playback.sample(&a.controller, &|i| a.clips.get(i)?.as_deref());
            for b in animator::apply_ugui(&self.scene, a.node, &values, &mut out) {
                let msg = format!("{}: unmapped animation binding class {} attribute {}", self.scene.path(a.node), b.class_id, b.attribute);
                if !self.log.contains(&msg) {
                    self.log.push(msg);
                }
            }
        }
        out
    }
}

type Loaded = (Arc<Controller>, Vec<Option<Arc<Clip>>>);

fn load_controller(unity: &mut Unity, file: &str, p: PPtr) -> Result<Loaded> {
    let (cfile, cid) = unity.target(file, p)?;
    let controller = Controller::decode(&unity.decode(&cfile, cid)?).context("AnimatorController")?;
    let mut clips = Vec::new();
    for cp in &controller.clips {
        let clip = unity.target(&cfile, *cp).and_then(|(f, id)| unity.decode(&f, id)).and_then(|f| Clip::decode(&f)).ok().map(Arc::new);
        clips.push(clip);
    }
    Ok((Arc::new(controller), clips))
}

/// `FadeCanvas` alpha animation (`StartFadeAsync`): `In` covers the screen
/// with the fade colour, `Out` reveals it, over `_time` seconds.
#[derive(Clone, Debug)]
pub struct Fade {
    pub color: [f32; 3],
    from: f32,
    to: f32,
    start: u64,
    duration_ms: u64,
}

impl Fade {
    pub fn covered(color: [f32; 3]) -> Self {
        Self { color, from: 1., to: 1., start: 0, duration_ms: 0 }
    }
    pub fn start(&mut self, now: u64, covering: bool, seconds: f32) {
        self.from = self.alpha(now);
        self.to = if covering { 1. } else { 0. };
        self.start = now;
        self.duration_ms = (seconds * 1000.) as u64;
    }
    pub fn alpha(&self, now: u64) -> f32 {
        if self.duration_ms == 0 || now >= self.start + self.duration_ms {
            return self.to;
        }
        let t = (now - self.start) as f32 / self.duration_ms as f32;
        self.from + (self.to - self.from) * t
    }
    /// `Manager.Scene.IsFadeNow`.
    pub fn fading(&self, now: u64) -> bool {
        self.duration_ms > 0 && now < self.start + self.duration_ms
    }
}
