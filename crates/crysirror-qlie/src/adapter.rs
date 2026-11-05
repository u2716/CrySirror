//! Thin QLIE semantic bridge. All retained images, draw trees, interpolation,
//! media clocks and audio voices are owned by CrySirror's shared services.
use crate::{
    expression::Value,
    machine::Machine,
    program::{Instruction, Program},
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{HostServices, InputFrame, UploadedImage},
    state::ClientId,
    vm::{VmAdapter, VmYield},
    Error,
};
use crysirror_runtime::{model::ClientSpec, runtime::VmFactory, AdapterRegistry};
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod animated_transition;
mod animation;
mod animation_clock;
mod camera;
mod capture;
mod dialogs;
mod filters;
mod generator;
mod message;
mod objects;
mod render;
mod replacement;
mod resources;
mod save_metadata;
mod visual;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub script: String,
    pub canvas: [u32; 2],
    pub assets: BTreeMap<String, String>,
    /// Original script parameters, distinct from the desktop host CLI flags.
    #[serde(default)]
    pub parameters: Vec<String>,
}
#[derive(Default)]
struct EmbeddedMedia {
    id: u32,
    path: String,
    native_ms: u64,
    clock_ms: u64,
}
#[derive(Default)]
struct Object {
    kind: String,
    layer: u32,
    created: bool,
    visible: bool,
    position: [i32; 3],
    locked: bool,
    properties: BTreeMap<String, Vec<Value>>,
    lines: Vec<String>,
    table: BTreeMap<String, String>,
    asset: Option<String>,
    bundle: Option<resources::Bundle>,
    images: Vec<Option<UploadedImage>>,
    originals: BTreeMap<usize, UploadedImage>,
    preparing: bool,
    pending_blob: Option<(String, Arc<[u8]>)>,
    motion: usize,
    started: u64,
    offset: u64,
    children: Vec<u32>,
    published_children: Vec<serde_json::Value>,
    ui_rect: Option<[f32; 4]>,
    channel: u32,
    gain: f32,
    media: Option<u32>,
    embedded_media: BTreeMap<usize, EmbeddedMedia>,
    skip_flags: [bool; 32],
    jump_targets: BTreeMap<u8, u8>,
    animation_clocks: BTreeMap<usize, animation_clock::Clock>,
    events: BTreeMap<(usize, usize, usize), (usize, u64)>,
    // Pending publication belongs to a native channel, so a stop event can
    // cancel its decoder before it publishes a late, unwanted voice.
    audio_tickets: BTreeMap<u64, u32>,
    audio_channels: BTreeSet<u32>,
    hit_bounds: Option<[f32; 4]>,
    button_state: (bool, bool, u32),
    ui_registered: bool,
    message: Option<message::Message>,
    generator: Option<generator::Group>,
    capture_views: BTreeMap<usize, serde_json::Value>,
    colour: Option<[u8; 3]>,
}
#[derive(Clone)]
struct StopWait {
    label: String,
    deadline: Option<u64>,
}
#[derive(Clone)]
enum Wait {
    Clock(u64),
    Stop(StopWait),
    FixedClock(u64),
}
struct FrameEvent {
    depth: usize,
    wait: Option<Wait>,
    poll: Option<(usize, StopWait)>,
    scope: String,
    selection: Vec<String>,
    scope_next: usize,
}
pub struct QlieVm {
    client: ClientId,
    config: Config,
    pool: TaskPool,
    boot: Option<Task<resources::Prepared>>,
    machine: Option<Machine>,
    text: BTreeMap<String, String>,
    objects: BTreeMap<String, Object>,
    next_layer: u32,
    next_image: u32,
    load: Option<(String, Task<resources::Loaded>)>,
    image_job: Option<crysirror_core::image_processing::ImageJob>,
    wait: Option<Wait>,
    input: InputFrame,
    initialized: bool,
    capture: bool,
    second_capture: bool,
    // Native TImoGraphic +77==1 is a captured buffer awaiting a fade, distinct
    // from the lifetime of the engine Canvas allocation and a running fade.
    capture_waiting: bool,
    effect_priority: i32,
    effect_clock: bool,
    rule_fade: Option<(String, f32)>,
    animated_fade: Option<u64>,
    audio_ticket: Option<u64>,
    audio_seek: Option<(String, u64)>,
    api_calls: u64,
    error: Option<String>,
    stop_poll: Option<(usize, StopWait)>,
    media_ids: BTreeSet<u32>,
    file_query: Option<(String, Task<bool>)>,
    save_query: Option<(String, Task<save_metadata::ReadResult>)>,
    skip_until: u64,
    frame_event: Option<FrameEvent>,
    last_frame_event: Option<u64>,
    next_object_collection: u64,
    button_callbacks: std::collections::VecDeque<String>,
    regex_rules: BTreeMap<String, regex::Regex>,
    last_tick_ms: Option<u64>,
    tick_ms: u64,
    selection: Vec<String>,
    scope_next: usize,
    read_lines: BTreeSet<(String, usize)>,
    last_read: bool,
    dialog: Option<dialogs::TextDialog>,
    unavailable: BTreeSet<String>,
    random: Option<animation_clock::RandomTable>,
    random_seed: Option<Box<dyn crysirror_core::service::SessionSeedQuery>>,
}
impl QlieVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        ensure!(
            config.canvas.iter().all(|v| *v > 0 && *v <= 8192) && config.assets.len() <= 65536,
            "QLIE config limits"
        );
        ensure!(
            config.parameters.len() <= 256 && config.parameters.iter().all(|p| p.len() <= 16384),
            "QLIE parameter budget"
        );
        VfsPath::new(Mount::Asset, &config.script)?;
        for (path, value) in &config.assets {
            ensure!(
                crate::program::canonical(value)? == *path,
                "QLIE resource catalog key"
            );
            VfsPath::new(Mount::Asset, value)?;
        }
        Ok(Self {
            client,
            config,
            pool: TaskPool::new("qlie-resource", 2, 8)?,
            boot: None,
            machine: None,
            text: BTreeMap::new(),
            objects: BTreeMap::new(),
            next_layer: 2,
            next_image: 0,
            load: None,
            image_job: None,
            wait: None,
            input: InputFrame::default(),
            initialized: false,
            capture: false,
            second_capture: false,
            capture_waiting: false,
            effect_priority: 65535,
            effect_clock: false,
            rule_fade: None,
            animated_fade: None,
            audio_ticket: None,
            audio_seek: None,
            api_calls: 0,
            error: None,
            stop_poll: None,
            media_ids: BTreeSet::new(),
            file_query: None,
            save_query: None,
            skip_until: 0,
            frame_event: None,
            last_frame_event: None,
            next_object_collection: 0,
            button_callbacks: std::collections::VecDeque::new(),
            regex_rules: BTreeMap::new(),
            last_tick_ms: None,
            tick_ms: 0,
            selection: vec![],
            scope_next: 0,
            read_lines: BTreeSet::new(),
            last_read: false,
            dialog: None,
            unavailable: BTreeSet::new(),
            random: None,
            random_seed: None,
        })
    }
    fn submit(&mut self, h: &mut dyn HostServices, batch: serde_json::Value) -> Result<()> {
        h.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
        self.api_calls += 1;
        Ok(())
    }
    fn resolve(&self, m: &Machine, name: &str) -> Result<String> {
        let directory = m.frame.path.rsplit_once('/').map(|v| v.0).unwrap_or("");
        for path in [
            if directory.is_empty() {
                name.to_owned()
            } else {
                format!("{directory}/{name}")
            },
            name.to_owned(),
        ] {
            if let Ok(logical) = crate::program::canonical(&path) {
                if let Some(actual) = self.config.assets.get(&logical) {
                    return Ok(actual.clone());
                }
            }
        }
        bail!(
            "QLIE resource absent: {} relative to {}",
            name,
            m.frame.path
        )
    }
    fn run(&mut self, h: &mut dyn HostServices, budget: u32) -> Result<VmYield> {
        if self.random.is_none() {
            use crysirror_core::service::{SessionSeedSource, SessionSeedState};
            if self.random_seed.is_none() {
                self.random_seed =
                    Some(h.begin_session_seed(self.client, SessionSeedSource::Os { words: 1 })?);
            }
            match self.random_seed.as_mut().unwrap().poll()? {
                SessionSeedState::Pending => return Ok(VmYield::WaitUntil(h.now_ms() + 1)),
                SessionSeedState::Ready(seed) => {
                    ensure!(
                        seed.client == self.client && seed.words.len() == 1,
                        "QLIE random seed ownership"
                    );
                    self.random = Some(animation_clock::RandomTable::new(seed.words[0]));
                    self.random_seed = None;
                }
                SessionSeedState::Failed(error) => bail!("QLIE random seed: {error}"),
                SessionSeedState::Cancelled => bail!("QLIE random seed cancelled"),
            }
        }
        if self.machine.is_none() {
            if self.boot.is_none() {
                let storage = h.storage(self.client)?;
                let assets = self.config.assets.clone();
                self.boot = Some(
                    self.pool
                        .spawn(move |cancel| resources::prepare(storage, &assets, &cancel))?,
                );
            }
            let Some(prepared) = self.boot.as_mut().unwrap().try_take()? else {
                return Ok(VmYield::WaitUntil(h.now_ms() + 1));
            };
            self.machine = Some(Machine::new(prepared.programs, &self.config.script)?);
            self.text = prepared.text;
            self.boot = None;
            return Ok(VmYield::BudgetExhausted);
        }
        if !self.initialized {
            self.submit(h,json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"paint":{"kind":"solid","color":[0,0,0,0]}}}]}))?;
            self.initialized = true;
        }
        self.update_visuals(h)?;
        let mut m = self.machine.take().unwrap();
        let result = (|| {
            if m.executed >= self.next_object_collection && m.object_types.len() > 256 {
                self.collect_objects(h, &mut m)?;
                self.next_object_collection = m.executed.saturating_add(16384);
            }
            if self.frame_event.is_none()
                && self.last_frame_event != Some(h.now_ms())
                && self.load.is_none()
                && self.image_job.is_none()
                && self.audio_ticket.is_none()
                && self.audio_seek.is_none()
                && self.save_query.is_none()
            {
                let callback = if let Some(id) = self.button_callbacks.pop_front() {
                    if let Some(o) = self.objects.get(&id) {
                        for i in 0..4 {
                            m.globals.insert(
                                format!("resultbtnstr[{i}]"),
                                o.properties
                                    .get("us")
                                    .and_then(|v| v.get(i))
                                    .cloned()
                                    .unwrap_or(Value::Text(String::new())),
                            );
                            m.globals.insert(
                                format!("resultbtnint[{i}]"),
                                o.properties
                                    .get("ui")
                                    .and_then(|v| v.get(i))
                                    .cloned()
                                    .unwrap_or(Value::Int(0)),
                            );
                        }
                        let strings = o.properties.get("us").cloned().unwrap_or_default();
                        let label = strings.get(1).map(Value::text).unwrap_or_default();
                        let file = strings
                            .get(2)
                            .map(Value::text)
                            .filter(|s| !s.is_empty() && s != "self")
                            .unwrap_or_else(|| {
                                o.properties
                                    .get("callback_file")
                                    .and_then(|v| v.first())
                                    .map(Value::text)
                                    .unwrap_or_default()
                            });
                        Some(vec![Value::Text(label), Value::Text(file)])
                    } else {
                        None
                    }
                } else if let Some(callback) = self.key_callback()? {
                    Some(callback)
                } else {
                    self.objects
                        .get("sys")
                        .and_then(|o| o.properties.get("eve:onframe"))
                        .cloned()
                };
                if let Some(callback) =
                    callback.filter(|v| v.first().is_some_and(|v| !v.text().is_empty()))
                {
                    self.last_frame_event = Some(h.now_ms());
                    let context = FrameEvent {
                        depth: m.stack.len(),
                        wait: self.wait.take(),
                        poll: self.stop_poll.take(),
                        scope: m.scope.clone(),
                        selection: self.selection.clone(),
                        scope_next: self.scope_next,
                    };
                    let label = callback[0].text();
                    let file = callback[1].text();
                    let resume = m.frame.pc;
                    m.jump(&label, Some(&file), Some(vec![]), false)?;
                    m.stack.last_mut().unwrap().pc = resume;
                    self.frame_event = Some(context);
                }
            }
            if let Some(yielded) = self.poll_wait(h, &mut m)? {
                return Ok(yielded);
            }
            for _ in 0..budget {
                let Some(ins) = m.current()? else {
                    if m.return_call()? {
                        if let Some(yielded) = self.finish_frame_event(h, &mut m)? {
                            return Ok(yielded);
                        }
                        continue;
                    }
                    return Ok(VmYield::Halted);
                };
                let context = format!(
                    "QLIE {}:{} pc={} \\{} {:?}",
                    m.frame.path, ins.line, m.frame.pc, ins.op, ins.args
                );
                if m.control(&ins).with_context(|| context.clone())? {
                    if let Some(yielded) = self.finish_frame_event(h, &mut m)? {
                        return Ok(yielded);
                    }
                    if let Some((depth, label)) = &self.stop_poll {
                        if m.stack.len() == *depth {
                            self.wait = Some(Wait::Stop(label.clone()));
                            self.stop_poll = None;
                            return Ok(VmYield::WaitUntil(h.now_ms() + 1));
                        }
                    }
                    continue;
                }
                let before = (m.frame.path.clone(), m.frame.pc);
                if !self
                    .native(h, &mut m, &ins)
                    .with_context(|| context.clone())?
                {
                    return Ok(VmYield::WaitUntil(h.now_ms() + 1));
                }
                if m.frame.path == before.0 && m.frame.pc == before.1 {
                    m.frame.pc += 1;
                }
                m.executed += 1;
                if self.wait.is_some() {
                    return Ok(match self.wait {
                        Some(Wait::Clock(t) | Wait::FixedClock(t)) => {
                            VmYield::WaitUntil(t.min(h.now_ms() + 1))
                        }
                        Some(Wait::Stop(_)) => VmYield::WaitUntil(h.now_ms() + 1),
                        _ => VmYield::WaitForInput,
                    });
                }
            }
            Ok(VmYield::BudgetExhausted)
        })();
        self.machine = Some(m);
        result
    }
    fn key_callback(&self) -> Result<Option<Vec<Value>>> {
        let Some(keys) = self.objects.get("skey").filter(|o| !o.locked) else {
            return Ok(None);
        };
        let locks = self
            .objects
            .get("sys")
            .and_then(|o| o.properties.get("lk"))
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(0);
        for (key, binding) in &keys.properties {
            let Ok(code) = key.parse::<i32>() else {
                continue;
            };
            let Some(label) = binding.get(1).map(Value::text).filter(|v| !v.is_empty()) else {
                continue;
            };
            let action = binding
                .first()
                .map(Value::text)
                .unwrap_or_default()
                .to_ascii_lowercase();
            let lock = match action.as_str() {
                "save" => 4,
                "load" => 8,
                "config" => 16,
                "log" => 32,
                "view" => 64,
                "auto" => 128,
                "skip" => 256,
                "jump" => 512,
                "menu" => 1024,
                "reset" => 2048,
                _ => 0,
            };
            if locks & lock != 0 {
                continue;
            }
            // Callback registrations are edge-triggered. Held Click/KeyClick
            // bindings are consumed by native input/wait policy, not reentered
            // as a fresh script call on every scheduler quantum.
            if code & 0x10000 == 0 && !matches!(code & 0xffff, 0xf0 | 0xf1) {
                continue;
            }
            if code & 0xffff == 0x120 {
                continue;
            } // Gestures have a separate enabled-provider check.
            let mask = objects::native_key(code)?;
            if self.input.buttons_pressed & mask == 0 {
                continue;
            }
            // skey.a's fourth argument is numeric policy, not a filename.
            // Only the fifth argument selects the callback module. "self"
            // resolves at registration; an empty module keeps native lookup.
            let file = binding.get(3).map(Value::text).unwrap_or_default();
            let file = if file.eq_ignore_ascii_case("self") {
                keys.properties
                    .get(&format!("source/{code}"))
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_default()
            } else {
                file
            };
            return Ok(Some(vec![Value::Text(label), Value::Text(file)]));
        }
        Ok(None)
    }
    fn poll_wait(&mut self, h: &mut dyn HostServices, m: &mut Machine) -> Result<Option<VmYield>> {
        if let Some(wait) = self.wait.take() {
            let pressed = self.input.buttons_pressed != 0;
            let skip = self.input.buttons_down & crysirror_core::service::buttons::CTRL != 0;
            match wait {
                Wait::Clock(t) if h.now_ms() < t && !pressed && !skip => {
                    self.wait = Some(Wait::Clock(t));
                    return Ok(Some(VmYield::WaitUntil(t.min(h.now_ms() + 1))));
                }
                Wait::FixedClock(t) if h.now_ms() < t => {
                    self.wait = Some(Wait::FixedClock(t));
                    return Ok(Some(VmYield::WaitUntil(t.min(h.now_ms() + 1))));
                }
                Wait::Stop(stop) => {
                    if stop.deadline.is_some_and(|t| h.now_ms() >= t) {
                        m.globals.insert("resultstopint".into(), Value::Int(0));
                        return Ok(None);
                    }
                    self.stop_poll = Some((m.stack.len(), stop.clone()));
                    m.frame.pc = m
                        .frame
                        .pc
                        .checked_sub(1)
                        .context("QLIE stop continuation")?;
                    m.jump(&stop.label, None, Some(vec![]), false)?;
                }
                _ => {}
            }
        }
        Ok(None)
    }
    fn finish_frame_event(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
    ) -> Result<Option<VmYield>> {
        if self
            .frame_event
            .as_ref()
            .is_some_and(|e| e.depth == m.stack.len())
        {
            let event = self.frame_event.take().unwrap();
            self.wait = event.wait;
            self.stop_poll = event.poll;
            m.scope = event.scope;
            self.selection = event.selection;
            self.scope_next = event.scope_next;
            return self.poll_wait(h, m);
        }
        Ok(None)
    }
    fn native(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool> {
        let a = &ins.args;
        match ins.op.as_str() {
            "scp" => {
                ensure!(!a.is_empty() && a.len() <= 16, "QLIE object selection ABI");
                let mut selection = Vec::new();
                for name in a {
                    let key = m.key(name)?;
                    let id = m.value(&key)?.text();
                    ensure!(
                        m.object_types.contains_key(&id),
                        "QLIE scope is not an object: {key}"
                    );
                    selection.push(id);
                }
                m.scope = selection[0].clone();
                self.selection = selection;
                self.scope_next = 0;
            }
            "wait" => {
                let duration = m
                    .eval(a.first().context("QLIE fixed wait duration")?)?
                    .integer()?
                    .max(0) as u64;
                self.wait = Some(Wait::FixedClock(h.now_ms().saturating_add(duration)));
            }
            "skip" => {
                ensure!(a.len() == 1, "QLIE clock skip ABI");
                let duration = u64::try_from(m.eval(&a[0])?.integer()?)?;
                ensure!(duration <= 86_400_000, "QLIE clock skip limit");
                self.submit(
                    h,
                    json!({"layers":[{"op":"advance_time","delta_ms":duration}]}),
                )?;
                for o in self.objects.values_mut().filter(|o| o.visible) {
                    o.offset = o.offset.saturating_add(duration);
                }
                self.skip_until = h.now_ms().saturating_add(1);
            }
            "clk" => {
                let n = a
                    .first()
                    .map(|v| m.eval(v)?.integer())
                    .transpose()?
                    .unwrap_or(0);
                self.wait = Some(Wait::Clock(h.now_ms().saturating_add(n.max(0) as u64)));
            }
            "draw" | "zero" => {
                self.update_visuals(h)?;
                self.wait = Some(Wait::Clock(h.now_ms() + 1));
            }
            "stop" => {
                let label = m.eval(a.first().context("QLIE stop handler")?)?.text();
                let duration = a
                    .get(1)
                    .map(|v| m.eval(v)?.integer())
                    .transpose()?
                    .unwrap_or(0);
                m.globals.insert("resultstopint".into(), Value::Int(0));
                self.wait = Some(Wait::Stop(StopWait {
                    label,
                    deadline: (duration > 0).then(|| h.now_ms().saturating_add(duration as u64)),
                }));
            }
            "play" => {
                m.globals.insert("resultstopint".into(), Value::Int(1));
                self.wait = None;
                self.stop_poll = None;
            }
            "dbrk" => bail!("QLIE script debug break"),
            "derr" => bail!(
                "QLIE script error: {}",
                a.first()
                    .map(|s| m.eval(s).map(|v| v.text()))
                    .transpose()?
                    .unwrap_or_default()
            ),
            "del" => self.delete(h, m, a)?,
            "fexs" => {
                let name = m
                    .eval(a.first().context("QLIE file-existence path")?)?
                    .text();
                if self.resolve(m, &name).is_ok() {
                    m.result(0, Value::Int(1));
                } else {
                    let path = crate::program::canonical(&name)?;
                    if self.file_query.is_none() {
                        let storage = h.storage(self.client)?;
                        let source = path.clone();
                        self.file_query = Some((
                            path.clone(),
                            self.pool.spawn(move |cancel| {
                                cancel.check()?;
                                Ok(storage
                                    .metadata(Mount::Save, &source)?
                                    .is_some_and(|s| !s.is_directory))
                            })?,
                        ));
                    }
                    let (expected, job) = self.file_query.as_mut().unwrap();
                    ensure!(expected == &path, "QLIE file-existence completion identity");
                    let Some(found) = job.try_take()? else {
                        return Ok(false);
                    };
                    self.file_query = None;
                    m.result(0, Value::Int(i32::from(found)));
                }
            }
            "copy" | "cpy" | "len" => {
                let text = m.eval(a.first().context("QLIE string argument")?)?.text();
                let units = text.encode_utf16().collect::<Vec<_>>();
                if ins.op == "len" {
                    m.result(0, Value::Int(i32::try_from(units.len())?));
                } else {
                    ensure!(a.len() == 3, "QLIE copy ABI");
                    let start = m.eval(&a[1])?.integer()?.max(1) as usize - 1;
                    let length = m.eval(&a[2])?.integer()?.max(0) as usize;
                    let start = start.min(units.len());
                    let end = start.saturating_add(length).min(units.len());
                    m.result_text(
                        0,
                        String::from_utf16(&units[start..end])
                            .context("QLIE copy splits UTF16 surrogate")?,
                    );
                }
            }
            "zenc" => {
                let text = m.eval(a.first().context("QLIE full-width text")?)?.text();
                m.result_text(0, crysirror_text::width::to_full_width(&text));
            }
            "efn" | "cfe" => {
                let path = m.eval(a.first().context("QLIE path operation")?)?.text();
                let output = if ins.op == "efn" {
                    path.rsplit(['\\', '/']).next().unwrap_or("").to_owned()
                } else {
                    let extension = m
                        .eval(a.get(1).context("QLIE replacement extension")?)?
                        .text();
                    let start = path.rfind(['\\', '/']).map(|i| i + 1).unwrap_or(0);
                    let dot = path[start..]
                        .rfind('.')
                        .map(|i| i + start)
                        .unwrap_or(path.len());
                    format!("{}{extension}", &path[..dot])
                };
                m.result_text(0, output);
            }
            "bsub" => {
                ensure!(a.len() == 3, "QLIE regex replacement ABI");
                let text = m.eval(&a[0])?.text();
                let pattern = m.eval(&a[1])?.text();
                let replacement = m.eval(&a[2])?.text();
                let regex = regex::RegexBuilder::new(&pattern)
                    .size_limit(1024 * 1024)
                    .build()?;
                let result = regex.replace_all(&text, replacement).into_owned();
                ensure!(result.len() <= 1024 * 1024, "QLIE regex output limit");
                m.result_text(0, result);
            }
            "min" | "max" => {
                let values = a
                    .iter()
                    .map(|v| m.eval(v)?.integer())
                    .collect::<Result<Vec<_>>>()?;
                let value = if ins.op == "min" {
                    values.into_iter().min()
                } else {
                    values.into_iter().max()
                }
                .context("QLIE min/max arguments")?;
                m.result(0, Value::Int(value));
            }
            "pos" => {
                ensure!(a.len() == 2, "QLIE substring position ABI");
                let needle = m.eval(&a[0])?.text();
                let text = m.eval(&a[1])?.text();
                let position = if needle.is_empty() {
                    0
                } else {
                    text.find(&needle)
                        .map(|at| text[..at].encode_utf16().count() + 1)
                        .unwrap_or(0)
                };
                m.result(0, Value::Int(i32::try_from(position)?));
            }
            "trim" => {
                let text = m.eval(a.first().context("QLIE trim argument")?)?.text();
                m.result_text(0, text.trim());
            }
            "swap" => {
                ensure!(a.len() == 2, "QLIE swap ABI");
                let first = m.eval(&a[0])?;
                let second = m.eval(&a[1])?;
                m.assign(&a[0], second)?;
                m.assign(&a[1], first)?;
            }
            "fmt" => {
                let mut output = String::new();
                let mut i = 0;
                while i < a.len() {
                    let format = m.eval(&a[i])?.text();
                    i += 1;
                    let chars = format.chars().collect::<Vec<_>>();
                    let mut at = 0;
                    while at < chars.len() {
                        if chars[at] == '%' && at + 1 < chars.len() {
                            if chars[at + 1] == '%' {
                                output.push('%');
                                at += 2;
                                continue;
                            }
                            let start = at;
                            at += 1;
                            while at < chars.len() && !chars[at].is_ascii_alphabetic() {
                                at += 1;
                            }
                            ensure!(at < chars.len() && i < a.len(), "QLIE format arguments");
                            let spec = chars[start..=at].iter().collect::<String>();
                            let value = m.eval(&a[i])?;
                            i += 1;
                            use crysirror_text::printf32::{
                                format_printf32_bytes, Printf32Argument,
                            };
                            let text = value.text();
                            let argument = if chars[at] == 's' {
                                ensure!(
                                    spec == "%s",
                                    "QLIE Unicode string field formatting profile is not installed"
                                );
                                Printf32Argument::Bytes(text.as_bytes())
                            } else {
                                Printf32Argument::Word(value.integer()? as u32)
                            };
                            let formatted = format_printf32_bytes(
                                spec.as_bytes(),
                                &[argument],
                                1024 * 1024 - output.len(),
                            )?;
                            output += &String::from_utf8(formatted.bytes)
                                .context("QLIE formatted Unicode output")?;
                            at += 1;
                        } else {
                            output.push(chars[at]);
                            at += 1
                        }
                    }
                }
                m.result_text(0, output);
            }
            "rep" | "div" => {
                ensure!(a.len() >= 2, "QLIE string operation ABI");
                let first = m.eval(&a[0])?.text();
                let second = m.eval(&a[1])?.text();
                if ins.op == "rep" {
                    let replacement = a
                        .get(2)
                        .map(|v| m.eval(v).map(|v| v.text()))
                        .transpose()?
                        .unwrap_or_default();
                    m.result_text(0, first.replace(&second, &replacement));
                } else {
                    ensure!(a.len() <= 5, "QLIE split ABI");
                    let strip_space = a
                        .get(2)
                        .map(|v| m.eval(v)?.integer())
                        .transpose()?
                        .unwrap_or(0)
                        == 1;
                    let raw = a
                        .get(3)
                        .map(|v| m.eval(v)?.integer())
                        .transpose()?
                        .unwrap_or(0)
                        == 1;
                    let values = crate::program::split_native(&second, &first, strip_space, raw);
                    for i in 0..64 {
                        m.result_text(i, "");
                    }
                    if let Some(default) = a.get(4).filter(|s| !s.trim().is_empty()) {
                        let default = m.eval(default)?.integer()?;
                        for i in 0..16 {
                            m.result(
                                i,
                                Value::Int(
                                    values
                                        .get(i)
                                        .and_then(|v| crate::expression::parse_integer(v))
                                        .unwrap_or(default),
                                ),
                            );
                        }
                    } else {
                        m.result(0, Value::Int(i32::try_from(values.len())?));
                    }
                    for (i, s) in values.into_iter().take(16).enumerate() {
                        m.result_text(i, s);
                    }
                }
            }
            "stid" => {
                let s = m.eval(a.first().context("QLIE stid")?)?.text();
                let fallback = a
                    .get(1)
                    .map(|s| m.eval(s)?.integer())
                    .transpose()?
                    .unwrap_or(0);
                m.result(
                    0,
                    Value::Int(crate::expression::parse_integer(&s).unwrap_or(fallback)),
                );
            }
            "exop" => {
                let source = m.eval(a.first().context("QLIE dynamic commands")?)?.text();
                m.inject(&source, BTreeMap::new())?;
            }
            "text" | "blank" => return self.expand_line(m, ins),
            op if op.starts_with('^') => {
                return self.expand_line(m, ins);
            }
            _ => {
                if self.selection.len() <= 1 {
                    return self.object_method(h, m, ins);
                }
                ensure!(
                    !ins.op.starts_with('?'),
                    "QLIE multi-object query result packing is not installed"
                );
                while self.scope_next < self.selection.len() {
                    m.scope = self.selection[self.scope_next].clone();
                    if !self.object_method(h, m, ins)? {
                        return Ok(false);
                    }
                    self.scope_next += 1;
                }
                m.scope = self.selection[0].clone();
                self.scope_next = 0;
                return Ok(true);
            }
        }
        Ok(true)
    }
}
impl VmAdapter for QlieVm {
    fn preparation_pending(&self) -> bool {
        self.machine.is_none() && self.error.is_none()
    }
    fn presentation_pending(&self) -> bool {
        self.load.is_some() || self.image_job.is_some()
    }
    fn engine_id(&self) -> &'static str {
        "qlie"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn step(
        &mut self,
        h: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        if let Some(error) = &self.error {
            return Err(Error::Io(error.clone()));
        }
        self.input = input;
        if self.last_tick_ms != Some(h.now_ms()) {
            self.tick_ms = self
                .last_tick_ms
                .map(|t| h.now_ms().saturating_sub(t))
                .unwrap_or(0);
            self.last_tick_ms = Some(h.now_ms());
        }
        self.run(h, budget).map_err(|e| {
            self.error = Some(format!("{e:#}"));
            Error::Io(format!("{e:#}"))
        })
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(Error::Unsupported("QLIE VM/resource checkpoint"))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(Error::Unsupported("QLIE VM/resource checkpoint"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"vm":"qlie","module":self.machine.as_ref().map(|m|&m.frame.path),"pc":self.machine.as_ref().map(|m|m.frame.pc),"line":self.machine.as_ref().and_then(|m|m.current().ok().flatten().map(|i|i.line)),"instructions":self.machine.as_ref().map(|m|m.executed),"call_depth":self.machine.as_ref().map(|m|m.stack.len()),"callers":self.machine.as_ref().map(|m|m.stack.iter().map(|f|json!({"module":f.path,"pc":f.pc,"line":m.programs.get(&f.path).and_then(|p|p.code.get(f.pc)).map(|i|i.line)})).collect::<Vec<_>>()),"objects":self.objects.len(),"unavailable_capabilities":self.unavailable,"engine_api_calls":self.api_calls,"interactive_objects":self.objects.iter().filter(|(_,o)|o.ui_registered&&o.visible).map(|(id,o)|json!({"id":id,"layer":o.layer,"asset":o.asset,"variables":self.machine.as_ref().map(|m|m.globals.iter().filter(|(_,v)|v.text()==*id).map(|(k,_)|k.clone()).collect::<Vec<_>>())})).collect::<Vec<_>>(),"error":self.error})).unwrap_or_default()
    }
}
struct Factory;
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(spec.program.is_empty(), "QLIE requires original scripts");
        Ok(Box::new(QlieVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("qlie", Arc::new(Factory))
}
