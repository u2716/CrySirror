//! D.C.II P.C. ABI 0x5769. Native 41C530/41A300/41A5C0 declarations and
//! control state are separate from DC3WY and DC4PH. The host owns resources,
//! presentation and completion; unknown instructions retain their original PC.
#[path = "dc2_audio.rs"]
pub mod audio;
use crate::machine::{NativeHost, Outcome};
use crate::{Arguments, Instruction, Profile, Program};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[path = "dc2_draw.rs"]
mod draw;
#[path = "dc2_resources.rs"]
mod resources;
#[path = "dc2_stage.rs"]
mod stage;
#[path = "dc2_fade.rs"]
mod fade;
#[path = "dc2_crossfade.rs"]
mod crossfade;
#[path = "dc2_map.rs"]
pub(crate) mod map;
#[path = "dc2_wipe.rs"]
pub(crate) mod wipe;
#[path = "dc2_movie.rs"]
mod movie;
#[path = "dc2_portrait.rs"]
pub(crate) mod portrait;
#[path = "dc2_pan.rs"]
mod pan;
#[path = "dc2_scene.rs"]
mod scene;
#[path = "dc2_music.rs"]
mod music;
#[path = "dc2_sound.rs"]
mod sound;
#[path = "dc2_menu.rs"]
pub(crate) mod menu;
#[path = "dc2_ui.rs"]
mod ui;
#[path = "dc2_values.rs"]
mod values;
#[path = "dc2_wait.rs"]
mod wait;
#[path = "dc2_shatter.rs"]
mod shatter;
#[path = "dc2_weather.rs"]
mod weather;
pub use weather::{WeatherFrame, WeatherGroup};
#[path = "dc2_text.rs"]
mod text;
pub use text::{Glyph, TextFrame};

pub const CANVAS: [u32; 2] = [800, 600];
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pose {
    pub position: [i16; 2],
    pub percent: i16,
    ///B0B800/B0B804: 41BA90 scale centre, written only by 0x65.
    #[serde(default)]
    pub center: [i16; 2],
    ///B0B7E8..B0B7F0: the x, y and percent 41A950 draws with; 41BCE0
    ///copies the declared pose here after a load.
    #[serde(default)]
    pub shown: [i16; 3],
    ///B0B808..B0B814: the rectangle last drawn (x, y, w, h).
    #[serde(default)]
    pub drawn: [i32; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub(crate) pan: Option<pan::Pan>,
    pub(crate) fade: Option<fade::Fade>,
    ///41F630 case20 crossfade progress.
    #[serde(default)]
    pub(crate) crossfade: Option<crossfade::Crossfade>,
    ///Process28 map screen (408560 selector20).
    #[serde(default)]
    pub(crate) map: Option<map::MapWait>,
    ///41F630-independent 424CA0 field wipe (case13 selector13).
    #[serde(default)]
    pub(crate) wipe: Option<wipe::Wipe>,
    ///PC of the running blocking 4153E0 movie.
    #[serde(default)]
    pub(crate) movie_pc: Option<u32>,
    ///41F630 case3 flash progress.
    #[serde(default)]
    pub(crate) flash: Option<crossfade::Flash>,
    ///Native5 (41AB60's centred image); natives1..4 use primary_names.
    #[serde(default)]
    pub native5_name: Option<String>,
    ///41BCE0/41B920 portrait dirty rectangle and transition.
    #[serde(default)]
    pub(crate) portrait: portrait::PortraitState,
    pub(crate) menu: Option<menu::MenuWait>,
    ///41C530 case8 (CEBA4C=1): AFE800 rows appended from the AC7058 template.
    #[serde(default)]
    pub(crate) choice: Option<menu::Choice>,
    ///process18 (opcode0x12) wait record and4239B0's two release callbacks.
    #[serde(default)]
    pub(crate) native_wait: Option<wait::Wait>,
    #[serde(default)]
    pub release_callbacks: [Vec<u8>; 2],
    ///4188A0 screen-break frame loop (41C530 case13 selector14).
    #[serde(default)]
    pub(crate) shatter: Option<shatter::Shatter>,
    ///417010 weather pools (B0CE20 kind, timer5 state).
    #[serde(default)]
    pub(crate) weather: Option<weather::Weather>,
    ///Cases 75/76/7 name, message and window presentation.
    #[serde(default)]
    pub(crate) text: text::TextState,
    pub scene: String,
    pub pc: u32,
    pub instructions: u64,
    pub last_opcode: Option<u8>,
    pub settings: crate::dc2_settings::SystemData,
    pub script_bytes: Vec<u8>,
    pub scene_completed: Vec<u8>,
    pub draw: draw::DrawState,
    pub resources: resources::Resources,
    pub audio: audio::AudioState,
    pub runtime: BTreeMap<i16, i32>,
    pub version: Vec<u8>,
    ///419E30 initializes just the first history word. The full bounded backing
    ///is zero-initialized in Rust; previously unused words are never read before
    ///native03 writes them (except the native0x8000 retained marker).
    pub history: Vec<u16>,
    pub history_count: usize,
    pub current_label: u16,
    pub scene_jump: Vec<u8>,
    pub other_jump: Vec<u8>,
    pub poses: [Pose; 5],
    pub primary_names: [Option<String>; 5],
    pub primary_ids: [i32; 5],
    pub builder: ui::Builder,
    pub templates: BTreeMap<u8, ui::Builder>,
    pub palette: BTreeMap<u8, [u8; 3]>,
    pub palette_published: u8,
    pub pending_check: i32,
    pub image_flag: i16,
    ///Host observations belong to a single instruction through asynchronous
    ///preference-save/resource retries; they are discarded only on native commit.
    pub observations: BTreeMap<(i16, i16), i32>,
    pub wait_until: Option<u64>,
    pub halted: bool,
}
impl State {
    pub fn new(scene: String) -> Self {
        const RAW: [[u8; 3]; 13] = [
            [0, 0, 0],
            [0, 0, 255],
            [0, 255, 0],
            [0, 255, 255],
            [255, 0, 0],
            [255, 0, 255],
            [255, 255, 0],
            [255, 255, 255],
            [78, 133, 40],
            [204, 204, 255],
            [68, 119, 0],
            [127, 255, 127],
            [204, 204, 204],
        ];
        Self {
            pan: None,
            fade: None,
            crossfade: None,
            movie_pc: None,
            flash: None,
            native5_name: None,
            portrait: Default::default(),
            menu: None,
            choice: None,
            wipe: None,
            map: None,
            native_wait: None,
            release_callbacks: Default::default(),
            shatter: None,
            weather: None,
            text: Default::default(),
            scene,
            pc: 2,
            instructions: 0,
            last_opcode: None,
            settings: Default::default(),
            script_bytes: vec![0; 1000],
            scene_completed: vec![0; 1500],
            draw: Default::default(),
            resources: Default::default(),
            audio: Default::default(),
            runtime: [(3, 1), (243, -1), (312, 100)].into_iter().collect(),
            version: Vec::new(),
            history: vec![0; 1000],
            history_count: 0,
            current_label: 0,
            scene_jump: Vec::new(),
            other_jump: Vec::new(),
            poses: Default::default(),
            primary_names: Default::default(),
            primary_ids: [-1; 5],
            builder: Default::default(),
            templates: BTreeMap::new(),
            palette: (0..64)
                .map(|i| (i, RAW.get(i as usize).copied().unwrap_or([0; 3])))
                .collect(),
            palette_published: 0,
            pending_check: -1,
            image_flag: 0,
            observations: Default::default(),
            wait_until: None,
            halted: false,
        }
    }
}
pub struct Machine {
    pub state: State,
}
///Actual41C640 default-target entries. Opcode52 is a default only when
///dispatched independently; opcode50 consumes its following9-byte operand.
pub fn native_noop(opcode: u8) -> bool {
    opcode >= 105
        || matches!(
            opcode,
            1 | 2 | 0x1f | 0x24 | 0x28 | 0x29 | 0x2f | 0x35 | 0x45 | 0x4a | 0x4d | 0x4e | 0x52
        )
}
impl Machine {
    pub fn new(scene: String) -> Self {
        Self {
            state: State::new(scene),
        }
    }
    pub fn step(
        &mut self,
        program: &Program,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome> {
        ensure!(
            program.profile == Profile::Dc2PlusCommunication,
            "DC2 executor requires native ABI0x5769"
        );
        if self.state.halted {
            return Ok(Outcome::Halted);
        }
        if !self.tick_music_fade(host, now)? {
            return Ok(Outcome::Pending);
        }
        if !self.tick_sound_fade(host, now)? {
            return Ok(Outcome::Pending);
        }
        self.tick_weather(host, now)?;
        let outcome = self.step_inner(program, host, now, control)?;
        //Timers2/5 keep running beneath script waits.
        let timer = self
            .state
            .audio
            .music_fade
            .as_ref()
            .map(|_| now + 10)
            .into_iter()
            .chain(self.state.audio.sound_fade.as_ref().map(|_| now + 10))
            .chain(self.weather_due())
            .min();
        Ok(match (outcome, timer) {
            (Outcome::WaitUntil(deadline), Some(timer)) => Outcome::WaitUntil(deadline.min(timer.max(now + 1))),
            (Outcome::WaitForInput, Some(timer)) => Outcome::WaitUntil(timer.max(now + 1)),
            (outcome, _) => outcome,
        })
    }
    fn step_inner(
        &mut self,
        program: &Program,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome> {
        if let Some(deadline) = self.state.wait_until {
            if now < deadline {
                return Ok(Outcome::WaitUntil(deadline));
            }
            self.state.wait_until = None;
        }
        while self.state.palette_published < 64 {
            let index = self.state.palette_published;
            let raw = self.state.palette[&index];
            if !host.emit(&crate::machine::Event::Palette {
                index,
                rgb: [raw[1], raw[0], raw[2]],
            })? {
                return Ok(Outcome::Pending);
            }
            self.state.palette_published += 1;
        }
        let instruction = program.instruction(self.state.pc)?;
        let outcome = self.execute(program, instruction, host, now, control)
            .with_context(|| {
                format!(
                    "Circus DC2 {} pc={:#x} opcode={:#04x}",
                    self.state.scene, instruction.offset, instruction.opcode
                )
            });
        if crate::trace_enabled() {
            eprintln!("DC2 {} pc={:#x} op={:#04x} args={:?} -> {:?}", self.state.scene, instruction.offset, instruction.opcode, instruction.arguments, outcome.as_ref().ok());
        }
        outcome
    }
    fn execute(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome> {
        let mut a = [0i16; 4];
        let text = match &instruction.arguments {
            Arguments::Bytes(v) => {
                a[0] = v[0].into();
                a[1] = v[1].into();
                ""
            }
            Arguments::Words(v) => {
                a = *v;
                ""
            }
            Arguments::SlotString { slot, text } => {
                a[0] = (*slot).into();
                text
            }
            Arguments::String(text) => text,
        };
        let mut next = instruction.next;
        if let Some(outcome)=self.execute_menu(program,instruction,a,host)? {return Ok(outcome)}
        if let Some(outcome) = self.execute_map(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_scene(instruction, a[0], text, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_resources(program, instruction, a, text, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_shatter(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_fade_in(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_shake(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_flash(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_redraw(instruction, a, host)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_wipe(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_crossfade(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_fade(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_pan(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_stage(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_text(instruction, text, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_motion(instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_expression(program, instruction, a, text, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_portrait(program, instruction, a, text, host, now, control)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_movie(instruction, text, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_sound(instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_music(program, instruction, a, host, now)? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.execute_audio(instruction, text, host, control)? {
            return Ok(outcome);
        }
        if matches!(instruction.opcode, 0x21 | 0x53 | 0x60 | 0x61) {
            let mut draw = self.state.draw.clone();
            let mut check = self.state.pending_check;
            let next = draw
                .declare(program, instruction, a, &mut check)?
                .context("DC2 draw opcode declaration")?;
            if instruction.opcode == 0x21 && a[0] == 1 {
                let records = draw
                    .snapshots
                    .get(&1)
                    .context("DC2 current window template")?;
                let mut visible = Vec::with_capacity(records.len());
                for record in records {
                    let enabled = if record.check == -1 {
                        true
                    } else {
                        ensure!(record.check >= 0, "DC2 negative native draw binding");
                        let bank = ((record.check / 256) & 0x7f) as i16;
                        let index = (record.check % 256) as i16;
                        let Some(value) = self.value(bank, index, host)? else {
                            return Ok(Outcome::Pending);
                        };
                        if record.check & 0x8000 == 0 {
                            value != 0
                        } else {
                            value == 0
                        }
                    };
                    visible.push(enabled);
                }
                draw.apply_window_geometry(
                    &visible,
                    false,
                    self.state.settings.prefix(18)?.unwrap_or(0) != 0,
                )?;
            }
            self.state.draw = draw;
            self.state.pending_check = check;
            return self.advance(next, instruction.opcode);
        }
        if let Some(value) = self.execute_ui(program, instruction, a)? {
            next = value;
            return self.advance(next, instruction.opcode);
        }
        if let Some(outcome) = self.execute_wait(program, instruction, a, host, now, control)? {
            return Ok(outcome);
        }
        match instruction.opcode {
            //41C530 case38: 255 pauses/resumes timer5 (CEBA24); 7 is fog.
            0x26 if a[0] == 255 => {
                if let Some(weather) = &mut self.state.weather {
                    weather.paused = a[1] != 0;
                }
            }
            //Case38 selector7: 41BCE0(4, "system\\fog", 1) only; weather stays.
            0x26 if a[0] == 7 => {
                if !self.load_native5("system\\fog", host, control)? {
                    return Ok(Outcome::Pending);
                }
                if let Some(weather) = &mut self.state.weather {
                    weather.paused = false;
                }
            }
            0x26 => {
                //Selector0 with operand1 0 erases a loaded native5 first.
                if a[0] == 0 && a[1] == 0 && !self.load_native5("-", host, control)? {
                    return Ok(Outcome::Pending);
                }
                if !self.start_weather(i32::from(a[0]), i32::from(a[1]), host, now)? {
                    return Ok(Outcome::Pending);
                }
                if let Some(weather) = &mut self.state.weather {
                    weather.paused = false;
                }
            }
            //41C530 case17: B0AB1C=-1, then426EE0 starts the timer fade.
            0x11 => {
                self.state.runtime.insert(244, -1);
                self.begin_music_fade(host, now, 150)?;
            }
            0x16 if a[0] == 5 => {
                if !host.write_profile_system(
                    Profile::Dc2PlusCommunication,
                    self.state.settings.bytes(),
                )? {
                    return Ok(Outcome::Pending);
                }
            }
            0x16 if a[0] == 6 => {
                self.state.history_count = 0;
                self.state.history[0] = 0;
            }
            0x16 if matches!(a[0], 7 | 12 | 13) => {
                //Distinct native globals A24168,500C48,B0CE18. Retain
                //the source selector namespace rather than prefix99 aliases.
                self.state.runtime.insert(300 + a[0], i32::from(a[1]));
            }
            0x16 if a[0] == 8 => {
                use crate::machine::system_ops::{SystemReply, SystemRequest};
                if self.state.runtime.get(&3).copied().unwrap_or(0) != 0 {
                    let Some(previous) = self.value(99, 4, host)? else {
                        return Ok(Outcome::Pending);
                    };
                    //40ACA0 returns unchanged when both modes are nonzero,
                    //including1->2; only an actual recreation reseeds rand.
                    if (previous != 0) == (a[1] != 0) {
                        return self.advance(next, instruction.opcode);
                    }
                    match host.system(SystemRequest::NativeFullscreenMode(a[1]))? {
                        None => return Ok(Outcome::Pending),
                        Some(SystemReply::Done) => {}
                        Some(_) => bail!("DC2 window transition foreign receipt"),
                    }
                    self.advance(next, instruction.opcode)?;
                    return Ok(Outcome::WaitUntil(
                        now.checked_add(40)
                            .context("DC2 recreated window timer overflow")?,
                    ));
                }
            }
            0x16 if a[0] == 10 => {
                use crate::machine::system_ops::{SystemReply, SystemRequest};
                match host.system(SystemRequest::License)? {
                    None => return Ok(Outcome::Pending),
                    Some(SystemReply::Done) => {}
                    Some(SystemReply::Cancelled) => {
                        self.state.halted = true;
                        self.advance(next, instruction.opcode)?;
                        return Ok(Outcome::Halted);
                    }
                    Some(_) => bail!("DC2 installation-key foreign receipt"),
                }
            }
            0x16 if !matches!(a[0], 0..=13 | 17) => {}
            0x03 => {
                let label = a[0] as u16 | ((a[1] as u16) << 8);
                let word = self
                    .state
                    .history
                    .get_mut(self.state.history_count)
                    .context("DC2 history cursor extent")?;
                *word = if *word == 0x8000 {
                    label | 0x8000
                } else {
                    label
                };
                self.state.history_count += 1;
                if self.state.history_count >= 1000 {
                    self.state.history.copy_within(1..1000, 0);
                    self.state.history_count = 999;
                }
                self.state.current_label = label;
            }
            0x04 => self.state.current_label = a[0] as u16 | ((a[1] as u16) << 8),
            0x0f => self.state.script_bytes.fill(0),
            0x1e if a[0] == 0 => next = skip_marker(program, next, a[1] as u8)?,
            0x1e => {}
            0x22 => {
                let value = if a[0] == 0 {
                    0
                } else {
                    let Some(value) = host.random(a[0] as u16)? else {
                        return Ok(Outcome::Pending);
                    };
                    value
                };
                self.state.runtime.insert(2, value);
            }
            0x37 | 0x47 => {
                let raw = program
                    .literal_bytes(instruction.offset)
                    .context("DC2 callback original bytes")?;
                ensure!(raw.len() < 32, "DC2 callback exceeds native32-byte field");
                *if instruction.opcode == 0x37 {
                    &mut self.state.scene_jump
                } else {
                    &mut self.state.other_jump
                } = raw.to_vec();
            }
            0x3a => {
                next = named_label(
                    program,
                    program
                        .literal_bytes(instruction.offset)
                        .context("DC2 jump literal")?,
                )?
            }
            0x49 => {
                let raw = program
                    .literal_bytes(instruction.offset)
                    .context("DC2 version literal")?;
                if self.state.version.len() + raw.len() < 128 {
                    self.state.version.extend_from_slice(raw)
                }
            }
            0x5c => {
                let value = match a[3] {
                    0 => i32::from(a[2]),
                    1 | 2 => {
                        let Some(old) = self.value(a[0], a[1], host)? else {
                            return Ok(Outcome::Pending);
                        };
                        if a[3] == 1 {
                            old.wrapping_add(a[2].into())
                        } else {
                            old.wrapping_sub(a[2].into())
                        }
                    }
                    _ => return self.advance(next, instruction.opcode),
                };
                if !self.assign(a[0], a[1], value, host)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x5d | 0x5e => {
                let Some(right) = self.value(a[2], a[3], host)? else {
                    return Ok(Outcome::Pending);
                };
                let value = if instruction.opcode == 0x5e {
                    let Some(left) = self.value(a[0], a[1], host)? else {
                        return Ok(Outcome::Pending);
                    };
                    left.wrapping_add(right)
                } else {
                    right
                };
                if !self.assign(a[0], a[1], value, host)? {
                    return Ok(Outcome::Pending);
                }
            }
            0x5f => {
                let Some(left) = self.value(a[0], a[1], host)? else {
                    return Ok(Outcome::Pending);
                };
                let mode = if (a[3] as u16) & 0x80 != 0 {
                    i32::from(a[3] & 0x7f)
                } else {
                    i32::from(a[3])
                };
                let right = if (a[3] as u16) & 0x80 != 0 {
                    let Some(right) = self.value(a[2] >> 8, i16::from(a[2] as u8), host)? else {
                        return Ok(Outcome::Pending);
                    };
                    right
                } else {
                    i32::from(a[2])
                };
                //Native unknown comparison selectors retain the original left operand.
                let result = match mode {
                    0 => i32::from(left == right),
                    1 => i32::from(left != right),
                    2 => i32::from(left > right),
                    3 => i32::from(left < right),
                    4 => i32::from(left >= right),
                    5 => i32::from(left <= right),
                    _ => left,
                };
                let marker = program.instruction(next)?;
                let Arguments::Bytes(bytes) = marker.arguments else {
                    bail!("DC2 conditional requires native byte marker")
                };
                ensure!(marker.next - next == 3, "DC2 conditional marker width");
                next = marker.next;
                if result == 0 {
                    next = skip_marker(program, next, bytes[0])?
                }
            }
            0x64 => {
                if (0..5).contains(&a[2]) {
                    //Case100 writes x, y and percent=100; the centre stays.
                    let pose = &mut self.state.poses[a[2] as usize];
                    pose.position = [a[0], a[1]];
                    pose.percent = 100;
                } else if a[2] == 255 {
                    //Case100 also swaps the native1/2 bitmap handles.
                    ensure!(
                        self.state.primary_names[1].is_none() && self.state.primary_names[2].is_none(),
                        "DC2 0x64 native1/2 bitmap swap with loaded portraits is not connected"
                    );
                    self.state.primary_names.swap(1, 2);
                    self.state.primary_ids.swap(1, 2)
                } else {
                    ensure!(a[2] >= 0, "DC2 negative native pose index")
                }
            }
            //Case101: B0B4BC[13*v5] aliases pose (v5-16)'s percent/centre
            //for v5 16..19; 255 sets pose0/pose1 percent (-1 keeps).
            0x65 => match a[3] {
                255 => {
                    if a[0] != -1 {
                        self.state.poses[0].percent = a[0];
                    }
                    if a[1] != -1 {
                        self.state.poses[1].percent = a[1];
                    }
                }
                16..=19 => {
                    let pose = &mut self.state.poses[(a[3] - 16) as usize];
                    pose.percent = a[0];
                    pose.center = [a[1], a[2]];
                }
                20.. => {}
                5..=15 => bail!("DC2 0x65 selector {} writes outside the pose table (B0B4BC alias)", a[3]),
                _ => bail!("DC2 0x65 negative selector {}", a[3]),
            },
            0x66 => {
                if (0..64).contains(&a[0]) {
                    let index = a[0] as u8;
                    let rgb = [a[1] as u8, a[2] as u8, a[3] as u8];
                    if !host.emit(&crate::machine::Event::Palette { index, rgb })? {
                        return Ok(Outcome::Pending);
                    }
                    self.state.palette.insert(index, [rgb[1], rgb[0], rgb[2]]);
                }
            }
            op if native_noop(op) => {}
            _ => bail!(
                "unsupported native DC2 instruction {:#04x} operands {a:?} text {text:?}; execution stopped without advancing",
                instruction.opcode
            ),
        }
        self.advance(next, instruction.opcode)
    }
    fn advance(&mut self, next: u32, opcode: u8) -> Result<Outcome> {
        self.state.pc = next;
        self.state.instructions = self
            .state
            .instructions
            .checked_add(1)
            .context("DC2 instruction counter overflow")?;
        self.state.last_opcode = Some(opcode);
        self.state.observations.clear();
        Ok(Outcome::Advanced)
    }
}
fn named_label(program: &Program, name: &[u8]) -> Result<u32> {
    ensure!(!name.is_empty(), "DC2 empty named jump");
    let mut pc = 2;
    for _ in 0..crate::MAX_INSTRUCTIONS {
        let instruction = program.instruction(pc)?;
        ensure!(
            instruction.opcode != 0,
            "DC2 native named jump target was not found"
        );
        if instruction.opcode == 0x35 && program.literal_bytes(pc) == Some(name) {
            return Ok(pc);
        }
        pc = instruction.next;
    }
    bail!("DC2 named-label scan exceeded its instruction budget")
}
fn skip_marker(program: &Program, mut pc: u32, tag: u8) -> Result<u32> {
    for _ in 0..crate::MAX_INSTRUCTIONS {
        let opcode = program
            .native_byte(pc)
            .context("DC2 conditional scan code extent")?;
        ensure!(
            opcode != 0,
            "DC2 native conditional delimiter was not found"
        );
        let next = if opcode < 41 {
            pc.checked_add(3)
        } else if opcode >= 78 {
            pc.checked_add(9)
        } else {
            let start = pc
                .checked_add(if opcode < 47 { 2 } else { 1 })
                .context("DC2 scanner literal start")?;
            let mut end = start;
            for _ in 0..=crate::MAX_STRING_BYTES {
                if program
                    .native_byte(end)
                    .context("DC2 scanner string extent")?
                    == 0
                {
                    break;
                }
                end = end.checked_add(1).context("DC2 scanner string overflow")?;
            }
            ensure!(
                program.native_byte(end) == Some(0),
                "DC2 scanner string length limit"
            );
            Some(end + 1)
        }
        .context("DC2 scanner PC overflow")?;
        //419FA0 tests byteAAD3B7[next], the second byte argument, not the
        //first marker argument consumed by the conditional expression itself.
        if opcode == 0x1e && program.native_byte(next - 1) == Some(tag) {
            program.instruction(next)?;
            return Ok(next);
        }
        pc = next;
    }
    bail!("DC2 conditional scan exceeded its instruction budget")
}
