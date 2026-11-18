// SPDX-License-Identifier: MPL-2.0
//! Native tag interpreter 62DA70 and its handlers. Several native tags are
//! macros that format further tags through 62FFA0; those expansions are kept
//! verbatim here so the same layer/camera policy executes them.
//!
//! | tag | native | here |
//! | --- | --- | --- |
//! | layer layerclear layerwait layercamera layercmd | 50B860/641B70 | `adv` |
//! | sound soundwait soundstop | 628D40 628FC0 628F60 | BGM channel |
//! | wait | 4FFE20 | VM wait |
//! | cg cg_h cutin cutinclear | 62E340 62E3E0 62E660 62E800 | expansions |
//! | ef | 62E8C0 (`<effect class='ef'>` frame) | effect frame |
//! | msgframe | 62EAA0 | window state |
//! | char char-setup charclear | 6470A0 647850/6461C0 | expansions |
//! | movie | 62E9D0 | expansion |
//! | se sestop | 627D40 62EF50 | SE channels |
//! | shake | 612E20/54F240 | shake state |
//! | openurl | 62EDB0 | yes/no box; host URL launch is a gap |
//! | screen | 62EE70 | screen switch |
//! | imgblk /imgblk | 62E920 | screen transition |
//! | voice voicewait | 62F1A0 630050 | voice channel |
//! | save autosave | 62EE10 62E330 | save policy |
//! | message | 62E970 | text colour |
//! | wbchar wbclear wbblk /wbblk | 63B6A0 63B7E0 63B670 | word-balloon state |
//! | systemvoice systemse systembgm | 62F180 62F000 62EFE0 | ini tables |
//! | automode textmode novel | 610F40 62F7A0 648790 | text policy |
//! | chapter cgfilter charfilter | 62E5E0 62E590 62E610 | state |
//! | face faceclear clear | 634B80 634DC0 630190 | expansions |
//! | scene label k await event | 62EE50 62F8B0 6137D0 6138F0 | VM/save policy |
//! | textspeed /textspeed maliesystem bgv | 62F940 62F8F0 630900 | state |
//! Any other name is ignored by the native dispatcher.
use super::*;

/// Native runtime state touched by tags and MalieSystem messages; part of
/// the save record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagState {
    pub save_enabled: bool,
    pub automode_showmsg: bool,
    pub text_mode: u32,
    pub novel: bool,
    pub ruby: bool,
    pub chapter: String,
    pub cg_filter: String,
    pub char_filter: String,
    /// `textspeed time=`: fixed per-glyph interval (ms) overriding config.
    pub text_speed: Option<u64>,
    pub overlap: Option<f32>,
    pub msg_type: u32,
    pub msg_dir: u32,
    pub font: Option<i32>,
    pub font_size: Option<i32>,
    pub wb_font_size: Option<i32>,
    pub text_color: Option<[u8; 3]>,
    pub user_color: bool,
    pub bgm: Option<String>,
    pub bgm_loop: (u64, Option<u64>),
    pub bgm_volume: f32,
    pub bgm_started: u64,
    pub wb_block: bool,
    pub wb_chars: BTreeMap<String, ([f32; 3], String)>,
    pub read_labels: BTreeSet<String>,
    pub scenes: Vec<String>,
    pub bgv: BTreeMap<String, String>,
    pub auto_click: bool,
    pub extra_debug: bool,
    pub draw_debug: bool,
}
impl Default for TagState {
    fn default() -> Self {
        Self {
            save_enabled: true,
            automode_showmsg: false,
            text_mode: 0,
            novel: false,
            ruby: true,
            chapter: String::new(),
            cg_filter: String::new(),
            char_filter: String::new(),
            text_speed: None,
            overlap: None,
            msg_type: 0,
            msg_dir: 0,
            font: None,
            font_size: None,
            wb_font_size: None,
            text_color: None,
            user_color: false,
            bgm: None,
            bgm_loop: (0, None),
            bgm_volume: 1.,
            bgm_started: 0,
            wb_block: false,
            wb_chars: BTreeMap::new(),
            read_labels: BTreeSet::new(),
            scenes: Vec::new(),
            bgv: BTreeMap::new(),
            auto_click: false,
            extra_debug: false,
            draw_debug: false,
        }
    }
}
fn get<'a>(a: &'a Attrs, k: &str) -> Option<&'a str> {
    a.get(k).map(String::as_str).filter(|v| !v.is_empty())
}
fn float(v: &str) -> Result<f32> {
    let v = v.trim().trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let f = v.parse::<f32>().with_context(|| format!("Malie number {v:?}"))?;
    ensure!(f.is_finite(), "Malie number range");
    Ok(f)
}
/// 63E7A0 seconds; `time` defaults via 630550.
fn secs(a: &Attrs, k: &str, default: f32) -> Result<f32> {
    Ok(get(a, k).map(|v| adv::seconds(v).map(|ms| ms as f32 / 1000.)).transpose()?.unwrap_or(default))
}
/// 63B810: `who face dress` with empty trailing parts.
fn split_char(src: &str) -> (String, String, String) {
    let mut p = src.split_whitespace().map(str::to_ascii_lowercase);
    (p.next().unwrap_or_default(), p.next().unwrap_or_default(), p.next().unwrap_or_default())
}
/// 646E80 direction names.
fn direction(v: Option<&str>) -> Option<u8> {
    match v.map(str::to_ascii_lowercase).as_deref() {
        Some("l" | "left") => Some(0),
        Some("r" | "right") => Some(1),
        Some("u" | "up") => Some(2),
        Some("d" | "down") => Some(3),
        _ => None,
    }
}
/// 646F40 place names and the 7B5410 slot table (x, y, z).
const PLACES: [(&str, [f32; 3]); 15] = [
    ("lo", [0., 0., 6.]),
    ("l", [0., 0., 2.]),
    ("c", [0., 0., 0.]),
    ("r", [0., 0., 4.]),
    ("ro", [0., 0., 8.]),
    ("lbo", [0., 0., 16.]),
    ("lb", [0., 0., 12.]),
    ("cb", [0., 0., 10.]),
    ("rb", [0., 0., 14.]),
    ("rbo", [0., 0., 18.]),
    ("lfo", [0., 0., -4.]),
    ("lf", [0., 0., -8.]),
    ("cf", [0., 0., -10.]),
    ("rf", [0., 0., -6.]),
    ("rfo", [0., 0., -2.]),
];
impl MalieVm {
    /// 62FFA0: format, then execute through the tag interpreter.
    pub(super) fn emit(&mut self, h: &mut dyn HostServices, text: String) -> Result<()> {
        self.tag(h, &text)
    }
    fn skipping(&self) -> bool {
        self.skip_active
    }
    pub(super) fn tag_one(&mut self, h: &mut dyn HostServices, name: &str, a: &Attrs) -> Result<()> {
        let now = h.now_ms();
        let storage = self.storage.clone().context("Malie storage")?;
        let mut reader = move |path: &str| -> Result<Option<String>> {
            if path.ends_with(".webm") {
                let p = VfsPath::new(Mount::Asset, path)?;
                return Ok(storage.metadata(Mount::Asset, p.relative())?.map(|_| String::new()));
            }
            Ok(read_optional(storage.as_ref(), path, 1024 * 1024)?.map(|b| decode_text(&b)))
        };
        match name {
            "layer" => {
                if let Some(env) = get(a, "env-src") {
                    return self.env_layer(h, a, env);
                }
                self.adv.as_mut().unwrap().layer(a, now, &mut reader)?;
            }
            "layerclear" => {
                if let Some(id) = get(a, "id") {
                    self.env_sources.remove(&id.to_ascii_lowercase());
                    if let Some(c) = self.env_channels.remove(&id.to_ascii_lowercase()) {
                        let fade = get(a, "time").map(adv::seconds).transpose()?.unwrap_or(0) as f32;
                        h.submit_audio(self.client, AudioRequest::Stop { channel: c, fade_ms: fade })?;
                    }
                }
                self.adv.as_mut().unwrap().layer_clear(a, now)?;
            }
            "layerwait" => self.wait = Wait::Layer(a.clone()),
            "layercamera" => self.adv.as_mut().unwrap().camera(a, now)?,
            // 642B10 → 510960 sends message 1000 with the tag to both OL3D
            // content slots. None of the content classes (Sprite3D, MOVIE3D,
            // Mesh, SVGMMovie, Wordballoon, SVG2) registers a message
            // callback and SVG's is 4B7700 (returns 0), so it has no effect.
            "layercmd" => {}
            "sound" => self.sound(h, a)?,
            "soundwait" => {
                // 628FC0: wait for the remaining BGM time.
                if let Some(info) = h.audio_info(self.client, BGM)? {
                    if info.source_playing && !info.looping {
                        self.wait = Wait::Channel(BGM);
                    }
                }
            }
            "soundstop" => {
                let t = secs(a, "time", 0.)?;
                let fade = if self.skipping() { 0. } else { t * 1000. };
                self.tags.bgm = None;
                self.bgm = None;
                h.submit_audio(self.client, AudioRequest::Stop { channel: BGM, fade_ms: fade })?;
            }
            "wait" => {
                let t = adv::seconds(get(a, "time").context("Malie wait time")?)?;
                self.wait = Wait::Until(now + t);
            }
            "cg" => {
                // 62E340: background CG with an optional transition.
                let src = get(a, "src").unwrap_or("").to_owned();
                let time = secs(a, "time", 1.)?;
                let hide = get(a, "msg-hide").is_none_or(|v| v != "0" && !v.eq_ignore_ascii_case("false"));
                if hide {
                    self.adv.as_mut().unwrap().state.frame.visible = false;
                }
                self.begin_transition(a, time);
                self.emit(h, "<layerclear parent='back' id='bgh'>".into())?;
                self.emit(h, format!("<layer parent='back' id='bg' src='{src}' opacity='1' time='0'>"))?;
                self.end_transition(now);
            }
            "cg_h" => {
                // 62E3E0: tall CG scrolled top-down/bottom-up over 3 s.
                self.emit(h, "<layerwait parent='chara'>".into())?;
                let src = get(a, "src").context("Malie cg_h src")?.to_owned();
                let size = self.adv.as_mut().unwrap().picture_size(&src, &mut reader)?;
                let screen_h = self.config.canvas[1] as f32;
                let (from, to) = match get(a, "type").map(str::to_ascii_lowercase).as_deref() {
                    Some("td") => (0., screen_h - size[1]),
                    Some("bu") => (screen_h - size[1], 0.),
                    _ => (0., 0.),
                };
                let time = secs(a, "time", 1.)?;
                self.begin_transition(a, time);
                self.emit(h, format!("<layer parent='back' id='bgh' src='{src}' position='0 {from}'>"))?;
                self.end_transition(now);
                self.emit(h, format!("<layer parent='back' id='bgh' position='0 {to}' time='3s'>"))?;
                self.emit(h, "<layerwait parent='back' id='bgh'>".into())?;
            }
            "cutin" | "cutinclear" => {
                // 62E660/62E800: centred cut-in, offset 100 px towards `dir`.
                let [w, hgt] = self.config.canvas.map(|v| v as f32);
                let (mut x, mut y) = (w / 2., hgt / 2.);
                match get(a, "dir").map(str::to_ascii_lowercase).as_deref() {
                    Some("l") => x -= 100.,
                    Some("r") => x += 100.,
                    Some("t") => y -= 100.,
                    Some("b") => y += 100.,
                    _ => {}
                }
                let time = get(a, "time").unwrap_or("").to_owned();
                if name == "cutin" {
                    let src = get(a, "src").context("Malie cutin src")?.to_owned();
                    let size = match self.adv.as_mut().unwrap().picture_size(&src, &mut reader) {
                        Ok(s) => s,
                        Err(_) => {
                            // 62E660: missing picture plays the error system SE.
                            return self.system_se(h, "error");
                        }
                    };
                    self.emit(h, format!("<layer id='cutin' center='{} {}' position='{x} {y}'>", size[0] / 2., size[1] / 2.))?;
                    self.emit(h, format!("<layer id='cutin' src='{src}' position='{} {}' time='{time}'>", w / 2., hgt / 2.))?;
                } else {
                    self.emit(h, format!("<layerclear id='cutin' position='{x} {y}' time='{time}'>"))?;
                }
                if !time.is_empty() {
                    self.emit(h, format!("<wait time='{time}'>"))?;
                }
            }
            "ef" => {
                // 62E8C0: effect frame unless skipping.
                if !self.skipping() {
                    let src = get(a, "src").unwrap_or("").to_owned();
                    self.adv.as_mut().unwrap().effect(&src, now)?;
                }
            }
            "msgframe" => self.msgframe(a)?,
            "char" | "char-setup" => self.character(h, a, &mut reader)?,
            "charclear" => self.char_clear(h, a)?,
            "movie" => {
                // 62E9D0
                let src = get(a, "src").context("Malie movie src")?.to_owned();
                let looping = get(a, "loop").unwrap_or("false").to_owned();
                let cancel = get(a, "cancel").unwrap_or("").to_owned();
                self.emit(h, "<msgframe visibility='hidden'>".into())?;
                self.emit(h, format!("<layer id=\"bg\" parent=\"back\" src=\"<param src-type='movie' src='{src}' width='1024' height='600' loop='{looping}'>\">"))?;
                self.emit(h, "<layerclear parent='chara'>".into())?;
                self.emit(h, "<layerclear>".into())?;
                let mut wait = Attrs::new();
                wait.insert("id".into(), "bg".into());
                wait.insert("parent".into(), "back".into());
                wait.insert("type".into(), "movie".into());
                wait.insert("cancel".into(), cancel);
                self.wait = Wait::Layer(wait);
                self.after_wait = Some("<msgframe visibility='visible'>".into());
            }
            "se" => self.se(h, a)?,
            "sestop" => {
                // 62EF50: stop SE (fade), including SE carried by layers.
                let t = secs(a, "time", 0.)? * 1000.;
                for c in SE_FIRST..SE_FIRST + SE_COUNT {
                    h.submit_audio(self.client, AudioRequest::Stop { channel: c, fade_ms: t })?;
                }
                self.se_channels.clear();
            }
            "shake" => {
                let src = get(a, "src").unwrap_or("stop").to_owned();
                self.adv.as_mut().unwrap().shake(&src, now)?;
            }
            "openurl" => {
                let message = get(a, "message").unwrap_or("").to_owned();
                let href = get(a, "href").unwrap_or("").to_owned();
                self.open_message_box("mb_yesno", &message, MessageBoxPurpose::OpenUrl(href))?;
            }
            "screen" => {
                // 62EE70: adv → 300 then resume; map → 445, other → -1, both
                // unregistered so ChangeScreen does nothing, then the script
                // is suspended (6137D0) until a screen resumes it.
                match get(a, "src").map(str::to_ascii_lowercase).as_deref() {
                    Some("adv") => {
                        self.change_screen(h, 300)?;
                        if matches!(self.wait, Wait::Halted) {
                            self.wait = Wait::None;
                        }
                    }
                    Some("map") => {
                        self.change_screen(h, 445)?;
                        self.wait = Wait::Halted;
                    }
                    _ => self.wait = Wait::Halted,
                }
            }
            "imgblk" => {
                let mask = get(a, "mask").or(get(a, "wipe")).unwrap_or("ol").to_ascii_lowercase();
                let time = get(a, "time").map(adv::seconds).transpose()?.unwrap_or(0);
                self.imgblk = Some((mask, time));
            }
            "/imgblk" => {
                let (mask, time) = self.imgblk.take().context("Malie </imgblk> without <imgblk>")?;
                if time > 0 && !self.skipping() {
                    self.screen_transition(&mask, time, now);
                }
            }
            "voice" => {
                let src = get(a, "src").unwrap_or("").to_owned();
                if src.is_empty() {
                    return Ok(());
                }
                if get(a, "type").is_some_and(|t| t.eq_ignore_ascii_case("system")) {
                    return self.system_voice(h, &src);
                }
                let gain = get(a, "volume").map(float).transpose()?.unwrap_or(1.).clamp(0., 1.);
                self.play_voice(h, &src, gain)?;
            }
            "voicewait" => self.wait = Wait::Channel(VOICE),
            "save" => {
                if let Some(v) = get(a, "enable") {
                    self.tags.save_enabled = v.eq_ignore_ascii_case("true");
                }
            }
            "autosave" => self.request_autosave(),
            "message" => {
                if let Some(c) = get(a, "color") {
                    if c.eq_ignore_ascii_case("user") {
                        self.tags.user_color = true;
                    } else {
                        self.tags.user_color = false;
                        self.tags.text_color = Some(parse_color(c)?);
                    }
                }
            }
            "wbchar" => {
                let (who, face, dress) = split_char(get(a, "src").unwrap_or(""));
                let p = get(a, "position").map(|v| v.split_whitespace().map(float).collect::<Result<Vec<_>>>()).transpose()?.unwrap_or_default();
                let pos = [p.first().copied().unwrap_or(0.), p.get(1).copied().unwrap_or(0.), p.get(2).copied().unwrap_or(0.)];
                self.tags.wb_chars.insert(who, (pos, format!("{face} {dress}").trim().to_owned()));
            }
            "wbclear" => self.tags.wb_chars.clear(),
            "wbblk" => self.tags.wb_block = true,
            "/wbblk" => self.tags.wb_block = false,
            "systemvoice" => {
                let src = get(a, "src").unwrap_or("").to_owned();
                self.system_voice(h, &src)?;
            }
            "systemse" => {
                let src = get(a, "src").unwrap_or("").to_ascii_lowercase();
                if matches!(src.as_str(), "ok" | "cancel" | "focus" | "mouseover" | "setfocus" | "error" | "check" | "page" | "start" | "logo" | "title") {
                    self.system_se(h, &src)?;
                }
            }
            "systembgm" => {
                let src = get(a, "src").unwrap_or("").to_owned();
                self.system_bgm(h, &src)?;
            }
            "automode" => self.tags.automode_showmsg = get(a, "showmsg").is_some_and(|v| v.eq_ignore_ascii_case("true")),
            "textmode" => {
                // 62F7A0 with normal/sync/slow/auto = 0..3.
                if let Some(m) = match get(a, "type").map(str::to_ascii_lowercase).as_deref() {
                    Some("normal") => Some(0),
                    Some("sync") => Some(1),
                    Some("slow") => Some(2),
                    Some("auto") => Some(3),
                    _ => None,
                } {
                    self.tags.text_mode = m;
                }
            }
            "novel" => self.tags.novel = get(a, "enable").is_some_and(|v| v.eq_ignore_ascii_case("true")),
            // 62E5E0: also ends a `s_sceneskip` jump (883B20).
            "chapter" => {
                self.tags.chapter = get(a, "name").unwrap_or("").to_owned();
                if self.jump == Some(Jump::Chapter) {
                    self.stop_jump();
                }
            }
            "cgfilter" | "charfilter" => {
                if let Some(src) = get(a, "src") {
                    let v = if src.eq_ignore_ascii_case("none") { String::new() } else { src.to_owned() };
                    self.set_filter(name == "cgfilter", v);
                }
            }
            "face" => self.face(h, a, &mut reader)?,
            "faceclear" => self.emit(h, "<layerclear parent='front' id='face' time='0.3'>".into())?,
            "clear" => {
                // 630190
                self.emit(h, "<soundstop>".into())?;
                for t in ["<layerclear parent='back'>", "<layerclear parent='chara'>", "<layerclear>", "<layerclear parent='cutin'>", "<layerclear parent='front'>"] {
                    self.emit(h, t.into())?;
                }
                self.adv.as_mut().unwrap().state.chars.clear();
                self.emit(h, "<msgframe src='normal' visibility='hidden'>".into())?;
                self.adv.as_mut().unwrap().state.select = None;
                self.adv.as_mut().unwrap().shake("stop", now)?;
            }
            "scene" => {
                // 62EE50/629CF0: record a replayable scene caption.
                if let Some(c) = get(a, "caption") {
                    if !self.tags.scenes.iter().any(|s| s == c) {
                        self.tags.scenes.push(c.to_owned());
                        self.system_dirty = true;
                    }
                }
            }
            "label" => {
                if let Some(id) = get(a, "id") {
                    self.tags.read_labels.insert(id.to_owned());
                }
            }
            "k" | "await" => self.wait = Wait::Suspended,
            "event" => {
                if matches!(self.wait, Wait::Suspended) {
                    self.wait = Wait::None;
                }
            }
            "textspeed" => {
                if let Some(o) = get(a, "overlap") {
                    self.tags.overlap = if get(a, "time").is_some_and(|t| t.eq_ignore_ascii_case("normal")) { None } else { Some(float(o)?) };
                }
                if let Some(t) = get(a, "time") {
                    self.tags.text_speed = if t.eq_ignore_ascii_case("normal") { None } else { Some((float(t)? * 1000.).max(0.) as u64) };
                }
            }
            "/textspeed" => self.tags.text_speed = None,
            "maliesystem" => {
                for (k, v) in a {
                    self.maliesystem_config(k, v)?;
                }
            }
            "bgv" => {
                // 65EEA0: background voice loop keyed by id (or by voice name).
                let src = get(a, "src").unwrap_or("").to_owned();
                let id = get(a, "id").map(str::to_owned).unwrap_or_else(|| src.clone());
                if src.is_empty() && id.is_empty() {
                    self.tags.bgv.clear();
                    h.submit_audio(self.client, AudioRequest::Stop { channel: SYSTEM_VOICE, fade_ms: 300. })?;
                } else if src.is_empty() {
                    self.tags.bgv.remove(&id);
                    h.submit_audio(self.client, AudioRequest::Stop { channel: SYSTEM_VOICE, fade_ms: 300. })?;
                } else {
                    self.tags.bgv.insert(id, src.clone());
                    let path = Self::voice_path(&src)?;
                    self.audio_ticket(h, SYSTEM_VOICE, &path, AudioParameters { looping: true, ..Default::default() })?;
                }
            }
            // 62DA70 ignores every other tag name.
            _ => {}
        }
        Ok(())
    }
    fn env_layer(&mut self, h: &mut dyn HostServices, a: &Attrs, env: &str) -> Result<()> {
        let id = get(a, "id").unwrap_or("env").to_ascii_lowercase();
        let channel = match self.env_channels.get(&id) {
            Some(c) => *c,
            None => {
                ensure!(self.env_channels.len() < 16, "Malie environment channel budget");
                let c = (ENV_FIRST..ENV_FIRST + 16).find(|c| !self.env_channels.values().any(|v| v == c)).unwrap();
                self.env_channels.insert(id.clone(), c);
                c
            }
        };
        let base = get(a, "volume").map(float).transpose()?.unwrap_or(1.).clamp(0., 1.);
        self.env_sources.insert(id.clone(), (env.to_owned(), base));
        let gain = base * self.volume(Volume::Env);
        self.audio_ticket(h, channel, &format!("data/env/{env}.ogg"), AudioParameters { looping: true, gain, ..Default::default() })
    }
    /// 628D40
    fn sound(&mut self, h: &mut dyn HostServices, a: &Attrs) -> Result<()> {
        if get(a, "type").is_some_and(|t| t.eq_ignore_ascii_case("system")) {
            let src = get(a, "src").unwrap_or("").to_owned();
            return self.system_bgm(h, &src);
        }
        let src = get(a, "src").or(get(a, "name")).unwrap_or("").to_owned();
        let number = src.trim().parse::<i64>().unwrap_or(0);
        let volume = get(a, "volume").map(float).transpose()?;
        let fade = get(a, "fade-time");
        if number <= 0 {
            // Volume-only change with a 0.5 s default fade.
            if let Some(v) = volume {
                let t = fade.map(adv::seconds).transpose()?.unwrap_or(500);
                self.tags.bgm_volume = v.clamp(0., 1.);
                h.submit_audio(self.client, AudioRequest::SetGain { channel: BGM, gain: self.tags.bgm_volume * self.volume(Volume::Bgm), fade_ms: t as f32 })?;
            }
            return Ok(());
        }
        let looping = !get(a, "loop").is_some_and(|v| v.eq_ignore_ascii_case("false"));
        let begin = get(a, "begin-time").map(adv::seconds).transpose()?.unwrap_or(0);
        let fade = fade.map(adv::seconds).transpose()?.unwrap_or(0);
        let lb = get(a, "loop-begin").map(|v| v.trim().parse::<u64>()).transpose()?.unwrap_or(0);
        let le = get(a, "loop-end").map(|v| v.trim().parse::<u64>()).transpose()?.filter(|v| *v > 0);
        self.tags.bgm_volume = volume.unwrap_or(1.).clamp(0., 1.);
        self.start_bgm(h, number as u32, looping, begin, fade, (lb, le))
    }
    pub(super) fn start_bgm(&mut self, h: &mut dyn HostServices, number: u32, looping: bool, begin: u64, fade: u64, loop_points: (u64, Option<u64>)) -> Result<()> {
        let name = if number >= 100 { format!("{number}") } else { format!("{number:02}") };
        let gain = self.tags.bgm_volume * self.volume(Volume::Bgm);
        let mut p = AudioParameters { looping, gain: if fade > 0 { 0. } else { gain }, start_ms: begin, ..Default::default() };
        p.loop_start = loop_points.0;
        p.loop_end = loop_points.1;
        self.audio_ticket(h, BGM, &format!("data/bgm/{name}.ogg"), p)?;
        if fade > 0 {
            h.submit_audio(self.client, AudioRequest::SetGain { channel: BGM, gain, fade_ms: fade as f32 })?;
        }
        self.tags.bgm = Some(name);
        self.tags.bgm_loop = loop_points;
        self.tags.bgm_started = h.now_ms();
        self.bgm = Some(number);
        self.music_heard.insert(number);
        self.system_dirty = true;
        Ok(())
    }
    /// 627D40: numeric SE ids use `%02d.ogg` (from 100 `%d.ogg`); names ending
    /// in a letter play `data/se/<name>.ogg`.
    fn se(&mut self, h: &mut dyn HostServices, a: &Attrs) -> Result<()> {
        let src = get(a, "src").or(get(a, "sr")).unwrap_or("").to_ascii_lowercase();
        let looping = get(a, "loop").or(get(a, "roop")).is_some_and(|v| v.eq_ignore_ascii_case("true"));
        let volume = get(a, "volume").map(float).transpose()?;
        let fade = secs(a, "fade-time", 0.)?;
        let lettered = src.len() > 2 && src.chars().last().is_some_and(|c| c.is_ascii_alphabetic());
        let number = src.parse::<i64>().unwrap_or(0);
        if !lettered && number <= 0 {
            match volume {
                Some(v) => {
                    for c in SE_FIRST..SE_FIRST + SE_COUNT {
                        h.submit_audio(self.client, AudioRequest::SetGain { channel: c, gain: v.clamp(0., 1.) * self.volume(Volume::Se), fade_ms: fade * 1000. })?;
                    }
                }
                None => self.adv.as_mut().unwrap().gap(format!("SE error {src:?}")),
            }
            return Ok(());
        }
        if self.skipping() && !looping {
            return Ok(());
        }
        let file = if lettered { src.clone() } else if number >= 100 { number.to_string() } else { format!("{number:02}") };
        let gain = volume.unwrap_or(1.).clamp(0., 1.) * self.volume(Volume::Se);
        let channel = SE_FIRST + self.se_next % SE_COUNT;
        self.se_next += 1;
        self.se_channels.retain(|_, c| *c != channel);
        self.se_channels.insert(src.clone(), channel);
        let delay = get(a, "delay").map(adv::seconds).transpose()?.unwrap_or(0);
        let p = AudioParameters { looping, gain, ..Default::default() };
        if delay > 0 {
            self.delayed_se.push((h.now_ms() + delay, channel, format!("data/se/{file}.ogg"), looping, gain));
            return Ok(());
        }
        self.audio_ticket(h, channel, &format!("data/se/{file}.ogg"), p)
    }
    /// 62EAA0
    fn msgframe(&mut self, a: &Attrs) -> Result<()> {
        if let Some(t) = get(a, "type") {
            self.tags.msg_type = match t.to_ascii_lowercase().as_str() {
                "wb" | "wordballoon" => 1,
                "normal" => 0,
                "auto" => 2,
                "bbs" => 3,
                _ => self.tags.msg_type,
            };
        }
        if let Some(d) = get(a, "dir") {
            self.tags.msg_dir = match d.to_ascii_lowercase().as_str() {
                "h" | "horizontal" => 0,
                "v" | "vertical" => 1,
                _ => self.tags.msg_dir,
            };
        }
        let adv = self.adv.as_mut().unwrap();
        if let Some(src) = get(a, "src") {
            adv.state.frame.src = src.to_ascii_lowercase();
        }
        // 648C30: visible/true/show = 1, hidden/false = 0, none = 2, other 3.
        if let Some(v) = get(a, "visibility") {
            match v.to_ascii_lowercase().as_str() {
                "visible" | "true" | "show" => adv.state.frame.visible = true,
                "hidden" | "false" | "none" => adv.state.frame.visible = false,
                _ => {}
            }
        }
        if let Some(f) = get(a, "font") {
            self.tags.font = f.trim().parse().ok();
        }
        let size = |v: &str| if v.eq_ignore_ascii_case("normal") { None } else { v.trim().parse::<i32>().ok() };
        if let Some(f) = get(a, "fontsize") {
            self.tags.font_size = size(f);
        }
        if let Some(f) = get(a, "wb-fontsize") {
            self.tags.wb_font_size = size(f);
        }
        if let Some(r) = get(a, "ruby") {
            if r.eq_ignore_ascii_case("true") || r.eq_ignore_ascii_case("false") {
                self.tags.ruby = r.eq_ignore_ascii_case("true");
            }
        }
        if let Some(o) = get(a, "opacity") {
            let adv = self.adv.as_mut().unwrap();
            adv.state.frame.opacity = if o.eq_ignore_ascii_case("user") { self.settings.window_opacity } else { float(o)?.clamp(0., 1.) };
        }
        Ok(())
    }
    /// 6470A0 + 6464D0: register/update the character, then express it as a
    /// `chara`-root layer.
    fn character(&mut self, h: &mut dyn HostServices, a: &Attrs, read: &mut adv::Read<'_>) -> Result<()> {
        let (who, mut face, mut dress) = match get(a, "src") {
            Some(src) => split_char(src),
            None => (
                get(a, "na").or(get(a, "name")).unwrap_or("").to_ascii_lowercase(),
                get(a, "ex").or(get(a, "exp")).unwrap_or("").to_ascii_lowercase(),
                get(a, "dr").or(get(a, "dress")).unwrap_or("").to_ascii_lowercase(),
            ),
        };
        if who.is_empty() {
            return Ok(());
        }
        let now = h.now_ms();
        for key in ["wb-pos", "wordballoon-pos", "wb-src", "wb-camera", "wb-name", "wb-char", "msg-type", "msg-dir", "emote-neutral"] {
            if let Some(v) = get(a, key) {
                self.tags.wb_chars.entry(who.clone()).or_insert(([0.; 3], String::new())).1 = format!("{key}={v}");
            }
        }
        let fresh = !self.adv.as_ref().unwrap().state.chars.contains_key(&who);
        let rec = self.adv.as_ref().unwrap().state.chars.get(&who).cloned().unwrap_or(adv::CharRecord { opacity: 1., ..Default::default() });
        if face.is_empty() {
            face = if rec.face.is_empty() { "none".into() } else { rec.face.clone() };
        }
        if dress.is_empty() {
            dress = if rec.dress.is_empty() { "def".into() } else { rec.dress.clone() };
        }
        let mut rec = rec;
        if let Some(o) = get(a, "opacity") {
            rec.opacity = float(o)?;
        }
        if let Some(f) = get(a, "filter") {
            rec.filter = if f.eq_ignore_ascii_case("none") { String::new() } else { f.to_owned() };
        }
        // 646CA0: position/pos, r-pos; otherwise place slot.
        let mut pos = rec.position;
        let mut positioned = false;
        if let Some(p) = get(a, "position").or(get(a, "pos")) {
            let v: Vec<f32> = p.split(|c: char| c == ',' || c.is_whitespace()).filter(|v| !v.is_empty()).map(float).collect::<Result<_>>()?;
            for (i, x) in v.iter().take(3).enumerate() {
                pos[i] = *x;
            }
            positioned = true;
        }
        if let Some(p) = get(a, "r-pos") {
            let v: Vec<f32> = p.split_whitespace().map(float).collect::<Result<_>>()?;
            for (i, x) in v.iter().take(3).enumerate() {
                pos[i] += *x;
            }
            positioned = true;
        }
        if !positioned {
            let slot = get(a, "pl").or(get(a, "place")).and_then(|p| PLACES.iter().position(|(n, _)| p.eq_ignore_ascii_case(n))).or(rec.place);
            if let Some(i) = slot {
                // Another character in the slot leaves first.
                let other = self.adv.as_ref().unwrap().state.chars.iter().find(|(n, r)| **n != who && r.place == Some(i) && r.shown).map(|(n, _)| n.clone());
                if let Some(other) = other {
                    self.emit(h, format!("<charclear src='{other}' time='0.2'>"))?;
                    self.emit(h, "<wait time='0.2'>".into())?;
                }
                rec.place = Some(i);
                pos = PLACES[i].1;
            }
        }
        rec.position = pos;
        rec.face = face.clone();
        rec.dress = dress.clone();
        let time = match get(a, "tm") {
            Some(t) => t.trim().parse::<f32>().unwrap_or(0.) / 1000.,
            None => secs(a, "time", 0.5)?,
        };
        let accel = get(a, "accel").and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
        let mask = get(a, "mask-src").map(|m| format!("mask?src={m}"));
        let wipe = get(a, "wipe-src").map(str::to_owned).or(mask).unwrap_or_default();
        let shake = get(a, "shake-src").unwrap_or("").to_owned();
        let fliph = get(a, "fliph").unwrap_or("").to_owned();
        let style = get(a, "transition-style").unwrap_or("").to_owned();
        let [x, y, z] = pos;
        let mut accel_out = accel;
        if fresh || !rec.shown {
            let size = self.adv.as_mut().unwrap().char_picture_size(&who, &dress, &face, read)?.unwrap_or([0., 0.]);
            let (mut sx, mut sy) = (x, y);
            match direction(get(a, "dir")) {
                Some(0) => sx += 100.,
                Some(1) => sx -= 100.,
                Some(2) => sy += 100.,
                Some(3) => sy -= 100.,
                _ => {}
            }
            if direction(get(a, "dir")).is_some() {
                accel_out = -1;
            }
            self.emit(h, format!("<layer parent='chara' id='{who}' position='{sx} {sy} {z}' center='{} {}' fliph='{fliph}' opacity='0'>", size[0] * 0.5, size[1]))?;
        }
        rec.shown = face != "none";
        self.adv.as_mut().unwrap().state.chars.insert(who.clone(), rec.clone());
        let char_src = if face == "none" { String::new() } else { format!(" char-src='{who} {face} {dress}'") };
        let opacity = if face == "none" { 0. } else { rec.opacity };
        let _ = now;
        self.emit(h, format!("<layer parent='chara' type='char' id='{who}'{char_src} wipe-src='{wipe}' shake-src='{shake}' position='{x} {y} {z}' fliph='{fliph}' opacity='{opacity}' time='{time}s' accel='{accel_out}' transition-style='{style}'>"))?;
        if let Some(layer) = self.adv.as_mut().unwrap().state.layers.get_mut(&(adv::Group::Chara, who.clone())) {
            layer.filter = rec.filter.clone();
        }
        // Emotion overlays (em-type/em-src/em-pos) use screen/emotion SVGs.
        if let Some(src) = get(a, "em-src") {
            let pos = get(a, "em-pos").map(|p| p.split_whitespace().map(float).collect::<Result<Vec<_>>>()).transpose()?.unwrap_or_default();
            let em_pos = [pos.first().copied().unwrap_or(0.), pos.get(1).copied().unwrap_or(0.), pos.get(2).copied().unwrap_or(0.)];
            let kind = get(a, "em-type").unwrap_or("").to_owned();
            self.adv.as_mut().unwrap().emotion(&who, src, &kind, em_pos, now, read)?;
        }
        Ok(())
    }
    /// 647850/6461C0
    fn char_clear(&mut self, h: &mut dyn HostServices, a: &Attrs) -> Result<()> {
        let who = match get(a, "src") {
            Some(src) => Some(split_char(src).0),
            None => get(a, "na").or(get(a, "name")).map(str::to_ascii_lowercase),
        };
        let time = match get(a, "tm") {
            Some(t) => t.trim().parse::<f32>().unwrap_or(0.) / 1000.,
            None => secs(a, "time", 0.5)?,
        };
        let dir = direction(get(a, "dir"));
        let mask = get(a, "mask-src").map(|m| format!("mask?src={m}"));
        let wipe = get(a, "wipe-src").map(str::to_owned).or(mask).unwrap_or_default();
        let style = get(a, "transition-style").unwrap_or("").to_owned();
        let names: Vec<String> = match &who {
            Some(w) => vec![w.clone()],
            None => self.adv.as_ref().unwrap().state.chars.keys().cloned().collect(),
        };
        for n in names {
            let Some(mut rec) = self.adv.as_ref().unwrap().state.chars.get(&n).cloned() else { continue };
            let [mut x, mut y, z] = rec.position;
            match dir {
                Some(0) => x -= 100.,
                Some(1) => x += 100.,
                Some(2) => y -= 100.,
                Some(3) => y += 100.,
                _ => {}
            }
            let accel = i32::from(dir.is_some());
            self.emit(h, format!("<layerclear parent='chara' id='{n}' wipe-src='{wipe}' position='{x} {y} {z}' time='{time}s' accel='{accel}' transition-style='{style}'>"))?;
            rec.shown = false;
            self.adv.as_mut().unwrap().state.chars.insert(n.clone(), rec);
            self.adv.as_mut().unwrap().clear_emotion(&n);
        }
        Ok(())
    }
    /// 634B80: face window icon on the `front` root.
    fn face(&mut self, h: &mut dyn HostServices, a: &Attrs, read: &mut adv::Read<'_>) -> Result<()> {
        let (who, mut face, mut dress) = split_char(get(a, "src").unwrap_or(""));
        let Some(rec) = self.adv.as_ref().unwrap().state.chars.get(&who).cloned() else {
            self.adv.as_mut().unwrap().gap(format!("face error: {who}"));
            return Ok(());
        };
        if dress.is_empty() {
            dress = rec.dress.clone();
        }
        if face.is_empty() {
            face = rec.face.clone();
        }
        let Some(size) = self.adv.as_mut().unwrap().char_picture_size(&who, &dress, &face, read)? else {
            return Ok(());
        };
        // The face box comes from the message frame's `face` element (scaled 0.533333).
        let rect = self.adv.as_ref().unwrap().face_rect;
        let scale = rect[2] / size[0].max(1.);
        self.emit(h, format!("<layer parent='front' id='face' position='{} {}' scale='{scale}'>", rect[0], rect[1]))?;
        self.emit(h, format!("<layer parent='front' id='face' char-src='{who} {face} {dress}' time='0.3'>"))
    }
    fn begin_transition(&mut self, a: &Attrs, time: f32) {
        // 630590: transitions are skipped while skipping/fast-forwarding.
        if !self.skipping() && time > 0. {
            let mask = get(a, "mask").or(get(a, "wipe")).unwrap_or("ol").to_ascii_lowercase();
            self.imgblk = Some((mask, (time * 1000.) as u64));
        }
    }
    fn end_transition(&mut self, now: u64) {
        if let Some((mask, time)) = self.imgblk.take() {
            self.screen_transition(&mask, time, now);
        }
    }
    pub(super) fn screen_transition(&mut self, mask: &str, time: u64, now: u64) {
        let effect = if mask == "ol" || mask.is_empty() {
            json!({"kind":"crossfade"})
        } else {
            json!({"kind":"mask","asset":format!("data/picture/mask/{mask}.png"),"vague":64})
        };
        self.transition = Some((json!({"op":"transition","transition":{"effect":effect,"duration_ms":time}}), now + time));
        self.wait = Wait::Until(now + time);
    }
    fn set_filter(&mut self, cg: bool, v: String) {
        if cg {
            self.tags.cg_filter = v.clone();
        } else {
            self.tags.char_filter = v.clone();
        }
        let adv = self.adv.as_mut().unwrap();
        if cg {
            adv.state.cg_filter = v;
        } else {
            adv.state.char_filter = v;
        }
    }
    /// 4F6D30
    pub(super) fn maliesystem_config(&mut self, key: &str, value: &str) -> Result<()> {
        match key.to_ascii_lowercase().as_str() {
            "textcolor" => self.tags.text_color = Some(parse_color(value)?),
            "cgfilter" => self.set_filter(true, value.to_owned()),
            "charfilter" => self.set_filter(false, value.to_owned()),
            "extra-debug" => self.tags.extra_debug = value.eq_ignore_ascii_case("true"),
            "draw-debug" => self.tags.draw_debug = value.eq_ignore_ascii_case("true"),
            "auto-click" => self.tags.auto_click = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
        Ok(())
    }
    /// 627B20: system SE table (`[SYSTEMSE]` in malie.ini).
    pub(super) fn system_se(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        let Some(path) = self.ini("systemse", name).map(str::to_owned) else { return Ok(()) };
        let path = path.trim_start_matches(".\\").replace('\\', "/");
        if path.is_empty() {
            return Ok(());
        }
        let gain = self.volume(Volume::SystemSe);
        if let Err(e) = self.audio_ticket(h, SYSTEM_SE, &path, AudioParameters { gain, ..Default::default() }) {
            self.adv.as_mut().unwrap().gap(format!("system SE {name}: {e:#}"));
        }
        Ok(())
    }
    /// 627B20 for voices: systemvoice.csv, else the native fallbacks.
    pub(super) fn system_voice(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        if let Some(v) = self.system_voices.get(&name.to_ascii_lowercase()).cloned() {
            let path = Self::voice_path(&v)?;
            return self.audio_ticket(h, VOICE, &path, AudioParameters { gain: self.volume(Volume::Voice), ..Default::default() });
        }
        match name.to_ascii_lowercase().as_str() {
            "logo" | "title" | "end2" | "save2" | "qsave" | "title2" | "end" | "save" | "qs" => Ok(()),
            "test" => {
                let Some(v) = self.ini("config", "voicetest").map(str::to_owned) else { return Ok(()) };
                let path = Self::voice_path(&v)?;
                self.audio_ticket(h, VOICE, &path, AudioParameters { gain: self.volume(Volume::Voice), ..Default::default() })
            }
            _ => self.system_se(h, "ok"),
        }
    }
    /// 627AD0: `[SYSTEMBGM]` name; restarts only on change.
    pub(super) fn system_bgm(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        let Some(stem) = self.ini("systembgm", name).map(str::to_owned) else { return Ok(()) };
        let Ok(n) = stem.trim().parse::<u32>() else { return Ok(()) };
        if n == 0 || self.bgm == Some(n) {
            return Ok(());
        }
        self.tags.bgm_volume = 1.;
        self.start_bgm(h, n, true, 0, 0, (0, None))
    }
    pub(super) fn play_voice(&mut self, h: &mut dyn HostServices, name: &str, gain: f32) -> Result<()> {
        let path = Self::voice_path(name)?;
        let who = name.to_ascii_lowercase().trim_start_matches("v_").chars().take_while(|c| c.is_ascii_alphabetic()).collect::<String>();
        if !self.voice_enabled(&who) {
            return Ok(());
        }
        let gain = gain * self.volume(Volume::Voice) * self.character_volume(&who);
        self.last_voice = Some(name.to_owned());
        if let Err(e) = self.audio_ticket(h, VOICE, &path, AudioParameters { gain, ..Default::default() }) {
            self.adv.as_mut().unwrap().gap(format!("voice {e:#}"));
        }
        Ok(())
    }
}
/// 647B30: `#rrggbb` or a named colour.
pub(super) fn parse_color(v: &str) -> Result<[u8; 3]> {
    let t = v.trim().to_ascii_lowercase();
    Ok(match t.as_str() {
        "white" => [255, 255, 255],
        "black" => [0, 0, 0],
        "red" => [255, 0, 0],
        "green" => [0, 128, 0],
        "blue" => [0, 0, 255],
        "yellow" => [255, 255, 0],
        "gray" | "grey" => [128, 128, 128],
        _ => {
            let hex = t.strip_prefix('#').or(t.strip_prefix("0x")).context("Malie colour")?;
            ensure!(hex.len() == 6, "Malie colour {v:?}");
            let n = u32::from_str_radix(hex, 16)?;
            [(n >> 16) as u8, (n >> 8) as u8, n as u8]
        }
    })
}
