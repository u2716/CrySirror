//! Original D.C. scenario runtime: the 404600 #ADV_TXT dispatcher, the
//! 408F70 presentation state machine, 4013F0 per-character reveal and the
//! 414760 day flow. This module owns native state only. Images, glyphs,
//! audio and composition are projected by the adapter over shared services;
//! nothing here draws, mixes or touches a window.
use crate::{
    advtxt::{Program, Record},
    dc1::State as Flags,
    dc1_frontend::Story as Tables,
};
use anyhow::{Context, Result, bail, ensure};
use std::{collections::BTreeMap, sync::Arc};

/// 6858E8/687320 hold 25 native 256-byte lines.
pub const LINES: usize = 25;
const LINE_BYTES: usize = 256;
/// 408A60 pen origin and 411ED0's 30px cells over the h[4] 30x15 font.
const PEN_X: i32 = 48;
const PEN_Y: i32 = 448;
const ADVANCE: i32 = 28;
const LINE_HEIGHT: i32 = 30;
/// 418160 defaults that the scenario path reads.
const RUBY_ENABLED: bool = true; // BA9DD8
const DEFAULT_GRADE: i32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// 42A130: native directory index and loop policy.
    Music { id: i32, looping: bool },
    /// 42A360(0): 1 dB per timer17 tick from the current level to -50 dB.
    MusicFade,
    /// 42A4E0(1) from the option screen's zero BGM level.
    MusicStopNow,
    /// 42A970(0): DirectSound attenuation in hundredths of dB.
    MusicAttenuation(i32),
    VoiceAttenuation(i32),
    SoundAttenuation(i32),
    Sound { id: i32, looping: bool },
    SoundStop,
    /// 42A690 path after 404600's scene-directory rewrite.
    Voice(String),
    VoiceStop,
    /// 4250D0 captures the displayed frame before the wipe input changes.
    WipeBegin(Box<Visual>),
    WipeEnd,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub x: i32,
    pub y: i32,
    /// 30 for 411ED0 cells, 12 for the GDI ruby font (F9B784).
    pub size: u8,
    pub text: String,
    pub color: u8,
    /// Shadow offset and palette index (411ED0 6 at +2, 402E60 2 at +1).
    pub shadow: ([i32; 2], u8),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sprite {
    /// 68BF90: slot owns loaded images.
    pub present: bool,
    /// 684F90 source identity without the char/ prefix.
    pub name: String,
    pub x: i32,
    pub y: i32,
}
impl Sprite {
    /// 403810/401040 strip the first '@' or '*' before appending 'm'.
    pub fn mask(&self) -> String {
        let end = self
            .name
            .bytes()
            .position(|b| b == b'@' || b == b'*')
            .unwrap_or(self.name.len());
        format!("{}m", &self.name[..end])
    }
    pub fn image(&self) -> &str {
        let end = self
            .name
            .bytes()
            .position(|b| b == b'@' || b == b'*')
            .unwrap_or(self.name.len());
        &self.name[..end]
    }
}

/// 42B8C0 map picture: hovered row (154135C) and preview (1541360).
#[derive(Clone, Debug, PartialEq)]
pub struct MapScreen {
    pub hover: i32,
    pub preview: i32,
    /// 403E10 month, day and weekday composed from Messwin3.
    pub date: [i32; 3],
}

/// Retained inputs of 4030D0's background/overlay/sprite pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Visual {
    /// 42B3C0 replaces the scenario picture with the map.
    pub map: Option<MapScreen>,
    /// 68C060 without the char/ prefix; None before the first load.
    pub background: Option<String>,
    pub offset: [i32; 2],
    /// 68D518 and 68A668 (1..4), with 68A66C for modes 3 and 4.
    pub overlay: Option<(String, i32, i32)>,
    pub sprites: [Sprite; 4],
    /// 68BFB4/68BFBC: slot-0 char/bsteff overlay drawn by 403AB0.
    pub effect: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choice {
    pub title: Vec<u8>,
    pub title_at: [i32; 2],
    pub items: Vec<(i32, Vec<u8>)>,
    pub enabled: [bool; 15],
    pub selected: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub x: i32,
    pub y: i32,
    /// BA9F58: left button or Enter.
    pub click: bool,
    /// BA9F48: Space (hides the window in the scenario).
    pub hide: bool,
    /// BA9F70: right button, Escape or Backspace.
    pub cancel: bool,
    /// BA9F94/BA9F98: wheel or PageUp/PageDown.
    pub wheel_up: bool,
    pub wheel_down: bool,
    /// GetAsyncKeyState(VK_CONTROL).
    pub skip: bool,
    /// Left button still held (40B7E0 waits for its release).
    pub held: bool,
}

/// 40B7E0 system menu (6B0E88 hover, 440978 chosen, 6B0E90 armed).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemMenu {
    pub hover: Option<usize>,
    pub chosen: Option<usize>,
}
/// 414500 confirmation (BCE05C hover).
#[derive(Clone, Debug, PartialEq)]
pub struct Dialog {
    pub prompt: Vec<u8>,
    pub at: [i32; 2],
    pub hover: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// 408F70 state32 yes: BAB40C, back to the title (413190).
    ReturnToTitle,
}
/// One backlog entry: speaker (when a new name started the page) and the
/// page's raw lines (46F768/658CB8/68AFEC).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryEntry {
    pub speaker: Option<Vec<u8>>,
    pub lines: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Running,
    /// A scenario file must be supplied before this state can continue.
    NeedScene(String),
    /// The day flow reached an original submode not yet ported.
    Unsupported(&'static str),
}

/// Native state named by the original globals it replaces.
pub struct Story {
    pub tables: Arc<Tables>,
    pub option_tables: crate::dc1_frontend::ConfigTables,
    /// 418160/407CC0 system settings consulted by the scenario.
    pub settings: crate::dc1_config::Settings,
    pub flags: Flags,
    programs: BTreeMap<String, Arc<Program>>,
    program: Option<Arc<Program>>,
    /// Format: current scenario identity, e.g. sinario/0223a_f01.
    pub scene: String,
    pc: u32,
    pub now: u64,
    pub events: Vec<Event>,
    /// 684C58 and 684C5C.
    pub mode: i32,
    prev_mode: i32,
    // 404600 state.
    dispatching: bool,
    line_cursor: usize,
    yield_values: [i32; 5],
    text_field: Vec<u8>,
    // Pending lines and reveal cursor.
    pending: [Vec<u8>; LINES],
    line: usize,
    pos: usize,
    pen: [i32; 2],
    saved_pen: [i32; 2],
    shake: [i32; 2],
    ruby_line: bool,
    emphasis: bool,
    pub glyphs: Vec<Glyph>,
    // Window/name state.
    pub window: bool,
    pub name_shown: bool,
    name_drawn: bool,
    pub name: Vec<u8>,
    pub name_glyphs: Vec<Glyph>,
    text_color: i32,
    text_speed: u64,
    last_char: u64,
    fast: bool,
    saved_speed: u64,
    wait_at: u64,
    wait_len: u64,
    auto_text: bool,
    keep_page: bool,
    force_reset: bool,
    auto_advance: bool,
    no_cursor: bool,
    read_scene: bool,
    auto_skip: bool,
    // Cursor (403D00).
    pub cursor: Option<u32>,
    cursor_on: bool,
    cursor_frame: u32,
    cursor_at: u64,
    // Visual inputs.
    pub visual: Visual,
    bg_base: [i32; 2],
    scroll: [i32; 2],
    scroll_speed: i32,
    overlay_mode: i32,
    overlay_offset: i32,
    overlay_name: Option<String>,
    next_background: String,
    transition: i32,
    pub wipe: Option<i32>,
    pub grade: i32,
    pub choice: Option<Choice>,
    choice_capture_title: bool,
    choice_capture_item: bool,
    choice_building: Choice,
    /// 658C3C: 408A60/401040 set all fifteen; 76/77 do not reset them.
    choice_enabled: [bool; 15],
    // 40BAA0 slot1 place name.
    pub place: Option<(i32, i32)>,
    place_counter: i32,
    place_index: i32,
    // Shake (mode 23).
    shake_phase: i32,
    pub screen_shake: [i32; 2],
    // Day flow (414760).
    day_state: i32,
    pub day: usize,
    timers: [u64; 21],
    /// BA9E88 intervals: 4125F0(-1) sets 10; 408A60 sets slot14 200 and
    /// slot15 3000; opcode 97 writes slot10 (BA9EB0).
    intervals: [u64; 21],
    /// 68A640: heroine eyecatch index, shown by 4030D0 instead of the scene.
    pub eyecatch: Option<usize>,
    // History marks.
    seen_events: Vec<bool>,
    effects_enabled: bool,
    saved_window: (bool, bool),
    voice_playing: bool,
    text_field_scene: Option<String>,
    yield_sprite: Option<String>,
    music: i32,
    /// 6855AC..6855B4 month/day/weekday and 6855B8/BC time of day.
    pub date: [i32; 3],
    pub time_of_day: [i32; 2],
    reset_scroll_on_wipe: bool,
    choice_last_pointer: Option<[i32; 2]>,
    blank_after: bool,
    /// BA9F30: when the current page finished (auto-advance origin).
    page_done_at: u64,
    /// 408F70 window buttons: hovered and pressed rows (43D0A8).
    pub window_hover: Option<usize>,
    pub window_pressed: Option<usize>,
    pub menu: Option<SystemMenu>,
    pub dialog: Option<Dialog>,
    pub option: Option<crate::dc1_config::ConfigScreen>,
    pub request: Option<Request>,
    pub history: Vec<HistoryEntry>,
    /// 6B0480 first shown entry; Some while 40BCD0 owns the screen.
    pub backlog: Option<usize>,
    pub backlog_glyphs: Vec<Glyph>,
    /// 68A58C/68A588 and 6B0E34/6B0E38 saved name/window flags.
    menu_saved: (bool, bool),
    /// 68BFB0[0]: 52 requested an overlay applied by 403810.
    effect_pending: Option<usize>,
    /// 68BFB8: overlay clear deferred to the next 403810/403B80/403C90.
    effect_clear: [bool; 4],
    /// 42B3C0 dword_1541358.
    map_state: i32,
    wipe_step: i32,
}

fn copy_line(bytes: &[u8]) -> Vec<u8> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    bytes[..end.min(LINE_BYTES - 1)].to_vec()
}
/// 404600 swaps each encoded byte pair before the CP932 path.
pub fn swap_text(operands: &[u8]) -> Result<Vec<u8>> {
    let length = operands
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(operands.len());
    ensure!(length < LINE_BYTES, "DC1 native text field exceeds its 256-byte line");
    let mut out = Vec::with_capacity(length + 1);
    for at in (0..length).step_by(2) {
        out.push(if at + 1 < length { operands[at + 1] } else { 0 });
        out.push(operands[at]);
    }
    Ok(copy_line(&out))
}
pub fn decode(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS
        .decode_without_bom_handling(bytes)
        .0
        .into_owned()
}
fn c_string(bytes: &[u8]) -> Result<String> {
    let raw = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(raw);
    ensure!(!error && raw.len() < 256, "DC1 native resource field");
    Ok(text.replace('\\', "/"))
}
fn starts(bytes: &[u8], at: usize, token: &[u8]) -> bool {
    bytes.get(at..at + token.len()) == Some(token)
}
fn starts_ci(bytes: &[u8], at: usize, token: &[u8]) -> bool {
    bytes
        .get(at..at + token.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(token))
}
const OPEN_RUBY: &[u8] = &[0x81, 0x6f]; // ｛
const CLOSE_RUBY: &[u8] = &[0x81, 0x70]; // ｝
const OPEN_DOTS: &[u8] = &[0x81, 0xe1]; // ≪
const CLOSE_DOTS: &[u8] = &[0x81, 0xe2]; // ≫
const BRACKET: &[u8] = &[0x81, 0x79]; // 【

impl Story {
    pub fn new(
        tables: Arc<Tables>,
        option_tables: crate::dc1_frontend::ConfigTables,
        first_scene: String,
        program: Arc<Program>,
    ) -> Result<Self> {
        let flags = Flags::new(first_scene.clone(), &program)?;
        let mut story = Self {
            settings: crate::dc1_config::Settings::default(),
            option_tables,
            tables,
            flags,
            programs: BTreeMap::new(),
            program: None,
            scene: String::new(),
            pc: 0,
            now: 0,
            events: Vec::new(),
            mode: 0,
            prev_mode: 0,
            dispatching: false,
            line_cursor: 0,
            yield_values: [-1; 5],
            text_field: Vec::new(),
            pending: Default::default(),
            line: 0,
            pos: 0,
            pen: [PEN_X, PEN_Y],
            saved_pen: [32, 400],
            shake: [0, 0],
            ruby_line: false,
            emphasis: false,
            glyphs: Vec::new(),
            window: false,
            name_shown: false,
            name_drawn: false,
            name: Vec::new(),
            name_glyphs: Vec::new(),
            text_color: 0,
            text_speed: 10,
            last_char: 0,
            fast: false,
            saved_speed: 0,
            wait_at: 0,
            wait_len: 1000,
            auto_text: false,
            keep_page: false,
            force_reset: false,
            auto_advance: false,
            no_cursor: false,
            read_scene: false,
            auto_skip: false,
            cursor: None,
            cursor_on: false,
            cursor_frame: 0,
            cursor_at: 0,
            visual: Visual::default(),
            bg_base: [0, 0],
            scroll: [0, 0],
            scroll_speed: 4,
            overlay_mode: 0,
            overlay_offset: 0,
            overlay_name: None,
            next_background: String::new(),
            transition: 0,
            wipe: None,
            grade: DEFAULT_GRADE,
            choice: None,
            choice_capture_title: false,
            choice_capture_item: false,
            choice_building: Choice::default(),
            choice_enabled: [true; 15],
            place: None,
            place_counter: 0,
            place_index: -1,
            shake_phase: 0,
            screen_shake: [0, 0],
            day_state: 0,
            day: 0,
            timers: [0; 21],
            intervals: {
                let mut intervals = [10; 21];
                intervals[14] = 200;
                intervals[15] = 3000;
                intervals
            },
            eyecatch: None,
            seen_events: vec![false; 999],
            effects_enabled: false,
            saved_window: (false, false),
            voice_playing: false,
            text_field_scene: None,
            yield_sprite: None,
            music: -1,
            date: [0; 3],
            time_of_day: [0; 2],
            reset_scroll_on_wipe: false,
            choice_last_pointer: None,
            blank_after: false,
            page_done_at: 0,
            window_hover: None,
            window_pressed: None,
            menu: None,
            dialog: None,
            option: None,
            request: None,
            history: Vec::new(),
            backlog: None,
            backlog_glyphs: Vec::new(),
            menu_saved: (false, false),
            effect_pending: None,
            effect_clear: [false; 4],
            map_state: 0,
            wipe_step: 32,
        };
        story.name = story.tables.default_speaker.clone();
        //408A60: 40D590(0, BLACK) before the first scenario record.
        story.visual.background = Some("BLACK".into());
        story.programs.insert(first_scene.to_ascii_lowercase(), program);
        Ok(story)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"scene":self.scene,"pc":self.pc,"mode":self.mode,"day":self.day,"day_state":self.day_state,"map_state":self.map_state,"wipe":self.wipe,"instructions":self.flags.instructions})
    }
    /// 684C60: the current scene is already marked read.
    pub fn read_scene(&self) -> bool {
        self.read_scene
    }
    pub fn insert_program(&mut self, name: &str, program: Arc<Program>) {
        self.programs.insert(name.to_ascii_lowercase(), program);
    }
    /// 4125F0: true once more than the slot interval (10 ms) has elapsed.
    fn timer(&mut self, slot: usize) -> bool {
        if self.now.saturating_sub(self.timers[slot]) <= self.intervals[slot] {
            return false;
        }
        self.timers[slot] = self.now;
        true
    }
    fn reset_timers(&mut self) {
        self.timers = [self.now; 21];
    }
    fn program(&self) -> Result<&Arc<Program>> {
        self.program.as_ref().context("DC1 scenario program missing")
    }
    /// 4042A0: open a scene and seek to its first record.
    fn load(&mut self, scene: &str) -> Result<bool> {
        let key = scene.replace('\\', "/").to_ascii_lowercase();
        let Some(program) = self.programs.get(&key).cloned() else {
            return Ok(false);
        };
        self.pc = u32::try_from(program.code_file_offset)?;
        self.program = Some(program);
        self.scene = key;
        self.pen = [PEN_X, PEN_Y];
        Ok(true)
    }
    fn jump(&mut self, label: i32) -> Result<()> {
        let program = self.program()?;
        let target = *program
            .labels
            .get(&label)
            .context("DC1 native label is not declared")?;
        program.instruction(target)?;
        self.pc = target;
        Ok(())
    }

    // ---------------------------------------------------------------
    // 414760 day flow (subset) and 408F70 per-frame state machine.
    // ---------------------------------------------------------------
    pub fn frame(&mut self, input: Input) -> Result<Step> {
        match self.day_state {
            0 => {
                self.day_state = 1;
                return Ok(Step::Running);
            }
            1 | 3 => {
                //408A60(0): new presentation state, then 4042A0.
                let scene = if self.day_state == 1 {
                    self.flags.scene.clone()
                } else {
                    self.tables.day_scenes[self.day]
                        .clone()
                        .context("DC1 native day scenario")?
                };
                if self.day_state == 3 {
                    self.reset_presentation();
                }
                if !self.load(&scene)? {
                    return Ok(Step::NeedScene(scene));
                }
                self.reset_timers();
                //414760 states 1/3: 6855B0/B4/B8 from the 44DC70 day row.
                let row = self.tables.days[self.day];
                self.date[1] = row[0];
                self.date[2] = row[1];
                self.time_of_day[0] = row[2];
                self.day_state += 1;
                return Ok(Step::Running);
            }
            2 | 4 => {
                if !self.tick(input)? {
                    return Ok(self.pending_scene());
                }
                if self.mode != -1 {
                    return Ok(Step::Running);
                }
                self.mode = 0;
                self.day_state = if self.day_state == 2 { 11 } else { 5 };
                return Ok(Step::Running);
            }
            11 => {
                if self.tables.days[self.day][5] == 0 {
                    self.day_state = 17;
                    return Ok(Step::Running);
                }
                //42B370: map resources, BGM 12 looping, choice flags cleared.
                self.map_state = 0;
                for bank in 10..13 {
                    self.flags.flags[bank * 99] = 0;
                }
                if self.music != 12 {
                    self.music = 12;
                    self.events.push(Event::Music { id: 12, looping: true });
                }
                self.day_state = 12;
                Ok(Step::Running)
            }
            12 => {
                if self.map(input)? {
                    self.day_state = 13;
                }
                Ok(Step::Running)
            }
            13 => {
                //414760 state13: 401000(0), then the 44DC28 scenario.
                let scene = self.tables.map_scenes[self.day]
                    .clone()
                    .context("DC1 native map scenario")?;
                self.reset_presentation();
                if !self.load(&scene)? {
                    return Ok(Step::NeedScene(scene));
                }
                self.day_state = 14;
                Ok(Step::Running)
            }
            14 => {
                if !self.tick(input)? {
                    return Ok(self.pending_scene());
                }
                if self.mode == -1 {
                    self.mode = 0;
                    self.day_state = 17;
                }
                Ok(Step::Running)
            }
            5 => Ok(Step::Unsupported("DC1 414760 day event submodes (42C090/42B9E0)")),
            17 => {
                self.day += 1;
                if self.day == 11 {
                    return Ok(Step::Unsupported("DC1 414760 final-day routing"));
                }
                self.day_state = 3;
                Ok(Step::Running)
            }
            _ => bail!("DC1 day flow state {}", self.day_state),
        }
    }
    /// 408A60(0) presentation reset used by 414760 before each scenario.
    fn reset_presentation(&mut self) {
        self.visual = Visual {
            background: Some("BLACK".into()),
            ..Visual::default()
        };
        self.effect_pending = None;
        self.effect_clear = [false; 4];
        self.window = false;
        self.name_shown = false;
        self.name_drawn = false;
        self.name = self.tables.default_speaker.clone();
        self.name_glyphs.clear();
        self.glyphs.clear();
        for line in &mut self.pending {
            line.clear();
        }
        self.pen = [PEN_X, PEN_Y];
        self.shake = [0, 0];
        self.bg_base = [0, 0];
        self.scroll = [0, 0];
        self.screen_shake = [0, 0];
        self.overlay_mode = 0;
        self.overlay_offset = 0;
        self.mode = 0;
        self.choice = None;
        self.place = None;
        self.text_speed = 10;
        self.auto_text = false;
        self.keep_page = false;
        self.force_reset = false;
        self.cursor_on = false;
        self.cursor = None;
        self.eyecatch = None;
        self.dispatching = false;
    }
    /// 42B3C0. Returns true when the selection has faded to black.
    fn map(&mut self, input: Input) -> Result<bool> {
        let date = [self.date[1], self.date[2], self.time_of_day[0]];
        match self.map_state {
            0 => {
                let before = self.current_visual();
                self.window = false;
                self.name_shown = false;
                self.glyphs.clear();
                self.name_glyphs.clear();
                self.cursor = None;
                self.visual = Visual {
                    map: Some(MapScreen { hover: -1, preview: -1, date }),
                    ..Visual::default()
                };
                self.events.push(Event::WipeBegin(Box::new(before)));
                //426260 advances 2*BA9D9C on its first call.
                self.wipe_step = 2 * self.settings.wipe_step;
                self.wipe = Some(self.wipe_step);
                self.map_state = 1;
            }
            1 | 7 => {
                if self.timer(10) {
                    let progress = self.wipe.context("DC1 map wipe")? + self.wipe_step;
                    if progress >= 512 {
                        self.wipe = None;
                        self.events.push(Event::WipeEnd);
                        self.map_state += 1;
                        return Ok(self.map_state == 8);
                    }
                    self.wipe = Some(progress);
                }
            }
            2 => {
                let map = self.visual.map.as_mut().context("DC1 map state")?;
                let mut chosen = false;
                for (index, rect) in self.tables.map_rects.iter().enumerate() {
                    if input.x >= rect[0] && input.y >= rect[1] && input.x < rect[2] && input.y < rect[3] {
                        if map.hover != index as i32 {
                            map.hover = index as i32;
                            map.preview = index as i32;
                            self.events.push(Event::Sound { id: 28, looping: false });
                        }
                        if input.click {
                            chosen = true;
                        }
                    }
                }
                if chosen {
                    self.events.push(Event::Sound { id: 19, looping: false });
                    map.preview = -1;
                    let bank = 10 + map.hover as usize;
                    self.flags.flags[bank * 99] = 1;
                    self.map_state = 6;
                }
            }
            6 => {
                //41A130 clears the back buffer: the map wipes to black.
                let before = self.current_visual();
                self.visual = Visual::default();
                self.events.push(Event::WipeBegin(Box::new(before)));
                self.wipe = Some(self.wipe_step);
                self.map_state = 7;
            }
            other => bail!("DC1 42B3C0 state {other}"),
        }
        Ok(false)
    }
    fn pending_scene(&self) -> Step {
        Step::NeedScene(self.text_field_scene.clone().unwrap_or_default())
    }

    /// One 408F70 call. Returns false when a scene file is pending.
    fn tick(&mut self, input: Input) -> Result<bool> {
        let mut skip = input.skip && self.read_scene;
        let mut click = input.click;
        let mut hide = input.hide;
        //LABEL_27: BA9F70 opens 40B7E0 from reading/choice/wait modes.
        if input.cancel
            && (matches!(self.mode, 1 | 15 | 25) || (self.mode == 17 && self.eyecatch.is_none()))
        {
            self.menu_saved = (self.window, self.name_shown);
            self.window = false;
            self.name_shown = false;
            self.prev_mode = self.mode;
            self.menu = Some(SystemMenu::default());
            self.mode = 26;
            return Ok(true);
        }
        //LABEL_37: Messwin3 buttons while the window is shown.
        self.window_pressed = None;
        if matches!(self.mode, 1 | 15 | 17 | 18) && self.window && self.eyecatch.is_none() {
            let mut action = None;
            self.window_hover = None;
            for (index, row) in self.tables.window_buttons.iter().enumerate() {
                if index == 3 && !self.read_scene {
                    continue;
                }
                let (left, top) = (696 + row[0], 440 + row[1]);
                if index == 1 && input.wheel_up {
                    action = Some(1);
                }
                if index == 2 && input.wheel_down {
                    action = Some(2);
                }
                if input.x >= left && input.y >= top && input.x < 696 + row[2] && input.y < 440 + row[3] {
                    self.window_hover = Some(index);
                    if click {
                        self.window_pressed = Some(index);
                        action = Some(index);
                    }
                }
            }
            match action {
                Some(0) => {
                    hide = true;
                    click = false;
                }
                Some(1) => {
                    self.prev_mode = self.mode;
                    self.mode = 19;
                    return Ok(true);
                }
                Some(2) => {
                    //v51: the down button acts as the skip key here.
                    skip = true;
                    click = false;
                }
                Some(_) => {
                    self.auto_skip = !self.auto_skip;
                    click = false;
                }
                None => {}
            }
        } else {
            self.window_hover = None;
        }
        //LABEL_86: BA9F48 hides the window and name until the next input.
        if hide && matches!(self.mode, 1 | 15 | 16 | 17 | 25) && self.wipe.is_none() {
            self.prev_mode = self.mode;
            self.saved_window = (self.window, self.name_shown);
            self.window = false;
            self.name_shown = false;
            self.mode = 14;
            return Ok(true);
        }
        match self.mode {
            0 => {
                let Some(op) = self.dispatch()? else {
                    return Ok(false);
                };
                self.after_dispatch(op)?;
            }
            1 => self.choose(input)?,
            4..=7 => self.scroll_step(click),
            8 | 9 => self.overlay_slide(click),
            10 => self.wipe_step()?,
            11 => {
                self.window = false;
                self.name_shown = false;
                self.name_drawn = false;
                self.mode = 0;
            }
            19 => self.open_backlog(),
            20 => self.backlog_step(input),
            26 => self.menu_step(input)?,
            27 => {
                let mut screen = self.option.take().context("DC1 in-game option state")?;
                let action = screen.input(
                    &self.option_tables,
                    &mut self.settings,
                    &crate::dc1_config::ConfigInput {
                        x: input.x,
                        y: input.y,
                        pointer: true,
                        select: input.click || input.hide,
                        cancel: input.cancel,
                    },
                );
                match action {
                    crate::dc1_config::ConfigAction::Close => {
                        self.menu = Some(SystemMenu::default());
                        self.mode = 26;
                    }
                    crate::dc1_config::ConfigAction::NameEntry => {
                        bail!("DC1 41E180 player-name entry is not ported")
                    }
                    crate::dc1_config::ConfigAction::MusicOff => {
                        self.events.push(Event::MusicStopNow);
                        self.option = Some(screen);
                    }
                    crate::dc1_config::ConfigAction::VoiceOff => {
                        self.events.push(Event::VoiceStop);
                        self.option = Some(screen);
                    }
                    crate::dc1_config::ConfigAction::PreviewSound => {
                        self.events.push(Event::Sound { id: 19, looping: false });
                        self.option = Some(screen);
                    }
                    _ => self.option = Some(screen),
                }
            }
            31 | 32 => {
                let dialog = self.dialog.as_mut().context("DC1 414500 dialog")?;
                match dialog_step(dialog, &self.tables.dialog_rows, input) {
                    Some(true) if self.mode == 32 => {
                        self.dialog = None;
                        self.request = Some(Request::ReturnToTitle);
                        self.mode = -2;
                    }
                    Some(true) => bail!("DC1 408F70 state31 window-size change is not ported"),
                    Some(false) => {
                        self.dialog = None;
                        self.menu = Some(SystemMenu::default());
                        self.mode = 26;
                    }
                    None => {}
                }
            }
            -2 => {}
            14 => {
                if click || hide || input.cancel {
                    self.window = self.saved_window.0;
                    self.name_shown = self.saved_window.1;
                    self.mode = self.prev_mode;
                }
            }
            15 => {
                if !self.window {
                    self.show_window();
                }
                if self.name_shown && !self.name_drawn {
                    self.draw_name();
                }
                //v18: BA9E3C == 1 shows the page at once unless 684C74 is set.
                let instant = self.settings.text_mode == 1 && !self.auto_text;
                if (click && !self.fast) || skip || instant {
                    if !self.auto_text {
                        self.saved_speed = self.text_speed;
                        self.mode = 24;
                        self.fast = true;
                    }
                    return Ok(true);
                }
                let pace = if self.auto_text { 150 } else { self.text_speed };
                if self.now.saturating_sub(self.last_char) <= pace {
                    return Ok(true);
                }
                self.last_char = self.now;
                if self.reveal()? {
                    self.mode = 16;
                }
            }
            16 => {
                self.cursor_on = true;
                self.cursor_frame = 0;
                self.cursor_at = self.now;
                self.mode = 17;
                self.line = 0;
                self.pos = 0;
                self.wait_at = self.now;
                self.wait_len = 1000;
                self.page_done_at = self.now;
                if self.blank_after {
                    self.pen[1] += 32;
                }
                if self.fast {
                    self.fast = false;
                    self.text_speed = self.saved_speed;
                }
                self.animate_cursor();
            }
            17 => {
                let mut done = (self.read_scene && skip) || (!self.read_scene && skip && !self.force_reset);
                if self.auto_advance {
                    self.auto_advance = false;
                    done = true;
                }
                //BA9E4C: BA9F34 ms after the page, once the voice has ended.
                if let Some(interval) = self.settings.auto_advance {
                    if self.now.saturating_sub(self.page_done_at) > interval && !self.voice_playing {
                        self.page_done_at = self.now;
                        done = true;
                    }
                }
                if self.eyecatch.is_some() {
                    done = self.timer(15);
                }
                if click || done {
                    self.cursor_on = false;
                    self.cursor = None;
                    self.mode = 0;
                    self.no_cursor = false;
                    self.eyecatch = None;
                    if !self.keep_page || self.force_reset {
                        self.force_reset = false;
                        self.new_page();
                    }
                    for line in &mut self.pending {
                        line.clear();
                    }
                } else if !self.no_cursor {
                    self.animate_cursor();
                }
            }
            18 => {
                if self.now.saturating_sub(self.wait_at) > self.wait_len || click {
                    self.wait_at = self.now;
                    self.mode = self.prev_mode;
                }
            }
            21 | 22 => {
                if !self.voice_playing {
                    self.mode = 0;
                }
            }
            23 => self.shake_step(),
            24 => {
                for _ in 0..1000 {
                    if self.reveal()? {
                        self.mode = 16;
                        break;
                    }
                    if self.mode != 24 {
                        break;
                    }
                }
            }
            25 => {
                if click || skip {
                    self.cursor_on = false;
                    self.cursor = None;
                    self.mode = 15;
                } else {
                    self.animate_cursor();
                }
            }
            -1 => {}
            other => bail!("DC1 408F70 presentation mode {other} is not ported"),
        }
        //408F70 tail: 40BB30 runs in modes 0/11/15/17/18 only.
        if matches!(self.mode, 0 | 11 | 15 | 17 | 18) {
            self.place_step();
        }
        Ok(true)
    }

    /// 40B7E0. Returns to the saved mode, or enters a submode.
    fn menu_step(&mut self, input: Input) -> Result<()> {
        let menu = self.menu.as_mut().context("DC1 system menu")?;
        if let Some(chosen) = menu.chosen {
            //6B0E90: act once the left button is released.
            if input.held {
                return Ok(());
            }
            self.menu = None;
            match chosen {
                0 => bail!("DC1 4237C0 save screen is not ported"),
                1 => bail!("DC1 4237C0 load screen is not ported"),
                2 => {
                    self.dialog = Some(Dialog { prompt: self.tables.window_prompt.clone(), at: [520, 32], hover: -1 });
                    self.mode = 31;
                }
                3 => {
                    self.dialog = Some(Dialog { prompt: self.tables.return_prompt.clone(), at: [520, 32], hover: -1 });
                    self.mode = 32;
                }
                4 => {
                    self.option = Some(crate::dc1_config::ConfigScreen::default());
                    self.mode = 27;
                }
                _ => self.close_menu(),
            }
            return Ok(());
        }
        let mut close = input.cancel;
        for (index, row) in self.tables.menu_rows.iter().enumerate() {
            if input.x >= row[0] + 496 && input.y >= row[1] && input.x < row[2] + 496 && input.y < row[3] {
                if menu.hover != Some(index) {
                    menu.hover = Some(index);
                    self.events.push(Event::Sound { id: 28, looping: false });
                }
                if input.click {
                    self.events.push(Event::Sound { id: 19, looping: false });
                    menu.chosen = Some(index);
                    close = false;
                    break;
                }
            }
        }
        if close {
            self.menu = None;
            self.close_menu();
        }
        Ok(())
    }
    fn close_menu(&mut self) {
        self.mode = self.prev_mode;
        self.window = true;
        self.name_shown = self.menu_saved.1;
    }
    /// 40BC70: open at the last page of four entries.
    fn open_backlog(&mut self) {
        self.menu_saved = (self.window, self.name_shown);
        self.window = false;
        self.name_shown = false;
        let count = self.history.len();
        self.backlog = Some(if count <= 4 { 0 } else { 4 * ((count - 1) / 4) });
        self.layout_backlog();
        self.mode = 20;
    }
    /// 40BE70: four entries from 6B0480 at pen 48,32.
    fn layout_backlog(&mut self) {
        let first = self.backlog.unwrap_or(0);
        let saved = (std::mem::take(&mut self.glyphs), self.pen);
        self.pen = [PEN_X, 32];
        for entry in self.history.iter().skip(first).take(4).cloned().collect::<Vec<_>>() {
            if let Some(speaker) = &entry.speaker {
                let format = &self.tables.speaker_format;
                let at = format.windows(2).position(|pair| pair == b"%s").unwrap_or(0);
                let mut text = format[..at].to_vec();
                text.extend_from_slice(speaker);
                text.extend_from_slice(&format[at + 2..]);
                self.glyphs.extend(cells(&text, self.pen[0], self.pen[1], 0));
                self.pen[1] += 32;
            }
            for line in entry.lines.iter().take(3) {
                self.pen[0] = PEN_X;
                self.draw_line(line, 0);
            }
            self.pen[1] += 16;
        }
        self.backlog_glyphs = std::mem::replace(&mut self.glyphs, saved.0);
        self.pen = saved.1;
    }
    /// 40BCD0.
    fn backlog_step(&mut self, input: Input) {
        let mut action = None;
        self.window_hover = None;
        for (index, row) in self.tables.backlog_rows.iter().enumerate() {
            if index == 1 && input.wheel_up {
                action = Some(1);
            }
            if index == 2 && input.wheel_down {
                action = Some(2);
            }
            let (left, top) = (696 + row[0], 440 + row[1]);
            if input.x >= left && input.y >= top && input.x < 696 + row[2] && input.y < 440 + row[3] {
                self.window_hover = Some(index);
                if input.click {
                    action = Some(index);
                }
            }
        }
        let first = self.backlog.unwrap_or(0);
        let count = self.history.len();
        match action {
            Some(0) => {
                self.close_backlog();
                return;
            }
            Some(1) => {
                self.backlog = Some(first.saturating_sub(4));
                self.layout_backlog();
            }
            Some(2) => {
                if first + 4 < count {
                    self.backlog = Some(first + 4);
                }
                self.layout_backlog();
            }
            _ => {}
        }
        if input.cancel {
            self.close_backlog();
        }
    }
    fn close_backlog(&mut self) {
        self.backlog = None;
        self.backlog_glyphs.clear();
        self.window = self.menu_saved.0;
        self.name_shown = self.menu_saved.1;
        //408F70 state20: a reading page resumes at 16.
        self.mode = if self.prev_mode == 15 { 16 } else { self.prev_mode };
    }
    fn show_window(&mut self) {
        //403610(1) on a hidden window draws Messwin3 freshly.
        self.window = true;
    }
    fn new_page(&mut self) {
        self.pen = [PEN_X, PEN_Y];
        self.events.push(Event::VoiceStop);
        self.glyphs.clear();
        //403610(1) restores the clean window (slot 11), erasing the name too;
        //68A580 stays set, so only a new opcode 26 draws a name again.
        self.name_glyphs.clear();
    }
    fn animate_cursor(&mut self) {
        //403D00(1): 100 ms per frame, eight frames.
        if !self.cursor_on {
            self.cursor = None;
            return;
        }
        self.cursor = Some(self.cursor_frame);
        if self.now.saturating_sub(self.cursor_at) > 100 {
            self.cursor_frame = (self.cursor_frame + 1) % 8;
            self.cursor_at = self.now;
        }
    }
    /// 4028D0 via 411ED0 at 54C + 12 * (6 - bytes / 2), 550.
    fn draw_name(&mut self) {
        self.name_drawn = true;
        let len = self.name.len() as i32;
        let half = if len != 0 { len / 2 } else { len };
        let x = (56 + 12 * (6 - half)).max(0);
        self.name_glyphs = cells(&self.name, x, 392, 0);
    }

    /// 4013F0 with BA9D88 set. Returns true when the page is complete.
    fn reveal(&mut self) -> Result<bool> {
        if self.line >= LINES {
            return Ok(true);
        }
        let text = self.pending[self.line].clone();
        let pos = self.pos;
        if pos >= text.len() {
            return Ok(self.line_end(LINE_HEIGHT));
        }
        if starts(&text, pos, BRACKET) {
            bail!("DC1 4013F0 cannot advance over a leading 【 in revealed text");
        }
        if starts_ci(&text, pos, b"@K") {
            self.mode = 25;
            self.pos += 2;
            self.cursor_on = true;
            self.cursor_frame = 0;
            self.cursor_at = self.now;
            return Ok(self.line_end_check(&text, LINE_HEIGHT));
        }
        if starts_ci(&text, pos, b"@R") {
            if RUBY_ENABLED {
                self.ruby_line = true;
            }
            self.pos += 2;
            return Ok(false);
        }
        if starts_ci(&text, pos, b"@N") {
            let rest = text[pos + 2..].to_vec();
            self.draw_line(&rest, 0);
            self.pos = 0;
            self.line += 1;
            return Ok(false);
        }
        if starts_ci(&text, pos, b"@W") || starts_ci(&text, pos, b"@TW") {
            let duration = if starts_ci(&text, pos, b"@TW") {
                let digit = text.get(pos + 3).copied().unwrap_or(b'0');
                self.pos += 4;
                200 * u64::from(digit.is_ascii_digit().then(|| digit - b'0').unwrap_or(0))
            } else {
                self.pos += 2;
                1000
            };
            if self.mode != 18 {
                self.prev_mode = self.mode;
                self.wait_at = self.now;
                self.wait_len = duration;
                self.mode = 18;
            }
            return Ok(self.line_end_check(&text, LINE_HEIGHT));
        }
        if starts(&text, pos, OPEN_DOTS) {
            if RUBY_ENABLED {
                self.emphasis = true;
            }
            self.pos += 2;
            return Ok(false);
        }
        if starts(&text, pos, CLOSE_DOTS) {
            self.emphasis = false;
            self.pos += 2;
            return Ok(false);
        }
        if starts(&text, pos, OPEN_RUBY) {
            self.pos += 2;
            let count = text[self.pos..]
                .chunks(2)
                .take_while(|pair| *pair != CLOSE_RUBY)
                .count() as i32;
            let shift = match count - 1 {
                0 => 4,
                1 => 5,
                _ => 6,
            };
            let mut x = self.shake[0] - 6 * shift + self.pen[0] + 6;
            let mut y = self.shake[1] + self.pen[1] - 10;
            if self.ruby_line {
                y += 8;
            }
            let color = if self.text_color < 15 { self.text_color as u8 } else { 0 };
            while self.pos < text.len() {
                if starts(&text, self.pos, CLOSE_RUBY) {
                    self.pos += 2;
                    break;
                }
                let pair = &text[self.pos..(self.pos + 2).min(text.len())];
                if RUBY_ENABLED {
                    self.glyphs.push(Glyph {
                        x,
                        y,
                        size: 12,
                        text: decode(pair),
                        color,
                        shadow: ([1, 1], 2),
                    });
                    x += 12;
                }
                self.pos += 2;
            }
            let _ = &mut x;
            return Ok(text.get(self.pos.saturating_sub(2)).is_none());
        }
        //Ordinary 411ED0 character.
        let x = (self.shake[0] + self.pen[0]).max(0);
        let mut y = (self.shake[1] + self.pen[1]).max(0);
        if self.ruby_line {
            y += 8;
        }
        if self.emphasis {
            let color = if self.text_color < 15 { self.text_color as u8 } else { 0 };
            let dot = self.tables.emphasis_dot.clone();
            self.glyphs.extend(cells(&dot, 7 + x - 8, y - 18, color));
        }
        let pair = &text[pos..(pos + 2).min(text.len())];
        self.glyphs.extend(cells(pair, x, y, 0));
        self.pen[0] += ADVANCE;
        self.pos += 2;
        Ok(self.line_end_check(&text, LINE_HEIGHT))
    }
    /// 4013F0 LABEL_115.
    fn line_end_check(&mut self, text: &[u8], height: i32) -> bool {
        if self.pos < text.len() {
            return false;
        }
        self.line_end(height)
    }
    fn line_end(&mut self, height: i32) -> bool {
        self.pen[1] += height + 2;
        self.line += 1;
        self.pos = 0;
        self.pen[0] = PEN_X;
        if self.ruby_line {
            self.pen[1] += 8;
        }
        self.ruby_line = false;
        self.line >= LINES || self.pending[self.line].is_empty()
    }
    /// 402030(line, 0) with BA9D88: whole-line draw at the current pen.
    fn draw_line(&mut self, raw: &[u8], color_mode: i32) {
        let mut text = Vec::new();
        let mut ruby_line = false;
        let mut at = 0;
        while at < raw.len() {
            if starts_ci(raw, at, b"@N") || starts_ci(raw, at, b"@W") || starts_ci(raw, at, b"@K") {
                at += 2;
            } else if starts_ci(raw, at, b"@TW") {
                at += 4;
            } else if starts_ci(raw, at, b"@B") {
                at += 2;
                let len = text.len();
                if len >= 2 {
                    text.truncate(len - 2);
                }
            } else if starts_ci(raw, at, b"@R") {
                at += 2;
                ruby_line = RUBY_ENABLED;
            } else {
                text.push(raw[at]);
                at += 1;
            }
        }
        if text.is_empty() {
            return;
        }
        let color = match color_mode {
            0 => {
                if self.text_color < 15 {
                    self.text_color as u8
                } else {
                    0
                }
            }
            1 => 1,
            2 => 4,
            _ => 3,
        };
        let mut x = (self.shake[0] + self.pen[0]).max(0);
        let mut y = (self.pen[1] + self.shake[1]).max(0);
        if ruby_line {
            y += 8;
        }
        let mut dots = false;
        let mut i = 0;
        while i < text.len() {
            let pair = &text[i..(i + 2).min(text.len())];
            if pair == b"@K" {
                break;
            }
            if pair == OPEN_DOTS {
                dots = RUBY_ENABLED;
            } else if pair == CLOSE_DOTS {
                dots = false;
            } else if pair == OPEN_RUBY {
                i += 2;
                let count = text[i..]
                    .chunks(2)
                    .take_while(|pair| *pair != CLOSE_RUBY)
                    .count() as i32;
                let shift = match count - 1 {
                    0 => 4,
                    1 => 5,
                    _ => 6,
                };
                let mut rx = self.shake[0] - shift * 6 + x + 6;
                let ry = self.shake[1] + y - 10;
                while i < text.len() && &text[i..(i + 2).min(text.len())] != CLOSE_RUBY {
                    if RUBY_ENABLED {
                        self.glyphs.push(Glyph {
                            x: rx,
                            y: ry,
                            size: 12,
                            text: decode(&text[i..(i + 2).min(text.len())]),
                            color,
                            shadow: ([1, 1], 2),
                        });
                        rx += 12;
                    }
                    i += 2;
                }
            } else {
                if dots {
                    let dot = self.tables.emphasis_dot.clone();
                    self.glyphs.extend(cells(&dot, 7 + x - 8, y - 18, color));
                }
                self.glyphs.extend(cells(pair, x, y, color));
                x += 2 * 15 - 2;
            }
            i += 2;
        }
        self.pen[0] = PEN_X;
        self.pen[1] += LINE_HEIGHT + 2;
        if ruby_line {
            self.pen[1] += 8;
        }
    }

    // 404600 ----------------------------------------------------------
    /// Run records until one returns control. None: a scene must be loaded.
    fn dispatch(&mut self) -> Result<Option<u8>> {
        if !self.dispatching {
            self.dispatching = true;
            self.line_cursor = 0;
            self.yield_values = [-1; 5];
        }
        for _ in 0..crate::MAX_INSTRUCTIONS {
            let program = self.program()?.clone();
            let record = program.instruction(self.pc)?.clone();
            let next = record.next;
            let result = self.execute(&record).with_context(|| {
                format!(
                    "Circus DC1 {} pc={:#x} opcode={:#04x}",
                    self.scene, record.offset, record.opcode
                )
            })?;
            match result {
                Exec::Pending => return Ok(None),
                Exec::Continue => {
                    if self.pc == record.offset {
                        self.pc = next;
                    }
                }
                Exec::Jumped => {}
                Exec::Yield => {
                    if self.pc == record.offset {
                        self.pc = next;
                    }
                    self.dispatching = false;
                    return Ok(Some(record.opcode));
                }
            }
            self.flags.instructions = self
                .flags
                .instructions
                .checked_add(1)
                .context("DC1 instruction counter overflow")?;
        }
        bail!("DC1 404600 instruction budget without a yield")
    }
    fn execute(&mut self, record: &Record) -> Result<Exec> {
        let int = |at: usize| record.integer(at);
        let string = |at: usize| -> Result<String> {
            c_string(record.operands.get(at..).context("DC1 string operand")?)
        };
        let mut exec = Exec::Continue;
        match record.opcode {
            0 => {
                let text = swap_text(&record.operands)?;
                if self.choice_capture_title {
                    self.choice_building.title = text;
                    self.choice_capture_title = false;
                } else if self.choice_capture_item {
                    let mut text = text;
                    self.substitute(&mut text);
                    let count = self.choice_building.items.len();
                    if let Some(item) = self.choice_building.items.get_mut(count.wrapping_sub(1)) {
                        item.1 = text;
                    }
                    self.choice_capture_item = false;
                } else if self.line_cursor < LINES && !text.is_empty() {
                    self.pending[self.line_cursor] = text;
                    self.line_cursor += 1;
                }
            }
            2 => self.effects_enabled = true,
            3 => self.effects_enabled = false,
            4..=10 | 12 | 13 | 27 | 29 | 31 | 37..=40 | 42 | 59 | 80 | 113 | 123 | 124 => {
                exec = Exec::Yield
            }
            11 => {
                self.pen[1] += 32;
            }
            14 => self.yield_values[0] = int(0)?,
            15 | 16 => bail!("DC1 404600 opcode 15/16 (4030D0 forced redraw) is not ported"),
            17 => self.auto_text = true,
            18 => self.auto_text = false,
            19..=21 => bail!("DC1 416AE0 particle owner is not ported"),
            22 => {
                int(0)?;
            }
            23 => {
                self.jump(int(0)?)?;
                exec = Exec::Jumped;
            }
            24 | 25 => {
                //404600 24/25: mark the current scene read, then 4042A0.
                let (label, name) = if record.opcode == 24 {
                    (None, string(0)?)
                } else {
                    (Some(int(0)?), string(4)?)
                };
                let scene = format!("sinario/{name}");
                if !self.load(&scene)? {
                    self.text_field_scene = Some(scene);
                    return Ok(Exec::Pending);
                }
                if let Some(label) = label {
                    self.jump(label)?;
                }
                //404600 24/25 clear 6B0480/6B0484.
                self.history.clear();
                self.yield_values[0] = 0;
                exec = Exec::Jumped;
            }
            26 => {
                let mut name = swap_text(&record.operands)?;
                let family = self.tables.family_token.clone();
                let token = self.tables.name_token.clone();
                if !name.starts_with(&family) && name.starts_with(&token) {
                    let mut replaced = self.tables.player_name.clone();
                    replaced.extend_from_slice(&name[6..]);
                    name = copy_line(&replaced);
                }
                self.name = name;
                self.name_shown = true;
                self.name_drawn = false;
            }
            28 => self.transition = int(0)?,
            30 | 46 | 70 | 71 | 73 | 74 | 95 | 96 => {
                self.yield_values[0] = int(0)?;
                exec = Exec::Yield;
            }
            32 => {
                self.grade = int(0)?;
                ensure!(
                    (0..self.tables.grades.len() as i32).contains(&self.grade),
                    "DC1 40E0D0 grade index"
                );
            }
            33 => {
                int(0)?;
            }
            34..=36 | 41 | 94 => {
                self.yield_values[0] = int(0)?;
                self.yield_values[1] = int(4)?;
                exec = Exec::Yield;
            }
            43 | 84 | 85 | 100 => {
                int(0)?;
            }
            44 => {
                let value = int(0)?;
                //BA9DE0 (text colour option) defaults off.
                self.text_color = if value >= 15 { 0 } else { 0 };
            }
            45 => {
                self.text_speed = match int(0)? {
                    3 => 30,
                    4 => 60,
                    _ => 10,
                };
            }
            48 => {
                let slot = int(0)?;
                ensure!((0..4).contains(&slot), "DC1 403810 sprite slot");
                let name = string(4)?;
                //A different 7-byte identity defers the overlay clear.
                let old = &self.visual.sprites[slot as usize].name;
                let differs = old.len() < 7
                    || name.len() < 7
                    || !old.as_bytes()[..7].eq_ignore_ascii_case(&name.as_bytes()[..7]);
                if slot == 0 && self.visual.effect.is_some() && differs {
                    self.effect_clear[0] = true;
                }
                self.yield_values[0] = slot;
                self.yield_sprite = Some(name);
                exec = Exec::Yield;
            }
            52 => {
                let effect = usize::try_from(int(0)?).context("DC1 bsteff index")?;
                ensure!(effect < self.tables.sprite_effects.len(), "DC1 40E840 bsteff index");
                self.effect_pending = Some(effect);
                self.yield_values[0] = 0;
                self.yield_sprite = Some(string(4)?);
                exec = Exec::Yield;
            }
            49 => {
                let slot = int(0)?;
                if slot == 0 && self.visual.effect.is_some() {
                    self.effect_clear[0] = true;
                }
                self.yield_values[0] = slot;
                exec = Exec::Yield;
            }
            50 => {
                self.yield_values[0] = int(0)?;
                self.yield_values[1] = int(4)?;
                self.yield_values[2] = int(8)?;
                exec = Exec::Yield;
            }
            51 => {
                self.yield_values[0] = int(0)?;
                self.yield_values[1] = int(4)?;
                self.yield_values[2] = int(8)?;
                if self.visual.effect.is_some() {
                    self.effect_clear[0] = true;
                }
                exec = Exec::Yield;
            }
            53 => {
                if self.visual.effect.is_some() {
                    self.effect_clear[0] = true;
                }
                exec = Exec::Yield;
            }
            54 => {
                let effect = usize::try_from(int(0)?).context("DC1 bsteff index")?;
                ensure!(effect < self.tables.sprite_effects.len(), "DC1 40E840 bsteff index");
                self.visual.effect = Some(self.tables.sprite_effects[effect].clone());
            }
            55 | 56 => bail!("DC1 404600 sprite effect/pose opcode {} is not ported", record.opcode),
            57 => {
                //Immediate background replacement; 59 publishes it.
                self.visual.background = Some(string(0)?);
            }
            58 => {
                let slot = usize::try_from(int(0)?).context("DC1 sprite slot")?;
                ensure!(slot < 4, "DC1 sprite slot");
                let sprite = &mut self.visual.sprites[slot];
                sprite.present = true;
                sprite.name = string(4)?;
                if slot == 0 {
                    if let Some(effect) = self.effect_pending {
                        self.visual.effect = Some(self.tables.sprite_effects[effect].clone());
                    }
                }
            }
            60 => {
                let id = int(0)?;
                let once = int(4)?;
                if self.music != id {
                    self.music = id;
                    self.events.push(Event::Music {
                        id,
                        looping: once == 1,
                    });
                }
            }
            61 => {
                self.music = -1;
                self.events.push(Event::MusicFade);
            }
            62 => bail!("DC1 404600 opcode 62 (42A4E0) is not ported"),
            63 => {
                let id = int(0)?;
                let looping = int(4)? != 0;
                self.events.push(Event::Sound { id, looping });
            }
            64 => self.events.push(Event::SoundStop),
            65 => {
                let voice = string(0)?;
                //42A690: BA9DB0 gates suginami/sgaku7, BA9DB4 every other voice.
                let lower = voice.to_ascii_lowercase();
                let special = lower.starts_with("voice/suginami") || lower.starts_with("voice/sgaku7");
                let enabled = if special { self.settings.voice_special } else { self.settings.voice_other };
                if enabled && self.settings.voice.0 {
                    self.events.push(Event::Voice(self.voice_path(&voice)));
                }
            }
            66 => self.events.push(Event::VoiceStop),
            67 => {
                self.next_background = string(0)?;
                self.place_index = self
                    .tables
                    .places
                    .iter()
                    .position(|(name, _)| {
                        format!("back/{name}").eq_ignore_ascii_case(&self.next_background)
                    })
                    .map_or(-1, |i| i as i32);
                self.place_reset();
                exec = Exec::Yield;
            }
            68 => bail!("DC1 404600 opcode 68 (42A800 voice variant) is not ported"),
            69 | 72 => {
                self.overlay_name = Some(string(0)?);
                exec = Exec::Yield;
            }
            76 | 77 => {
                self.choice_building = Choice::default();
                self.choice_capture_title = true;
                self.choice_capture_item = false;
                if record.opcode == 77 {
                    bail!("DC1 404600 opcode 77 (positioned choice) is not ported");
                }
            }
            78 => {
                if self.choice_building.items.len() < 15 {
                    let label = int(0)?;
                    self.choice_building.items.push((label, Vec::new()));
                    self.choice_capture_item = true;
                }
                self.choice_capture_title = false;
            }
            79 => {
                int(0)?;
                int(4)?;
            }
            81 | 82 => {
                let index = int(0)?;
                if (0..15).contains(&index) {
                    self.choice_enabled[index as usize] = record.opcode == 82;
                }
            }
            83 => self.choice_enabled = [true; 15],
            86 | 90 => {
                int(0)?;
                int(4)?;
            }
            87 => {
                let index = int(0)?;
                if (0..999).contains(&index) {
                    self.seen_events[index as usize] = true;
                }
            }
            88 | 89 => {
                let index = int(0)?;
                let label = int(4)?;
                //BA9D80 defaults on.
                if self.settings.seen_skip && (0..999).contains(&index) && self.seen_events[index as usize] {
                    self.jump(label)?;
                    exec = Exec::Jumped;
                }
            }
            91 => {
                self.date = [int(0)?, int(4)?, int(8)?];
            }
            92 => {
                self.window = false;
                self.name_shown = false;
                self.name_drawn = false;
                self.visual.background = Some("BLACK".into());
                exec = Exec::Yield;
            }
            93 => {
                //403E10 composes the TimeChip date card; no presentation yield.
                self.time_of_day = [int(0)?, int(4)?];
                exec = Exec::Yield;
            }
            97 => {
                let value = int(0)?.max(1);
                self.flags.text_pacing = value;
                self.intervals[10] = value as u64;
            }
            98 | 99 => bail!("DC1 406DA0(50/51) flash transition is not ported"),
            101 => {
                int(0)?;
                int(4)?;
                int(8)?;
            }
            104 | 105 => {
                let (bank, index) = (int(0)?, int(4)?);
                if (0..13).contains(&bank) && (0..99).contains(&index) {
                    self.flags.flags[bank as usize * 99 + index as usize] =
                        i32::from(record.opcode == 104);
                }
            }
            106 => {
                let (bank, index, label) = (int(0)?, int(4)?, int(8)?);
                if (0..13).contains(&bank)
                    && (0..99).contains(&index)
                    && self.flags.flags[bank as usize * 99 + index as usize] != 0
                {
                    self.jump(label)?;
                    exec = Exec::Jumped;
                }
            }
            107 | 108 => {
                let index = usize::try_from(int(0)?).context("DC1 affection index")?;
                ensure!(index < 15, "DC1 native affection field extent");
                let amount = int(4)?;
                let old = self.flags.affection[index];
                self.flags.previous_affection[index] = old;
                self.flags.affection[index] = if record.opcode == 107 {
                    old.wrapping_add(amount).min(100)
                } else {
                    old.wrapping_sub(amount).max(0)
                };
            }
            109 | 110 => {
                let index = usize::try_from(int(0)?).context("DC1 affection index")?;
                let value = *self
                    .flags
                    .affection
                    .get(index)
                    .context("DC1 affection field extent")?;
                let minimum = int(4)?;
                let (matches, label) = if record.opcode == 109 {
                    (value >= minimum, int(8)?)
                } else {
                    (value >= minimum && value <= int(8)?, int(12)?)
                };
                if matches {
                    self.jump(label)?;
                    exec = Exec::Jumped;
                }
            }
            111 => self.flags.phase = int(0)?,
            112 => bail!("DC1 404600 opcode 112 (configuration toggles) is not ported"),
            114 | 115 => {
                self.flags_read_marks();
                exec = Exec::Yield;
            }
            116 => self.events.push(Event::MusicAttenuation(100 * int(0)?)),
            117 => self.events.push(Event::VoiceAttenuation(100 * int(0)?)),
            118 => self.events.push(Event::SoundAttenuation(100 * int(0)?)),
            119..=121 => bail!("DC1 42ACE0 pan opcode {} is not ported", record.opcode),
            122 => {
                let at = if self.flags.affection[..4].iter().any(|value| *value > 9) {
                    let mut selected = 0;
                    for index in 1..4 {
                        if self.flags.affection[index] > self.flags.affection[selected] {
                            selected = index
                        }
                    }
                    selected * 99 + 11
                } else {
                    7 * 99 + 11
                };
                self.flags.flags[at] = 1;
            }
            125 | 126 => {
                let offset = int(0)?;
                let name = string(4)?;
                self.overlay_name = Some(name.clone());
                self.overlay_mode = if record.opcode == 125 { 3 } else { 4 };
                self.overlay_offset = offset;
                self.visual.overlay = Some((name, self.overlay_mode, offset));
            }
            127 => self.reset_scroll_on_wipe = true,
            _ => {} //404600 default consumes unknown records.
        }
        Ok(exec)
    }
    fn flags_read_marks(&mut self) {
        self.pen = [PEN_X, PEN_Y];
    }
    /// 402EE0: byte-wise replacement of 純一１ unless 朝倉 starts there.
    fn substitute(&self, text: &mut Vec<u8>) {
        let family = &self.tables.family_token;
        let token = &self.tables.name_token;
        let mut at = 0;
        while at <= text.len() {
            if !starts(text, at, family) && starts(text, at, token) {
                let tail = text[at + 6..].to_vec();
                text.truncate(at);
                text.extend_from_slice(&self.tables.player_name);
                let next = text.len();
                text.extend_from_slice(&tail);
                at = next;
                continue;
            }
            at += 1;
        }
        text.truncate(LINE_BYTES - 1);
    }
    /// 404600 opcode 65: insert the scene directory after the second '\'.
    fn voice_path(&self, voice: &str) -> String {
        let scene = self.scene.rsplit('/').next().unwrap_or(&self.scene);
        let mut parts: Vec<&str> = voice.split('/').collect();
        if parts.len() >= 3 {
            let file = parts.split_off(2);
            format!("{}/{}/{}", parts.join("/"), scene, file.join("/"))
        } else {
            voice.to_owned()
        }
    }

    // 408F70 yielded opcodes -------------------------------------------
    fn after_dispatch(&mut self, op: u8) -> Result<()> {
        match op {
            4 | 6 | 8 | 9 | 10 => {
                if op == 9 {
                    self.auto_advance = true;
                }
                if self.pending[0].is_empty() {
                    self.mode = 0;
                    return Ok(());
                }
                self.saved_pen = self.pen;
                self.mode = 15;
                self.line = 0;
                self.pos = 0;
                if op == 10 {
                    self.keep_page = true;
                } else {
                    self.keep_page = false;
                    if op == 6 {
                        self.force_reset = true;
                    }
                }
                for index in 0..LINES {
                    let mut line = std::mem::take(&mut self.pending[index]);
                    if !line.is_empty() {
                        self.substitute(&mut line);
                    }
                    self.pending[index] = line;
                }
                //687258: 8/9 pages do not add the deferred 32px blank line.
                self.blank_after = op != 8 && op != 9;
                //LABEL_361: backlog entry; a name not yet drawn is its speaker.
                if self.history.len() < 999 {
                    let speaker = (self.name_shown && !self.name_drawn).then(|| self.name.clone());
                    let lines = self.pending.iter().filter(|line| !line.is_empty()).cloned().collect();
                    self.history.push(HistoryEntry { speaker, lines });
                }
            }
            5 => {
                self.new_page();
            }
            7 => {
                self.mode = 17;
                self.no_cursor = true;
                self.cursor_on = true;
            }
            12 => self.mode = 21,
            13 => self.mode = 22,
            27 => {
                self.name_shown = false;
                self.name_drawn = false;
                self.name_glyphs.clear();
                self.text_color = 0;
            }
            37 => {
                self.screen_shake = [0, 0];
                self.scroll = [0, 0];
            }
            38 => self.shake = [0, 0],
            39 => {
                self.bg_base = [0, 0];
                self.shake = [0, 0];
            }
            34 => self.bg_base = [8 * self.yield_values[0], 8 * self.yield_values[1]],
            35 => self.shake = [8 * self.yield_values[0], 8 * self.yield_values[1]],
            36 => {
                self.screen_shake = [8 * self.yield_values[0], 8 * self.yield_values[1]];
            }
            40 => {
                self.screen_shake = [0, 0];
                self.shake_phase = 0;
                self.mode = 23;
            }
            41 => {
                let [x, y] = [8 * self.yield_values[0], 8 * self.yield_values[1]];
                self.shake = [x, y];
                self.bg_base = [x, y];
                self.visual.sprites[0].x = x;
                self.visual.sprites[0].y = y;
            }
            42 => {
                self.bg_base = [0, 0];
                self.shake = [0, 0];
                self.visual.sprites[0].x = 0;
                self.visual.sprites[0].y = 0;
                self.scroll = [0, 0];
                self.overlay_mode = 0;
                self.overlay_offset = 0;
                self.visual.overlay = None;
            }
            46 => {
                self.prev_mode = self.mode;
                self.wait_at = self.now;
                self.wait_len = 20 * self.yield_values[0].max(0) as u64;
                self.mode = 18;
            }
            48 | 52 => {
                let slot = self.yield_values[0] as usize;
                let name = self.yield_sprite.take().context("DC1 403810 sprite name")?;
                self.hide_for_sprite();
                let sprite = &mut self.visual.sprites[slot];
                sprite.present = true;
                sprite.name = name;
                //403810 consults slot 0's overlay flags for every slot.
                if self.effect_clear[0] {
                    self.effect_clear[0] = false;
                    self.effect_pending = None;
                    self.visual.effect = None;
                } else if let Some(effect) = self.effect_pending.take() {
                    self.visual.effect = Some(self.tables.sprite_effects[effect].clone());
                }
                self.mode = 11;
            }
            49 => {
                self.hide_for_sprite();
                let slot = self.yield_values[0];
                if (0..4).contains(&slot) && self.visual.sprites[slot as usize].present {
                    self.visual.sprites[slot as usize].present = false;
                    if self.effect_clear[slot as usize] {
                        self.effect_clear[slot as usize] = false;
                        self.effect_pending = None;
                        self.visual.effect = None;
                    }
                }
                self.mode = 11;
            }
            50 => {
                let slot = self.yield_values[0];
                ensure!((0..4).contains(&slot), "DC1 403B50 sprite slot");
                let sprite = &mut self.visual.sprites[slot as usize];
                sprite.x = 8 * self.yield_values[1];
                sprite.y = 8 * self.yield_values[2];
            }
            51 => {
                self.hide_for_sprite();
                for slot in 0..3 {
                    if self.yield_values[slot] != 0 && self.visual.sprites[slot].present {
                        self.visual.sprites[slot].present = false;
                        if self.effect_clear[slot] {
                            self.effect_clear[slot] = false;
                            self.effect_pending = None;
                            self.visual.effect = None;
                        }
                    }
                }
                self.mode = 11;
            }
            53 | 59 => {}
            67 => {
                self.window = false;
                self.name_shown = false;
                self.name_drawn = false;
                self.begin_transition(self.transition)?;
            }
            69 | 72 => {
                let name = self.overlay_name.clone().context("DC1 overlay name")?;
                self.overlay_mode = if op == 69 { 1 } else { 2 };
                self.visual.overlay = Some((name, self.overlay_mode, 0));
            }
            70 | 71 | 73 | 74 => {
                self.mode = 4 + i32::from(op - 70).min(1) + if op >= 73 { 2 } else { 0 };
                self.scroll_speed = self.yield_values[0];
            }
            80 => {
                self.choice_capture_title = false;
                self.choice_capture_item = false;
                if !self.window {
                    self.show_window();
                }
                let mut choice = std::mem::take(&mut self.choice_building);
                choice.enabled = self.choice_enabled;
                //406910(0) saves the pen; 406C30 draws 657C00 there with 402030.
                choice.title_at = self.pen;
                choice.selected = 0;
                let title = choice.title.clone();
                self.draw_line(&title, 0);
                self.choice = Some(choice);
                self.choice_last_pointer = None;
                self.mode = 1;
            }
            92 => {
                //408F70 case 92: four strict max passes over BAD8F0[0..4].
                let affection = self.flags.affection;
                let mut best = 0;
                for _ in 0..4 {
                    for index in 0..4 {
                        if affection[best] < affection[index] {
                            best = index;
                        }
                    }
                }
                let mut heroine = [0, 2, 4, 1][best];
                if affection[..4].iter().all(|value| *value <= 9) {
                    heroine = 7;
                }
                let flag = |bank: usize| self.flags.flags[bank * 99 + 10] != 0;
                if flag(4) {
                    heroine = 5;
                }
                if flag(5) {
                    heroine = 3;
                }
                if flag(6) {
                    heroine = 6;
                }
                self.eyecatch = Some(heroine);
                self.timer(15);
                self.mode = 17;
                self.no_cursor = true;
            }
            93 => {}
            115 | 114 => {
                self.mode = -1;
            }
            123 => {
                self.mode = 8;
                self.window = false;
                self.name_shown = false;
            }
            124 => {
                self.mode = 9;
                self.window = false;
                self.name_shown = false;
            }
            other => bail!("DC1 408F70 yielded opcode {other} is not ported"),
        }
        Ok(())
    }
    fn hide_for_sprite(&mut self) {
        //403810/403B80: hide window/name, 4030D0, then state 11 (BA9DDC off).
        self.window = false;
        self.name_shown = false;
        self.name_drawn = false;
    }
    /// 406DA0 for types 0/1 with BA9D4C (MMX present): grade rule wipe.
    fn begin_transition(&mut self, kind: i32) -> Result<()> {
        ensure!(
            matches!(kind, 0 | 1),
            "DC1 406DA0 transition type {kind} is not ported"
        );
        ensure!(
            self.settings.effect_mode,
            "DC1 406DA0(-1) transition without BA9D4C (424D70/424ED0) is not ported"
        );
        let before = self.current_visual();
        if self.reset_scroll_on_wipe {
            self.reset_scroll_on_wipe = false;
            self.scroll = [0, 0];
        }
        self.visual.background = Some(self.next_background.clone());
        self.mode = 10;
        //408F70 state10 without 6855A0/BA9D3C finishes at once.
        if !self.settings.wipe_enabled && !self.effects_enabled {
            self.wipe = Some(512);
            return Ok(());
        }
        self.events.push(Event::WipeBegin(Box::new(before)));
        self.wipe = Some(0);
        self.wipe_step = self.settings.wipe_step;
        Ok(())
    }
    /// 408F70 state 10 with 426350/426030.
    fn wipe_step(&mut self) -> Result<()> {
        let progress = self.wipe.context("DC1 wipe state")?;
        if !self.timer(10) {
            return Ok(());
        }
        let progress = if self.auto_skip { 512 } else { progress };
        if progress >= 512 {
            if self.wipe_step != 0 && progress < 1024 {
                self.events.push(Event::WipeEnd);
            }
            self.wipe = None;
            self.mode = 0;
            if self.grade != DEFAULT_GRADE {
                self.grade = DEFAULT_GRADE;
            }
            return Ok(());
        }
        self.wipe = Some(progress + self.wipe_step);
        Ok(())
    }
    pub fn current_visual(&self) -> Visual {
        let mut visual = self.visual.clone();
        visual.offset = [
            self.scroll[0] + self.bg_base[0],
            self.scroll[1] + self.bg_base[1],
        ];
        visual.overlay = self.visual.overlay.clone();
        visual
    }
    fn scroll_step(&mut self, click: bool) {
        //States 4..7: 68A660 moves by 68A664 (or 2) per timer10 tick.
        let speed = if self.scroll_speed > 0 { self.scroll_speed } else { 2 };
        let (target, sign) = match self.mode {
            4 => (0, 1),
            5 => (-600, -1),
            6 => (600, 1),
            _ => (0, -1),
        };
        if click {
            self.scroll[1] = target;
            self.mode = 0;
            return;
        }
        if !self.timer(10) {
            return;
        }
        self.scroll[1] += sign * speed;
        if (sign > 0 && self.scroll[1] >= target) || (sign < 0 && self.scroll[1] <= target) {
            self.scroll[1] = target;
            self.mode = 0;
        }
    }
    fn overlay_slide(&mut self, click: bool) {
        let target = self.overlay_offset;
        if click {
            self.scroll[0] = if self.mode == 8 { target } else { 0 };
            self.mode = 0;
            return;
        }
        if !self.timer(10) {
            return;
        }
        if self.mode == 8 {
            self.scroll[0] += 4;
            if self.scroll[0] >= target {
                self.scroll[0] = target;
                self.mode = 0;
            }
        } else {
            self.scroll[0] -= 4;
            if self.scroll[0] <= 0 {
                self.scroll[0] = 0;
                self.mode = 0;
            }
        }
    }
    fn shake_step(&mut self) {
        //408F70 state 23: eight phases of -32/0/+32 px horizontal offsets.
        self.screen_shake[0] = match self.shake_phase {
            0 | 3 | 7 => -32,
            2 | 5 => 32,
            _ => 0,
        };
        self.screen_shake[1] = 0;
        self.shake_phase += 1;
        if self.shake_phase >= 8 {
            self.screen_shake = [0, 0];
            self.shake_phase = 0;
            self.mode = 0;
        }
    }
    fn place_reset(&mut self) {
        //40BAA0(1,0).
        if self.place_index < 0 {
            self.place = None;
            return;
        }
        let frame = self.tables.places[self.place_index as usize].1;
        self.place = Some((frame, 256));
        self.place_counter = 0;
    }
    /// 40BB30 slot1: hold 50 timer20 ticks, then drop alpha by 16 per tick.
    fn place_step(&mut self) {
        let Some((frame, alpha)) = self.place else {
            return;
        };
        if alpha <= 0 {
            self.place = None;
            return;
        }
        if self.timer(20) {
            self.place_counter += 1;
            if self.place_counter >= 50 {
                self.place = Some((frame, (alpha - 16).max(0)));
            }
        }
    }
    /// 4069C0 pointer selection over 216..720 x 216+56n..+32.
    fn choose(&mut self, input: Input) -> Result<()> {
        let choice = self.choice.as_mut().context("DC1 choice state")?;
        let mut activate = false;
        for (index, _) in choice.items.iter().enumerate() {
            let top = 216 + 56 * index as i32;
            if choice.enabled[index]
                && input.x >= 216
                && input.y >= top
                && input.x < 720
                && input.y < top + 32
            {
                if self.choice_last_pointer != Some([input.x, input.y]) {
                    choice.selected = index;
                }
                if input.click {
                    activate = true;
                }
            }
        }
        self.choice_last_pointer = Some([input.x, input.y]);
        if !activate {
            return Ok(());
        }
        let selected = choice.selected;
        if !choice.enabled[selected] {
            return Ok(());
        }
        let label = choice.items[selected].0;
        self.choice = None;
        self.jump(label)?;
        self.pen = [PEN_X, PEN_Y];
        self.glyphs.clear();
        self.mode = 0;
        Ok(())
    }
    pub fn set_voice_playing(&mut self, playing: bool) {
        self.voice_playing = playing;
    }
}

/// 414500: Some(true) yes, Some(false) no; BA9F70 counts as no.
fn dialog_step(dialog: &mut Dialog, rows: &[[i32; 7]], input: Input) -> Option<bool> {
    if input.cancel {
        return Some(false);
    }
    let [x, y] = dialog.at;
    let mut inside = false;
    for (index, row) in rows.iter().enumerate() {
        if input.x >= x + row[0] && input.y >= y + row[1] && input.x < x + row[2] && input.y < y + row[3] {
            dialog.hover = index as i32;
            inside = true;
        }
    }
    (input.click && inside).then_some(dialog.hover == 0)
}

enum Exec {
    Continue,
    Jumped,
    Yield,
    Pending,
}

/// 411ED0 cells: full-width pairs advance 30, single bytes 15; each cell has
/// a palette-6 shadow at +2,+2.
fn cells(bytes: &[u8], x: i32, y: i32, color: u8) -> Vec<Glyph> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut x = x;
    while at < bytes.len() && bytes[at] != 0 {
        let lead = bytes[at];
        let width = if lead >= 0x80 && !(0xa1..=0xdf).contains(&lead) { 2 } else { 1 };
        let part = &bytes[at..(at + width).min(bytes.len())];
        if width == 2 || (lead >= 32 && lead != 127) {
            out.push(Glyph {
                x,
                y,
                size: 30,
                text: decode(part),
                color,
                shadow: ([2, 2], 6),
            });
        }
        x += if width == 2 { 30 } else { 15 };
        at += width;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dc1_frontend::Atlas;
    fn tables() -> Tables {
        let atlas = Atlas { path: "system/x.CRX".into(), frames: vec![[0, 0, 1, 1]] };
        Tables {
            palette: vec![[255, 255, 255]; 15],
            window_buttons: Vec::new(),
            key_wait: atlas.clone(),
            time_chip: atlas.clone(),
            map_name: atlas.clone(),
            places: Vec::new(),
            days: vec![[2, 23, 6, 0, 0, 0]; 11],
            day_scenes: vec![None; 11],
            grades: vec!["system/grade/grade00.CRX".into(); 5],
            player_name: encoding_rs::SHIFT_JIS.encode("純一").0.into_owned(),
            family_token: encoding_rs::SHIFT_JIS.encode("朝倉").0.into_owned(),
            name_token: encoding_rs::SHIFT_JIS.encode("純一１").0.into_owned(),
            default_speaker: encoding_rs::SHIFT_JIS.encode("名無し").0.into_owned(),
            emphasis_dot: encoding_rs::SHIFT_JIS.encode("・").0.into_owned(),
            eyecatches: Vec::new(),
            eyecatch_layouts: Vec::new(),
            sprite_effects: Vec::new(),
            timer_back: String::new(),
            timer_chip: atlas.clone(),
            map_rects: Vec::new(),
            map_scenes: vec![None; 11],
            menu_chip: atlas.clone(),
            menu_rows: Vec::new(),
            backlog_rows: Vec::new(),
            dialog_chip: atlas.clone(),
            dialog_rows: Vec::new(),
            return_prompt: Vec::new(),
            window_prompt: Vec::new(),
            speaker_format: b"[%s]".to_vec(),
        }
    }
    fn option() -> crate::dc1_frontend::ConfigTables {
        let atlas = Atlas { path: "system/x.CRX".into(), frames: Vec::new() };
        crate::dc1_frontend::ConfigTables {
            rows: Vec::new(),
            radio: Vec::new(),
            back: String::new(),
            chip: atlas.clone(),
            sys_chip: atlas,
        }
    }
    /// Encode one 404600 record: pair-swapped CP932 for text fields.
    fn record(opcode: u8, operands: &[u8]) -> Vec<u8> {
        let mut out = vec![opcode];
        out.extend_from_slice(operands);
        out.extend_from_slice(&[10, 13]);
        out
    }
    fn text(value: &str) -> Vec<u8> {
        let bytes = encoding_rs::SHIFT_JIS.encode(value).0.into_owned();
        let mut out = Vec::new();
        for pair in bytes.chunks(2) {
            out.push(*pair.get(1).unwrap_or(&0));
            out.push(pair[0]);
        }
        out.push(0);
        out
    }
    fn program() -> Arc<Program> {
        let mut bytes = b"#ADV_TXT".to_vec();
        bytes.extend(0i32.to_le_bytes());
        bytes.extend([10, 13]);
        bytes.extend(record(26, &text("純一１")));
        bytes.extend(record(0, &text("あい")));
        bytes.extend(record(4, &[]));
        bytes.extend(record(115, &[]));
        Arc::new(Program::parse(&bytes).unwrap())
    }
    #[test]
    fn page_reveal_follows_4013f0_pen_and_402ee0_name() {
        let mut story = Story::new(Arc::new(tables()), option(), "sinario/test".into(), program()).unwrap();
        let mut input = Input::default();
        for step in 0..200u64 {
            story.now = step * 11;
            if story.mode == 17 {
                input.click = true;
            }
            story.frame(input).unwrap();
            input.click = false;
            if story.day_state == 11 {
                break;
            }
        }
        //Name: 純一１ -> 純一 at 56 + 12 * (6 - 4/2), 392.
        assert_eq!(decode(&story.name), "純一");
        //The page completed, was clicked away, and 115 ended the scene.
        assert_eq!(story.day_state, 11);
        assert!(story.glyphs.is_empty());
    }
    #[test]
    fn reveal_positions_use_28px_advance_and_30px_cells() {
        let mut story = Story::new(Arc::new(tables()), option(), "sinario/test".into(), program()).unwrap();
        story.pending[0] = encoding_rs::SHIFT_JIS.encode("あい").0.into_owned();
        assert!(!story.reveal().unwrap());
        assert!(story.reveal().unwrap());
        let main: Vec<_> = story.glyphs.iter().map(|g| (g.x, g.y, g.size)).collect();
        assert_eq!(main, vec![(48, 448, 30), (76, 448, 30)]);
        assert_eq!(story.pen, [48, 448 + 32]);
    }
}
