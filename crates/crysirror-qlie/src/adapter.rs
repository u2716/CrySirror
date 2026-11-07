//! Thin QLIE semantic bridge. All retained images, draw trees, interpolation,
//! media clocks and audio voices are owned by CrySirror's shared services.
use crate::{
    expression::Value,
    machine::{Machine, ThreadKind},
    program::{Instruction, Program},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{HostServices, InputFrame, UploadedImage},
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use crysirror_tasks::{Task, TaskPool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
mod abmp_native;
mod animated_transition;
mod animation;
mod animation_clock;
mod camera;
mod clip;
mod cfg_settings;
mod capture;
mod dialogs;
mod files;
mod filters;
mod generator;
mod glyph_pictures;
mod image_native;
mod gesture_guide;
mod input_device;
mod keys;
mod math;
mod minor_kinds;
mod misc_native;
mod message;
mod native_motion;
mod native_wait;
mod objects;
mod text_statements;
mod pointer_pick;
mod render;
mod replacement;
mod reset;
mod resources;
mod save_metadata;
mod reg;
mod save_state;
mod sdat;
mod rule_prepare;
mod snd_native;
mod sound_filters;
mod sys_native;
mod system_data;
mod theora_native;
mod thread;
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
    /// The original executable in the game folder (ParamStr(0)), if found.
    #[serde(default)]
    pub executable: Option<String>,
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
    thread: ThreadKind,
    native_position: Option<native_motion::Position>,
    kind: String,
    layer: u32,
    created: bool,
    visible: bool,
    position: [i32; 3],
    locked: bool,
    properties: BTreeMap<String, Vec<Value>>,
    lines: Vec<String>,
    table: BTreeMap<String, String>,
    regex_rules: BTreeMap<String, regex::Regex>,
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
    wave_channel: Option<u32>,
    wave_multiplier: Option<f32>,
    hit_bounds: Option<[f32; 4]>,
    hit_world_space: bool,
    hit_mask: Option<crysirror_scene::ui::WidgetHitMask>,
    hit_mask_job: Option<(String, Task<crysirror_scene::ui::WidgetHitMask>)>,
    hit_mask_key: Option<String>,
    hit_mask_plane_key: Option<String>,
    hit_mask_sources: BTreeMap<String, crysirror_scene::ui::WidgetHitMask>,
    hit_mask_bounds: Option<[f32; 4]>,
    draw_images: BTreeMap<(usize, usize, usize), render::DrawImage>,
    button_state: (bool, bool, u32),
    ui_registered: bool,
    message: Option<message::Message>,
    generator: Option<generator::Group>,
    capture_views: BTreeMap<usize, serde_json::Value>,
    colour: Option<[u8; 3]>,
}
#[derive(Clone, Serialize, Deserialize)]
struct StopWait {
    label: String,
    deadline: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
enum Wait {
    Clock(u64),
    Stop(StopWait),
    FixedClock(u64),
    /// time,ms[,label]: absolute host deadline of the reader clock target.
    Time { deadline: u64, label: String },
    /// sndt,channel,ms[,label].
    Sound { channel: i32, until: i32, label: String },
    /// msg: any message of this thread still revealing.
    Message,
    /// btn[,ms].
    Button { deadline: Option<u64> },
    /// wav,channel.
    Wave { channel: i32 },
}
#[derive(Clone, Serialize, Deserialize)]
struct FrameEvent {
    depth: usize,
    wait: Option<Wait>,
    poll: Option<(usize, StopWait)>,
    scope: String,
    selection: Vec<String>,
    scope_next: usize,
}
pub struct QlieVm {
    host_thread: ThreadKind,
    suspended_host: Option<thread::HostContext>,
    cross_host: Option<thread::HostContext>,
    system_root: bool,
    system_input_active: bool,
    game_pause_ms: Option<u64>,
    paused_random: Option<animation_clock::RandomTable>,
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
    glyph_sheet: Option<glyph_pictures::SheetJob>,
    sound_scan: Option<(String, Task<f64>)>,
    /// Reader clock origins (time), game and system readers.
    script_clock: [u64; 2],
    /// msd records of non-running frames, by stack index.
    frame_waits: BTreeMap<usize, native_wait::Record>,
    /// Button clicked while a btn wait runs.
    button_clicked: Option<String>,
    thread_image: Option<(String, u32, crysirror_core::image_processing::ImageJob)>,
    file_hash: Option<(String, Task<Option<String>>)>,
    pack_slots: BTreeSet<u32>,
    filter_job: Option<filters::Preparation>,
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
    rule_fade: Option<(String, f32, bool)>,
    animated_fade: Option<u64>,
    audio_ticket: Option<u64>,
    audio_seek: Option<(String, u64)>,
    api_calls: u64,
    error: Option<String>,
    stop_poll: Option<(usize, StopWait)>,
    media_ids: BTreeSet<u32>,
    file_query: Option<(String, Task<bool>)>,
    file_operation: Option<files::Pending>,
    list_io: Option<files::ListIo>,
    save_query: Option<(String, Task<save_metadata::ReadResult>)>,
    skip_until: u64,
    frame_event: Option<FrameEvent>,
    last_frame_event: Option<u64>,
    next_object_collection: u64,
    button_callbacks: std::collections::VecDeque<String>,
    last_tick_ms: Option<u64>,
    tick_ms: u64,
    selection: Vec<String>,
    scope_next: usize,
    read_lines: BTreeSet<(String, usize)>,
    last_read: bool,
    dialog: Option<dialogs::TextDialog>,
    choice: Option<dialogs::ChoiceDialog>,
    file_dialog: Option<dialogs::FilePending>,
    color_dialog: Option<dialogs::ColorDialog>,
    unavailable: BTreeSet<String>,
    random: Option<animation_clock::RandomTable>,
    math_random: Option<[crysirror_core::random::Mt19937; 2]>,
    random_seed: Option<Box<dyn crysirror_core::service::SessionSeedQuery>>,
    /// mdat/tdat memory-save stores (TImoObjectDataStackSave).
    stacks: BTreeMap<String, save_state::StackStore>,
    rebuild: Option<save_state::Rebuild>,
    save_job: Option<save_metadata::SaveJob>,
    system_data: system_data::State,
    swipe: crysirror_core::pointer_swipe::SwipeTracker,
    /// Config.txt pairs for engine settings read before cfg is first used.
    boot_settings: Option<BTreeMap<String, String>>,
    keys: keys::NativeKeys,
    /// 708048 request byte: a system label call after the input pass.
    system_request: Option<thread::Callback>,
    halt_requested: bool,
    /// sys\hdr output limiter [min, max, gain, attack, release].
    limiter: Option<[i32; 5]>,
    misc_io: Option<misc_native::MiscPending>,
    sdat_io: Option<sdat::SdatPending>,
    clip_io: Option<clip::ClipPending>,
    registry: reg::Registry,
    png_pool: BTreeSet<String>,
    gesture_guide: gesture_guide::GestureGuide,
    id_counter: u64,
    /// Process-wide native strings (sys fc, form caption).
    global_strings: BTreeMap<String, String>,
    input_device: input_device::InputDevice,
    /// Anchor object id -> proxy layer carrying only its transform (5D1644).
    anchor_proxies: BTreeMap<String, u32>,
    /// Transformed (filtered) sound preparation: ticket and owning object.
    audio_resource: Option<(u64, String)>,
    /// Running 6B9010 case-10 mosaic transition (its priority).
    mosaic: Option<i32>,
    rule_preparation: rule_prepare::RulePreparation,
    colour_flash: bool,
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
            host_thread: ThreadKind::Game,
            suspended_host: None,
            cross_host: None,
            system_root: false,
            system_input_active: false,
            game_pause_ms: None,
            paused_random: None,
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
            glyph_sheet: None,
            sound_scan: None,
            script_clock: [0; 2],
            frame_waits: BTreeMap::new(),
            button_clicked: None,
            thread_image: None,
            file_hash: None,
            pack_slots: BTreeSet::new(),
            filter_job: None,
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
            file_operation: None,
            list_io: None,
            save_query: None,
            skip_until: 0,
            frame_event: None,
            last_frame_event: None,
            next_object_collection: 0,
            button_callbacks: std::collections::VecDeque::new(),
            last_tick_ms: None,
            tick_ms: 0,
            selection: vec![],
            scope_next: 0,
            read_lines: BTreeSet::new(),
            last_read: false,
            dialog: None,
            choice: None,
            file_dialog: None,
            color_dialog: None,
            unavailable: BTreeSet::new(),
            random: None,
            math_random: None,
            random_seed: None,
            stacks: BTreeMap::new(),
            rebuild: None,
            save_job: None,
            system_data: Default::default(),
            swipe: Default::default(),
            boot_settings: None,
            keys: Default::default(),
            system_request: None,
            halt_requested: false,
            limiter: None,
            misc_io: None,
            sdat_io: None,
            clip_io: None,
            registry: Default::default(),
            png_pool: BTreeSet::new(),
            gesture_guide: Default::default(),
            id_counter: 0,
            global_strings: BTreeMap::new(),
            input_device: Default::default(),
            anchor_proxies: BTreeMap::new(),
            audio_resource: None,
            mosaic: None,
            rule_preparation: Default::default(),
            colour_flash: false,
        })
    }
    fn submit(&mut self, h: &mut dyn HostServices, batch: serde_json::Value) -> Result<()> {
        h.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
        self.api_calls += 1;
        Ok(())
    }
    fn resolve(&self, m: &Machine, name: &str) -> Result<String> {
        for logical in self.resource_candidates(m, name)? {
            if let Some(actual) = self.config.assets.get(&logical) {
                return Ok(actual.clone());
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
                    self.math_random = Some(core::array::from_fn(|_| {
                        crysirror_core::random::Mt19937::new(seed.words[0])
                    }));
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
            self.pack_slots = prepared.pack_slots;
            self.boot = None;
            return Ok(VmYield::BudgetExhausted);
        }
        if !self.system_data.loaded {
            // 6E633C -> 6D5904 loads SystemData before startup.s executes.
            let mut m = self.machine.take().unwrap();
            let ready = self.poll_system_data_boot(h, &mut m);
            self.machine = Some(m);
            if !ready? {
                return Ok(VmYield::WaitUntil(h.now_ms() + 1));
            }
        }
        if !self.initialized {
            self.submit(h,json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"transform":{"z":1000,"canvas_projection":{"plane":1000,"near":50,"far":5000,"near_clamp":true,"affine_uv":true}},"paint":{"kind":"solid","color":[0,0,0,0]}}}]}))?;
            self.put_thread_ui_root(h, ThreadKind::Game)?;
            self.script_clock = [h.now_ms(); 2];
            self.initialized = true;
        }
        self.update_visuals(h)?;
        let mut m = self.machine.take().unwrap();
        // A scheduler quantum can end after a pure control transfer, before
        // the next syscall has refreshed the host's selected-object cache.
        self.selection = m.frame.selection.clone();
        m.scope = self
            .selection
            .get(self.scope_next)
            .or_else(|| self.selection.first())
            .cloned()
            .unwrap_or_default();
        let result = (|| {
            if m.executed >= self.next_object_collection && m.object_types.len() > 256 {
                self.collect_objects(h, &mut m)?;
                self.next_object_collection = m.executed.saturating_add(16384);
            }
            if self.frame_event.is_none()
                && self.cross_host.is_none()
                && self.last_frame_event != Some(h.now_ms())
                && self.load.is_none()
                && self.image_job.is_none()
                && self.glyph_sheet.is_none()
                && self.sound_scan.is_none()
                && self.thread_image.is_none()
                && self.file_hash.is_none()
                && self.filter_job.is_none()
                && self.audio_ticket.is_none()
                && self.audio_seek.is_none()
                && self.save_query.is_none()
                && self.file_operation.is_none()
                && self.list_io.is_none()
                && self.misc_io.is_none()
                && self.sdat_io.is_none()
                && self.clip_io.is_none()
            {
                let callback = if let Some(id) = self.button_callbacks.pop_front() {
                    if let Some(o) = self.objects.get(&id) {
                        for i in 0..4 {
                            m.thread_globals_mut().insert(
                                format!("resultbtnstr[{i}]"),
                                o.properties
                                    .get("us")
                                    .and_then(|v| v.get(i))
                                    .cloned()
                                    .unwrap_or(Value::Text(String::new())),
                            );
                            m.thread_globals_mut().insert(
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
                        Some(thread::Callback {
                            action: strings
                                .first()
                                .map(Value::text)
                                .unwrap_or_default()
                                .to_ascii_lowercase(),
                            label,
                            file,
                            same_reader: strings
                                .get(2)
                                .map(Value::text)
                                .is_some_and(|v| v.eq_ignore_ascii_case("self")),
                        })
                    } else {
                        None
                    }
                } else if let Some(callback) = self.system_request.take() {
                    if callback.action == "engine_reset" {
                        self.reset_engine(h, &mut m, &callback.label, &callback.file)?;
                    } else {
                        self.dispatch_system_request(h, &mut m, callback)?;
                    }
                    None
                } else if let Some(callback) = self.key_callback(&m)? {
                    Some(callback)
                } else if let Some(callback) = self.due_timer(&m) {
                    Some(callback)
                } else {
                    self.objects
                        .get(&Self::builtin("sys", m.thread_kind()))
                        .and_then(|o| o.properties.get("eve:onframe"))
                        .map(|event| thread::Callback {
                            action: String::new(),
                            label: event.first().map(Value::text).unwrap_or_default(),
                            file: event.get(1).map(Value::text).unwrap_or_default(),
                            same_reader: true,
                        })
                };
                if let Some(callback) = callback.filter(thread::Callback::has_entry) {
                    self.dispatch_callback(h, &mut m, callback)?;
                }
            }
            if let Some(yielded) = self.poll_wait(h, &mut m)? {
                return Ok(yielded);
            }
            for _ in 0..budget {
                let Some(ins) = m.current()? else {
                    if m.return_call()? {
                        self.finish_thread_call(h, &m)?;
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
                    self.apply_frame_wait(h, &mut m)?;
                    self.finish_thread_call(h, &m)?;
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
                        Some(Wait::Button { deadline: Some(t) }) => VmYield::WaitUntil(t.min(h.now_ms() + 1)),
                        Some(Wait::Time { deadline, .. }) => VmYield::WaitUntil(deadline.min(h.now_ms() + 1)),
                        _ => VmYield::WaitUntil(h.now_ms() + 1),
                    });
                }
            }
            Ok(VmYield::BudgetExhausted)
        })();
        let persisted = self.persist_system_data(h, &m, false);
        self.machine = Some(m);
        persisted?;
        result
    }
    /// 70A3CC applies the engine settings and the input thread's masks to
    /// the device each frame: gesture mask = MouseGestureEnable & inp\mge,
    /// swipe buttons = SwipeEnable & inp\swe (the setting alone without an
    /// input object), SwipeIntense and SwipePermitFrame. TouchDeviceMode only
    /// selects the Windows pointer source (5C7D48) and has no host analogue.
    fn advance_swipe(&mut self, now: u64) {
        use crysirror_core::{pointer_swipe::SwipeButtons, service::buttons as B};
        let input = self
            .objects
            .get(&Self::builtin("inp", self.input_thread()))
            .map(|o| (o.properties.get("swe").cloned().unwrap_or_default(), o.properties.get("mge").cloned()));
        let gesture_setting = self.engine_setting("mge");
        let swipe_setting = self.engine_setting("swe");
        let (buttons, inertia, gesture) = match &input {
            Some((swe, mge)) => {
                let int = |i: usize, default: i32| {
                    swe.get(i).and_then(|v| v.integer().ok()).unwrap_or(default)
                };
                let mge = mge
                    .as_ref()
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(0);
                (swipe_setting & int(0, 0), int(1, 1) > 0, gesture_setting & mge)
            }
            None => (swipe_setting, false, gesture_setting),
        };
        self.swipe.config.buttons = (7 & buttons) as u8;
        self.swipe.config.inertia = inertia;
        self.swipe.config.intensity = self.engine_setting("swi");
        self.swipe.config.permit_frames = self.engine_setting("swp");
        let down = self.input.buttons_down;
        let held = u8::from(down & B::MOUSE_LEFT != 0)
            | (u8::from(down & B::MOUSE_RIGHT != 0) << 1)
            | (u8::from(down & B::MOUSE_MIDDLE != 0) << 2);
        // 5C7798: a modifier held with a plain button is a different code.
        let held = if down & (B::SHIFT | B::CTRL | B::ALT) != 0 { 0 } else { held };
        self.swipe.step(
            [self.input.pointer_x, self.input.pointer_y],
            SwipeButtons(held),
            self.tick_ms.min(u64::from(u32::MAX)) as u32,
        );
        self.advance_keys(now, (7 & gesture) as u8);
    }
    /// 6AA598: an enabled timer whose group intersects its reader's System
    /// input mask accumulates elapsed time; each due interval calls its label.
    fn advance_timers(&mut self) {
        let delta = self.tick_ms.min(i32::MAX as u64) as i32;
        let masks: [i32; 2] = [ThreadKind::Game, ThreadKind::System].map(|thread| {
            self.objects
                .get(&Self::builtin("sys", thread))
                .and_then(|s| s.properties.get("in"))
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .unwrap_or(0xFFFFFFF)
        });
        let paused = self.game_pause_ms.is_some() && self.system_input_active;
        for o in self.objects.values_mut().filter(|o| o.kind == "timer") {
            let get = |o: &Object, key: &str, default: i32| {
                o.properties
                    .get(key)
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(default)
            };
            let group = get(o, "timer/in", 255);
            let active = o.properties.contains_key("timer/enabled")
                && group & masks[usize::from(o.thread == ThreadKind::System)] != 0
                && !(paused && o.thread == ThreadKind::Game);
            if !active {
                o.properties.remove("timer/acc");
                continue;
            }
            let acc = get(o, "timer/acc", 0).saturating_add(delta);
            o.properties.insert("timer/acc".into(), vec![Value::Int(acc)]);
        }
    }
    fn due_timer(&mut self, m: &Machine) -> Option<thread::Callback> {
        let thread = m.thread_kind();
        let (id, label, file) = self.objects.iter().find_map(|(id, o)| {
            if o.kind != "timer" || o.thread != thread {
                return None;
            }
            let get = |key: &str| {
                o.properties
                    .get(key)
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
            };
            let interval = get("timer/interval").unwrap_or(0);
            let acc = get("timer/acc")?;
            (acc >= interval).then(|| {
                let text = |key: &str| {
                    o.properties
                        .get(key)
                        .and_then(|v| v.first())
                        .map(Value::text)
                        .unwrap_or_default()
                };
                (id.clone(), text("timer/label"), text("timer/file"))
            })
        })?;
        let o = self.objects.get_mut(&id).unwrap();
        let interval = o
            .properties
            .get("timer/interval")
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0);
        let acc = o
            .properties
            .get("timer/acc")
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0);
        // One call per dispatch opportunity; the remainder stays pending.
        let rest = if interval >= 1 { acc - interval } else { 0 };
        o.properties
            .insert("timer/acc".into(), vec![Value::Int(rest.min(interval.max(1) * 256))]);
        // 6AA528 only dispatches an @@ label.
        if !label.starts_with("@@") {
            return None;
        }
        Some(thread::Callback {
            action: String::new(),
            label,
            file,
            same_reader: true,
        })
    }
    fn key_callback(&self, m: &Machine) -> Result<Option<thread::Callback>> {
        let Some(keys) = self
            .objects
            .get(&Self::builtin("skey", m.thread_kind()))
            .filter(|o| !o.locked)
        else {
            return Ok(None);
        };
        let locks = self
            .objects
            .get(&Self::builtin("sys", m.thread_kind()))
            .and_then(|o| o.properties.get("lk"))
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(0);
        for (key, binding) in &keys.properties {
            let Ok(code) = key.parse::<i32>() else {
                continue;
            };
            let label = binding.get(1).map(Value::text).unwrap_or_default();
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
            // 6CB09C: the first registration whose code is active in its
            // 5C8074 mode is dispatched. Click/KeyClick/SkipClick bindings
            // have no script entry; native input/wait policy consumes them.
            if code & 0xffff == 0x120 {
                continue;
            } // Gestures have a separate enabled-provider check.
            if !self.native_key_active(code)? {
                continue;
            }
            // skey.a's fourth argument is numeric policy, not a filename.
            // Only the fifth argument selects the callback module. "self"
            // resolves at registration; an empty module keeps native lookup.
            let file = binding.get(3).map(Value::text).unwrap_or_default();
            let same_reader = file.eq_ignore_ascii_case("self");
            let file = if same_reader {
                keys.properties
                    .get(&format!("source/{code}"))
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_default()
            } else {
                file
            };
            let callback = thread::Callback {
                action,
                label,
                file,
                same_reader,
            };
            if callback.has_entry() {
                return Ok(Some(callback));
            }
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
                wait @ (Wait::Time { .. }
                | Wait::Sound { .. }
                | Wait::Message
                | Wait::Button { .. }
                | Wait::Wave { .. }) => {
                    if let Some(wait) = self.poll_native_wait(h, m, wait, pressed)? {
                        self.wait = Some(wait);
                        return Ok(Some(VmYield::WaitUntil(h.now_ms() + 1)));
                    }
                }
                Wait::Stop(stop) => {
                    if stop.deadline.is_some_and(|t| h.now_ms() >= t) {
                        m.thread_globals_mut()
                            .insert("resultstopint".into(), Value::Int(0));
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
        self.selection = m.frame.selection.clone();
        m.scope = self
            .selection
            .get(self.scope_next)
            .or_else(|| self.selection.first())
            .cloned()
            .unwrap_or_default();
        let a = &ins.args;
        // 6D648C statements precede object dispatch.
        if self.wait_statement(h, m, &ins.op, a)? || self.text_statement(m, &ins.op, a)? {
            return Ok(true);
        }
        match ins.op.as_str() {
            "thrd" | "srct" | "tsub" => return self.thread_instruction(h, m, ins),
            "fsha" => return self.file_hash(h, m, a),
            "vget" | "vset" => {
                ensure!(a.len() == 2, "QLIE cross-thread object value ABI");
                let (destination, source) = if ins.op == "vget" {
                    (m.value(&m.key(&a[0])?)?, m.game_value(&m.game_key(&a[1])?)?)
                } else {
                    (m.game_value(&m.game_key(&a[0])?)?, m.eval(&a[1])?)
                };
                self.copy_native_value_object(h, m, &destination.text(), &source.text())?;
            }
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
                m.frame.selection = selection.clone();
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
                for o in self
                    .objects
                    .values_mut()
                    .filter(|o| o.thread == self.host_thread)
                {
                    if o.visible {
                        o.offset = o.offset.saturating_add(duration);
                    }
                    if let Some(position) = &mut o.native_position {
                        position.advance_clock(duration);
                    }
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
                m.thread_globals_mut()
                    .insert("resultstopint".into(), Value::Int(0));
                self.wait = Some(Wait::Stop(StopWait {
                    label,
                    deadline: (duration > 0).then(|| h.now_ms().saturating_add(duration as u64)),
                }));
            }
            "play" => {
                m.thread_globals_mut()
                    .insert("resultstopint".into(), Value::Int(1));
                self.wait = None;
                self.stop_poll = None;
            }
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
            "lowc" | "upc" => {
                ensure!(a.len() == 1, "QLIE string case ABI");
                let text = m.eval(&a[0])?.text();
                m.result_text(
                    0,
                    if ins.op == "lowc" {
                        text.to_lowercase()
                    } else {
                        text.to_uppercase()
                    },
                );
            }
            "elen" => {
                ensure!(a.len() == 1, "QLIE scalar length ABI");
                m.result(
                    0,
                    Value::Int(i32::try_from(m.eval(&a[0])?.text().chars().count())?),
                );
            }
            "epos" => {
                ensure!(a.len() == 2, "QLIE scalar position ABI");
                let needle = m.eval(&a[0])?.text();
                let text = m.eval(&a[1])?.text();
                let position = if needle.is_empty() {
                    0
                } else {
                    text.find(&needle)
                        .map(|byte| text[..byte].chars().count() + 1)
                        .unwrap_or(0)
                };
                m.result(0, Value::Int(i32::try_from(position)?));
            }
            "ecpy" => {
                ensure!(a.len() == 3, "QLIE scalar substring ABI");
                let text = m.eval(&a[0])?.text();
                let start = m.eval(&a[1])?.integer()?.max(1) as usize - 1;
                let length = m.eval(&a[2])?.integer()?;
                let length = if length < 1 {
                    text.len()
                } else {
                    length as usize
                };
                m.result_text(0, text.chars().skip(start).take(length).collect::<String>());
            }
            "lexs" => {
                ensure!(a.len() == 1, "QLIE label existence ABI");
                let label = m.call_parameter(&a[0])?.text();
                m.result(0, Value::Int(i32::from(m.source_label_exists(&label)?)));
            }
            "char" => {
                ensure!(a.len() == 1, "QLIE UTF16 character ABI");
                let unit = m.eval(&a[0])?.integer()? as u16;
                let character = char::from_u32(u32::from(unit))
                    .context("QLIE isolated UTF16 surrogate character")?;
                m.result_text(0, character.to_string());
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
                                Printf32Argument, format_printf32_bytes,
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
                    for i in 0..16 {
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
            // 6D648C: dthm and ddlg are explicit release-build no-ops.
            "dthm" | "ddlg" => {}
            // 6D648C/68B02C bmat,text,pattern: native reads argument 2 again
            // as the mode integer, so mode 1 (all matches) is selected only
            // when the pattern itself evaluates to 1. Mode 0 returns the
            // first match's groups; a failed match has no groups.
            "bmat" => {
                ensure!(a.len() >= 2, "QLIE regex match ABI");
                let text = m.eval(&a[0])?.text();
                let pattern_value = m.eval(&a[1])?;
                let pattern = pattern_value.text();
                let all = crate::expression::parse_integer(&pattern) == Some(1);
                let regex = regex::RegexBuilder::new(&pattern)
                    .size_limit(1024 * 1024)
                    .build()?;
                let found: Vec<String> = if all {
                    regex
                        .find_iter(&text)
                        .map(|m| m.as_str().to_owned())
                        .collect()
                } else {
                    regex
                        .captures(&text)
                        .map(|c| {
                            c.iter()
                                .map(|g| g.map(|g| g.as_str().to_owned()).unwrap_or_default())
                                .collect()
                        })
                        .unwrap_or_default()
                };
                m.result(0, Value::Int(i32::try_from(found.len())?));
                for (i, s) in found.into_iter().take(16).enumerate() {
                    m.result_text(i, s);
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
                // Native 6D1B24 dispatches every selected object in order.
                // Query handlers reset their own result-bank cursor (6EA550),
                // so the final selected object owns the observable results;
                // this is not an aggregate result-packing operation.
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
        self.load.is_some()
            || self.image_job.is_some()
            || self.thread_image.is_some()
            || self.filter_job.is_some()
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
        let new_tick = self.last_tick_ms != Some(h.now_ms());
        self.filter_input(h.now_ms(), new_tick);
        if new_tick {
            self.tick_ms = self
                .last_tick_ms
                .map(|t| h.now_ms().saturating_sub(t))
                .unwrap_or(0);
            self.last_tick_ms = Some(h.now_ms());
            self.advance_timers();
            self.advance_swipe(h.now_ms());
            if let Err(e) = self.advance_cursor(h).and_then(|_| self.advance_gesture_guide(h)) {
                self.error = Some(format!("{e:#}"));
                return Err(Error::Io(format!("{e:#}")));
            }
        }
        if self.halt_requested {
            return Ok(VmYield::Halted);
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
        let messages = self.objects.iter().filter(|(_,o)|(o.created&&o.visible)||o.message.is_some())
            .map(|(id,o)|json!({"id":id,"kind":o.kind,"thread":o.thread.native_name(),"layer":o.layer,
                "asset":o.asset,"visible":o.visible,"created":o.created,"has_message":o.message.is_some(),
                "text_layer":o.message.as_ref().map(|m|m.layer),
                "text_created":o.message.as_ref().map(|m|m.created),
                "text_parent":o.message.as_ref().and_then(|m|m.bound_parent),
                "images":o.bundle.as_ref().map(|b|b.images.iter().map(|i|json!({
                    "slot":i.slot,"name":i.name,"native_name":i.metadata["native_name"]})).collect::<Vec<_>>())}))
            .collect::<Vec<_>>();
        serde_json::to_vec(&json!({"vm":"qlie","thread":self.machine.as_ref().map(|m|m.thread_kind().native_name()),"source_thread":self.machine.as_ref().map(|m|m.source_thread_kind().native_name()),"game_paused":self.suspended_host.is_some(),"module":self.machine.as_ref().map(|m|&m.frame.path),"pc":self.machine.as_ref().map(|m|m.frame.pc),"line":self.machine.as_ref().and_then(|m|m.current().ok().flatten().map(|i|i.line)),"instructions":self.machine.as_ref().map(|m|m.executed),"call_depth":self.machine.as_ref().map(|m|m.stack.len()),"callers":self.machine.as_ref().map(|m|m.stack.iter().map(|f|json!({"module":f.path,"pc":f.pc,"line":m.programs.get(&f.path).and_then(|p|p.code.get(f.pc)).map(|i|i.line)})).collect::<Vec<_>>()),"objects":self.objects.len(),"visual_objects":messages,"unavailable_capabilities":self.unavailable,"engine_api_calls":self.api_calls,"interactive_objects":self.objects.iter().filter(|(_,o)|o.ui_registered&&o.visible).map(|(id,o)|json!({"id":id,"layer":o.layer,"asset":o.asset,"variables":self.machine.as_ref().map(|m|m.global_bindings().filter(|(_,v)|v.text()==*id).map(|(k,_)|k.clone()).collect::<Vec<_>>())})).collect::<Vec<_>>(),"error":self.error})).unwrap_or_default()
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
