// SPDX-License-Identifier: MPL-2.0
//! Talking to a heroine on the map: the ADV window (`ADVCore` of the
//! Addressables manager bundle: `MsgWindowCanvas` name/message, the
//! `background_Choices` buttons cloned from `Choice`) over an
//! [`adv::Program`] of her personality's scenarios (`adv/scene/cNN`).
//!
//! SATD-amanatsu-port: `CommandScene` (the talk/touch/date command menu,
//! parameters and favourability) chooses the scenarios in the original; a
//! talk here plays `e1-approach` and one `e1-N` first-meeting event.
//! Expressions, character motions, event CGs, auto/skip/log/config buttons
//! and the text reveal animation are not ported.
use crate::{
    adv::{self, Program, Wait},
    level::Level,
    unity::Unity,
};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// The Addressables bundle holding `ADVCore` (and the scene fade canvas).
pub const ADV_BUNDLE: &str = "AmanatsuLocation_Data/StreamingAssets/aa/StandaloneWindows64/6d3223da354de646ab79d5660dcac9d2.bundle";

/// Scenario name → (file key, object id) of a personality's bundles and
/// the shared `g` scenarios (`morning`, `rest`).
pub fn scenario_index(unity: &mut Unity, personality: i64) -> Result<HashMap<String, (String, i64)>> {
    let dir = format!("lib/adv/scene/c{personality:02}/");
    let mut out = HashMap::new();
    for key in unity.bundle_files().into_iter().filter(|k| k.contains(&format!("bundle:{dir}")) || k.contains("bundle:lib/adv/scene/g/")) {
        let file = unity.file(&key)?;
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let f = unity.decode(&key, id)?;
            if let Some(n) = f.string("m_Name") {
                out.insert(n.to_owned(), (key.clone(), id));
            }
        }
    }
    Ok(out)
}

pub fn load_scenario(unity: &mut Unity, index: &HashMap<String, (String, i64)>, name: &str) -> Result<Vec<adv::ScenarioCommand>> {
    let (k, id) = index.get(name).with_context(|| format!("scenario {name}"))?.clone();
    let f = unity.decode(&k, id)?;
    match f.get("_bytes") {
        Some(crysirror_unity::managed::Value::Bytes(b)) => adv::parse_scenario(b),
        _ => anyhow::bail!("scenario {name} has no bytes"),
    }
}

/// Show a node with its fade canvases at full alpha (the `FadeCanvas` /
/// `ADVMsgWindow` open state; their tweens are not ported).
pub fn show(level: &mut Level, node: usize) {
    fade_in(level, node);
}

fn fade_in(level: &mut Level, node: usize) {
    level.set_active(node, true);
    let group = |level: &mut Level, n: usize| {
        if level.scene.nodes[n].components.iter().any(|c| c.class_id == crysirror_unity::scene6::CANVAS_GROUP) {
            let st = level.states.entry(n).or_default();
            st.group_alpha = Some(1.);
            st.interactable = Some(true);
            st.blocks_raycasts = Some(true);
        }
    };
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        group(level, n);
        stack.extend(level.scene.nodes[n].children.clone());
    }
    let mut up = level.scene.nodes[node].parent;
    while let Some(n) = up {
        group(level, n);
        up = level.scene.nodes[n].parent;
    }
}

/// A random scenario of a family (`ct8`, `agree-3`, …) in `index`.
pub fn pick(index: &HashMap<String, (String, i64)>, family: &str, seed: u64) -> Option<String> {
    let mut names: Vec<&String> = index
        .keys()
        .filter(|k| k.strip_prefix(family).is_some_and(|r| { let r = r.strip_prefix('-').unwrap_or(r); !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()) }))
        .collect();
    names.sort();
    (!names.is_empty()).then(|| names[(seed as usize) % names.len()].clone())
}

/// The scenarios of a first meeting: `e1-approach` and one `e1-N`.
pub fn first_meeting(index: &HashMap<String, (String, i64)>, seed: u64) -> Vec<String> {
    let mut v = vec!["e1-approach".to_owned()];
    v.extend(pick(index, "e1", seed));
    v
}

/// `CommandUI` buttons → the scenario family they play after an `agree`
/// voice (SATD-amanatsu-port: `EventManager.Get*Asset` choose by
/// parameters, map and time; massage, invite and H continue into scenes
/// that are not ported).
pub fn command_family(button: &str) -> Option<&'static str> {
    Some(match button {
        "btnTalk" => "ct",
        "btnListen" => "cl",
        "btnHTopic" => "cht",
        "btnMassage" => "msg",
        "btnWith" => "with",
        "btnInvite" | "btnH" => "agree",
        _ => return None,
    })
}

/// What the session needs from the host this frame.
pub enum Step {
    None,
    Voice(String, String),
    /// `CharaExpression` values (see `adv::Effect::Face`).
    Face([Option<f32>; 6]),
    /// An expression preset id.
    Expression(i64),
    StopVoice,
    /// ADV character/event commands (`adv::Effect::EventCg` … `ClothState`).
    Chara(adv::Effect),
    End,
}

pub struct Talk {
    pub program: Program,
    index: HashMap<String, (String, i64)>,
    /// Scenarios still to play after the current one.
    queue: Vec<String>,
    pub wait: Wait,
    wait_until: u64,
    name_node: Option<usize>,
    message_node: Option<usize>,
    window: Option<usize>,
    choices_root: Option<usize>,
    choice_template: Option<usize>,
    /// Cloned choice buttons and their target tags.
    choice_nodes: Vec<(usize, String)>,
    /// `ADVCore.PackData.SelectNum`: the last choice index (none: -1).
    pub selected: Option<usize>,
    pub log: Vec<String>,
    /// `TextController`: characters per second of the message reveal
    /// (config `Text/FontSpeed`; 100 shows at once). SATD-amanatsu-port:
    /// the native speed curve is not decoded; the value is read as
    /// characters per second.
    pub chars_per_sec: f32,
    /// The message being revealed and when it started (set on the next
    /// frame).
    reveal: Option<(String, Option<u64>)>,
}

impl Talk {
    /// A conversation playing `scenarios` in order (names of the
    /// heroine's personality bundles).
    pub fn start(unity: &mut Unity, level: &mut Level, personality: i64, heroine_name: &str, seed: u64, scenarios: Vec<String>) -> Result<Self> {
        let index = scenario_index(unity, personality)?;
        let mut queue = scenarios;
        queue.reverse();
        let first = queue.pop().context("talk scenario")?;
        let mut program = Program::new(&first, load_scenario(unity, &index, &first)?, seed);
        program.names.insert("H".into(), heroine_name.to_owned());
        let find = |path: &str| level.find(path);
        let mut t = Self {
            program,
            index,
            queue,
            wait: Wait::End,
            wait_until: 0,
            name_node: find("ADVCore/Canvas_Main/MsgWindowCanvas/t_tw_move/Window/Name"),
            message_node: find("ADVCore/Canvas_Main/MsgWindowCanvas/t_tw_move/Window/Message"),
            window: find("ADVCore/Canvas_Main/MsgWindowCanvas"),
            choices_root: find("ADVCore/Canvas_Main/background_Choices"),
            choice_template: find("ADVCore/Canvas_Main/background_Choices/Choices/Choice"),
            choice_nodes: Vec::new(),
            selected: None,
            log: Vec::new(),
            chars_per_sec: 100.,
            reveal: None,
        };
        // Hidden until used: fades, filters, back log, letterbox, choices.
        for path in [
            "ADVCore/Canvas_Main/FadeFront",
            "ADVCore/Canvas_Main/FadeBack",
            "ADVCore/Canvas_Main/FadeFrontTransition",
            "ADVCore/Canvas_Main/FadeBackTransition",
            "ADVCore/Canvas_Main/FilterImage",
            "ADVCore/Canvas_Main/SplitScreenOutput",
            "ADVCore/Canvas_Main/Hide",
            "ADVCore/Canvas_Main/background_Choices",
            "ADVCore/Canvas_BackLog",
            "ADVCore/Canvas_Letterbox",
            "ADVCore/Camera_Background",
        ] {
            if let Some(n) = level.find(path) {
                level.set_active(n, false);
            }
        }
        if let Some(w) = t.window {
            level.set_active(w, false);
        }
        Ok(t)
    }

    fn show_text(&mut self, level: &mut Level, speaker: &str, text: &str) {
        if let Some(w) = self.window {
            // `ADVMsgWindow` open.
            fade_in(level, w);
        }
        if let Some(n) = self.name_node {
            level.states.entry(n).or_default().text = Some(speaker.to_owned());
        }
        if let Some(n) = self.message_node {
            let shown = if self.chars_per_sec >= 100. { text.to_owned() } else { String::new() };
            level.states.entry(n).or_default().text = Some(shown);
        }
        self.reveal = (self.chars_per_sec < 100.).then(|| (text.to_owned(), None));
    }

    /// `ADVMsgWindow` background alpha (config `Text/WindowAlpha`).
    pub fn set_window_alpha(&self, level: &mut Level, alpha: f32) {
        let Some(w) = self.message_node.and_then(|m| level.scene.nodes[m].parent) else { return };
        let base = level.scene.nodes[w].components.iter().find(|c| c.script.ends_with("UI.Image")).and_then(|c| match c.fields.get("m_Color") {
            Some(crysirror_unity::managed::Value::Struct(s)) => Some(["r", "g", "b", "a"].map(|k| s.float(k).unwrap_or(1.) as f32)),
            _ => None,
        });
        if let Some(c) = base {
            level.states.entry(w).or_default().color = Some([c[0], c[1], c[2], c[3] * alpha.clamp(0., 1.)]);
        }
    }

    /// The revealed part of the message at `now`; true once complete.
    fn update_reveal(&mut self, level: &mut Level, now: u64) -> bool {
        let Some((text, start)) = self.reveal.as_mut() else { return true };
        let start = *start.get_or_insert(now);
        let total = text.chars().count();
        let n = ((now - start) as f32 / 1000. * self.chars_per_sec.max(1.)) as usize;
        let done = n >= total;
        if let Some(m) = self.message_node {
            level.states.entry(m).or_default().text = Some(text.chars().take(n.min(total)).collect());
        }
        if done {
            self.reveal = None;
        }
        done
    }

    fn show_choices(&mut self, level: &mut Level, items: &[(String, String)]) {
        let (Some(root), Some(template)) = (self.choices_root, self.choice_template) else { return };
        fade_in(level, root);
        let parent = level.scene.nodes[template].parent.unwrap_or(root);
        for (label, tag) in items {
            let n = level.scene.clone_subtree(template, parent);
            fade_in(level, n);
            if let Some(text) = level.scene.nodes[n].children.iter().flat_map(|&c| level.scene.nodes[c].children.clone()).find(|&c| level.scene.nodes[c].name == "Text") {
                level.states.entry(text).or_default().text = Some(label.clone());
            }
            self.choice_nodes.push((n, tag.clone()));
        }
    }

    fn clear_choices(&mut self, level: &mut Level) {
        for (n, _) in self.choice_nodes.drain(..) {
            level.set_active(n, false);
        }
        if let Some(r) = self.choices_root {
            level.set_active(r, false);
        }
    }

    /// Advance until the next wait; returns host requests.
    pub fn advance(&mut self, unity: &mut Unity, level: &mut Level, now: u64) -> Vec<Step> {
        let mut steps = Vec::new();
        loop {
            let mut effects = Vec::new();
            let w = self.program.run(&mut effects);
            for e in effects {
                match e {
                    adv::Effect::Voice { bundle, asset, .. } => steps.push(Step::Voice(bundle, asset)),
                    adv::Effect::VoiceStop(_) => steps.push(Step::StopVoice),
                    adv::Effect::Unported(name, args) => self.log.push(format!("SATD-amanatsu-port: ADV {name} {args:?}")),
                    adv::Effect::Face { values, .. } => steps.push(Step::Face(values)),
                    adv::Effect::Expression { id, .. } => steps.push(Step::Expression(id)),
                    adv::Effect::Fade { .. } => {}
                    e => steps.push(Step::Chara(e)),
                }
            }
            match &w {
                Wait::Text { speaker, text } => self.show_text(level, speaker, text),
                Wait::Choice(items) => {
                    let items = items.clone();
                    self.show_choices(level, &items);
                }
                Wait::Time(s) => self.wait_until = now + (s * 1000.) as u64,
                Wait::Voice => self.wait_until = now + 300,
                Wait::Open(_, name) => {
                    match load_scenario(unity, &self.index, name) {
                        Ok(list) => {
                            self.program.call(name, list);
                            continue;
                        }
                        Err(e) => self.log.push(format!("ADV Open {name}: {e:#}")),
                    }
                }
                Wait::End => {
                    if let Some(next) = self.queue.pop() {
                        match load_scenario(unity, &self.index, &next) {
                            Ok(list) => {
                                self.program.open(&next, list);
                                continue;
                            }
                            Err(e) => self.log.push(format!("ADV {next}: {e:#}")),
                        }
                    }
                    steps.push(Step::End);
                }
            }
            self.wait = w;
            return steps;
        }
    }

    /// A frame: timed waits resume by themselves.
    pub fn tick(&mut self, unity: &mut Unity, level: &mut Level, now: u64) -> Vec<Step> {
        self.update_reveal(level, now);
        match self.wait {
            Wait::Time(_) | Wait::Voice if now >= self.wait_until => self.advance(unity, level, now),
            _ => Vec::new(),
        }
    }

    /// A click: the message advances; a choice button picks its tag.
    pub fn click(&mut self, unity: &mut Unity, level: &mut Level, node: Option<usize>, now: u64) -> Vec<Step> {
        match &self.wait {
            // A click during the reveal shows the whole message.
            Wait::Text { text, .. } if self.reveal.is_some() => {
                let text = text.clone();
                self.reveal = None;
                if let Some(m) = self.message_node {
                    level.states.entry(m).or_default().text = Some(text);
                }
                Vec::new()
            }
            Wait::Text { .. } => self.advance(unity, level, now),
            Wait::Choice(_) => {
                let Some(n) = node else { return Vec::new() };
                let mut cur = Some(n);
                let mut tag = None;
                while let Some(c) = cur {
                    if let Some(i) = self.choice_nodes.iter().position(|(x, _)| *x == c) {
                        tag = Some(self.choice_nodes[i].1.clone());
                        self.selected = Some(i);
                        break;
                    }
                    cur = level.scene.nodes[c].parent;
                }
                let Some(tag) = tag else { return Vec::new() };
                self.clear_choices(level);
                self.program.choose(&tag);
                self.advance(unity, level, now)
            }
            _ => Vec::new(),
        }
    }
}
