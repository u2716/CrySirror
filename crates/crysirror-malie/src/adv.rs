// SPDX-License-Identifier: MPL-2.0
//! Malie ADV presentation policy: the native `layer`, `layercamera`, `char`,
//! `msgframe` and message-window state, mapped onto shared retained layers.
//!
//! Geometry (native 6427B0/640D10/63F4C0): each layer owns `position`,
//! `center`, `scale`, `rotate` and `opacity` tracks; a timed write starts at
//! the previous *target* and eases with the `accel` curve to the new target.
//! `back`/`chara` layers are camera-relative planes: the default camera sits
//! at z=-300 with a 90° vertical field of view, so a plane at depth d appears
//! scaled by (H/2)/tan(fov/2)/d about the canvas centre. Planes parallel to
//! the image plane remain exactly affine, so each is a 2D retained transform.
//! `cutin` layers use canvas pixel coordinates (movie layers place
//! `pos='512 300'` with their own centre to fill the 1024x600 canvas).
use crate::{
    message::{self, Token},
    picture::{self, Paint, Picture},
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_scene::layer::Easing;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Native layer roots (50B880): `back`, `chara`, `cutin`, `front`, `ui`;
/// any other parent selects the unnamed default root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Group {
    Back,
    Chara,
    Default,
    Cutin,
    Front,
    Ui,
}
impl Group {
    fn parse(text: Option<&str>) -> Result<Self> {
        Ok(match text.map(str::to_ascii_lowercase).as_deref() {
            Some("back") => Self::Back,
            Some("chara") => Self::Chara,
            Some("cutin") => Self::Cutin,
            Some("front") => Self::Front,
            Some("ui") => Self::Ui,
            _ => Self::Default,
        })
    }
    fn major(self) -> i32 {
        match self {
            Self::Back => 10,
            Self::Chara => 20,
            Self::Default => 25,
            Self::Cutin => 30,
            Self::Front => 35,
            Self::Ui => 46,
        }
    }
    fn perspective(self) -> bool {
        matches!(self, Self::Back | Self::Chara)
    }
}
/// Native `accel` table (6401A0) selects an interpolation curve; the shared
/// easing families reproduce acceleration (+), deceleration (-) and linear.
fn easing(accel: i32) -> Easing {
    match accel {
        -1 | -2 => Easing::EaseOut,
        -3 | -4 => Easing::EaseOutQuart,
        1 => Easing::EaseIn,
        2 => Easing::Smooth,
        4 => Easing::EaseInQuart,
        _ => Easing::Linear,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    from: [f32; 3],
    to: [f32; 3],
    start: u64,
    duration: u64,
    accel: i32,
}
impl Track {
    fn new(v: [f32; 3]) -> Self {
        Self { from: v, to: v, start: 0, duration: 0, accel: 0 }
    }
    pub fn at(&self, now: u64) -> [f32; 3] {
        if self.duration == 0 || now >= self.start + self.duration {
            return self.to;
        }
        let p = now.saturating_sub(self.start) as f32 / self.duration as f32;
        let p = easing(self.accel).apply(p);
        [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * p)
    }
    /// 63F4C0: a timed write starts at the old target, an untimed one jumps.
    fn set(&mut self, v: [f32; 3], now: u64, duration: u64, accel: i32) {
        self.from = if duration > 0 { self.to } else { v };
        self.to = v;
        self.start = now;
        self.duration = duration;
        self.accel = accel;
    }
    fn busy(&self, now: u64) -> bool {
        self.duration > 0 && now < self.start + self.duration
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Visual {
    pub picture: Option<Picture>,
    /// Outgoing picture during a timed source change: (picture, start, ms).
    pub previous: Option<(Picture, u64, u64)>,
    pub position: Track,
    pub center: Track,
    pub scale: Track,
    pub rotate: Track,
    pub opacity: Track,
    pub fliph: bool,
    pub created: u64,
    /// Remove after the opacity fade completes (`layerclear time=`).
    pub closing: bool,
    pub movie_started: u64,
    /// When the current picture was assigned (animation clock origin).
    #[serde(default)]
    pub shown_at: u64,
    /// `filter='col|sil <color> <percent>'` (645F10).
    #[serde(default)]
    pub filter: String,
}
impl Visual {
    fn new(created: u64, opacity: f32) -> Self {
        Self {
            picture: None,
            previous: None,
            position: Track::new([0.; 3]),
            center: Track::new([0.; 3]),
            scale: Track::new([1., 1., 1.]),
            rotate: Track::new([0.; 3]),
            opacity: Track::new([opacity, 0., 0.]),
            fliph: false,
            created,
            closing: false,
            movie_started: 0,
            shown_at: created,
            filter: String::new(),
        }
    }
    fn busy(&self, now: u64) -> bool {
        [&self.position, &self.center, &self.scale, &self.rotate, &self.opacity].iter().any(|t| t.busy(now))
            || self.previous.as_ref().is_some_and(|(_, s, d)| now < s + d)
            || self.picture.as_ref().is_some_and(|p| {
                // A finite exported animation is awaited like a tween.
                let end = p.elements.iter().filter_map(|e| e.animated.as_ref()).flat_map(|a| &a.motions).filter(|m| !m.repeat).map(|m| m.duration_ms).max();
                end.is_some_and(|d| now < self.shown_at + d)
            })
    }
}
/// Native character registry entry (644E00/645xxx): remembered expression,
/// dress, position, place slot, opacity and filter between `char` tags.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CharRecord {
    pub face: String,
    pub dress: String,
    pub position: [f32; 3],
    pub opacity: f32,
    pub filter: String,
    pub place: Option<usize>,
    pub shown: bool,
    pub emotion: Option<(String, String, [f32; 3])>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub position: Track,
    pub rotate: Track,
    pub fov: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Shake {
    pub amp: f32,
    pub period: u64,
    pub until: Option<u64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Page {
    pub tokens: Vec<Token>,
    /// Glyph count already visible before the current `_ms_out`.
    pub shown: usize,
    pub reveal_start: u64,
    pub complete: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub src: String,
    pub visible: bool,
    pub opacity: f32,
    pub user_color: bool,
}
/// Persistent ADV state; the document caches and transmitted-layer diff are
/// reconstructed after a restore.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    #[serde(with = "pairs")]
    pub layers: BTreeMap<(Group, String), Visual>,
    pub chars: BTreeMap<String, CharRecord>,
    pub camera: Camera,
    pub frame: Frame,
    pub page: Page,
    pub name: String,
    pub segments: Vec<Vec<Token>>,
    pub segment: usize,
    pub shake: Option<Shake>,
    pub chapter: String,
    #[serde(default)]
    pub select: Option<Select>,
    #[serde(default)]
    pub cg_filter: String,
    #[serde(default)]
    pub char_filter: String,
    #[serde(default)]
    pub effect: Option<Effect>,
    /// Emotion icons per character: (picture, offset, shown at).
    #[serde(default)]
    pub emotions: BTreeMap<String, (Picture, [f32; 3], u64)>,
    /// Camera 1 of the native split view and its splitter line.
    #[serde(default)]
    pub split: Option<Split>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Split {
    pub camera: Camera,
    pub visible: bool,
    pub position: [f32; 2],
    pub angle: Track,
}
/// EFFECT_ef (533BF0): Tweener sequences over a screen snapshot. Each step
/// is (property, from, to, delay ms, duration ms, curve).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub start: u64,
    pub blend: String,
    pub steps: Vec<(String, f32, f32, u64, u64, i32)>,
}
/// Native choice list (63AB10 add, 63AC10 show); drawn with select.svg.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Select {
    pub items: Vec<(u32, String)>,
    pub shown: bool,
    pub focus: Option<usize>,
}
/// Tuple-keyed maps serialize as pair lists (JSON object keys are strings).
mod pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;
    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(m: &BTreeMap<K, V>, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(m.iter())
    }
    pub fn deserialize<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<K, V>, D::Error> {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}
pub struct Adv {
    /// The user-hidden message window (7DC458, shortcut `hide`, PSP_HIDE).
    pub window_hidden: bool,
    /// The ADV screen (300) is not the active screen, or a jump hides it.
    pub hidden: bool,
    /// Screen 12346 (the TakeScreen capture) is shown: the ADV picture holds.
    pub frozen: bool,
    pub canvas: [f32; 2],
    pub state: State,
    pictures: BTreeMap<String, Option<Picture>>,
    ids: BTreeMap<String, u32>,
    next_id: u32,
    sent: BTreeMap<u32, Value>,
    pub gaps: BTreeSet<String>,
    pub text_interval_ms: u64,
    pub font: String,
    pub font_size: f32,
    pub text_rect: [f32; 4],
    pub name_rect: [f32; 4],
    pub frame_layers: Vec<Value>,
    pub frame_present: bool,
    /// Movie streams the current frame draws: element key -> (asset, loop).
    pub movies_wanted: BTreeMap<String, (String, bool)>,
    emit_root: u32,
    pub seen_pictures: Vec<String>,
    pub seen_movies: Vec<String>,
    /// Face-window rectangle (canvas) from the message frame's `face` element.
    pub face_rect: [f32; 4],
    pub text_color: [u8; 4],
    /// Started shared-media players: element key -> (asset, frame surface).
    pub movie_surfaces: BTreeMap<String, (String, String)>,
}
pub const ROOT: u32 = 100_000;
const SPLIT: u32 = ROOT + 2;
pub type Read<'a> = dyn FnMut(&str) -> Result<Option<String>> + 'a;
pub type Advance<'a> = dyn Fn(char, f32) -> f32 + 'a;

fn floats(text: &str) -> Result<Vec<f32>> {
    text.split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|v| !v.is_empty())
        .map(|v| {
            let v = v.trim_end_matches(|c: char| c.is_ascii_alphabetic());
            v.parse::<f32>()
                .ok()
                .filter(|v| v.is_finite() && v.abs() < 1.0e7)
                .with_context(|| format!("Malie numeric attribute {text:?}"))
        })
        .collect()
}
/// Native time attributes accept `0.5`, `0.5s` and `500ms`.
pub fn seconds(text: &str) -> Result<u64> {
    let t = text.trim().to_ascii_lowercase();
    let (n, scale) = if let Some(v) = t.strip_suffix("ms") {
        (v, 1.)
    } else if let Some(v) = t.strip_suffix('s') {
        (v, 1000.)
    } else {
        (t.as_str(), 1000.)
    };
    let v = n.trim().parse::<f64>().with_context(|| format!("Malie time {text:?}"))?;
    ensure!(v.is_finite() && (0. ..=3600.0 * 1000.).contains(&(v * scale)), "Malie time range");
    Ok((v * scale) as u64)
}
fn vec3(text: &str, base: [f32; 3]) -> Result<[f32; 3]> {
    let v = floats(text)?;
    ensure!((1..=3).contains(&v.len()), "Malie vector attribute {text:?}");
    let mut out = base;
    out[..v.len()].copy_from_slice(&v);
    Ok(out)
}
pub type Attrs = BTreeMap<String, String>;
fn attr<'a>(a: &'a Attrs, k: &str) -> Option<&'a str> {
    a.get(k).map(String::as_str).filter(|v| !v.is_empty())
}

impl Adv {
    pub fn new(canvas: [f32; 2], font: String, font_size: f32) -> Self {
        Self {
            canvas,
            state: State {
                layers: BTreeMap::new(),
                chars: BTreeMap::new(),
                camera: Camera { position: Track::new([0., 0., -300.]), rotate: Track::new([0.; 3]), fov: 90. },
                frame: Frame { src: "normal".into(), visible: false, opacity: 0.8, user_color: false },
                page: Page::default(),
                name: String::new(),
                segments: Vec::new(),
                segment: 0,
                shake: None,
                chapter: String::new(),
                select: None,
                cg_filter: String::new(),
                char_filter: String::new(),
                effect: None,
                emotions: BTreeMap::new(),
                split: None,
            },
            pictures: BTreeMap::new(),
            ids: BTreeMap::new(),
            next_id: ROOT + 16,
            sent: BTreeMap::new(),
            gaps: BTreeSet::new(),
            text_interval_ms: 30,
            font,
            font_size,
            text_rect: [150., 434., 722., 114.],
            name_rect: [193., 407., 232., 19.],
            frame_layers: Vec::new(),
            frame_present: false,
            movies_wanted: BTreeMap::new(),
            emit_root: ROOT,
            window_hidden: false,
            hidden: false,
            frozen: false,
            seen_pictures: Vec::new(),
            seen_movies: Vec::new(),
            face_rect: [150., 420., 120., 120.],
            text_color: [255, 255, 255, 255],
            movie_surfaces: BTreeMap::new(),
        }
    }
    fn id(&mut self, key: &str) -> Result<u32> {
        if let Some(id) = self.ids.get(key) {
            return Ok(*id);
        }
        ensure!(self.next_id < ROOT + 400_000, "Malie retained layer id budget");
        let id = self.next_id;
        self.next_id += 1;
        self.ids.insert(key.to_owned(), id);
        Ok(id)
    }
    pub fn gap(&mut self, what: String) {
        if self.gaps.insert(what.clone()) {
            log::warn!("Malie capability gap: {what}");
        }
    }
    /// Load and cache a picture by layer source, trying the native folders.
    pub fn picture(&mut self, src: &str, read: &mut Read<'_>) -> Result<Picture> {
        if src.trim_start().starts_with('<') {
            return self.inline_picture(src, read);
        }
        let (name, params) = picture::split_source(src)?;
        let key = format!("{name}?{params:?}");
        if let Some(p) = self.pictures.get(&key) {
            return p.clone().with_context(|| format!("Malie picture {src:?} is absent"));
        }
        let mut found = None;
        for dir in picture::DIRECTORIES {
            if let Some(text) = read(&format!("{dir}/{name}.svg"))? {
                found = Some(picture::parse(&name, dir, &text, &params)?);
                break;
            }
        }
        ensure!(self.pictures.len() < 4096, "Malie picture cache budget");
        self.pictures.insert(key, found.clone());
        found.with_context(|| format!("Malie picture {src:?} is absent"))
    }
    /// `src='<param src-type="movie" src=name width= height= loop=>'`.
    fn inline_picture(&mut self, src: &str, read: &mut Read<'_>) -> Result<Picture> {
        let tags = parse_tags(src)?;
        let [(tag, a)] = tags.as_slice() else { bail!("Malie inline picture {src:?}") };
        ensure!(tag == "param", "Malie inline picture <{tag}>");
        let kind = attr(a, "src-type").context("Malie inline picture type")?.to_ascii_lowercase();
        ensure!(kind == "movie", "Malie inline picture type {kind}");
        let name = attr(a, "src").context("Malie inline movie source")?.to_ascii_lowercase();
        ensure!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "Malie inline movie name {name:?}");
        let w = attr(a, "width").map(|v| floats(v)).transpose()?.and_then(|v| v.first().copied()).unwrap_or(1920.);
        let h = attr(a, "height").map(|v| floats(v)).transpose()?.and_then(|v| v.first().copied()).unwrap_or(1080.);
        let looping = attr(a, "loop").is_some_and(|v| v.eq_ignore_ascii_case("true"));
        let mut path = None;
        for dir in ["data/picture/event", "data/picture/event/movie", "data/picture/effect", "data/picture"] {
            let candidate = format!("{dir}/{name}.webm");
            if read(&candidate)?.is_some() {
                path = Some(candidate);
                break;
            }
        }
        let path = path.with_context(|| format!("Malie movie {name:?} is absent"))?;
        Ok(Picture {
            name,
            size: [w, h],
            elements: vec![picture::Element { rect: [0., 0., w, h], paint: Paint::Movie(path, looping), opacity: 1., additive: false, animated: None }],
        })
    }
    fn char_picture(&mut self, who: &str, pose: &str, face: &str, read: &mut Read<'_>) -> Result<Option<Picture>> {
        if face == "none" {
            return Ok(None);
        }
        let key = format!("chara/{who}/{pose}/{face}");
        if let Some(p) = self.pictures.get(&key) {
            return Ok(p.clone());
        }
        let dir = format!("data/picture/chara/{who}/{pose}");
        let found = match read(&format!("{dir}/{face}.svg"))? {
            Some(text) => Some(picture::parse(face, &dir, &text, &BTreeMap::new())?),
            None => bail!("Malie character picture {who}/{pose}/{face} is absent"),
        };
        self.pictures.insert(key, found.clone());
        Ok(found)
    }
    fn apply_geometry(v: &mut Visual, a: &Attrs, now: u64, duration: u64, accel: i32, chara: bool) -> Result<()> {
        // 646CA0: `position` or `pos`, then additive `r-pos`.
        let mut position = None;
        if let Some(p) = attr(a, "position").or(attr(a, "pos")) {
            position = Some(vec3(p, v.position.to)?);
        }
        if let Some(r) = attr(a, "r-pos") {
            let r = vec3(r, [0.; 3])?;
            let base = position.unwrap_or(v.position.to);
            position = Some([base[0] + r[0], base[1] + r[1], base[2] + r[2]]);
        }
        if let Some(p) = position {
            v.position.set(p, now, duration, accel);
        }
        if let Some(c) = attr(a, "center").or(attr(a, "centar")) {
            let c = vec3(c, [0.; 3])?;
            v.center.set(c, now, duration, accel);
        }
        if let Some(s) = attr(a, "scale") {
            let s = floats(s)?;
            ensure!(!s.is_empty() && s.len() <= 3, "Malie scale");
            let sy = *s.get(1).unwrap_or(&s[0]);
            v.scale.set([s[0], sy, 1.], now, duration, accel);
        }
        if let Some(r) = attr(a, "rotate") {
            let r = vec3(r, [0.; 3])?;
            v.rotate.set(r, now, duration, accel);
        }
        if let Some(o) = attr(a, "opacity") {
            let o = if o.eq_ignore_ascii_case("user") { 1. } else { floats(o)?.first().copied().context("Malie opacity")? };
            v.opacity.set([o.clamp(0., 1.), 0., 0.], now, duration, accel);
        }
        if let Some(f) = attr(a, "fliph") {
            v.fliph = f.eq_ignore_ascii_case("true");
        }
        if let Some(f) = attr(a, "filter") {
            v.filter = if f.eq_ignore_ascii_case("none") { String::new() } else { f.to_owned() };
        }
        let _ = chara;
        Ok(())
    }
    fn timing(a: &Attrs) -> Result<(u64, i32, u64)> {
        let duration = attr(a, "time").or(attr(a, "tme")).map(seconds).transpose()?.unwrap_or(0);
        let accel = attr(a, "accel").or(attr(a, "acccel")).map(|v| v.trim().parse::<i32>()).transpose()?.unwrap_or(0);
        let delay = attr(a, "delay").map(seconds).transpose()?.unwrap_or(0);
        Ok((duration, accel, delay))
    }
    /// `<layer>` (native 6427B0).
    pub fn layer(&mut self, a: &Attrs, now: u64, read: &mut Read<'_>) -> Result<()> {
        let group = Group::parse(attr(a, "parent"))?;
        let id = attr(a, "id").context("Malie layer id")?.to_ascii_lowercase();
        let (duration, accel, delay) = Self::timing(a)?;
        let now = now + delay;
        if attr(a, "env-src").is_some() || attr(a, "src-type").is_some_and(|t| t.eq_ignore_ascii_case("env")) {
            // Environment-sound layers carry no picture; audio is owned by the adapter.
            return Ok(());
        }
        let picture = match (attr(a, "src"), attr(a, "char-src")) {
            (Some(src), _) => Some(self.picture(src, read)?),
            (None, Some(src)) => {
                let parts: Vec<_> = src.split_whitespace().map(str::to_ascii_lowercase).collect();
                ensure!(!parts.is_empty(), "Malie char-src");
                let pose = parts.get(2).cloned().unwrap_or_else(|| "def".into());
                let face = parts.get(1).cloned().unwrap_or_else(|| "none".into());
                Some(self.char_picture(&parts[0], &pose, &face, read)?.unwrap_or(Picture { name: "none".into(), size: [1., 1.], elements: Vec::new() }))
            }
            _ => None,
        };
        for key in ["script-src", "shake-src"] {
            if let Some(v) = attr(a, key) {
                if !(key == "shake-src" && v.eq_ignore_ascii_case("stop")) {
                    self.gap(format!("layer {key}={v}"));
                }
            }
        }
        let fresh = !self.state.layers.contains_key(&(group, id.clone()));
        let v = self.state.layers.entry((group, id)).or_insert_with(|| Visual::new(now, 1.));
        v.closing = false;
        if let Some(p) = picture {
            let mask = attr(a, "mask-src").or(attr(a, "mak-src")).or(attr(a, "wipe-src"));
            if fresh && attr(a, "char-src").is_some() && attr(a, "center").is_none() {
                // 6464D0: a new character plane is anchored at its bottom centre.
                v.center = Track::new([p.size[0] / 2., p.size[1], 0.]);
            }
            if let (false, Some(old), true) = (fresh, v.picture.take(), duration > 0) {
                if mask.is_some() || duration > 0 {
                    v.previous = Some((old, now, duration));
                }
            }
            if p.movie().is_some() {
                v.movie_started = now;
            }
            v.shown_at = now;
            if let Some((m, _)) = p.movie() {
                let stem = m.rsplit('/').next().unwrap_or(m).trim_end_matches(".webm").to_owned();
                self.seen_movies.push(stem);
            } else {
                self.seen_pictures.push(p.name.clone());
            }
            v.picture = Some(p);
            // A source change carries its own transition time; opacity/position
            // writes in the same tag share it.
            Self::apply_geometry(v, a, now, if mask.is_some() { 0 } else { duration }, accel, false)?;
        } else {
            Self::apply_geometry(v, a, now, duration, accel, false)?;
        }
        Ok(())
    }
    /// `<layerclear>`: one id, or a whole parent.
    pub fn layer_clear(&mut self, a: &Attrs, now: u64) -> Result<()> {
        let (duration, accel, _) = Self::timing(a)?;
        let id = attr(a, "id").map(str::to_ascii_lowercase);
        // 50C980: the root is always resolved; no parent selects the default root.
        let parent = Some(Group::parse(attr(a, "parent"))?);
        let keys: Vec<_> = self
            .state
            .layers
            .keys()
            .filter(|(g, i)| id.as_ref().is_none_or(|id| id == i) && parent.is_none_or(|p| p == *g))
            .cloned()
            .collect();
        for key in keys {
            if duration == 0 {
                self.state.layers.remove(&key);
            } else if let Some(v) = self.state.layers.get_mut(&key) {
                let mut b = a.clone();
                b.remove("opacity");
                Self::apply_geometry(v, &b, now, duration, accel, false)?;
                v.opacity.set([0., 0., 0.], now, duration, accel);
                v.closing = true;
            }
        }
        Ok(())
    }
    /// Movie element keys drawn by the layers `a` selects.
    pub fn layer_movies(&self, a: &Attrs) -> Result<Vec<String>> {
        let id = attr(a, "id").map(str::to_ascii_lowercase);
        let parent = attr(a, "parent").map(|p| Group::parse(Some(p))).transpose()?;
        Ok(self
            .movie_surfaces
            .keys()
            .filter(|k| {
                let mut parts = k.splitn(3, '/');
                let (_, g, rest) = (parts.next(), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                let lid = rest.split('#').next().unwrap_or("");
                id.as_ref().is_none_or(|id| id == lid) && parent.is_none_or(|p| format!("{p:?}") == g)
            })
            .cloned()
            .collect())
    }
    /// 642B60: one layer, or every layer of the root plus the camera.
    pub fn layer_busy(&self, a: &Attrs, now: u64) -> Result<bool> {
        let id = attr(a, "id").map(str::to_ascii_lowercase);
        let parent = Group::parse(attr(a, "parent"))?;
        let layers = self.state.layers.iter().any(|((g, i), v)| id.as_ref().is_none_or(|id| id == i) && parent == *g && v.busy(now));
        Ok(layers || (id.is_none() && self.camera_busy(now)))
    }
    /// Picture size of a loaded layer (for macro tags that centre on it).
    pub fn picture_size(&mut self, src: &str, read: &mut Read<'_>) -> Result<[f32; 2]> {
        Ok(self.picture(src, read)?.size)
    }
    pub fn char_picture_size(&mut self, who: &str, pose: &str, face: &str, read: &mut Read<'_>) -> Result<Option<[f32; 2]>> {
        Ok(self.char_picture(who, pose, face, read)?.map(|p| p.size))
    }
    pub fn camera(&mut self, a: &Attrs, now: u64) -> Result<()> {
        let index = attr(a, "id").map(|v| v.trim().parse::<u32>()).transpose()?.unwrap_or(0);
        ensure!(index <= 1, "Malie layer camera {index}");
        let (duration, accel, delay) = Self::timing(a)?;
        let now = now + delay;
        if index == 1 {
            let [w, h] = self.canvas;
            let main = self.state.camera.clone();
            let split = self.state.split.get_or_insert_with(|| Split { camera: main, visible: false, position: [w / 2., h / 2.], angle: Track::new([0.; 3]) });
            if let Some(v) = attr(a, "visible") {
                split.visible = v.eq_ignore_ascii_case("true");
            }
            if let Some(p) = attr(a, "splitter-position") {
                let p = floats(p)?;
                ensure!(p.len() >= 2, "Malie splitter position");
                split.position = [p[0], p[1]];
            }
            if let Some(r) = attr(a, "splitter-rotate") {
                split.angle.set([floats(r)?[0], 0., 0.], now, duration, accel);
            }
            let cam = &mut split.camera;
            if let Some(f) = attr(a, "fieldofview") {
                cam.fov = floats(f)?[0].clamp(1., 170.);
            }
            if let Some(p) = attr(a, "position").or(attr(a, "pos")) {
                let p = vec3(p, cam.position.to)?;
                cam.position.set(p, now, duration, accel);
            }
            if let Some(r) = attr(a, "rotate") {
                let r = vec3(r, [0.; 3])?;
                cam.rotate.set(r, now, duration, accel);
            }
            return Ok(());
        }
        if let Some(f) = attr(a, "fieldofview") {
            let f = floats(f)?[0];
            ensure!((1. ..=170.).contains(&f), "Malie camera field of view");
            self.state.camera.fov = f;
        }
        if let Some(p) = attr(a, "position").or(attr(a, "pos")) {
            let p = vec3(p, self.state.camera.position.to)?;
            self.state.camera.position.set(p, now, duration, accel);
        }
        if let Some(r) = attr(a, "r-pos") {
            let r = vec3(r, [0.; 3])?;
            let b = self.state.camera.position.to;
            self.state.camera.position.set([b[0] + r[0], b[1] + r[1], b[2] + r[2]], now, duration, accel);
        }
        if let Some(r) = attr(a, "rotate") {
            let r = vec3(r, [0.; 3])?;
            self.state.camera.rotate.set(r, now, duration, accel);
        }
        Ok(())
    }
    pub fn camera_busy(&self, now: u64) -> bool {
        self.state.camera.position.busy(now) || self.state.camera.rotate.busy(now)
    }
    /// After a load: finish pending tweens/crossfades and restart clocks.
    pub fn rebase(&mut self, now: u64) {
        let settle = |t: &mut Track| {
            t.from = t.to;
            t.duration = 0;
            t.start = now;
        };
        self.state.layers.retain(|_, v| !v.closing);
        for v in self.state.layers.values_mut() {
            for t in [&mut v.position, &mut v.center, &mut v.scale, &mut v.rotate, &mut v.opacity] {
                settle(t);
            }
            v.previous = None;
            v.shown_at = now;
            v.movie_started = now;
        }
        settle(&mut self.state.camera.position);
        settle(&mut self.state.camera.rotate);
        self.state.page.complete = true;
        self.state.effect = None;
        for e in self.state.emotions.values_mut() {
            e.2 = now;
        }
        self.movie_surfaces.clear();
    }
    /// 533BF0 effect1..5 (effect5 is the inverting variant).
    pub fn effect(&mut self, src: &str, now: u64) -> Result<()> {
        let s = |p: &str, a: f32, b: f32, d: u64, t: u64, c: i32| (p.to_owned(), a, b, d, t, c);
        let (blend, steps) = match src.to_ascii_lowercase().as_str() {
            "effect1" => ("normal", vec![s("alpha", 1., 0., 0, 1000, 2), s("scale", 1., 1.05, 0, 1000, 2)]),
            "effect2" => ("add", vec![s("alpha", 1., 0., 0, 3000, -1), s("scale", 1., 1.1, 0, 3000, -1)]),
            "effect3" | "effect4" => {
                let y = if src.eq_ignore_ascii_case("effect3") { 10. } else { 25. };
                ("normal", vec![s("y", 0., y, 0, 300, 2), s("y", y, 0., 450, 300, 1), s("alpha", 1., 0., 0, 1000, -1), s("scale", 1., 1.1, 0, 1000, -1)])
            }
            "effect5" => ("inv", vec![s("alpha", 1., 0., 0, 1000, 2), s("x", 0., 25., 0, 250, -1), s("x", 25., -25., 500, 250, 2), s("x", -25., 0., 750, 250, 1)]),
            other => {
                self.gap(format!("ef src={other}"));
                return Ok(());
            }
        };
        self.state.effect = Some(Effect { start: now, blend: blend.into(), steps });
        Ok(())
    }
    /// Character emotion icon (`em-src`) from data/screen/emotion.
    pub fn emotion(&mut self, who: &str, src: &str, _kind: &str, offset: [f32; 3], now: u64, read: &mut Read<'_>) -> Result<()> {
        if src.eq_ignore_ascii_case("none") {
            self.state.emotions.remove(who);
            return Ok(());
        }
        let name = src.to_ascii_lowercase();
        let text = read(&format!("data/screen/emotion/{name}.svg"))?.with_context(|| format!("Malie emotion {name:?} is absent"))?;
        let pic = picture::parse(&name, "data/screen/emotion", &text, &BTreeMap::new())?;
        self.state.emotions.insert(who.to_owned(), (pic, offset, now));
        Ok(())
    }
    pub fn clear_emotion(&mut self, who: &str) {
        self.state.emotions.remove(who);
    }
    fn emit_effect(&mut self, now: u64, out: &mut Vec<(u32, Value)>) -> Result<()> {
        let Some(e) = self.state.effect.clone() else { return Ok(()) };
        let t = now.saturating_sub(e.start);
        let mut v = BTreeMap::from([("alpha", 1f32), ("scale", 1.), ("x", 0.), ("y", 0.)]);
        let mut end = 0;
        for (p, a, b, d, dur, curve) in &e.steps {
            end = end.max(d + dur);
            if t >= *d {
                let k = if *dur == 0 { 1. } else { ((t - d) as f32 / *dur as f32).min(1.) };
                let k = easing(*curve).apply(k);
                if let Some(slot) = v.get_mut(p.as_str()) {
                    *slot = a + (b - a) * k;
                }
            }
        }
        if t >= end {
            self.state.effect = None;
            return Ok(());
        }
        let id = self.id("effect")?;
        let [w, h] = self.canvas;
        let sc = v["scale"];
        let blend = match e.blend.as_str() {
            "add" => "add",
            "inv" => "inverse_destination",
            _ => "normal",
        };
        out.push((id, json!({"id":id,"rect":[v["x"],v["y"],w,h],"paint":{"kind":"backdrop"},"blend":blend,
            "opacity":(gamma_alpha(v["alpha"])*255.).round() as u8,"order":[37,0],
            "transform":{"scale_x":sc,"scale_y":sc,"pivot_x":w/2.,"pivot_y":h/2.}})));
        Ok(())
    }
    pub fn shake(&mut self, src: &str, now: u64) -> Result<()> {
        let (name, params) = picture::split_source(src)?;
        if name == "stop" {
            self.state.shake = None;
            return Ok(());
        }
        ensure!(matches!(name.as_str(), "shake" | "random" | "randomloop"), "Malie shake source {src:?}");
        let amp = params.get("amp").map(|v| v.parse::<f32>()).transpose()?.unwrap_or(5.);
        let time = params.get("time").map(|v| seconds(v)).transpose()?.unwrap_or(300);
        self.state.shake = Some(Shake {
            amp: amp.clamp(0., 200.),
            period: time.clamp(16, 10_000),
            until: (name != "randomloop").then_some(now + time.max(100) * if name == "shake" { 1 } else { 3 }),
        });
        Ok(())
    }
    // ---- message window -------------------------------------------------
    pub fn message(&mut self, segments: Vec<Vec<Token>>) {
        self.state.segments = segments;
        self.state.segment = 0;
    }
    /// `_ms_out`: append the next pause segment and start its reveal.
    pub fn out(&mut self, now: u64) -> Option<String> {
        let seg = self.state.segments.get(self.state.segment).cloned().unwrap_or_default();
        self.state.segment += 1;
        let voice = seg.iter().find_map(|t| match t {
            Token::Voice(v) => Some(v.clone()),
            _ => None,
        });
        self.state.page.shown = glyph_count(&self.state.page.tokens);
        self.state.page.tokens.extend(seg);
        self.state.page.reveal_start = now;
        self.state.page.complete = false;
        self.state.frame.visible = true;
        voice
    }
    pub fn clear(&mut self) -> String {
        let text = message::plain(&self.state.page.tokens);
        self.state.page = Page::default();
        self.state.name.clear();
        text
    }
    pub fn revealing(&self, now: u64) -> bool {
        if self.state.page.complete {
            return false;
        }
        let total = glyph_count(&self.state.page.tokens);
        let pending = total.saturating_sub(self.state.page.shown) as u64;
        now < self.state.page.reveal_start + pending * self.text_interval_ms
    }
    pub fn complete_reveal(&mut self) {
        self.state.page.complete = true;
    }
    // ---- rendering ------------------------------------------------------
    fn focal(&self) -> f32 {
        (self.canvas[1] / 2.) / (self.state.camera.fov.to_radians() / 2.).tan()
    }
    /// Screen-space affine for a perspective (back/chara) or canvas (cutin)
    /// plane: anchor point, uniform scale factor and roll.
    fn place(&self, group: Group, v: &Visual, now: u64) -> Option<([f32; 2], f32, f32)> {
        let p = v.position.at(now);
        // A single `rotate` operand is the in-plane roll.
        let r = v.rotate.at(now);
        let rot = if r[2] != 0. { r[2] } else { r[0] }.to_radians();
        match group.perspective() {
            false => Some(([p[0], p[1]], 1., rot)),
            true => {
                let cam = self.state.camera.position.at(now);
                let roll = self.state.camera.rotate.at(now);
                let roll = if roll[2] != 0. { roll[2] } else { roll[0] }.to_radians();
                let d = p[2] - cam[2];
                if d <= 1. {
                    return None;
                }
                let k = self.focal() / d;
                let (x, y) = ((p[0] - cam[0]) * k, (p[1] - cam[1]) * k);
                let (s, c) = (-roll).sin_cos();
                let sx = self.canvas[0] / 2. + x * c - y * s;
                let sy = self.canvas[1] / 2. + x * s + y * c;
                Some(([sx, sy], k, rot - roll))
            }
        }
    }
    fn emit_picture(
        &mut self,
        key: &str,
        parent: u32,
        picture: &Picture,
        center: [f32; 3],
        opacity: u8,
        order: [i32; 2],
        t: u64,
        out: &mut Vec<(u32, Value)>,
    ) -> Result<()> {
        let group = self.id(key)?;
        out.push((group, json!({"id":group,"parent":parent,"rect":[0.,0.,1.,1.],"paint":{"kind":"group"},"opacity":opacity,"order":order})));
        for (n, e) in picture.elements.iter().enumerate() {
            let id = self.id(&format!("{key}/{n}"))?;
            let paint = match &e.paint {
                Paint::Image(asset) => json!({"kind":"image","asset":asset}),
                Paint::Solid(c) => json!({"kind":"solid","color":c}),
                Paint::Movie(asset, looping) => {
                    // The shared media service owns decoding and the clock;
                    // its frame surface appears once playback has started.
                    let element = format!("{key}/{n}");
                    self.movies_wanted.insert(element.clone(), (asset.clone(), *looping));
                    match self.movie_surfaces.get(&element) {
                        Some((a, surface)) if a == asset => json!({"kind":"image","asset":surface}),
                        _ => continue,
                    }
                }
            };
            let mut layer = match e.sample(t) {
                Some((m, alpha)) => {
                    let det = m[0] * m[3] - m[1] * m[2];
                    let size = e.animated.as_ref().unwrap().size;
                    if alpha <= 0. || det.abs() < 1e-9 || size[0] <= 0. || size[1] <= 0. {
                        continue;
                    }
                    let matrix = [m[0], m[1], 0., 0., m[2], m[3], 0., 0., 0., 0., 1., 0., m[4] - center[0], m[5] - center[1], 0., 1.];
                    json!({"id":id,"parent":group,"rect":[0.,0.,size[0],size[1]],"paint":paint,"opacity":(gamma_alpha(alpha)*255.).round() as u8,
                        "order":[order[0],order[1]*64+n as i32+1],"transform":{"local_matrix":matrix}})
                }
                None => json!({"id":id,"parent":group,"rect":[e.rect[0]-center[0],e.rect[1]-center[1],e.rect[2].max(0.01),e.rect[3].max(0.01)],"paint":paint,"opacity":(gamma_alpha(e.opacity)*255.).round() as u8,"order":[order[0],order[1]*64+n as i32+1]}),
            };
            if e.additive {
                layer["blend"] = json!("add");
            }
            out.push((id, layer));
        }
        Ok(())
    }
    fn emit_visual(&mut self, key: &str, group: Group, v: &Visual, now: u64, minor: i32, out: &mut Vec<(u32, Value)>) -> Result<()> {
        let Some((anchor, k, rot)) = self.place(group, v, now) else {
            return Ok(());
        };
        // A nearly transparent plane (native 0.01 "loaded but hidden") still
        // owns its picture and movie clock.
        if v.opacity.at(now)[0] <= 0. {
            return Ok(());
        }
        let opacity = (gamma_alpha(v.opacity.at(now)[0]) * 255.).round() as u8;
        let s = v.scale.at(now);
        let id = self.id(key)?;
        let flip = if v.fliph { -1. } else { 1. };
        let global = match group {
            Group::Back => self.state.cg_filter.clone(),
            Group::Chara => self.state.char_filter.clone(),
            _ => String::new(),
        };
        let spec = if v.filter.is_empty() { global } else { v.filter.clone() };
        let paint = match filter_tint(&spec) {
            Ok(Some((tint, add))) => json!({"kind":"group","tint":tint,"add":add}),
            Ok(None) => json!({"kind":"group"}),
            Err(e) => {
                self.gap(format!("filter {spec:?}: {e:#}"));
                json!({"kind":"group"})
            }
        };
        out.push((id, json!({"id":id,"parent":self.emit_root,"rect":[anchor[0],anchor[1],1.,1.],"paint":paint,"opacity":opacity,
            "order":[group.major(),minor*256],"transform":{"scale_x":k*s[0]*flip,"scale_y":k*s[1],"rotate":rot}})));
        let center = v.center.at(now);
        let order = [group.major(), minor * 4];
        if let Some((old, start, dur)) = &v.previous {
            if now < start + dur {
                let p = (now - start) as f32 / *dur as f32;
                let fade = ((1. - p) * 255.) as u8;
                self.emit_picture(&format!("{key}#old"), id, old, center, fade, order, now.saturating_sub(*start), out)?;
                if let Some(p2) = &v.picture {
                    let fade_in = (p * 255.) as u8;
                    self.emit_picture(&format!("{key}#new"), id, p2, center, fade_in, [order[0], order[1] + 1], now.saturating_sub(v.shown_at), out)?;
                }
                return Ok(());
            }
        }
        if let Some(p) = &v.picture {
            self.emit_picture(&format!("{key}#new"), id, p, center, 255, [order[0], order[1] + 1], now.saturating_sub(v.shown_at), out)?;
        }
        Ok(())
    }
    /// Retire completed fades and build the retained layer list.
    pub fn render(&mut self, now: u64, advance: &Advance<'_>) -> Result<Vec<Value>> {
        if self.frozen {
            return Ok(Vec::new());
        }
        let mut out: Vec<(u32, Value)> = Vec::new();
        self.movies_wanted.clear();
        // Shake offsets the whole ADV root.
        let mut offset = [0f32, 0f32];
        if let Some(s) = self.state.shake.clone() {
            if s.until.is_some_and(|u| now >= u) {
                self.state.shake = None;
            } else {
                let phase = (now / s.period.max(1)) as u32;
                let h = phase.wrapping_mul(2654435761);
                offset = [((h & 0xff) as f32 / 127.5 - 1.) * s.amp, (((h >> 8) & 0xff) as f32 / 127.5 - 1.) * s.amp];
            }
        }
        out.push((ROOT, json!({"id":ROOT,"rect":[offset[0],offset[1],1.,1.],"paint":{"kind":"group"},"order":[5,0],"opacity":if self.hidden { 0 } else { 255 }})));
        // Retire finished fades.
        let now_ = now;
        self.state.layers.retain(|_, v| !(v.closing && !v.opacity.busy(now_)));
        for v in self.state.layers.values_mut() {
            if v.previous.as_ref().is_some_and(|(_, s, d)| now_ >= s + d) {
                v.previous = None;
            }
        }
        // Depth order inside a perspective group: far planes first.
        let mut entries: Vec<_> = self.state.layers.iter().map(|((g, id), v)| (*g, id.clone(), v.clone())).collect();
        entries.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| {
                    // Far planes first; canvas roots keep the same z convention.
                    b.2.position.to[2].partial_cmp(&a.2.position.to[2]).unwrap_or(std::cmp::Ordering::Equal)
                })
                .then(a.2.created.cmp(&b.2.created))
        });
        let split = self.state.split.clone().filter(|s| s.visible);
        if let Some(sp) = &split {
            // Each camera's world is clipped to its side of the rotated
            // splitter line: container (rotated, half-plane clip) ->
            // counter-rotation -> translation back to canvas coordinates.
            let a = sp.angle.at(now)[0].to_radians();
            for (n, side) in [(0u32, -1f32), (1, 1.)] {
                let base = SPLIT + n * 3;
                let clip = if side < 0. { [-8000., -8000., 16000., 8000.] } else { [-8000., 0., 16000., 8000.] };
                out.push((base, json!({"id":base,"parent":ROOT,"rect":[sp.position[0],sp.position[1],1.,1.],"paint":{"kind":"group"},"clip":clip,"order":[9,n as i32],"transform":{"rotate":a}})));
                out.push((base + 1, json!({"id":base+1,"parent":base,"rect":[0.,0.,1.,1.],"paint":{"kind":"group"},"order":[9,n as i32],"transform":{"rotate":-a}})));
                out.push((base + 2, json!({"id":base+2,"parent":base+1,"rect":[-sp.position[0],-sp.position[1],1.,1.],"paint":{"kind":"group"},"order":[9,n as i32]})));
            }
        }
        for (n, (g, id, v)) in entries.iter().enumerate() {
            if let (Some(sp), true) = (&split, g.perspective()) {
                self.emit_root = SPLIT + 2;
                self.emit_visual(&format!("L/{g:?}/{id}"), *g, v, now, n as i32, &mut out)?;
                let main = std::mem::replace(&mut self.state.camera, sp.camera.clone());
                self.emit_root = SPLIT + 5;
                let r = self.emit_visual(&format!("S/{g:?}/{id}"), *g, v, now, n as i32, &mut out);
                self.state.camera = main;
                self.emit_root = ROOT;
                r?;
            } else {
                self.emit_visual(&format!("L/{g:?}/{id}"), *g, v, now, n as i32, &mut out)?;
            }
        }
        // Emotion icons follow their character plane.
        for (who, (pic, off, shown)) in self.state.emotions.clone() {
            let Some(v) = self.state.layers.get(&(Group::Chara, who.clone())).cloned() else { continue };
            let Some((anchor, k, _)) = self.place(Group::Chara, &v, now) else { continue };
            let s = v.scale.at(now);
            let id = self.id(&format!("E/{who}"))?;
            out.push((id, json!({"id":id,"parent":ROOT,"rect":[anchor[0]+off[0]*k*s[0],anchor[1]+off[1]*k*s[1],1.,1.],"paint":{"kind":"group"},"order":[Group::Chara.major(),200_000],"transform":{"scale_x":k*s[0],"scale_y":k*s[1]}})));
            self.emit_picture(&format!("E/{who}#p"), id, &pic, [0.; 3], 255, [Group::Chara.major(), 200_001], now.saturating_sub(shown), &mut out)?;
        }
        self.emit_effect(now, &mut out)?;
        // Message window: frame document layers come from the adapter.
        if self.state.frame.visible && !self.window_hidden && !self.frame_layers.is_empty() {
            for l in self.frame_layers.clone() {
                let id = l["id"].as_u64().context("Malie frame layer id")? as u32;
                out.push((id, l));
            }
            self.emit_text(now, advance, &mut out)?;
        }
        self.emit_select(advance, &mut out)?;
        // Diff against the last transmitted list.
        let mut ops = Vec::new();
        let mut live = BTreeMap::new();
        for (id, layer) in out {
            if self.sent.get(&id) != Some(&layer) {
                ops.push(json!({"op":"put","layer":layer}));
            }
            live.insert(id, layer);
        }
        let gone = self.sent.keys().filter(|id| !live.contains_key(id)).copied().collect();
        ops.extend(crate::screen::removals(&gone, &self.parents()));
        self.sent = live;
        Ok(ops)
    }
    fn parents(&self) -> BTreeMap<u32, Option<u32>> {
        self.sent.iter().map(|(id, l)| (*id, l["parent"].as_u64().map(|v| v as u32))).collect()
    }
    pub fn forget(&mut self) -> Vec<Value> {
        let gone = self.sent.keys().copied().collect();
        let ops = crate::screen::removals(&gone, &self.parents());
        self.sent.clear();
        ops
    }
    /// Choice rows: select.svg is a 690x90 cell (1920-space art at 0.53333)
    /// whose `normal`/`focus` images sit at (12,20)/(8,16) and whose 30px
    /// rich text starts at (192,70). Rows stack centred above the window.
    pub fn select_rects(&self) -> Vec<[f32; 4]> {
        let Some(sel) = &self.state.select else { return Vec::new() };
        if !sel.shown {
            return Vec::new();
        }
        let n = sel.items.len() as f32;
        let k = 0.53333;
        let (w, h) = (1356. * k, 144. * k);
        let pitch = 90.;
        let top = (self.canvas[1] - 120.) / 2. - pitch * n / 2.;
        (0..sel.items.len())
            .map(|i| [(self.canvas[0] - w) / 2., top + pitch * i as f32 + (pitch - h) / 2., w, h])
            .collect()
    }
    fn emit_select(&mut self, advance: &Advance<'_>, out: &mut Vec<(u32, Value)>) -> Result<()> {
        let rects = self.select_rects();
        let Some(sel) = self.state.select.clone() else { return Ok(()) };
        let size = 16.;
        for (i, ((_, text), r)) in sel.items.iter().zip(rects).enumerate() {
            let focused = sel.focus == Some(i);
            let id = self.id(&format!("select/{i}"))?;
            let asset = if focused { "data/screen/select/focus.dzi" } else { "data/screen/select/normal.dzi" };
            out.push((id, json!({"id":id,"rect":r,"paint":{"kind":"image","asset":asset},"order":[47,i as i32*2]})));
            let width: f32 = text.chars().map(|c| advance(c, size)).sum();
            let mut x = (r[2] - width) / 2.;
            let mut glyphs = Vec::new();
            for c in text.chars() {
                glyphs.push(json!({"character":c,"baseline":[x,size]}));
                x += advance(c, size);
            }
            let tid = self.id(&format!("select/{i}/text"))?;
            let color = if focused { [255, 64, 64, 255] } else { [255, 255, 255, 255] };
            out.push((tid, json!({"id":tid,"rect":[r[0],r[1]+(r[3]-size*1.4)/2.,r[2],size*1.6],"paint":{"kind":"glyph_run","font":self.font,"size":size,"color":color,"glyphs":glyphs,"outline":{"radius":2,"color":[0,0,0,255],"kernel":"disk"}},"order":[47,i as i32*2+1]})));
        }
        Ok(())
    }
    fn emit_text(&mut self, now: u64, advance: &Advance<'_>, out: &mut Vec<(u32, Value)>) -> Result<()> {
        let size = self.font_size;
        let [x0, y0, w, _] = self.text_rect;
        let line = (size * 1.6).round();
        let ruby_size = (size * 0.5).round();
        let mut glyphs = Vec::new();
        let (mut x, mut y) = (0f32, ruby_size + size);
        let mut color = self.text_color;
        let mut count = 0usize;
        let mut steps = Vec::new();
        let tokens = self.state.page.tokens.clone();
        for t in &tokens {
            match t {
                Token::Newline => {
                    x = 0.;
                    y += line;
                }
                Token::Char(c) => {
                    let a = advance(*c, size);
                    if x + a > w && !"、。」』）！？…―".contains(*c) {
                        x = 0.;
                        y += line;
                    }
                    glyphs.push(json!({"character":c,"baseline":[x,y],"color":color}));
                    x += a;
                    count += 1;
                    steps.push(glyphs.len());
                }
                Token::Ruby { base, reading } => {
                    let bw: f32 = base.chars().map(|c| advance(c, size)).sum();
                    if x + bw > w {
                        x = 0.;
                        y += line;
                    }
                    let rw: f32 = reading.chars().map(|c| advance(c, ruby_size)).sum();
                    let n = reading.chars().count().max(1) as f32;
                    let gap = if rw < bw { (bw - rw) / n } else { 0. };
                    let mut rx = x + (bw - rw - gap * (n - 1.)) / 2.;
                    let start = glyphs.len();
                    for c in reading.chars() {
                        glyphs.push(json!({"character":c,"baseline":[rx,y-size*0.95],"size":ruby_size,"color":color}));
                        rx += advance(c, ruby_size) + gap;
                    }
                    let _ = start;
                    for c in base.chars() {
                        glyphs.push(json!({"character":c,"baseline":[x,y],"color":color}));
                        x += advance(c, size);
                        count += 1;
                        steps.push(glyphs.len());
                    }
                }
                Token::Color(c) => color = c.map_or(self.text_color, |c| [c[0], c[1], c[2], 255]),
                Token::Size(_) | Token::Voice(_) | Token::VoiceEnd => {}
            }
        }
        let _ = count;
        let page = &self.state.page;
        let shown_glyphs = if page.shown == 0 { 0 } else { steps.get(page.shown - 1).copied().unwrap_or(glyphs.len()) };
        let pending: Vec<usize> = steps.iter().copied().filter(|s| *s > shown_glyphs).collect();
        let elapsed = now.saturating_sub(page.reveal_start);
        let complete = page.complete || elapsed >= pending.len() as u64 * self.text_interval_ms;
        let visible = if complete {
            glyphs.len()
        } else {
            let n = (elapsed / self.text_interval_ms.max(1)) as usize;
            if n == 0 { shown_glyphs } else { pending.get(n - 1).copied().unwrap_or(glyphs.len()) }
        };
        let id = self.id("text/body")?;
        let outline = json!({"radius":2,"color":[0,0,0,255],"kernel":"disk"});
        let visible_glyphs: Vec<_> = glyphs.into_iter().take(visible).collect();
        if !visible_glyphs.is_empty() {
            out.push((id, json!({"id":id,"rect":[x0,y0,w+size*2.,self.text_rect[3]+size*4.],"paint":{"kind":"glyph_run","font":self.font,"size":size,"color":[255,255,255,255],"glyphs":visible_glyphs,"outline":outline},"order":[45,10]})));
        }
        if !self.state.name.is_empty() {
            let mut glyphs = Vec::new();
            let mut x = 0.;
            for c in self.state.name.chars() {
                glyphs.push(json!({"character":c,"baseline":[x,size]}));
                x += advance(c, size);
            }
            let nid = self.id("text/name")?;
            let [nx, ny, nw, nh] = self.name_rect;
            out.push((nid, json!({"id":nid,"rect":[nx+(nw-x).max(0.)/2.,ny+(nh-size).max(0.)/2.-2.,x.max(1.),size*1.5],"paint":{"kind":"glyph_run","font":self.font,"size":size,"color":[255,255,255,255],"glyphs":glyphs,"outline":outline},"order":[45,11]})));
        }
        Ok(())
    }
}
/// The original Direct3D 9 renderer blends gamma-encoded bytes; shared
/// layers blend in linear light. For a plane faded over black, gamma-space
/// coverage `a` equals linear coverage `srgb_to_linear(a)`, so whole-layer
/// opacities are converted (exact over black, approximate elsewhere).
pub fn gamma_alpha(a: f32) -> f32 {
    let a = a.clamp(0., 1.);
    if a <= 0.04045 { a / 12.92 } else { ((a + 0.055) / 1.055).powf(2.4) }
}
/// `col <color> <p>` colourises (multiply toward the colour); `sil <color>
/// <p>` replaces colour with a flat silhouette. Both map to the shared group
/// tint (multiply) and add terms: out = src*tint + add.
pub fn filter_tint(spec: &str) -> Result<Option<([u8; 4], [f32; 3])>> {
    let parts: Vec<&str> = spec.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }
    ensure!(parts.len() >= 2, "Malie filter {spec:?}");
    let c = css_color(parts[1]).with_context(|| format!("Malie filter colour {:?}", parts[1]))?;
    let p = parts.get(2).map(|v| v.parse::<f32>()).transpose()?.unwrap_or(100.).clamp(0., 100.) / 100.;
    let c = c.map(|v| f32::from(v) / 255.);
    Ok(Some(match parts[0].to_ascii_lowercase().as_str() {
        "col" => ([0, 1, 2].map(|i| ((1. - p + p * c[i]) * 255.).round() as u8).into_iter().chain([255]).collect::<Vec<_>>().try_into().unwrap(), [0.; 3]),
        "sil" => {
            let k = ((1. - p) * 255.).round() as u8;
            ([k, k, k, 255], [c[0] * p, c[1] * p, c[2] * p])
        }
        other => bail!("Malie filter kind {other:?}"),
    }))
}
fn css_color(name: &str) -> Option<[u8; 3]> {
    let n = name.to_ascii_lowercase();
    if let Some(hex) = n.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).ok()?;
        return Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    Some(match n.as_str() {
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "red" => [255, 0, 0],
        "green" => [0, 128, 0],
        "blue" => [0, 0, 255],
        "yellow" => [255, 255, 0],
        "gray" | "grey" => [128, 128, 128],
        "silver" => [192, 192, 192],
        "midnightblue" => [25, 25, 112],
        "navy" => [0, 0, 128],
        "darkblue" => [0, 0, 139],
        "firebrick" => [178, 34, 34],
        "darkred" => [139, 0, 0],
        "crimson" => [220, 20, 60],
        "burlywood" => [222, 184, 135],
        "chocolate" => [210, 105, 30],
        "sienna" => [160, 82, 45],
        "orange" => [255, 165, 0],
        "darkorange" => [255, 140, 0],
        "gold" => [255, 215, 0],
        "purple" => [128, 0, 128],
        "indigo" => [75, 0, 130],
        "skyblue" => [135, 206, 235],
        "steelblue" => [70, 130, 180],
        "darkslategray" | "darkslategrey" => [47, 79, 79],
        "sepia" | "tan" => [210, 180, 140],
        "pink" => [255, 192, 203],
        "brown" => [165, 42, 42],
        _ => return None,
    })
}
fn glyph_count(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .map(|t| match t {
            Token::Char(_) => 1,
            Token::Ruby { base, .. } => base.chars().count(),
            _ => 0,
        })
        .sum()
}
/// Parse every `<name attr=value ...>` in a native tag string. Attribute
/// values are kept verbatim (`&` separates picture parameters).
pub fn parse_tags(text: &str) -> Result<Vec<(String, Attrs)>> {
    ensure!(text.len() <= 64 * 1024, "Malie tag source budget");
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '<' {
            ensure!(chars[i].is_whitespace(), "Malie tag text {text:?} needs message layout");
            i += 1;
            continue;
        }
        i += 1;
        let mut name = String::new();
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '>' {
            name.push(chars[i]);
            i += 1;
        }
        let mut attrs = Attrs::new();
        loop {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            ensure!(i < chars.len(), "Malie unterminated tag {text:?}");
            if chars[i] == '>' {
                i += 1;
                break;
            }
            if chars[i] == '/' {
                i += 1;
                continue;
            }
            let mut key = String::new();
            while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() && chars[i] != '>' {
                key.push(chars[i]);
                i += 1;
            }
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            let mut value = String::new();
            if i < chars.len() && chars[i] == '=' {
                i += 1;
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                if i < chars.len() && (chars[i] == '\'' || chars[i] == '"') {
                    let q = chars[i];
                    i += 1;
                    while i < chars.len() && chars[i] != q {
                        value.push(chars[i]);
                        i += 1;
                    }
                    i += 1;
                } else {
                    while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '>' {
                        value.push(chars[i]);
                        i += 1;
                    }
                }
            }
            ensure!(attrs.len() < 64, "Malie tag attribute budget");
            attrs.insert(key.to_ascii_lowercase(), value);
        }
        out.push((name.to_ascii_lowercase(), attrs));
    }
    Ok(out)
}
