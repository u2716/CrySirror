// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native utility, control, input, hit-region and sound syscalls. Semantics
//! follow the examined module's handlers (401D00..403500, 409D90, 41C510,
//! 42EF90..42F1B0, 437E90..43B610); audio uses the shared channel mixer.
use super::*;
use crate::machine::{arrays, ArrayRef};
use crysirror_core::service::{AudioParameters, AudioRequest, EncodedAudio, PcmBus};

pub(super) const CH_MUSIC: u32 = 0;
pub(super) const CH_VOICE: u32 = 1;
pub(super) const CH_SOUND: u32 = 2; // ..=5 for Sound1..Sound4
pub(super) const CH_VOICE_BANK: u32 = 10; // ..=29 voice_ch 0..19

/// Native input snapshot (41D090): press timestamps and held flags.
pub(super) struct Input {
    pub held: [bool; 148],
    pub pressed: [bool; 148],
    pub cursor: [i32; 2],
    pub virtual_cursor: Option<[i32; 2]>,
    pub last_key: i32,
    pub mouse_mode: bool,
    previous_down: u64,
    previous_pointer: [i32; 2],
}
impl Default for Input {
    fn default() -> Self {
        Self {
            held: [false; 148],
            pressed: [false; 148],
            cursor: [0, 0],
            virtual_cursor: None,
            last_key: 0,
            mouse_mode: true,
            previous_down: 0,
            previous_pointer: [0, 0],
        }
    }
}
pub(super) const KEY_LEFT_CLICK: usize = 130;
pub(super) const KEY_RIGHT_CLICK: usize = 131;
pub(super) const KEY_UP: usize = 132;
pub(super) const KEY_DOWN: usize = 133;
pub(super) const KEY_RIGHT: usize = 134;
pub(super) const KEY_LEFT: usize = 135;
pub(super) const KEY_MIDDLE: usize = 136;
pub(super) const KEY_WHEEL_UP: usize = 33;
pub(super) const KEY_WHEEL_DOWN: usize = 34;

impl Input {
    /// Map the shared logical buttons through the native key table
    /// (word_481F04): Enter/Space act as left click, Escape/Insert/Delete as
    /// right click, Shift/Ctrl as middle, arrows as navigation codes.
    pub fn update(&mut self, frame: &InputFrame) {
        use crysirror_core::service::buttons as b;
        let down = frame.buttons_down;
        let pressed = frame.buttons_pressed | (down & !self.previous_down);
        self.previous_down = down;
        self.pressed = [false; 148];
        let map: &[(u64, &[usize], i32)] = &[
            (b::MOUSE_LEFT, &[KEY_LEFT_CLICK], 0),
            (b::MOUSE_RIGHT, &[KEY_RIGHT_CLICK], 0),
            (b::MOUSE_MIDDLE, &[KEY_MIDDLE], 0),
            (b::ENTER, &[13, KEY_LEFT_CLICK], 13),
            (b::SPACE, &[32, KEY_LEFT_CLICK], 32),
            (b::ESCAPE, &[27, KEY_RIGHT_CLICK], 27),
            (b::INSERT, &[45, KEY_RIGHT_CLICK], 45),
            (b::SHIFT, &[16, KEY_MIDDLE], 16),
            (b::CTRL, &[17, KEY_MIDDLE], 17),
            (b::UP, &[38, KEY_UP], 38),
            (b::DOWN, &[40, KEY_DOWN], 40),
            (b::LEFT, &[37, KEY_LEFT], 37),
            (b::RIGHT, &[39, KEY_RIGHT], 39),
            (b::WHEEL_UP, &[KEY_WHEEL_UP], 0),
            (b::WHEEL_DOWN, &[KEY_WHEEL_DOWN], 0),
            (b::PAGE_UP, &[KEY_WHEEL_UP], 0),
            (b::PAGE_DOWN, &[KEY_WHEEL_DOWN], 0),
        ];
        self.held = [false; 148];
        for (bit, codes, vk) in map {
            if down & bit != 0 {
                for code in *codes {
                    self.held[*code] = true;
                }
            }
            if pressed & bit != 0 {
                for code in *codes {
                    self.pressed[*code] = true;
                }
                if *vk != 0 {
                    self.last_key = *vk;
                    self.mouse_mode = false;
                }
            }
        }
        let pointer = [frame.pointer_x, frame.pointer_y];
        if pointer != self.previous_pointer
            || pressed & (b::MOUSE_LEFT | b::MOUSE_RIGHT | b::MOUSE_MIDDLE) != 0
        {
            self.mouse_mode = true;
            self.virtual_cursor = None;
        }
        self.previous_pointer = pointer;
        self.cursor = self.virtual_cursor.unwrap_or(pointer);
    }
    /// 41D150: consume a press edge.
    pub fn take(&mut self, code: usize) -> bool {
        std::mem::take(&mut self.pressed[code])
    }
    pub fn latch(&self) -> u8 {
        use crate::machine::arrays::*;
        (if self.pressed[KEY_LEFT_CLICK] { CLICK_LEFT } else { 0 })
            | (if self.pressed[KEY_RIGHT_CLICK] { CLICK_RIGHT } else { 0 })
            | (if self.pressed[KEY_MIDDLE] { CLICK_MIDDLE } else { 0 })
    }
}

/// 42F090 hit region (13 native words).
#[derive(Clone)]
pub(super) struct Region {
    id: i32,
    rect: [i32; 4],
    enter: i32,
    leave: i32,
    click: i32,
    state: i32,
    depth: usize,
    key: i32,
    focused: bool,
}
/// Per-frame region handlers (frame offsets 40..60) and per-thread regions.
#[derive(Clone, Default)]
pub(super) struct Regions {
    pub list: Vec<Region>,
    left: i32,
    right: i32,
    middle: i32,
    wheel: i32,
    focus: Option<usize>,
    last_focus: Option<usize>,
}

fn int(a: &[Value], i: usize) -> i32 {
    a.get(i).and_then(|v| v.integer().ok()).unwrap_or(0)
}
fn opt(a: &[Value], i: usize, default: i32) -> i32 {
    a.get(i).and_then(|v| v.integer().ok()).unwrap_or(default)
}
fn cstr(v: Option<&Value>) -> Vec<u8> {
    match v {
        Some(Value::String(b)) => b[..b.iter().position(|x| *x == 0).unwrap_or(b.len())].to_vec(),
        _ => vec![],
    }
}
fn number(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Integer(n)) => f64::from(*n),
        Some(Value::Float(f)) => f64::from(*f),
        _ => 0.0,
    }
}
/// CP932 character boundaries (_mbsinc).
fn mb_chars(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let lead = bytes[i];
        let n = if (0x81..=0x9f).contains(&lead) || (0xe0..=0xfc).contains(&lead) { 2 } else { 1 };
        let end = (i + n).min(bytes.len());
        out.push(&bytes[i..end]);
        i = end;
    }
    out
}
fn upper(bytes: &[u8]) -> Vec<u8> {
    mb_chars(bytes)
        .into_iter()
        .flat_map(|c| if c.len() == 1 { vec![c[0].to_ascii_uppercase()] } else { c.to_vec() })
        .collect()
}
fn trim(bytes: &[u8]) -> Vec<u8> {
    let start = bytes.iter().position(|b| *b > 0x20).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|b| *b > 0x20).map_or(start, |e| e + 1);
    bytes[start..end.max(start)].to_vec()
}
pub(super) enum Out {
    Value(Value),
    Wait,
    /// Completed through a VM control transfer already.
    Done,
}
fn s(bytes: Vec<u8>) -> Result<Out> {
    Ok(Out::Value(Value::native_string(&bytes)?))
}
fn i(n: i32) -> Result<Out> {
    Ok(Out::Value(Value::Integer(n)))
}

impl MajiroVm {
    fn read_audio_exists(&self, stem: &[u8]) -> bool {
        let name = encoding_rs::SHIFT_JIS.decode(stem).0.into_owned();
        let base = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_owned();
        let stem = base.rsplit_once('.').map_or(base.as_str(), |(s, _)| s);
        ["wav", "ogg"].iter().any(|ext| self.module_catalog.assets.contains_key(&format!("{stem}.{ext}").to_ascii_lowercase()))
    }
    fn read_audio(&self, host: &mut dyn HostServices, stem: &[u8]) -> Result<Option<Arc<[u8]>>> {
        let name = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(stem)
            .context("Majiro audio name CP932")?;
        let base = name.rsplit(['/', '\\']).next().unwrap_or(&name);
        let stem = base.rsplit_once('.').map_or(base, |(s, _)| s);
        for ext in ["wav", "ogg"] {
            if let Some(paths) = self.module_catalog.assets.get(&format!("{stem}.{ext}").to_ascii_lowercase()) {
                let storage = host.storage(self.client).map_err(core_error)?;
                let path = paths.last().unwrap();
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
                let bytes = crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
                return Ok(Some(bytes.into()));
            }
        }
        Ok(None)
    }
    pub(super) fn play_audio(
        &mut self,
        host: &mut dyn HostServices,
        channel: u32,
        name: &[u8],
        intro: Option<&[u8]>,
        looping: bool,
        bus: PcmBus,
        gain: f32,
        fade_ms: f32,
    ) -> Result<()> {
        let Some(bytes) = self.read_audio(host, name)? else {
            self.warn(format!("Majiro audio {:?} is absent", String::from_utf8_lossy(name)));
            return Ok(());
        };
        let intro = match intro.filter(|n| !n.is_empty()) {
            Some(n) => self.read_audio(host, n)?,
            None => None,
        };
        let (source, intro) = match intro {
            // The second name is the looped body after the first part.
            Some(body) => (body, Some(bytes)),
            None => (bytes, None),
        };
        let request = AudioRequest::Play {
            channel,
            source: EncodedAudio { bytes: source, frame_limit: None },
            intro: intro.map(|bytes| EncodedAudio { bytes, frame_limit: None }),
            parameters: AudioParameters {
                gain,
                looping,
                fade_ms,
                restart: true,
                bus,
                ..Default::default()
            },
        };
        if let Err(error) = host.submit_audio(self.client, request) {
            self.warn(format!("Majiro audio submission: {error}"));
        }
        Ok(())
    }
    fn stop_audio(&mut self, host: &mut dyn HostServices, channel: u32, fade_ms: f32) {
        let _ = host.submit_audio(self.client, AudioRequest::Stop { channel, fade_ms });
        self.audio_names.remove(&channel);
    }
    fn audio_playing(&self, host: &dyn HostServices, channel: u32) -> bool {
        host.audio_info(self.client, channel)
            .ok()
            .flatten()
            .is_some_and(|info| info.source_playing)
    }
    pub(super) fn music_gain(&self) -> f32 {
        self.volume("MusicVolume")
    }
    fn volume(&self, key: &str) -> f32 {
        let value = self
            .settings
            .as_ref()
            .and_then(|s| s.registry_integer(key, 1000).ok())
            .unwrap_or(1000);
        (value.clamp(0, 1000) as f32) / 1000.0
    }

    /// Returns None when the hash is not handled here.
    pub(super) fn try_syscall(
        &mut self,
        m: &mut Machine,
        hash: u32,
        a: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Out>> {
        let now = host.now_ms() as u32;
        let out = match hash {
            // ---------------------------------------------------- arrays
            0x96b07aee | 0x1f57b724 => {
                let element = if hash == 0x96b07aee { 0 } else { 2 };
                let v: Vec<i32> = a.iter().map(|v| v.integer().unwrap_or(1)).collect();
                let dims = match v.len() {
                    0 => [1, 1, 1],
                    1 => [1, 1, v[0]],
                    2 => [1, v[0], v[1]],
                    _ => [v[0], v[1], v[2]],
                };
                Ok(Out::Value(Value::Array(ArrayRef::new(element, dims)?)))
            }
            0x134d5585 => Ok(Out::Value(Value::Array(ArrayRef::new(2, [1, 1, 1])?))),
            // ---------------------------------------------------- math
            0x6cfc884a => i(int(a, 0).max(int(a, 1))),
            0x3d053317 => i(int(a, 0).min(int(a, 1))),
            0xee2b6d92 => Ok(Out::Value(Value::Float((number(a.first()) * 0.017453292519444).sin() as f32))),
            0x12877048 => Ok(Out::Value(Value::Float((number(a.first()) * 0.017453292519444).cos() as f32))),
            0xee6b328d => {
                let elapsed = (now.wrapping_sub(int(a, 0) as u32) as i32).wrapping_abs();
                let (duration, range) = (int(a, 1), int(a, 2));
                if duration > 0 {
                    i((i64::from(elapsed.min(duration)) * i64::from(range) / i64::from(duration)) as i32)
                } else {
                    i(range)
                }
            }
            // ---------------------------------------------------- strings
            0x06906970 | 0x25401a1f => {
                let bytes = cstr(a.first());
                let n = int(a, 1).max(0) as usize;
                let chars = mb_chars(&bytes);
                if chars.len() <= n {
                    return Ok(Some(s(bytes)?));
                }
                let part: Vec<u8> = if hash == 0x06906970 {
                    chars[..n].concat()
                } else {
                    chars[chars.len() - n..].concat()
                };
                s(part)
            }
            0x8aef9167 => {
                let bytes = cstr(a.first());
                let chars = mb_chars(&bytes);
                let start = int(a, 1).max(0) as usize;
                let len = int(a, 2).max(0) as usize;
                if chars.len() <= start {
                    return Ok(Some(s(vec![])?));
                }
                s(chars[start..(start + len).min(chars.len())].concat())
            }
            0xb0c8f550 => i(mb_chars(&cstr(a.first())).len() as i32),
            0x69db4512 => {
                let hay = upper(&cstr(a.first()));
                let needle = upper(&cstr(a.get(1)));
                let pos = (0..=hay.len().saturating_sub(needle.len()))
                    .find(|p| hay[*p..].starts_with(&needle) && !needle.is_empty());
                i(pos.map_or(-1, |p| mb_chars(&hay[..p]).len() as i32))
            }
            0x3fe05b39 => {
                let bytes = cstr(a.first());
                let mut index = int(a, 1);
                let mut rest: &[u8] = &bytes;
                while index > 0 {
                    match rest.iter().position(|b| *b == b',') {
                        Some(p) => rest = &rest[p + 1..],
                        None => break,
                    }
                    index -= 1;
                }
                let mut field = if index == 0 {
                    let end = rest.iter().position(|b| *b == b',').unwrap_or(rest.len());
                    trim(&rest[..end])
                } else {
                    vec![]
                };
                if field.len() >= 2 && field[0] == b'"' && field[field.len() - 1] == b'"' {
                    field = field[1..field.len() - 1].to_vec();
                }
                s(field)
            }
            0x6f769889 => s(int(a, 0).to_string().into_bytes()),
            0x109ca5db => i(crate::byte_policy::integer(&cstr(a.first())).unwrap_or(0)),
            0x45815f6d => {
                let (x, y) = (upper(&cstr(a.get(1))), upper(&cstr(a.first())));
                i(match x.cmp(&y) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                })
            }
            // ---------------------------------------------------- variables / flags
            0x6e83677a | 0xf9a3b675 | 0xcd290aef => {
                let mut name = cstr(a.first());
                if !name.get(1..).is_some_and(|rest| rest.contains(&b'@')) {
                    name.extend_from_slice(b"@GLOBAL");
                }
                let h = crate::program::name_hash(&name);
                if hash == 0xcd290aef {
                    if let Some(value) = a.get(1) {
                        m.set_global(h, value.clone());
                    }
                    i(0)
                } else {
                    match m.global(h) {
                        Some(Value::Integer(n)) => i(n),
                        Some(value) => Ok(Out::Value(value)),
                        None => i(0),
                    }
                }
            }
            0xeb5cc468 | 0x99a5de25 => {
                let mut name = cstr(a.first());
                if let Some(p) = name.iter().position(|b| *b == b'.') {
                    name.truncate(p);
                }
                let name = name.to_ascii_lowercase();
                let h = crate::program::name_hash(&name);
                if hash == 0xeb5cc468 {
                    if self.flags.len() < 10000 {
                        self.flags.insert(h);
                        i(0)
                    } else {
                        i(-1)
                    }
                } else {
                    i(i32::from(self.flags.contains(&h)))
                }
            }
            // ---------------------------------------------------- diagnostics
            0xf3679c34 | 0xdf644f85 => {
                let text: Vec<String> = a
                    .iter()
                    .map(|v| match v {
                        Value::Integer(n) => n.to_string(),
                        Value::Float(f) => f.to_string(),
                        Value::String(b) => encoding_rs::SHIFT_JIS.decode(&cstr(Some(&Value::String(b.clone())))).0.into_owned(),
                        Value::Array(_) => "<array>".into(),
                    })
                    .collect();
                self.warn(format!("script {}: {}", if hash == 0xdf644f85 { "dialog" } else { "debugout" }, text.join(" ")));
                i(0)
            }
            0x619de833 => i(0),
            0xc32e286f => s(vec![]),
            0x05ea6e4d => s(self.page_text.clone()),
            0x42d7c922 | 0x62374297 | 0xe3605cfa | 0x221c2cc2 => i(0),
            // ---------------------------------------------------- control
            0x718ef651 => {
                let name = cstr(a.first());
                let path = self.module_catalog.resolve_name(&name)?.to_owned();
                let program = match m.linked_module(&path) {
                    Some(p) => p,
                    None => {
                        let bytes = self.read_asset_bytes(host, &path)?;
                        Arc::new(Program::parse(&bytes)?)
                    }
                };
                let args = a[1..].to_vec();
                self.reset_thread_state(m.current_thread());
                m.run_script(path, program, args)?;
                Ok(Out::Done)
            }
            0x4ef903f3 => {
                let id = if a.is_empty() { m.current_thread() } else { int(a, 0) };
                m.acknowledge(if returns { vec![Value::Integer(0)] } else { vec![] })?;
                m.kill_thread(id)?;
                Ok(Out::Done)
            }
            0x3704c919 | 0x8d8fb32e => {
                self.exit_requested = true;
                self.save_system(m, host)?;
                for id in m.thread_ids() {
                    if id != m.current_thread() {
                        m.kill_thread(id)?;
                    }
                }
                m.acknowledge(if returns { vec![Value::Integer(0)] } else { vec![] })?;
                let current = m.current_thread();
                m.kill_thread(current)?;
                Ok(Out::Done)
            }
            0x818a4b92 | 0xfac4f361 | 0xb7e3e81c | 0x9ef1ddc3 | 0x86f0f608 | 0x4b306c5c | 0x3eabf498
            | 0xaccf4004 | 0x0542e1b7 | 0x9143dc9e | 0x95ff28f2 => i(0),
            0x307f28bc => {
                if !self.replay_step(m, int(a, 0), returns, host)? {
                    return Ok(Some(Out::Wait));
                }
                Ok(Out::Done)
            }
            0x6621f84d => i(self.history_page_start(int(a, 0))),
            0x2cb1bc41 => i(self.history_pages_back(int(a, 0))),
            0x82fd765b => i(self.console.font.as_ref().map_or(20, |f| f.height)),
            0x25d92502 | 0xbbf35806 => s(vec![]),
            0x88b4b26c
            | 0x78a6a112 | 0xe91b0504 | 0xdd2056f6 | 0x86e933d1 | 0x3c84073c | 0xac796720 => i(0),
            0xa79731a2 => {
                self.warn("movie_play is skipped (movie decoding not connected for Majiro)".into());
                i(0)
            }
            0xff75cd29 => {
                // -9056983: wait until start+duration; returns the deadline.
                let key = m.context_id();
                let deadline = match self.timed_waits.get(&key) {
                    Some(d) => *d,
                    None => {
                        let (start, duration) = (int(a, 0), int(a, 1));
                        let start = if (now.wrapping_sub(start as u32) as i32).wrapping_abs() >= duration.wrapping_mul(10) {
                            now as i32
                        } else {
                            start
                        };
                        let d = start.wrapping_add(duration);
                        self.timed_waits.insert(key, d);
                        d
                    }
                };
                if (now as i32).wrapping_sub(deadline) < 0 && ((now as i32).wrapping_sub(deadline)).abs() <= 43_200_000 {
                    return Ok(Some(Out::Wait));
                }
                self.timed_waits.remove(&key);
                i(deadline.max(now as i32 - 10))
            }
            0x804ff66d => {
                if self.audio_playing(host, CH_SOUND + 1) {
                    return Ok(Some(Out::Wait));
                }
                i(0)
            }
            0x777aa894 | 0x838de99b | 0xc9b362d0 => {
                let channel = match hash {
                    0x777aa894 => CH_SOUND,
                    0x838de99b => CH_SOUND + 2,
                    _ => CH_SOUND + 3,
                };
                self.stop_audio(host, channel, 0.0);
                i(0)
            }
            0x38721a26 | 0xb0ce081a | 0x507f22c6 | 0x824d124f | 0x89ce8c53 => i(0),
            0xa589dbd1 => {
                self.saved_music = self.audio_names.get(&CH_MUSIC).cloned();
                i(0)
            }
            0xf86fc950 => {
                let saved = self.saved_music.clone();
                if saved != self.audio_names.get(&CH_MUSIC).cloned() {
                    match saved {
                        Some(name) if !name.is_empty() => {
                            let gain = self.volume("MusicVolume");
                            self.play_audio(host, CH_MUSIC, &name, None, true, PcmBus::Music, gain, 0.0)?;
                            self.audio_names.insert(CH_MUSIC, name);
                        }
                        _ => self.stop_audio(host, CH_MUSIC, 0.0),
                    }
                }
                i(0)
            }
            0xc21f8b49 | 0x44bc7555 | 0xab13be44 | 0xbc42ca9a => i(0),
            0x1204d7e8 => {
                // Wait until click/skip inputs are released.
                let held = [KEY_LEFT_CLICK, KEY_RIGHT_CLICK, KEY_MIDDLE].iter().any(|k| self.input.held[*k]);
                if held {
                    return Ok(Some(Out::Wait));
                }
                m.take_clicks();
                i(0)
            }
            0x4662e95d => {
                // Wait for the next rendered frame.
                if self.frame_wait != Some(m.pc) {
                    self.frame_wait = Some(m.pc);
                    return Ok(Some(Out::Wait));
                }
                self.frame_wait = None;
                i(0)
            }
            0x45ad5f4d => {
                let name = cstr(a.first());
                let exists = self.read_audio_exists(&name);
                i(i32::from(exists))
            }
            0x60085fa4 => i(-1),
            // ---------------------------------------------------- input
            0x892e9b20 | 0x008acbc0 => {
                let bits = i32::from(self.input.take(KEY_WHEEL_UP)) * 8
                    + i32::from(self.input.take(KEY_WHEEL_DOWN)) * 0x10
                    + i32::from(m.take_clicks() & 7);
                if hash == 0x008acbc0 && bits == 0 {
                    return Ok(Some(Out::Wait));
                }
                i(bits)
            }
            0x2075c300 => {
                let h = |k: usize| i32::from(self.input.held[k] || self.input.pressed[k]);
                i(h(KEY_LEFT_CLICK) + 2 * (h(KEY_RIGHT_CLICK) + 2 * (h(KEY_MIDDLE) + 2 * (h(KEY_WHEEL_UP) + 2 * h(KEY_WHEEL_DOWN)))))
            }
            0xefd49bc9 => i(self.input.cursor[0]),
            0x72db7abf => i(self.input.cursor[1]),
            0xb84c9953 => {
                let (x, y) = (int(a, 0), int(a, 1));
                self.input.virtual_cursor = if x == -10000 { None } else { Some([x, y]) };
                if let Some(c) = self.input.virtual_cursor {
                    self.input.cursor = c;
                }
                i(0)
            }
            0x3670b283 => {
                let [x, y] = self.input.cursor;
                let [w, h] = self.native_client_size;
                i(i32::from(x >= 0 && y >= 0 && x < w && y < h))
            }
            0xf63cb36c => {
                // 41C510: wait for a click (or movie end); 0 disables clicks.
                let clickable = opt(a, 0, 1) != 0;
                if !clickable || m.take_clicks() != 0 {
                    i(0)
                } else {
                    return Ok(Some(Out::Wait));
                }
            }
            // ---------------------------------------------------- hit regions
            0xf8d18340 => {
                let depth = m.depth();
                let regions = self.regions.entry(m.current_thread()).or_default();
                regions.list.push(Region {
                    id: int(a, 0),
                    rect: [int(a, 1), int(a, 2), int(a, 3), int(a, 4)],
                    enter: int(a, 5),
                    leave: int(a, 6),
                    click: int(a, 7),
                    key: opt(a, 8, 0),
                    state: 0,
                    depth,
                    focused: false,
                });
                i(0)
            }
            0xc438ba98 => {
                // 421AC0: park this thread's regions while a menu runs.
                let parked = self.regions.remove(&m.current_thread()).unwrap_or_default();
                self.parked_regions = Some(parked);
                i(0)
            }
            0x3378635d => {
                // 421B60: restore the parked regions.
                if let Some(parked) = self.parked_regions.take() {
                    self.regions.insert(m.current_thread(), parked);
                } else {
                    self.regions.remove(&m.current_thread());
                }
                i(0)
            }
            0x4e1eb9d0 => {
                self.regions.remove(&m.current_thread());
                self.input.take(KEY_WHEEL_UP);
                self.input.take(KEY_WHEEL_DOWN);
                i(0)
            }
            0xa7c6e918 => {
                let regions = self.regions.entry(m.current_thread()).or_default();
                regions.left = int(a, 0);
                regions.right = int(a, 1);
                regions.middle = opt(a, 2, 0);
                regions.wheel = opt(a, 3, 0);
                i(0)
            }
            0x3a3bfb29 => return self.poll_regions(m, returns).map(Some),
            // ---------------------------------------------------- music
            0x4980f82c | 0x0dc634c4 => {
                let name = cstr(a.first());
                if name.is_empty() {
                    self.stop_audio(host, CH_MUSIC, 0.0);
                } else {
                    let (intro, looping) = if hash == 0x0dc634c4 {
                        (Some(cstr(a.get(1))), true)
                    } else {
                        (None, opt(a, 1, 1) != 0)
                    };
                    let gain = self.volume("MusicVolume");
                    self.play_audio(host, CH_MUSIC, &name, intro.as_deref(), looping, PcmBus::Music, gain, 0.0)?;
                    self.audio_names.insert(CH_MUSIC, name);
                }
                i(0)
            }
            0x15eedeaa | 0x4f6367b3 => {
                self.stop_audio(host, CH_MUSIC, int(a, 0).max(0) as f32);
                i(0)
            }
            0xe7b90df5 => {
                let gain = self.volume("MusicVolume") * int(a, 0) as f32 / 1000.0;
                let _ = host.submit_audio(self.client, AudioRequest::SetGain { channel: CH_MUSIC, gain, fade_ms: 0.0 });
                i(0)
            }
            // ---------------------------------------------------- sounds 1..4
            0xf62e3ca7 | 0x708b0256 | 0xed84e320 | 0x884949e0 | 0xb268f04f | 0xde1e2ed0 | 0x4311cfa6 | 0x26dc6566 => {
                let slot = match hash {
                    0xf62e3ca7 | 0xb268f04f => 0,
                    0x708b0256 | 0xde1e2ed0 => 1,
                    0xed84e320 | 0x4311cfa6 => 2,
                    _ => 3,
                };
                let looping = matches!(hash, 0xb268f04f | 0xde1e2ed0 | 0x4311cfa6 | 0x26dc6566);
                let channel = CH_SOUND + slot;
                let name = cstr(a.first());
                self.stop_audio(host, channel, 0.0);
                if !name.is_empty() {
                    let gain = self.volume("SoundVolume");
                    self.play_audio(host, channel, &name, None, looping, PcmBus::Sfx, gain, 0.0)?;
                    self.audio_names.insert(channel, name);
                }
                i(0)
            }
            0x926c13e5 | 0x83a53ffa => {
                self.stop_audio(host, CH_SOUND, int(a, 0).max(0) as f32);
                i(0)
            }
            0x546f69c3 | 0x2e861c51 => {
                self.stop_audio(host, CH_SOUND + 1, if hash == 0x2e861c51 { int(a, 0).max(0) as f32 } else { 0.0 });
                i(0)
            }
            0x39ae7891 => {
                self.stop_audio(host, CH_SOUND + 2, int(a, 0).max(0) as f32);
                i(0)
            }
            0x1d93fd7e => i(i32::from(self.audio_playing(host, CH_SOUND + 3))),
            0x5bf8f175 | 0xe712fec1 | 0xf03a9a01 => {
                let channel = match hash {
                    0x5bf8f175 => CH_SOUND,
                    0xe712fec1 => CH_SOUND + 1,
                    _ => CH_SOUND + 2,
                };
                let name = if self.audio_playing(host, channel) {
                    self.audio_names.get(&channel).cloned().unwrap_or_default()
                } else {
                    vec![]
                };
                s(name)
            }
            // ---------------------------------------------------- voice
            0x90d5298a | 0x32d0236c => {
                let name = cstr(a.first());
                self.stop_audio(host, CH_VOICE, 0.0);
                if !name.is_empty() {
                    let gain = self.volume("VoiceVolume");
                    self.play_audio(host, CH_VOICE, &name, None, false, PcmBus::Voice, gain, 0.0)?;
                    self.audio_names.insert(CH_VOICE, name);
                }
                i(0)
            }
            0xf1097a07 => i(i32::from(self.audio_playing(host, CH_VOICE))),
            0xd1f672c7 => {
                if self.audio_playing(host, CH_VOICE) && m.take_clicks() == 0 {
                    return Ok(Some(Out::Wait));
                }
                i(0)
            }
            0x801d7af9 => {
                let gain = self.volume("VoiceVolume") * int(a, 0) as f32 / 1000.0;
                let _ = host.submit_audio(self.client, AudioRequest::SetGain { channel: CH_VOICE, gain, fade_ms: 0.0 });
                i(0)
            }
            0x8e8167f2 => {
                let ch = int(a, 0);
                if (0..20).contains(&ch) {
                    let channel = CH_VOICE_BANK + ch as u32;
                    let name = cstr(a.get(1));
                    self.stop_audio(host, channel, 0.0);
                    if !name.is_empty() {
                        let gain = self.volume("VoiceVolume");
                        self.play_audio(host, channel, &name, None, opt(a, 2, 0) != 0, PcmBus::Voice, gain, 0.0)?;
                    }
                }
                i(0)
            }
            0xcb665ac0 | 0x19fc7cc7 => {
                let ch = int(a, 0);
                if (0..20).contains(&ch) {
                    let gain = self.volume("VoiceVolume") * int(a, 1) as f32 / 1000.0;
                    let fade = if hash == 0x19fc7cc7 { int(a, 2).max(0) as f32 } else { 0.0 };
                    let _ = host.submit_audio(
                        self.client,
                        AudioRequest::SetGain { channel: CH_VOICE_BANK + ch as u32, gain, fade_ms: fade },
                    );
                }
                i(0)
            }
            0x5a981e41 | 0x8f905b3f => {
                let ch = int(a, 0);
                if (0..20).contains(&ch) {
                    let fade = if hash == 0x8f905b3f { int(a, 1).max(0) as f32 } else { 0.0 };
                    self.stop_audio(host, CH_VOICE_BANK + ch as u32, fade);
                }
                i(0)
            }
            0x97bd691d | 0x02026ac6 => i(1),
            0x0c7a0e97 | 0x6eaec2a0 | 0x072e738c | 0x6ed38886 => {
                let key = match hash {
                    0x0c7a0e97 => "MusicVolume",
                    0x6eaec2a0 => "VoiceVolume",
                    0x072e738c => "SoundVolume",
                    _ => "AutoSpeed",
                };
                if let Some(settings) = &mut self.settings {
                    if let Ok(candidate) = settings.candidate_integer(key, int(a, 0), false) {
                        *settings = candidate;
                        self.settings_dirty = true;
                    }
                }
                if hash == 0x0c7a0e97 {
                    let gain = self.volume("MusicVolume");
                    let _ = host.submit_audio(self.client, AudioRequest::SetGain { channel: CH_MUSIC, gain, fade_ms: 0.0 });
                }
                i(0)
            }
            _ => return Ok(None),
        };
        out.map(Some)
    }

    /// 42F1B0 hit-region poll: hover/leave/click callbacks and handlers.
    fn poll_regions(&mut self, m: &mut Machine, returns: bool) -> Result<Out> {
        let output = if returns { vec![Value::Integer(0)] } else { vec![] };
        let thread = m.current_thread();
        let depth = m.depth();
        let latch = m.take_clicks();
        let (mut left, right, middle) = (
            latch & arrays::CLICK_LEFT != 0,
            latch & arrays::CLICK_RIGHT != 0,
            latch & arrays::CLICK_MIDDLE != 0,
        );
        let key = std::mem::take(&mut self.input.last_key);
        let mouse_mode = self.input.mouse_mode;
        let mut cursor = self.input.cursor;
        let Some(regions) = self.regions.get_mut(&thread) else {
            m.acknowledge(output)?;
            return Ok(Out::Done);
        };
        if mouse_mode {
            for r in &mut regions.list {
                r.focused = false;
            }
        } else if matches!(key, 37..=40) {
            // Keyboard navigation between keyed regions (nearest in direction).
            let current = regions.focus.or(regions.last_focus).and_then(|f| regions.list.get(f)).cloned();
            let target = match current {
                Some(cur) => {
                    let (cx, cy) = (cur.rect[0], cur.rect[1]);
                    regions
                        .list
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.key != 0 && r.id != cur.id)
                        .filter_map(|(i, r)| {
                            let d = match key {
                                38 => cy - r.rect[1],
                                40 => r.rect[1] - cy,
                                37 => cx - r.rect[0],
                                _ => r.rect[0] - cx,
                            };
                            (d > 0).then_some((d, i))
                        })
                        .min()
                        .map(|(_, i)| i)
                }
                None => regions.list.iter().position(|r| r.key != 0),
            };
            if let Some(t) = target {
                regions.focus = Some(t);
                regions.last_focus = Some(t);
                let r = &regions.list[t];
                cursor = [r.rect[0] + r.rect[2] / 2, r.rect[1] + r.rect[3] / 2];
                self.input.virtual_cursor = Some(cursor);
                self.input.cursor = cursor;
            }
        } else if key == 13 || key == 32 {
            if let Some(f) = regions.focus.or(regions.last_focus) {
                if regions.list.get(f).is_some_and(|r| r.state == 1) {
                    left = true;
                }
            }
        }
        let inside = |r: &Region, c: [i32; 2]| {
            c[0] >= r.rect[0] && c[0] < r.rect[0] + r.rect[2] && c[1] >= r.rect[1] && c[1] < r.rect[1] + r.rect[3]
        };
        let args = |r: &Region| {
            vec![
                Value::Integer(r.id),
                Value::Integer(r.rect[0]),
                Value::Integer(r.rect[1]),
                Value::Integer(r.rect[2]),
                Value::Integer(r.rect[3]),
            ]
        };
        let xy = vec![Value::Integer(cursor[0]), Value::Integer(cursor[1])];
        if regions.wheel != 0 {
            let up = self.input.take(KEY_WHEEL_UP);
            let down = self.input.take(KEY_WHEEL_DOWN);
            if up || down {
                let bits = i32::from(up) + 2 * i32::from(down);
                let f = regions.wheel;
                m.complete_and_call(output, f as u32, vec![Value::Integer(cursor[0]), Value::Integer(cursor[1]), Value::Integer(bits)])?;
                return Ok(Out::Done);
            }
        }
        if left {
            if let Some(r) = regions.list.iter_mut().find(|r| r.depth >= depth && r.state == 1) {
                r.state = 2;
                let (f, a) = (r.click, args(r));
                self.input.virtual_cursor = None;
                if f != 0 {
                    m.complete_and_call(output, f as u32, a)?;
                } else {
                    m.acknowledge(output)?;
                }
                return Ok(Out::Done);
            }
            if regions.left != 0 {
                let f = regions.left;
                m.complete_and_call(output, f as u32, xy)?;
                return Ok(Out::Done);
            }
        } else if right && regions.right != 0 {
            let f = regions.right;
            m.complete_and_call(output, f as u32, xy)?;
            return Ok(Out::Done);
        } else if middle && regions.middle != 0 {
            let f = regions.middle;
            m.complete_and_call(output, f as u32, xy)?;
            return Ok(Out::Done);
        } else {
            let mut any = false;
            for idx in 0..regions.list.len() {
                let r = &regions.list[idx];
                if r.state >= 1 {
                    if !inside(r, cursor) {
                        let (f, a) = (r.leave, args(r));
                        regions.list[idx].state = 0;
                        if mouse_mode {
                            regions.focus = None;
                        }
                        if f != 0 {
                            m.complete_and_call(output, f as u32, a)?;
                        } else {
                            m.acknowledge(output)?;
                        }
                        return Ok(Out::Done);
                    }
                    any = true;
                }
            }
            if !any {
                if let Some(idx) = regions.list.iter().position(|r| r.depth >= depth && inside(r, cursor)) {
                    regions.focus = Some(idx);
                    if regions.list[idx].key != 0 {
                        regions.last_focus = Some(idx);
                    }
                    regions.list[idx].state = 1;
                    let (f, a) = (regions.list[idx].enter, args(&regions.list[idx]));
                    if f != 0 {
                        m.complete_and_call(output, f as u32, a)?;
                    } else {
                        m.acknowledge(output)?;
                    }
                    return Ok(Out::Done);
                }
            }
        }
        m.acknowledge(output)?;
        Ok(Out::Done)
    }
    pub(super) fn reset_thread_state(&mut self, thread: i32) {
        self.regions.remove(&thread);
        self.fontout.remove(&thread);
    }
    fn read_asset_bytes(&self, host: &mut dyn HostServices, path: &str) -> Result<Vec<u8>> {
        let storage = host.storage(self.client).map_err(core_error)?;
        let reader = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
        crysirror_resource::read_all(reader.as_ref(), MAX_MODULE_BYTES as u64)
    }
}
