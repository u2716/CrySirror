//! Reader save state behind the native dat/mdat/tdat object ABI.
//!
//! The IMOSAVE110 envelope, slot names, metadata strings, JPEG thumbnail,
//! checksum and lane cipher stay native (save_metadata.rs). Its body is this
//! tagged, versioned CrySirror QLIE reader state. It is never presented as an
//! original 6D5218 stream; such a body is rejected with an explicit message.
//!
//! The state follows the native save scope (6D5218/6D4D24): only the game
//! script instance. That is its reader context and waits, its gvar bank, its
//! objects (images, buttons, messages, sounds, logs) and, when requested by
//! flag bit 0 (5D5B40), the mdat memory-save store. System-thread UI and
//! svar remain live; svar/read logs belong to SystemData.dat.
use super::*;
use crysirror_core::service::{AudioPreparePhase, AudioRequest};

pub(super) const BODY_TAG: &[u8; 16] = b"CrySirrorQLIEvm\0";
const VERSION: u32 = 1;
const BODY_LIMIT: usize = 48 * 1024 * 1024;
/// Engine-wide singletons are not part of the game script instance.
const GLOBAL_OBJECTS: &[&str] = &["cfg", "dat", "sdat", "mdat", "tdat", "dlgs", "misc", "math"];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedObject {
    kind: String,
    created: bool,
    visible: bool,
    position: [i32; 3],
    locked: bool,
    properties: BTreeMap<String, Vec<Value>>,
    lines: Vec<String>,
    table: BTreeMap<String, String>,
    regex_rules: BTreeMap<String, String>,
    asset: Option<String>,
    motion: usize,
    /// Visual age at save time; absolute session clocks are never stored.
    age: u64,
    offset: u64,
    gain: f32,
    skip_flags: Vec<bool>,
    jump_targets: BTreeMap<u8, u8>,
    animation_clocks: BTreeMap<usize, animation_clock::Clock>,
    events: Vec<((usize, usize, usize), (usize, u64))>,
    wave_channel: Option<u32>,
    wave_multiplier: Option<f32>,
    message: Option<message::Message>,
    generator: Option<generator::Group>,
    colour: Option<[u8; 3]>,
    /// Sampled engine state of the object's root layer (and a camera's
    /// colour overlay): pose, opacity, blend, filter and clip set by methods.
    layer: Option<serde_json::Value>,
    overlay: Option<serde_json::Value>,
}
const LAYER_FIELDS: &[(&str, &str)] = &[
    ("rect", "set_rect"),
    ("transform", "set_transform"),
    ("opacity", "set_opacity"),
    ("blend", "set_blend"),
    ("filter", "set_filter"),
    ("clip", "set_clip"),
];
fn layer_state(h: &mut dyn HostServices, client: ClientId, id: u32) -> Result<Option<serde_json::Value>> {
    let Ok(bytes) = h.query_engine(client, crysirror_core::service::EngineQuery::Layer(id)) else {
        return Ok(None);
    };
    let spec: serde_json::Value = serde_json::from_slice(&bytes)?;
    let mut out = serde_json::Map::new();
    for (field, _) in LAYER_FIELDS {
        out.insert((*field).into(), spec[*field].clone());
    }
    out.insert("order".into(), spec["order"].clone());
    Ok(Some(serde_json::Value::Object(out)))
}
fn apply_layer_state(id: u32, state: &serde_json::Value, commands: &mut Vec<serde_json::Value>) {
    for (field, op) in LAYER_FIELDS {
        let value = &state[*field];
        if value.is_null() && !matches!(*field, "filter" | "clip") {
            continue;
        }
        let key = match *field {
            "rect" => "rect",
            "transform" => "transform",
            "opacity" => "opacity",
            "blend" => "blend",
            "filter" => "filter",
            _ => "clip",
        };
        commands.push(json!({"op":op,"id":id,key:value}));
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedHost {
    wait: Option<Wait>,
    stop_poll: Option<(usize, StopWait)>,
    frame_event: Option<FrameEvent>,
    selection: Vec<String>,
    scope_next: usize,
    button_callbacks: Vec<String>,
    skip_until: u64,
    /// Game visual clock at which the above absolute deadlines were taken.
    paused_at: u64,
}
impl SavedHost {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        wait: Option<Wait>,
        stop_poll: Option<(usize, StopWait)>,
        frame_event: Option<FrameEvent>,
        selection: Vec<String>,
        scope_next: usize,
        button_callbacks: Vec<String>,
        skip_until: u64,
        paused_at: u64,
    ) -> Self {
        Self {
            wait,
            stop_poll,
            frame_event,
            selection,
            scope_next,
            button_callbacks,
            skip_until,
            paused_at,
        }
    }
    #[allow(clippy::type_complexity)]
    pub(super) fn rebased(
        mut self,
        to: u64,
    ) -> (
        Option<Wait>,
        Option<(usize, StopWait)>,
        Option<FrameEvent>,
        Vec<String>,
        usize,
        Vec<String>,
        u64,
    ) {
        let from = self.paused_at;
        shift_wait(&mut self.wait, from, to);
        if let Some((_, stop)) = &mut self.stop_poll {
            stop.deadline = stop.deadline.map(|t| shift(t, from, to));
        }
        if let Some(event) = &mut self.frame_event {
            shift_wait(&mut event.wait, from, to);
            if let Some((_, stop)) = &mut event.poll {
                stop.deadline = stop.deadline.map(|t| shift(t, from, to));
            }
        }
        (
            self.wait,
            self.stop_poll,
            self.frame_event,
            self.selection,
            self.scope_next,
            self.button_callbacks,
            if self.skip_until > from {
                shift(self.skip_until, from, to)
            } else {
                0
            },
        )
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StackEntry {
    pub name: String,
    pub envelope: Vec<u8>,
}
#[derive(Clone, Default)]
pub(super) struct StackStore {
    pub limit: Option<usize>,
    pub entries: Vec<StackEntry>,
}
impl StackStore {
    /// 5D585C: replace in place, else append and trim the oldest (5D5B04).
    pub(super) fn put(&mut self, entry: StackEntry) {
        if let Some(old) = self.entries.iter_mut().find(|e| e.name == entry.name) {
            *old = entry;
            return;
        }
        self.entries.push(entry);
        self.trim();
    }
    pub(super) fn trim(&mut self) {
        let limit = self.limit.unwrap_or(16);
        if limit > 0 && self.entries.len() > limit {
            let excess = self.entries.len() - limit;
            self.entries.drain(..excess);
        }
    }
    pub(super) fn get(&self, name: &str) -> Option<&StackEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedState {
    version: u32,
    machine: serde_json::Value,
    host: Option<SavedHost>,
    objects: BTreeMap<String, SavedObject>,
    memory: Option<Vec<StackEntry>>,
}
/// Objects whose resources and engine layers are being rebuilt after load.
pub(super) struct Rebuild {
    pending: std::collections::VecDeque<(String, Phase)>,
    /// Fields that load_image's native reset discards, reinstated after it.
    stash: BTreeMap<String, Stash>,
    layers: BTreeMap<String, (Option<serde_json::Value>, Option<serde_json::Value>)>,
}
struct Stash {
    visible: bool,
    locked: bool,
    lines: Vec<String>,
    table: BTreeMap<String, String>,
    regex_rules: BTreeMap<String, regex::Regex>,
    motion: usize,
    started: u64,
    offset: u64,
    skip_flags: [bool; 32],
    jump_targets: BTreeMap<u8, u8>,
    animation_clocks: BTreeMap<usize, animation_clock::Clock>,
    events: BTreeMap<(usize, usize, usize), (usize, u64)>,
    wave_channel: Option<u32>,
    wave_multiplier: Option<f32>,
    colour: Option<[u8; 3]>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Asset,
    Replacements,
    Present,
    Sound,
}
pub(super) fn shift(t: u64, from: u64, to: u64) -> u64 {
    if to >= from {
        t.saturating_add(to - from)
    } else {
        t.saturating_sub(from - to)
    }
}
pub(super) fn shift_wait(wait: &mut Option<Wait>, from: u64, to: u64) {
    match wait {
        Some(Wait::Clock(t) | Wait::FixedClock(t)) => *t = shift(*t, from, to),
        Some(Wait::Stop(s)) => s.deadline = s.deadline.map(|t| shift(t, from, to)),
        Some(Wait::Time { deadline, .. }) => *deadline = shift(*deadline, from, to),
        Some(Wait::Button { deadline }) => *deadline = deadline.map(|t| shift(t, from, to)),
        Some(Wait::Sound { .. } | Wait::Message | Wait::Wave { .. }) | None => {}
    }
}
/// Rename every logical object identity in a JSON tree, including map keys.
fn rename(value: &mut serde_json::Value, map: &BTreeMap<String, String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(new) = map.get(s.as_str()) {
                *s = new.clone();
            }
        }
        serde_json::Value::Array(values) => values.iter_mut().for_each(|v| rename(v, map)),
        serde_json::Value::Object(entries) => {
            let old = std::mem::take(entries);
            for (key, mut v) in old {
                rename(&mut v, map);
                entries.insert(map.get(&key).cloned().unwrap_or(key), v);
            }
        }
        _ => {}
    }
}
impl QlieVm {
    pub(super) fn is_game_save_object(id: &str, o: &Object) -> bool {
        o.thread == ThreadKind::Game && !GLOBAL_OBJECTS.contains(&id)
    }
    fn game_visual_now(&self, now: u64) -> u64 {
        if self.system_input_active || self.host_thread == ThreadKind::System {
            self.game_pause_ms.unwrap_or(now)
        } else {
            now
        }
    }
    /// Serialize the game script instance at a native safepoint. A save from
    /// the GameThread resumes after the saving instruction.
    pub(super) fn capture_state(
        &self,
        h: &mut dyn HostServices,
        m: &Machine,
        flags: i32,
    ) -> Result<Vec<u8>> {
        let now = h.now_ms();
        let game_now = self.game_visual_now(now);
        let on_game = m.thread_kind() == ThreadKind::Game;
        let machine: serde_json::Value = serde_json::from_slice(&m.game_checkpoint(on_game)?)?;
        let host = if on_game {
            None
        } else {
            self.suspended_host.as_ref().map(|c| c.to_saved())
        };
        let mut objects = BTreeMap::new();
        for (id, o) in &self.objects {
            if !Self::is_game_save_object(id, o) {
                continue;
            }
            let mut properties = o.properties.clone();
            if o.created {
                properties.insert("save/created".into(), vec![Value::Int(1)]);
            }
            // TssSaveData: PlayState, SeekPosition, LoopEnable/LoopCount. A
            // voice still sounding (one-shot or looped) is re-armed at its
            // position; a finished one is not.
            if o.kind == "snd" && o.channel != 0 {
                let info = h.audio_info(self.client, o.channel)?.filter(|i| i.source_playing);
                if let Some(info) = info {
                    let position = (info.position_frames as f64 * 1000. / f64::from(info.source_rate.max(1))) as i32;
                    let loops = match (info.looping, info.loop_passes) {
                        (false, _) => 0,
                        (true, None) => -1,
                        (true, Some(passes)) => passes.saturating_sub(1).min(i32::MAX as u32) as i32,
                    };
                    properties.insert("save/sound_loops".into(), vec![Value::Int(loops)]);
                    properties.insert("save/sound_position".into(), vec![Value::Int(position)]);
                    if info.paused {
                        properties.insert("save/sound_paused".into(), vec![Value::Int(1)]);
                    }
                } else {
                    properties.remove("save/sound_loops");
                }
            }
            let mut message = o.message.clone();
            if let Some(message) = message.as_mut().filter(|m| m.created) {
                properties.insert("save/message_created".into(), vec![Value::Int(1)]);
                let layer: crysirror_scene::layer::LayerSpec = serde_json::from_slice(
                    &h.query_engine(self.client, crysirror_core::service::EngineQuery::Layer(message.layer))?,
                )?;
                let visible = layer.paint.glyph_reveal_status().map_or(usize::MAX, |s| s.visible);
                message.prepare_save(visible);
            }
            let mut generator = o.generator.clone();
            if let Some(group) = &mut generator {
                group.retire_registration();
                group.to_ages(game_now);
            }
            objects.insert(
                id.clone(),
                SavedObject {
                    kind: o.kind.clone(),
                    created: o.created,
                    visible: o.visible,
                    position: o.position,
                    locked: o.locked,
                    properties,
                    lines: o.lines.clone(),
                    table: o.table.clone(),
                    regex_rules: o
                        .regex_rules
                        .iter()
                        .map(|(k, r)| (k.clone(), r.as_str().to_owned()))
                        .collect(),
                    // A live screen capture is a device-owned frame, not a
                    // resource path; it cannot be re-read from the archive.
                    asset: o
                        .asset
                        .clone()
                        .filter(|a| !a.to_ascii_lowercase().starts_with("$$$")),
                    motion: o.motion,
                    age: game_now.saturating_sub(o.started),
                    offset: o.offset,
                    gain: o.gain,
                    skip_flags: o.skip_flags.to_vec(),
                    jump_targets: o.jump_targets.clone(),
                    animation_clocks: o.animation_clocks.clone(),
                    events: o.events.iter().map(|(k, v)| (*k, *v)).collect(),
                    wave_channel: o.wave_channel,
                    wave_multiplier: o.wave_multiplier,
                    message,
                    generator,
                    colour: o.colour,
                    layer: if o.created {
                        layer_state(h, self.client, o.layer)?
                    } else {
                        None
                    },
                    overlay: if matches!(o.kind.as_str(), "cam" | "camex")
                        && o.properties.contains_key("col")
                    {
                        layer_state(h, self.client, o.layer + 0x20000000)?
                    } else {
                        None
                    },
                },
            );
        }
        let memory = (flags & 1 != 0)
            .then(|| self.stacks.get("mdat").map(|s| s.entries.clone()).unwrap_or_default());
        let state = SavedState {
            version: VERSION,
            machine,
            host,
            objects,
            memory,
        };
        let mut out = BODY_TAG.to_vec();
        out.extend_from_slice(&serde_json::to_vec(&state)?);
        ensure!(out.len() <= BODY_LIMIT, "QLIE reader save body budget");
        Ok(out)
    }
    /// Validate and install a reader save. Resources are then rebuilt by
    /// `poll_rebuild` on later ticks; the caller's instruction stays pending.
    pub(super) fn begin_restore(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        body: &[u8],
    ) -> Result<()> {
        ensure!(body.len() <= BODY_LIMIT, "QLIE reader save body budget");
        let Some(json) = body.strip_prefix(BODY_TAG.as_slice()) else {
            bail!("QLIE original-engine 6D5218 save body import is not implemented");
        };
        let mut state: SavedState = serde_json::from_slice(json)?;
        ensure!(state.version == VERSION, "unsupported QLIE reader save version");
        ensure!(state.objects.len() <= 4096, "QLIE reader save object budget");
        // Logical identities are monotonic per session. Rename every saved
        // object/N so that it can never alias a live System object.
        let mut names = BTreeMap::new();
        for id in state.objects.keys().cloned().collect::<Vec<_>>() {
            if id.starts_with("object/") {
                names.insert(id, m.allocate_object_identity()?);
            }
        }
        if let Some(types) = state.machine.get("object_types").and_then(|v| v.as_object()) {
            for id in types.keys() {
                if id.starts_with("object/") && !names.contains_key(id) {
                    names.insert(id.clone(), m.allocate_object_identity()?);
                }
            }
        }
        rename(&mut state.machine, &names);
        let mut objects = BTreeMap::new();
        for (id, mut saved) in std::mem::take(&mut state.objects) {
            for values in saved.properties.values_mut() {
                for value in values {
                    if let Value::Text(s) = value {
                        if let Some(new) = names.get(s.as_str()) {
                            *s = new.clone();
                        }
                    }
                }
            }
            objects.insert(names.get(&id).cloned().unwrap_or(id), saved);
        }
        let mut host = state.host;
        if let Some(host) = &mut host {
            let mut tree = serde_json::to_value(&*host)?;
            rename(&mut tree, &names);
            *host = serde_json::from_value(tree)?;
        }
        // Compile retained regex rules before replacing anything.
        let mut rules = BTreeMap::new();
        for (id, saved) in &objects {
            let mut compiled = BTreeMap::new();
            for (key, pattern) in &saved.regex_rules {
                compiled.insert(
                    key.clone(),
                    regex::RegexBuilder::new(pattern)
                        .size_limit(1024 * 1024)
                        .build()?,
                );
            }
            rules.insert(id.clone(), compiled);
        }
        m.restore_game_checkpoint(&serde_json::to_vec(&state.machine)?)?;
        // Retire the current game instance's objects and their device state.
        let current = self
            .objects
            .iter()
            .filter(|(id, o)| Self::is_game_save_object(id, o))
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        // Layer removal takes a whole subtree. Engine commands are batched,
        // so nested object roots are found from the pre-removal hierarchy.
        let removed: BTreeSet<u32> = current
            .iter()
            .filter_map(|id| self.objects.get(id))
            .filter(|o| o.created)
            .map(|o| o.layer)
            .collect();
        let mut nested = BTreeSet::new();
        for &layer in &removed {
            let mut cursor = layer;
            for _ in 0..64 {
                let Ok(bytes) = h.query_engine(
                    self.client,
                    crysirror_core::service::EngineQuery::Layer(cursor),
                ) else {
                    break;
                };
                let info: serde_json::Value = serde_json::from_slice(&bytes)?;
                let Some(parent) = info["parent"].as_u64().map(|p| p as u32) else {
                    break;
                };
                if removed.contains(&parent) {
                    nested.insert(layer);
                    break;
                }
                cursor = parent;
            }
        }
        for id in current {
            // Scrollbar parts may already have been retired with their bar.
            if !self.objects.contains_key(&id) {
                continue;
            }
            self.cancel_native_position(h, &id)?;
            let mut o = self.objects.remove(&id).unwrap();
            if nested.contains(&o.layer) {
                o.created = false;
            }
            self.free_object(h, o)?;
        }
        let now = h.now_ms();
        let on_game = m.thread_kind() == ThreadKind::Game;
        let game_now = if on_game { now } else { self.game_pause_ms.unwrap_or(now) };
        let mut rebuild = std::collections::VecDeque::new();
        let mut cameras = Vec::new();
        let mut layer_states = BTreeMap::new();
        for (id, saved) in objects {
            let layer = self.allocate_layer()?;
            let mut message = saved.message;
            if let Some(message) = &mut message {
                message.layer = self.allocate_layer()?;
                message.reset_for_restore();
            }
            let mut generator = saved.generator;
            if let Some(group) = &mut generator {
                group.layer = layer;
                group.from_ages(game_now);
            }
            let mut skip_flags = [false; 32];
            for (i, v) in saved.skip_flags.iter().take(32).enumerate() {
                skip_flags[i] = *v;
            }
            let has_asset = saved.asset.is_some() && !matches!(saved.kind.as_str(), "snd" | "theora");
            // Saves before positions were recorded re-arm only looped voices.
            let loops = saved
                .properties
                .get("save/sound_loops")
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .filter(|loops| *loops != 0 || saved.properties.contains_key("save/sound_position"));
            let present = saved.properties.contains_key("save/created")
                || saved.properties.contains_key("save/message_created");
            let camera = matches!(saved.kind.as_str(), "cam" | "camex");
            layer_states.insert(id.clone(), (saved.layer, saved.overlay));
            let o = Object {
                thread: ThreadKind::Game,
                kind: saved.kind,
                layer,
                created: false,
                visible: saved.visible,
                position: saved.position,
                locked: saved.locked,
                properties: saved.properties,
                lines: saved.lines,
                table: saved.table,
                regex_rules: rules.remove(&id).unwrap_or_default(),
                asset: saved.asset,
                motion: saved.motion,
                started: game_now.saturating_sub(saved.age),
                offset: saved.offset,
                gain: saved.gain,
                skip_flags,
                jump_targets: saved.jump_targets,
                animation_clocks: saved.animation_clocks,
                events: saved.events.into_iter().collect(),
                wave_channel: saved.wave_channel,
                wave_multiplier: saved.wave_multiplier,
                message,
                generator,
                colour: saved.colour,
                ..Default::default()
            };
            let kind = o.kind.clone();
            self.objects.insert(id.clone(), o);
            if camera {
                // Camera parents are reinstated before the objects they hold.
                if present {
                    cameras.push((id.clone(), Phase::Present));
                }
                continue;
            }
            if has_asset {
                rebuild.push_back((id.clone(), Phase::Asset));
            } else if present {
                rebuild.push_back((id.clone(), Phase::Present));
            }
            if kind == "snd" && loops.is_some() {
                rebuild.push_back((id, Phase::Sound));
            }
        }
        if let Some(memory) = state.memory {
            self.stacks.entry("mdat".into()).or_default().entries = memory;
        }
        if on_game {
            self.wait = None;
            self.stop_poll = None;
            self.frame_event = None;
            self.selection.clear();
            self.scope_next = 0;
            self.button_callbacks.clear();
        } else {
            let paused_at = self.game_pause_ms.unwrap_or(now);
            let context = match host {
                Some(saved) => thread::HostContext::from_saved(saved, paused_at),
                None => thread::HostContext::empty(paused_at),
            };
            self.suspended_host = Some(context);
        }
        for camera in cameras.into_iter().rev() {
            rebuild.push_front(camera);
        }
        self.rebuild = Some(Rebuild {
            pending: rebuild,
            stash: BTreeMap::new(),
            layers: layer_states,
        });
        Ok(())
    }
    pub(super) fn allocate_layer(&mut self) -> Result<u32> {
        let layer = self.next_layer;
        self.next_layer = self
            .next_layer
            .checked_add(1)
            .context("QLIE layer identity exhausted")?;
        Ok(layer)
    }
    /// Rebuild restored objects through the same resumable resource paths
    /// their native methods use. True once every object is presented.
    pub(super) fn poll_rebuild(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        loop {
            let Some(rebuild) = self.rebuild.as_mut() else {
                return Ok(true);
            };
            let Some((id, phase)) = rebuild.pending.front().cloned() else {
                self.rebuild = None;
                return Ok(true);
            };
            if !self.objects.contains_key(&id) {
                self.rebuild.as_mut().unwrap().pending.pop_front();
                continue;
            }
            let thread = self.host_thread;
            // Resource and presentation calls act on the restored GameThread.
            self.host_thread = ThreadKind::Game;
            let step = self.rebuild_step(h, &id, phase);
            self.host_thread = thread;
            let Some(next) = step? else {
                return Ok(false);
            };
            let rebuild = self.rebuild.as_mut().unwrap();
            rebuild.pending.pop_front();
            if let Some(next) = next {
                rebuild.pending.push_front((id, next));
            }
        }
    }
    /// None: still pending. Some(next): this phase completed.
    fn rebuild_step(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        phase: Phase,
    ) -> Result<Option<Option<Phase>>> {
        let o = &self.objects[id];
        let present = o.created_before_restore();
        match phase {
            Phase::Asset => {
                let path = o.asset.clone().unwrap();
                let rebuild = self.rebuild.as_mut().unwrap();
                if !rebuild.stash.contains_key(id) {
                    let o = self.objects.get_mut(id).unwrap();
                    let stash = Stash {
                        visible: o.visible,
                        locked: o.locked,
                        lines: std::mem::take(&mut o.lines),
                        table: std::mem::take(&mut o.table),
                        regex_rules: std::mem::take(&mut o.regex_rules),
                        motion: o.motion,
                        started: o.started,
                        offset: o.offset,
                        skip_flags: o.skip_flags,
                        jump_targets: std::mem::take(&mut o.jump_targets),
                        animation_clocks: std::mem::take(&mut o.animation_clocks),
                        events: std::mem::take(&mut o.events),
                        wave_channel: o.wave_channel,
                        wave_multiplier: o.wave_multiplier,
                        colour: o.colour,
                    };
                    o.asset = None;
                    self.rebuild.as_mut().unwrap().stash.insert(id.into(), stash);
                }
                if !self.load_image(h, id, &path)? {
                    return Ok(None);
                }
                let stash = self.rebuild.as_mut().unwrap().stash.remove(id).unwrap();
                let o = self.objects.get_mut(id).unwrap();
                o.visible = stash.visible;
                o.locked = stash.locked;
                o.lines = stash.lines;
                o.table = stash.table;
                o.regex_rules = stash.regex_rules;
                o.motion = stash.motion;
                o.started = stash.started;
                o.offset = stash.offset;
                o.skip_flags = stash.skip_flags;
                o.jump_targets = stash.jump_targets;
                o.animation_clocks = stash.animation_clocks;
                o.events = stash.events;
                o.wave_channel = stash.wave_channel;
                o.wave_multiplier = stash.wave_multiplier;
                o.colour = stash.colour;
                Ok(Some(Some(Phase::Replacements)))
            }
            Phase::Replacements => {
                let replacements = o
                    .properties
                    .iter()
                    .filter_map(|(k, v)| {
                        let slot = k.strip_prefix("save/replace/")?.parse::<usize>().ok()?;
                        Some((slot, v.first()?.text()))
                    })
                    .collect::<Vec<_>>();
                for (slot, path) in replacements {
                    let done = format!("save/rebuilt/{slot}");
                    if self.objects[id].properties.contains_key(&done) {
                        continue;
                    }
                    if !self.replace_image(h, id, slot, &path)? {
                        return Ok(None);
                    }
                    self.objects
                        .get_mut(id)
                        .unwrap()
                        .properties
                        .insert(done, vec![Value::Int(1)]);
                }
                let o = self.objects.get_mut(id).unwrap();
                o.properties.retain(|k, _| !k.starts_with("save/rebuilt/"));
                Ok(Some(present.then_some(Phase::Present)))
            }
            Phase::Present if matches!(o.kind.as_str(), "cam" | "camex") => {
                self.present_camera(h, id)?;
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .remove("save/created");
                Ok(Some(None))
            }
            Phase::Present => {
                let o = &self.objects[id];
                let message = o.properties.contains_key("save/message_created")
                    && o.message.is_some();
                let created = o.properties.contains_key("save/created");
                if message {
                    self.objects
                        .get_mut(id)
                        .unwrap()
                        .message
                        .as_mut()
                        .unwrap()
                        .reset_for_restore();
                    if !self.rebuild_picture_sheets(h, id)? {
                        return Ok(None);
                    }
                    self.publish_message(h, id)?;
                } else if created {
                    self.put_parent(h, id)?;
                }
                if created {
                    if let Some(state) = self
                        .rebuild
                        .as_mut()
                        .and_then(|r| r.layers.remove(id))
                        .and_then(|(layer, _)| layer)
                    {
                        let mut commands = Vec::new();
                        apply_layer_state(self.objects[id].layer, &state, &mut commands);
                        self.submit(h, json!({"layers":commands}))?;
                    }
                }
                let o = self.objects.get_mut(id).unwrap();
                o.properties.remove("save/created");
                o.properties.remove("save/message_created");
                Ok(Some(None))
            }
            Phase::Sound => {
                let loops = o
                    .properties
                    .get("save/sound_loops")
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(-1);
                if !o.properties.contains_key("save/sound_started") {
                    if !self.play_sound(h, id, loops)? {
                        return Ok(None);
                    }
                    self.objects
                        .get_mut(id)
                        .unwrap()
                        .properties
                        .insert("save/sound_started".into(), vec![Value::Int(1)]);
                }
                let o = &self.objects[id];
                let position = o
                    .properties
                    .get("save/sound_position")
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(0)
                    .max(0) as u64;
                let channel = o.channel;
                if position > 0 && channel != 0 {
                    if let Some((owner, ticket)) = self.audio_seek.clone() {
                        ensure!(owner == id, "QLIE restored sound seek owner");
                        let status = h.audio_seek_preparation(self.client, ticket)?;
                        match status.phase {
                            AudioPreparePhase::Pending => return Ok(None),
                            AudioPreparePhase::Ready => {
                                h.release_audio_seek(self.client, ticket)?;
                                self.audio_seek = None;
                            }
                            AudioPreparePhase::Failed => {
                                h.release_audio_seek(self.client, ticket)?;
                                self.audio_seek = None;
                                bail!("QLIE restored sound seek failed: {:?}", status.error);
                            }
                        }
                    } else if let Some(info) = h.audio_info(self.client, channel)? {
                        let frame = position.saturating_mul(u64::from(info.source_rate)) / 1000;
                        let ticket = h.begin_audio_seek(self.client, channel, frame)?;
                        self.audio_seek = Some((id.to_owned(), ticket));
                        return Ok(None);
                    }
                }
                let paused = self.objects[id].properties.contains_key("save/sound_paused");
                if paused && channel != 0 {
                    h.submit_audio(self.client, AudioRequest::Pause { channel, paused: true, fade_ms: 0. })?;
                }
                let o = self.objects.get_mut(id).unwrap();
                for key in ["save/sound_started", "save/sound_position", "save/sound_paused"] {
                    o.properties.remove(key);
                }
                Ok(Some(None))
            }
        }
    }
    /// camera_method's root layer, generator child and colour overlay.
    fn present_camera(&mut self, h: &mut dyn HostServices, id: &str) -> Result<()> {
        let layer = self.objects[id].layer;
        let root = self.root_layer_for(ThreadKind::Game);
        let [w, height] = self.config.canvas;
        let (state, overlay) = self
            .rebuild
            .as_mut()
            .and_then(|r| r.layers.remove(id))
            .unwrap_or((None, None));
        let mut commands = vec![json!({"op":"put","layer":{"id":layer,"parent":root,"rect":[0,0,w,height],"order":[-2000,0],"transform":{"pivot_x":w as f32/2.,"pivot_y":height as f32/2.},"paint":{"kind":"solid","color":[0,0,0,0]}}})];
        if let Some(state) = &state {
            apply_layer_state(layer, state, &mut commands);
        }
        if self.objects[id].generator.is_some() {
            let generator = self.allocate_layer()?;
            commands.push(json!({"op":"put","layer":{"id":generator,"parent":layer,"rect":[0,0,w,height],"paint":{"kind":"solid","color":[0,0,0,0]},"transform":{"pivot_x":w as f32/2.,"pivot_y":height as f32/2.}}}));
            self.objects
                .get_mut(id)
                .unwrap()
                .generator
                .as_mut()
                .unwrap()
                .layer = generator;
        }
        if let Some(overlay) = overlay {
            commands.push(json!({"op":"put","layer":{"id":layer+0x20000000,"parent":root,"rect":[0,0,w,height],"order":overlay["order"],"opacity":overlay["opacity"],"blend":overlay["blend"],"paint":{"kind":"solid","color":[255,255,255,255]},"filter":overlay["filter"]}}));
        }
        self.submit(h, json!({"layers":commands}))?;
        self.objects.get_mut(id).unwrap().created = true;
        Ok(())
    }
    /// Capture the GameThread composition below `priority` for a thumbnail.
    /// The pixels stay engine-owned until the shared canvas image job ends.
    pub(super) fn begin_thumbnail(&mut self, h: &mut dyn HostServices, priority: i32) -> Result<Option<crysirror_core::image_processing::ImageJob>> {
        //6E22A0: zero or an out-of-range priority selects 0xFFFF.
        let priority = if priority <= 0 || priority > 4095 { 0xffff } else { priority };
        let canvas = 0x13000000u32;
        let commands = vec![
            json!({"op":"create","id":canvas,"width":self.config.canvas[0],"height":self.config.canvas[1]}),
            json!({"op":"clear","id":canvas,"color":[0,0,0,255]}),
            json!({"op":"copy_subtree","source":self.root_layer_for(ThreadKind::Game),"destination":canvas,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"position":[0,0]}),
        ];
        let old = self.host_thread;
        self.host_thread = ThreadKind::Game;
        let ready = self.capture_scene(h, commands, priority);
        self.host_thread = old;
        if !ready? {
            // Media frames are not ready; retry next tick without a canvas.
            self.submit(h, json!({"canvases":[{"op":"destroy","id":canvas}]})).ok();
            return Ok(None);
        }
        Ok(Some(h.begin_canvas_image(self.client, canvas)?))
    }
    pub(super) fn end_thumbnail(&mut self, h: &mut dyn HostServices) -> Result<()> {
        self.submit(h, json!({"canvases":[{"op":"destroy","id":0x13000000u32}]}))
    }
    /// Thumbnail (6E22A0) uses System `tz` priority; `$8` skips it.
    pub(super) fn thumbnail_priority(&self) -> i32 {
        self.objects
            .get(&Self::builtin("sys", ThreadKind::Game))
            .and_then(|o| o.properties.get("tz"))
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0)
    }
    /// 5054F4 ThumWidth/ThumHeight (defaults 128x96).
    pub(super) fn thumbnail_size(&self) -> [u32; 2] {
        let setting = |name: &str, default: u32| {
            self.text
                .get("system/etc/game.txt")
                .into_iter()
                .flat_map(|s| s.lines())
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
                .and_then(|(_, value)| crate::expression::parse_integer(value.trim()))
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| (1..=4096).contains(v))
                .unwrap_or(default)
        };
        [setting("ThumWidth", 128), setting("ThumHeight", 96)]
    }
    /// 6B7218: seven System metadata strings (sys n/n2) plus an empty eighth.
    pub(super) fn save_metadata_strings(&self) -> [String; 8] {
        let sys = self.objects.get(&Self::builtin("sys", ThreadKind::Game));
        std::array::from_fn(|i| {
            if i == 7 {
                return String::new();
            }
            sys.and_then(|o| o.properties.get(&format!("meta/{i}")))
                .and_then(|v| v.first())
                .map(Value::text)
                .unwrap_or_default()
        })
    }
}
impl Object {
    fn created_before_restore(&self) -> bool {
        self.properties.contains_key("save/created")
            || self.properties.contains_key("save/message_created")
    }
}
