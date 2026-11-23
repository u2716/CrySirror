// SPDX-License-Identifier: MPL-2.0
// SATD-minori-ef-presentation: unbound events return an explicit error.
use crate::message_action;
#[path = "zoom_transition.rs"]
mod zoom_transition;
const ZOOM_PIXELS: u32 = 510;
#[path = "breath.rs"]
mod breath;
#[path = "audio_fade.rs"]
mod audio_fade;
#[path = "tone.rs"]
pub(crate) mod tone;
#[path = "face.rs"]
mod face;
pub(crate) use face::apply_fixed_poses;
const STAGE_FACE: u32 = 850;
#[path = "characters.rs"]
mod characters;
#[path = "cutin.rs"]
mod cutin;
#[path = "rotation.rs"]
mod rotation;
#[path = "foreground.rs"]
mod foreground;
use foreground::Foreground;
#[path = "crossfade.rs"]
mod crossfade;
#[path = "scene_effects.rs"]
mod scene_effects;
use scene_effects::SceneEffect;
#[path = "sequences.rs"]
mod sequences;
#[path = "mmv.rs"]
mod mmv;
use sequences::Published;
#[path = "settings.rs"]
mod settings;
#[path = "title.rs"]
pub(crate) mod title;
#[path = "wscroll.rs"]
mod wscroll;
use crate::camera::{Motion, Scroll};
use crate::message::Message;
use crate::shake::{ScreenAbi, Shake};
use crate::snow::Weather;
use crate::strips::Strips;
use crate::vm::{ SelectOption, StageRequest, TransitionConfig, Vm, VmEvent};
use anyhow::{Context, Result, bail, ensure};
use characters::Characters;
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioAsset, AudioParameters, AudioRequest, HostServices, InputFrame, MediaPhase,
        MediaRequest, PcmBus, buttons,
    },
    state::ClientId,
};
use crysirror_image::temporal_pattern::PatternRng;
use serde_json::{Value, json};

const WEATHER: u32 = 500;
const CUTIN: u32 = 501;
const WEATHER_LAYERS: [u32; 2] = [WEATHER, 503];
const TILE_MASK: u32 = 509;
const CUTIN_LAYERS: [u32; 2] = [CUTIN, 504];
const SCREEN_ROOT: u32 = 10000;
const STAGE_ROOT: u32 = 10001;
const WRAPPED_WORLD: u32 = 12000;
const PANEL: u32 = 100;
const OLD_PANEL: u32 = 99;
const TEXT: u32 = 101;
const SPEAKER_NAME: u32 = 102;
const MOVIE: u32 = 200;
const SECOND_MUSIC_CHANNEL: u32 = 6;
const CHOICE: u32 = 210;

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub(crate) enum PanelLayout {
    #[serde(rename = "columns-65-169")]
    Columns,
    #[serde(rename = "stacked-120")]
    Stacked,
}

#[derive(Default)]
pub struct Presentation {
    pub(crate) panel_layout: Option<PanelLayout>,
    panel_mode: i32,
    pub(crate) tile_grid: Option<crate::tile_transition::Grid>,
    pub(crate) effect_settings: crate::parameters::EffectSettings,
    tile_transition: Option<crate::tile_transition::TileTransition>,
    zoom: Option<zoom_transition::Zoom>,
    pub(crate) sound_settings: crate::parameters::SoundSettings,
    pub(crate) voice_actors: Vec<String>,
    pub(crate) voice_marks: Vec<[i32;2]>,
    canvas: [i32; 2],
    screen_abi: ScreenAbi,
    pub(crate) panel_color: u32,
    pub(crate) speeds: crate::parameters::Speeds,
    characters: Characters,
    message_actions: Vec<message_action::MessageAction>,
    shake: Option<Shake>,
    snow: [Option<Weather>; 2],
    cutin: [Option<cutin::Cutin>; 2],
    cutin_catalogue: cutin::Catalogue,
    walking_scroll: Option<wscroll::WalkingScroll>,
    scenic:[Option<SceneEffect>;2],
    crossfades:[Option<crossfade::Crossfade>;2],
    second_scroll: Option<wscroll::WalkingScroll>,
    late_requests: [Option<crate::vm::EffectRequest>;2],
    last_effect_commands: [Option<crate::vm::EffectRequest>;2],
    stage_walk: [Option<wscroll::StageWalk>; 2],
    foreground: Option<Sprite>,
    fg_parts: Foreground,
    stage_face: Option<face::Face>,
    speaker: String,
    voice_meter: Option<VoiceMeter>,
    voice_generation: u64,
    last_breath_callback: Option<(u64,u64)>,
    stage_face_position: [i32; 2],
    fg_ready: Option<PendingForeground>,
    random: Option<PatternRng>,
    published: Published,
    pub audio_ticket: Option<u64>,
    audio_channels: std::collections::BTreeSet<u32>,
    sounds: audio_fade::SoundSlots,
    breaths: [Option<breath::Breath>;2],
    frame_count: u32,
    stage_count: u32,
    strips: Strips,
    preparing_stage: Option<StagePreparation>,
    filtered_slots: Vec<u32>,
    filtered_inputs: std::collections::BTreeMap<String, StageImage>,
    rotations: [rotation::Plane; 2],
    animations: mmv::Animations,
    world: Vec<Sprite>,
    camera: [i32; 2],
    foreground_camera: [i32; 2],
    wrap_layers: u32,
    last_scroll_draw: Option<u64>,
    scroll: Option<Scroll>,
    transition: Option<TransitionConfig>,
    transition_until: Option<u64>,
    message: Option<Message>,
    pub auto_advance_message: bool,
    panel: Vec<Sprite>,
    fade: Option<PanelFade>,
    movie: Option<(u32, bool)>,
    movie_soundtrack: bool,
    choices: Vec<SelectOption>,
    selected: usize,
}

#[derive(Clone)]
struct Sprite {
    asset: String,
    mask_color: Option<u32>,
    rect: [i32; 4],
}
#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct VoiceLevel {
    generation: u64,
    serial: u64,
    amplitude: u32,
    playing: bool,
}

#[derive(Default)]
struct VoiceWindows {
    channels: u32,
    values: Vec<u16>,
}
struct VoiceMeter {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    job: Option<std::sync::mpsc::Receiver<Result<()>>>,
    windows: std::sync::Arc<std::sync::Mutex<VoiceWindows>>,
}
impl Drop for VoiceMeter {
    fn drop(&mut self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
impl VoiceMeter {
    fn open(h: &mut dyn HostServices, c: ClientId, name: &str) -> Result<Self> {
        let reader = h.storage(c)?.open_asset(&asset("voice", name)?)?;
        let ext = name.rsplit_once('.').map(|(_, x)| x.to_owned());
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = cancelled.clone();
        let windows = std::sync::Arc::new(std::sync::Mutex::new(VoiceWindows::default()));
        let destination = windows.clone();
        let job = crate::worker(move || {
            let mut source = crysirror_audio::decoder::open_asset(reader, ext.as_deref())?;
            crysirror_audio::pcm_levels::read_windows(
                source.as_mut(),
                4096,
                |channels, level| {
                    let mut state = destination
                        .lock()
                        .map_err(|_| anyhow::anyhow!("minori voice meter lock"))?;
                    ensure!(state.values.len() < 1 << 20, "minori voice level budget");
                    state.channels = channels;
                    state.values.push(level);
                    Ok(())
                },
                || flag.load(std::sync::atomic::Ordering::Relaxed),
            )
        })?;
        Ok(Self {
            cancelled,
            job: Some(job),
            windows,
        })
    }
    fn level(&mut self, h: &dyn HostServices, c: ClientId) -> Result<VoiceLevel> {
        if let Some(job) = &self.job
            && crate::poll(job)?.is_some()
        {
            self.job = None;
        }
        if let Some(info) = h.audio_info(c, 1)?
            && info.source_playing
        {
            let data = self
                .windows
                .lock()
                .map_err(|_| anyhow::anyhow!("minori voice meter lock"))?;
            let serial = (info
                .position_frames
                .saturating_mul(u64::from(data.channels))
                / 4096)
                .min(data.values.len() as u64);
            if let Some(i) = serial.checked_sub(1) {
                return Ok(VoiceLevel {
                    serial,
                    amplitude: u32::from(data.values[i as usize]),
                    playing: true,
                    ..Default::default()
                });
            }
        }
        Ok(VoiceLevel::default())
    }
}
type StageImage = std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>;
struct PendingForeground { parts: Foreground, job: std::sync::mpsc::Receiver<Result<Vec<(usize, crysirror_image::ImageSurface)>>> }
struct StagePreparation {
    fg: Foreground,
    items: Vec<Sprite>,
    world_len: usize,
    camera: [i32; 2],
    job: std::sync::mpsc::Receiver<Result<Vec<(usize, StageImage)>>>,
}
struct PanelFade {
    next: Vec<Sprite>,
    start: u64,
    speed: u64,
}

pub(super) fn asset(bank: &str, name: &str) -> Result<VfsPath> {
    let relative = name.replace('\\', "/").to_ascii_lowercase();
    let path = VfsPath::new(Mount::Asset, &relative)?;
    ensure!(path.relative() == relative, "noncanonical minori resource");
    Ok(VfsPath::new(Mount::Asset, &format!("{bank}/{relative}"))?)
}
fn remove(id: u32) -> Value {
    json!({"op":"remove","id":id})
}
fn put(id: u32, rect: [i32; 4], paint: Value, opacity: u8) -> Value {
    json!({"op":"put","layer":{"id":id,"order":[id,0],"rect":rect,"opacity":opacity,"paint":paint}})
}
impl Sprite {
    fn requires_frame_publication(&self) -> bool {
        self.asset.to_ascii_lowercase().ends_with(".sqz") || tone::path(&self.asset).is_some()
    }
    fn layer(&self, id: u32, opacity: u8) -> Value {
        if let Some(color) = self.mask_color {
            let [alpha, r, g, b] = color.to_be_bytes();
            let mut op = put(
                id,
                self.rect,
                json!({"kind":"solid","color":[r,g,b,255]}),
                opacity,
            );
            op["layer"]["filter"]["mask"] =
                json!({"asset":self.asset,"channel":"alpha","scale":f32::from(alpha)/256.0});
            return op;
        }
        put(
            id,
            self.rect,
            json!({"kind":"image","asset":self.asset}),
            opacity,
        )
    }
}
fn sprite(h: &mut dyn HostServices, c: ClientId, banks: &[&str], name: &str) -> Result<Sprite> {
    let storage = h.storage(c)?;
    let (tone, raw) = tone::split(name);
    for bank in banks {
        let path = asset(bank, raw)?;
        if storage.exists(&path) {
            let [w, height] = if path.relative().ends_with(".sqz") {
                crysirror_image::sqz::Sequence::open(storage.open_asset(&path)?.as_ref())?.size
            } else { h.image_size(c, path.relative())? };
            return Ok(Sprite {
                asset: if tone.is_some() { asset(bank,name)?.relative().to_owned() } else { path.relative().to_owned() },
                mask_color: None,
                rect: [0, 0, w.try_into()?, height.try_into()?],
            });
        }
    }
    bail!("minori image missing: {name}")
}
fn text(id: u32, rect: [i32; 4], content: &str, selected: bool) -> Value {
    let color = if selected {
        [255, 224, 148, 255]
    } else {
        [255, 255, 255, 255]
    };
    put(
        id,
        rect,
        json!({"kind":"text","font":"default","size":24,"text":content,"color":color}),
        255,
    )
}
fn panel_text(layout: PanelLayout, id: u32, height: i32, line: &str) -> Value {
    // Panel position is H - bitmap height + 64. Both text areas start
    // at the profile-specific inset from the bottom, with 12 pixels of leading.
    let (x, right, top) = match layout {
        PanelLayout::Columns if id == SPEAKER_NAME => (65, 221, 208),
        PanelLayout::Columns => (169, 1235, 208),
        PanelLayout::Stacked => (120, 1160, 232),
    };
    let y = height + 64 - top + 12;
    let mut op = text(id, [x, y, right - x, height - y], line, false);
    op["layer"]["paint"]["size"] = json!(26);
    op
}
fn submit(
    published: &mut Published,
    h: &mut dyn HostServices,
    c: ClientId,
    mut layers: Vec<Value>,
) -> Result<()> {
    // Shared layer removal requires an existing ID. Native clears are
    // idempotent, including .chain before the first message is displayed.
    if layers.is_empty() { return Ok(()); }
    let mut next = published.ids.clone();
    if published.contains(&SCREEN_ROOT) {
        for op in &mut layers {
            if op["op"] == "put" {
                let id = op["layer"]["id"].as_u64().unwrap() as u32;
                if id != SCREEN_ROOT && id != STAGE_ROOT {
                    op["layer"]["parent"] = json!(parent(id));
                }
            }
        }
    }
    layers.retain(|op| match op["op"].as_str() {
        Some("put") => {
            next.insert(op["layer"]["id"].as_u64().unwrap() as u32);
            true
        }
        Some("remove") => next.remove(&(op["id"].as_u64().unwrap() as u32)),
        _ => true,
    });
    if !layers.is_empty() {
        published.send(h,c,&mut layers)?;
    }
    published.ids = next;
    Ok(())
}

fn parent(id: u32) -> u32 {
    if Foreground::is_aux(id) || id < 50 || (wscroll::LAYER..=wscroll::LAST).contains(&id) || (wscroll::SECOND..=wscroll::SECOND+6).contains(&id) || id >= WRAPPED_WORLD {
        STAGE_ROOT
    } else {
        SCREEN_ROOT
    }
}

impl Presentation {
    pub fn new(canvas: [i32; 2], screen_abi: ScreenAbi) -> Self {
        Self {
            canvas,
            screen_abi,
            ..Default::default()
        }
    }
    pub fn set_eye_sequences(&mut self, programs: Vec<Vec<(u8, u64)>>) -> Result<()> {
        ensure!(
            programs.is_empty()
                || programs.len() == 6
                    && programs.iter().all(|p| !p.is_empty()
                        && p.len() <= 128
                        && p.iter()
                            .all(|&(pose, time)| pose <= 2 && (1..=36000).contains(&time))),
            "invalid minori eye sequence table"
        );
        self.characters.eye_sequences = programs.into();
        Ok(())
    }
    pub fn leave_story(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if self.zoom.take().is_some_and(|z| z.published) { h.release_surface(c, ZOOM_PIXELS)?; }
        if self.tile_transition.is_some() { self.end_tile_transition(h, c)?; }
        if let Some((id, _)) = self.movie {
            h.media_request(c, MediaRequest::Stop { id })?;
        }
        for (slot,item) in self.breaths.iter().enumerate() {
            if item.as_ref().is_some_and(|b| b.published) {h.release_surface(c,breath::BASE+slot as u32)?;}
        }
        self.characters.stop_movie(h, c)?;
        self.release_rotations(h,c)?;
        for slot in 0..2{if let Some(effect)=self.crossfades[slot].take(){effect.release(h,c,&mut self.published,slot)?;}}
        for &channel in &self.audio_channels {
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.0,
                },
            )?;
        }
        self.cutin_catalogue.apply(
            h,
            c,
            &mut self.published,
            crate::vm::CutinCommand::ReleaseAll,
        )?;
        // Remove children before the two optional shake groups.
        let mut ops: Vec<_> = self
            .published
            .iter()
            .filter(|id| **id != SCREEN_ROOT && **id != STAGE_ROOT)
            .copied()
            .map(remove)
            .collect();
        ops.extend([remove(STAGE_ROOT), remove(SCREEN_ROOT)]);
        submit(&mut self.published, h, c, ops)?;
        for &slot in &self.filtered_slots {
            h.release_surface(c, slot)?;
        }
        for (snow, layer) in self.snow.iter().zip(WEATHER_LAYERS) {
            if snow.as_ref().is_some_and(|s| s.published) { h.release_surface(c, layer)?; }
        }
        self.characters.release_faces(h, c)?;
        if let Some(face) = &self.stage_face { face.release(h, c, STAGE_FACE)?; }
        for (i, effect) in self.crossfades.iter_mut().enumerate() {
            if let Some(fade) = effect.take() { fade.release(h, c, &mut self.published, i)?; }
        }
        let random = self.random.take();
        let speeds = self.speeds;
        let panel_color = self.panel_color;
        let eye_sequences = self.characters.eye_sequences.clone();
        let voice_marks = std::mem::take(&mut self.voice_marks);
        let voice_actors = std::mem::take(&mut self.voice_actors);
        let sound_settings = std::mem::take(&mut self.sound_settings);
        let effect_settings = self.effect_settings;
        let grid = self.tile_grid;
        let panel_layout = self.panel_layout;
        *self = Self::new(self.canvas, self.screen_abi);
        self.panel_layout = panel_layout;
        self.tile_grid = grid;
        self.characters.eye_sequences = eye_sequences;
        self.voice_actors = voice_actors;
        self.voice_marks = voice_marks;
        self.sound_settings = sound_settings;
        self.effect_settings = effect_settings;
        self.random = random;
        self.speeds = speeds;
        self.panel_color = panel_color;
        Ok(())
    }
    fn shake_roots(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if self.published.contains(&SCREEN_ROOT) {
            return Ok(());
        }
        let rect = [0, 0, self.canvas[0], self.canvas[1]];
        let mut ops = Vec::new();
        for id in [SCREEN_ROOT, STAGE_ROOT] {
            let mut group = put(id, rect, json!({"kind":"group"}), 255);
            if id == SCREEN_ROOT {
                group["layer"]["clip"] = json!(rect);
            } else {
                group["layer"]["parent"] = json!(SCREEN_ROOT);
            }
            ops.push(group);
        }
        for &id in self.published.iter() {
            ops.push(json!({"op":"set_parent","id":id,"parent":parent(id)}));
        }
        submit(&mut self.published, h, c, ops)
    }

    pub fn shaking(&self) -> bool {
        self.shake.is_some()
    }
    pub fn shake_blocks(&self) -> bool {
        self.shake.as_ref().is_some_and(|s| !s.background())
    }
    pub fn poll_shake(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        skip: bool,
        advance: bool,
    ) -> Result<()> {
        let Some(shake) = &mut self.shake else {
            return Ok(());
        };
        let id = if shake.background() {
            STAGE_ROOT
        } else {
            SCREEN_ROOT
        };
        let transform = if shake.finished(h.now_ms(), skip, advance) {
            self.shake = None;
            json!({})
        } else {
            let rng = self.random.get_or_insert_with(|| PatternRng::msvc(1));
            let Some(pose) = shake.sample(h.now_ms(), self.canvas, rng)? else {
                return Ok(());
            };
            pose
        };
        submit(
            &mut self.published,
            h,
            c,
            vec![json!({"op":"set_transform","id":id,"transform":transform})],
        )
    }

    pub fn character_active(&self) -> bool {
        self.characters.active() || !self.message_actions.is_empty() || self.stage_face.as_ref().is_some_and(face::Face::active)
    }
    pub fn character_moving(&self) -> bool {
        self.characters.moving()
    }
    pub fn poll_message_actions(&mut self, h: &mut dyn HostServices, c: ClientId, force: bool) -> Result<()> {
        let mut ops = Vec::new();
        let mut i = 0;
        while i < self.message_actions.len() {
            if self.characters.message_action(h, c, &mut self.message_actions[i], force, &mut ops)? {
                self.message_actions.remove(i);
            } else {
                i += 1;
            }
        }
        submit(&mut self.published, h, c, ops)
    }

    fn draw_stage_face(&self, ops: &mut Vec<Value>) {
        if let (Some(face), Some(art)) = (&self.stage_face, &self.foreground) {
            let mut rect = art.rect;
            rect[..2].copy_from_slice(&self.stage_face_position);
            face.draw(STAGE_FACE, rect, [rect[2], rect[3]], [self.world.len() as i32 + 1, 0], 255, ops);
        }
    }

    fn bind_stage_face(&mut self, h: &mut dyn HostServices, c: ClientId, ops: &mut Vec<Value>) -> Result<()> {
        if let Some(face) = self.stage_face.take() {
            face.remove(STAGE_FACE, ops);
            face.release(h, c, STAGE_FACE)?;
        }
        if let Some(art) = &self.fg_parts.main {
            self.stage_face_position = [art.rect[0], art.rect[1]];
            self.stage_face = Some(face::Face::open(h, c, art, true, self.characters.eye_preset.unwrap_or(2),self.characters.eye_sequences.clone())?);
        }
        Ok(())
    }

    pub fn poll_characters(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        advance: bool,
    ) -> Result<()> {
        let mut ops = Vec::new();
        let mut level = self
            .voice_meter
            .as_mut()
            .map(|m| m.level(h, c))
            .transpose()?
            .unwrap_or_default();
        level.generation = self.voice_generation;
        let rng = self.random.get_or_insert_with(|| PatternRng::msvc(1));
        self.characters
            .poll(h, c, advance, &self.speaker, level, rng, &mut ops)?;
        if let Some(face) = &mut self.stage_face
            && face.poll(h, c, STAGE_FACE, &self.speaker, level, rng)?
        {
            self.draw_stage_face(&mut ops);
        }
        let now = h.now_ms() / 10;
        let callback = (level.generation, level.serial);
        let new_callback = level.playing && self.last_breath_callback != Some(callback);
        self.last_breath_callback = level.playing.then_some(callback);
        if let Some(b) = self.breaths[0].as_mut()
            && new_callback
        {
            let mut centers = Vec::new();
            self.characters.mouth_centers(&self.speaker, &mut centers);
            if let (Some(face), Some(art)) = (&self.stage_face, &self.foreground) {
                let mut rect = art.rect;
                rect[..2].copy_from_slice(&self.stage_face_position);
                face.mouth_centers(&self.speaker, rect, [rect[2], rect[3]], &mut centers);
            }
            for center in centers {
                b.emit(center, level.amplitude, now, self.random.as_mut().unwrap())?;
            }
        }
        let interval = self.sound_settings.get("effectDrawInterval", 10).max(0) as u32;
        for (slot, b) in self.breaths.iter_mut().enumerate() {
            if let Some(b) = b {
                b.poll(h, c, slot, self.canvas, interval, &mut ops)?;
            }
        }
        submit(&mut self.published, h, c, ops)
    }

    pub fn snow_loading(&self) -> bool {
        self.snow.iter().flatten().any(|snow| !snow.published)
    }
    pub fn snow_active(&self) -> bool {
        self.breaths.iter().any(Option::is_some) || self.snow.iter().any(Option::is_some)
            || self.cutin.iter().any(Option::is_some)
            || self.walking_scroll.is_some() || self.second_scroll.is_some()
            || self.stage_walk.iter().any(Option::is_some) || self.scenic.iter().any(Option::is_some) || self.crossfades.iter().any(Option::is_some)
    }

    fn clear_snow(&mut self, h: &mut dyn HostServices, c: ClientId, slot: usize) -> Result<()> {
        if let Some(snow)=self.snow[slot].take() && snow.published {
            let layer = WEATHER_LAYERS[slot];
            submit(&mut self.published, h, c, vec![remove(layer)])?;
            h.release_surface(c, layer)?;
        }
        Ok(())
    }
    fn clear_cutin(&mut self, h: &mut dyn HostServices, c: ClientId, slot: usize) -> Result<()> {
        if self.cutin[slot].take().is_some() {
            submit(&mut self.published, h, c, vec![remove(CUTIN_LAYERS[slot])])?;
        }
        Ok(())
    }
    fn clear_walking_scroll(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if let Some(effect) = self.walking_scroll.take() {
            let mut ops = Vec::new();
            effect.remove(&mut self.characters, &mut ops);
            submit(&mut self.published, h, c, ops)?;
        }
        Ok(())
    }
    pub fn poll_walking_scroll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        vm: &Vm,
    ) -> Result<()> {
        for (slot, walk) in self.stage_walk.iter_mut().enumerate() {
            let Some(walk) = walk else { continue };
            let variable = |name: &str| vm.local(name).or_else(|| vm.global(name));
            let paused = variable("wscroll2pause").is_some() || (self.screen_abi == ScreenAbi::Early && variable("wscroll2Pause").is_some());
            let ratio = if self.screen_abi == ScreenAbi::Early { 5 } else { variable("wscroll2Ratio").unwrap_or(5) };
            if self.world.is_empty() || !walk.advance(h.now_ms(), paused, ratio)? { continue; }
            let layer = if slot == 0 { wscroll::LAYER } else { wscroll::SECOND };
            let mut ops = Vec::new();
            walk.draw(&self.world, layer, self.canvas, &mut ops);
            if let Some(fg) = &self.foreground {
                let camera = walk.foreground_offset(fg, self.canvas, self.fg_parts.walking_frame(&self.published));
                let mut art = fg.clone();
                art.rect[..2].copy_from_slice(&camera.map(i32::saturating_neg));
                let id = self.world.len() as u32 + 1;
                ops.push(Foreground::draw_main(&art, id, &fg.asset));
                self.fg_parts.walking_auxiliary(id, camera, |art| walk.secondary_offset(art, self.canvas), &mut ops);
                self.stage_face_position = [art.rect[0], art.rect[1]];
                if let Some(face) = &self.stage_face {
                    face.draw(STAGE_FACE, art.rect, [art.rect[2],art.rect[3]], [id as i32,0], 255, &mut ops);
                }
            }
            submit(&mut self.published, h, c, ops)?;
        }
        for effect in [&mut self.walking_scroll,&mut self.second_scroll].into_iter().flatten() {
            let mut ops = Vec::new();
            effect.draw(h.now_ms(), self.canvas, &mut self.characters, &mut ops);
            submit(&mut self.published, h, c, ops)?;
        }
        for slot in 0..2 {
            let mut ops=Vec::new();
            if let Some(effect)=&mut self.crossfades[slot]{ effect.poll(h,c,&mut self.published,slot,self.camera)?; }
            if let Some(effect)=&mut self.scenic[slot] {
                let bell=effect.draw(h.now_ms(),slot,self.canvas,&mut ops);
                submit(&mut self.published,h,c,ops)?;
                if let Some((index,volume))=bell {
                    let path=asset("sys","crossing.wav")?;
                    if h.storage(c)?.exists(&path) {
                        let channel=24+2*slot as u32+index;
                        h.submit_audio(c,AudioRequest::PlayAsset{
                            channel,source:AudioAsset{path,frame_limit:None},intro:None,
                            parameters:AudioParameters{bus:PcmBus::Sfx,gain:f32::from(volume)/100.,..Default::default()},
                        })?;
                        self.audio_channels.insert(channel);
                    }
                }
            }
        }
        Ok(())
    }
    fn clear_late_effect(&mut self,h:&mut dyn HostServices,c:ClientId,slot:usize)->Result<()>{
        if let Some(b)=self.breaths[slot].take() && b.published {
            let id=breath::BASE+slot as u32;
            submit(&mut self.published,h,c,vec![remove(id)])?;
            h.release_surface(c,id)?;
        }
        if self.stage_walk[slot].take().is_some() {
            let layer = if slot == 0 { wscroll::LAYER } else { wscroll::SECOND };
            let mut ops = vec![remove(layer), remove(layer + 1)];
            for (i, source) in self.world.iter().take(2).enumerate() {
                let mut art = source.clone();
                for axis in 0..2 { art.rect[axis] = art.rect[axis].saturating_sub(self.camera[axis].max(0)); }
                ops.push(art.layer(i as u32 + 1, 255));
            }
            if let Some(art) = &self.foreground {
                let id = self.world.len() as u32 + 1;
                ops.push(Foreground::draw_main(art, id, &art.asset));
                self.fg_parts.auxiliary(id, self.foreground_camera, &mut ops);
                self.stage_face_position = [art.rect[0], art.rect[1]];
                self.draw_stage_face(&mut ops);
            }
            submit(&mut self.published, h, c, ops)?;
        }
        if slot == 0 { self.clear_walking_scroll(h, c)?; }
        if let Some(w)=self.second_scroll.take_if(|_| slot == 1){
            let mut ops=Vec::new();w.remove(&mut self.characters,&mut ops);submit(&mut self.published,h,c,ops)?;
        }
        self.clear_snow(h,c,slot)?;
        self.clear_cutin(h,c,slot)?;
        if let Some(effect)=self.crossfades[slot].take(){effect.release(h,c,&mut self.published,slot)?;}
        if self.scenic[slot].take().is_some(){
            let base=SceneEffect::base(slot);
            submit(&mut self.published,h,c,vec![remove(base),remove(base+1)])?;
            for channel in (24+2*slot as u32)..=(25+2*slot as u32) {
                if self.audio_channels.remove(&channel) {h.submit_audio(c,AudioRequest::Stop{channel,fade_ms:0.})?;}
            }
        }
        self.late_requests[slot]=None;
        Ok(())
    }
    fn late_effect(&mut self,h:&mut dyn HostServices,c:ClientId,vm:&Vm,slot:usize,r:crate::vm::EffectRequest)->Result<()>{
        self.apply_late_effect(h, c, vm, slot, &r)?;
        // The native scenario record remembers controls (including fadeout)
        // separately from the arguments of the live object.
        self.last_effect_commands[slot] = Some(r);
        Ok(())
    }
    fn apply_late_effect(&mut self,h:&mut dyn HostServices,c:ClientId,vm:&Vm,slot:usize,r:&crate::vm::EffectRequest)->Result<()>{
        let current=if slot==0{&mut self.walking_scroll}else{&mut self.second_scroll};
        let snow=&mut self.snow[slot];
        let cutin=&mut self.cutin[slot];
        if self.last_effect_commands[slot].as_ref()==Some(r)
            && (self.stage_walk[slot].as_ref().is_some_and(|w| !w.retiring)
                || current.as_ref().is_some_and(|w|!w.retiring)
                || self.scenic[slot].as_ref().is_some_and(|s|!s.retiring)
                || self.crossfades[slot].as_ref().is_some_and(|s|!s.retiring)
                || snow.as_ref().is_some_and(Weather::reusable)
                || cutin.as_ref().is_some_and(|s| !s.retiring)) {return Ok(());}
        if self.screen_abi == ScreenAbi::PersistentZoom && !self.effect_settings.screen_effect
            && (r.kind != "WScrollST" || current.is_some() || snow.is_some() || cutin.is_some()
                || self.breaths[slot].is_some() || self.stage_walk[slot].is_some()
                || self.scenic[slot].is_some() || self.crossfades[slot].is_some()) {
            return self.clear_late_effect(h, c, slot);
        }
        if r.kind=="Breath" && self.breaths[slot].is_some() && self.late_requests[slot].as_ref().is_some_and(|old|old.resource==r.resource) {return Ok(());}
        match r.kind.as_str(){
            "Breath"=>{
                let b=breath::Breath::load(h,c)?;
                self.clear_late_effect(h,c,slot)?;
                self.breaths[slot]=Some(b);
            }
            "*"|""=>return self.clear_late_effect(h,c,slot),
            "end"=>{
                if let Some(b)=&mut self.breaths[slot]{b.retiring=true;}
                if let Some(w)=current{w.retiring=true;}
                if let Some(w)=&mut self.stage_walk[slot]{w.retiring=true;}
                if let Some(s)=&mut self.crossfades[slot]{s.retiring=true;}
                if let Some(s)=&mut self.scenic[slot]{s.retiring=true;}
                if let Some(s)=snow{s.retiring=true;}
                if let Some(s)=cutin{s.retiring=true;}
                return Ok(());
            }
            "fadeout"=>{
                if let Some(b)=&mut self.breaths[slot]{b.fading=true;}
                if let Some(s)=&mut self.scenic[slot]{s.fade();}
                if let Some(s)=snow{s.fade();}
                if let Some(s)=cutin{s.fade();}
                return Ok(());
            }
            "CrossFade"|"CrossFade2"=>{
                let next=crossfade::Crossfade::load(h,c,r)?;
                self.clear_late_effect(h,c,slot)?;
                self.crossfades[slot]=Some(next);
                self.poll_walking_scroll(h,c,vm)?;
            }
            "WScroll"|"Crossing"=>{
                let next=SceneEffect::load(h,c,r,self.world.first())?;
                self.clear_late_effect(h,c,slot)?;
                self.scenic[slot]=Some(next);
                self.poll_walking_scroll(h,c,vm)?;
            }
            "WScroll2" | "WScroll3" => {
                let next = wscroll::StageWalk::load(h, c, r, self.screen_abi == ScreenAbi::PersistentZoom)?;
                self.clear_late_effect(h, c, slot)?;
                self.stage_walk[slot] = Some(next);
                self.poll_walking_scroll(h, c, vm)?;
            }
            "WScrollST"=>{
                let next=wscroll::WalkingScroll::load(h,c,vm,r,if slot==0{wscroll::LAYER}else{wscroll::SECOND})?;
                self.clear_late_effect(h,c,slot)?;
                if slot==0{self.walking_scroll=Some(next)}else{self.second_scroll=Some(next)}
                self.poll_walking_scroll(h,c,vm)?;
            }
            "Cutin"=>{
                let path=asset("bg",&r.resource)?;
                let next=if h.storage(c)?.exists(&path){
                    Some(cutin::Cutin::new(sprite(h,c,&["bg"],&r.resource)?,self.canvas,&r.resource,r.first,r.second))
                }else{None};
                self.clear_late_effect(h,c,slot)?;
                self.cutin[slot]=next;
                self.poll_cutin_slot(h,c,slot)?;
            }
            "Firefly"=>{
                if let Some(s)=snow && s.retarget(&r.resource,r.first,r.second)? {
                    self.late_requests[slot]=Some(r.clone());
                    return Ok(());
                }
                let next=Weather::firefly(h.storage(c)?,self.random.get_or_insert_with(||PatternRng::msvc(1)),self.canvas,&r.resource,r.first,r.second)?;
                self.clear_late_effect(h,c,slot)?;
                self.snow[slot]=Some(next);
                self.poll_snow_slot(h,c,slot)?;
            }
            "Snow2"|"Snow2H"|"Snow2HR"=>{
                if let Some(prev)=&self.late_requests[slot] && prev.kind==r.kind && prev.resource==r.resource
                    && prev.second.unsigned_abs()==r.second.unsigned_abs() && prev.first!=r.first
                    && let Some(s)=snow && s.reusable() {
                    s.change_population(r.first)?;
                } else {
                    let next=Weather::snow2(h.storage(c)?,self.random.get_or_insert_with(||PatternRng::msvc(1)),self.canvas,r)?;
                    self.clear_late_effect(h,c,slot)?;
                    self.snow[slot]=Some(next);
                    self.poll_snow_slot(h,c,slot)?;
                }
            }
            "Rain"|"RainH"=>{
                let next=Weather::rain(h.storage(c)?,self.random.get_or_insert_with(||PatternRng::msvc(1)),self.canvas,r.kind=="RainH")?;
                self.clear_late_effect(h,c,slot)?;
                self.snow[slot]=Some(next);
                self.poll_snow_slot(h,c,slot)?;
            }
            "Snow"|"SnowH"|"Sakura"=>{
                let next=Weather::snow(h.storage(c)?,self.random.get_or_insert_with(|| PatternRng::msvc(1)),self.canvas,r.kind=="SnowH",r.kind=="Sakura")?;
                self.clear_late_effect(h,c,slot)?;
                self.snow[slot]=Some(next);
                self.poll_snow_slot(h,c,slot)?;
            }
            _=>bail!("unsupported minori effect in slot {}: {:?}",slot+1,r),
        }
        self.late_requests[slot]=Some(r.clone());
        Ok(())
    }

    pub fn loading_cutin(&self) -> bool {
        self.cutin_catalogue.loading()
    }

    pub fn poll_cutin_catalogue(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        self.cutin_catalogue.poll(h, c, &mut self.published)
    }

    pub fn poll_cutin(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        for slot in 0..2 { if self.audio_ticket.is_none() { self.poll_cutin_slot(h,c,slot)?; } }
        Ok(())
    }
    fn poll_cutin_slot(&mut self, h:&mut dyn HostServices, c:ClientId, slot:usize)->Result<()>{
        use cutin::Draw;
        let Some(effect) = self.cutin[slot].as_mut() else {
            return Ok(());
        };
        let action = effect.step(h.now_ms());
        match action {
            Draw::None => Ok(()),
            Draw::Finished => self.clear_cutin(h, c, slot),
            Draw::Move | Draw::Arrived => {
                let sound = if matches!(action, Draw::Arrived) {
                    effect.sound.take()
                } else {
                    None
                };
                let mut layer = effect.art.layer(CUTIN_LAYERS[slot], 255);
                layer["layer"]["order"] = json!([49, 2+2*slot]);
                submit(&mut self.published, h, c, vec![layer])?;
                if let Some(sound) = sound {
                    self.sound(
                        h,
                        c,
                        18+slot as u32,
                        "se",
                        &sound,
                        AudioParameters {
                            bus: PcmBus::Sfx,
                            ..Default::default()
                        },
                    )?;
                }
                Ok(())
            }
        }
    }

    pub fn finish_stage(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        if self
                .walking_scroll
                .as_ref()
                .is_some_and(|effect| effect.retiring)
        {
            self.clear_walking_scroll(h, c)?;
        }
        if self.second_scroll.as_ref().is_some_and(|e|e.retiring) {
            self.clear_late_effect(h,c,1)?;
        }
        for slot in 0..2 {
            if self.breaths[slot].as_ref().is_some_and(|b|b.retiring) || self.stage_walk[slot].as_ref().is_some_and(|w| w.retiring) || self.scenic[slot].as_ref().is_some_and(|s|s.retiring) || self.crossfades[slot].as_ref().is_some_and(|s|s.retiring){self.clear_late_effect(h,c,slot)?;}
            if self.cutin[slot].as_ref().is_some_and(|effect|effect.retiring) { self.clear_cutin(h,c,slot)?; }
            if self.snow[slot].as_ref().is_some_and(|effect|effect.retiring) { self.clear_snow(h,c,slot)?; }
        }
        Ok(())
    }

    pub fn poll_snow(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        for slot in 0..2 { self.poll_snow_slot(h,c,slot)?; }
        Ok(())
    }
    fn poll_snow_slot(&mut self, h:&mut dyn HostServices,c:ClientId,slot:usize)->Result<()>{
        let layer_id=WEATHER_LAYERS[slot];
        let Some(snow) = &mut self.snow[slot] else {
            return Ok(());
        };
        if let Some(frame) = snow.frame()? {
            let path = h.present_surface(c, layer_id, frame.width, frame.height, &frame.rgba)?;
            if !snow.published {
                let mut layer = put(
                    layer_id,
                    [0, 0, self.canvas[0], self.canvas[1]],
                    json!({"kind":"image","asset":path.relative()}),
                    255,
                );
                layer["layer"]["order"] = json!([49, 1+2*slot]);
                submit(&mut self.published, h, c, vec![layer])?;
                snow.published = true;
            }
        }
        let random = self.random.get_or_insert_with(|| PatternRng::msvc(1));
        if !snow.advance(h.now_ms(), random, self.effect_settings.snow_interval)? {
            self.clear_snow(h, c, slot)?;
        }
        Ok(())
    }

    fn skips_retail_transition(&self, config: &TransitionConfig) -> bool {
        self.tile_grid.is_some_and(|grid| !self.effect_settings.screen_effect || grid.finishes_immediately(config.duration))
    }

    fn stage(&mut self, h: &mut dyn HostServices, c: ClientId, r: StageRequest) -> Result<()> {
        if let Some(config) = &self.transition && (2..=13).contains(&config.transition_type) {
            ensure!(self.tile_grid.is_some(),
                "unsupported minori stage transition {}", config.transition_type);
        }
        let [x, y] = [r.x.max(0), r.y.max(0)];
        let mut items = Vec::new();
        // The native stage clears a canvas-sized 32-bit background for '*'.
        // Keep a world plane so camera, rotation and standing-image indices
        // remain the same as for a bank-backed background.
        let clear_background = r.image == "*";
        let canvas = self.canvas;
        let mut bg = if clear_background {
            Sprite { asset: String::new(), mask_color: None, rect: [0, 0, canvas[0], canvas[1]] }
        } else {
            sprite(h, c, &["sys", "st", "bg"], &r.image)?
        };
        bg.rect[0] = -x;
        bg.rect[1] = -y;
        items.push(bg);
        let mut overrides = std::collections::BTreeMap::new();
        for extra in r.extras {
            let name = if let Some((name, suffix)) = extra.name.rsplit_once(':') {
                let material = self.strips.standing(h.storage(c)?.as_ref(), suffix)?;
                overrides.insert(items.len(), material);
                name
            } else {
                &extra.name
            };
            let mut art = sprite(h, c, &["st", "bg", "sys"], name)?;
            art.rect[0] = extra
                .first
                .saturating_sub(art.rect[2] / 2)
                .saturating_sub(x);
            art.rect[1] = (if extra.first == extra.second {
                self.canvas[1] - art.rect[3]
            } else {
                extra.second
            })
            .saturating_sub(y);
            items.push(art);
        }
        if let Some(config) = &self.transition && config.transition_type == 2 && !self.skips_retail_transition(config) {
            let rect = [0, 0, canvas[0], canvas[1]];
            let plain = |s: &Sprite| s.rect == rect && s.asset.to_ascii_lowercase().ends_with(".png")
                && !s.requires_frame_publication();
            ensure!(self.world.len() == 1 && items.len() == 1 && r.base_spec == "*" && r.x == 0 && r.y == 0
                && self.camera == [0,0] && self.filtered_slots.is_empty() && !self.strips.active()
                && self.published.iter().all(|id| matches!(*id, 1 | STAGE_ROOT | SCREEN_ROOT))
                && !self.rotation_active(false) && !self.shaking() && !self.scrolling()
                && plain(&self.world[0]) && plain(&items[0]), "unsupported minori stage transition 2 with composite sources");
            self.zoom = Some(zoom_transition::Zoom::begin(h,c,&self.world[0],items.remove(0),config.duration)?);
            self.transition_until = Some(u64::MAX);
            return Ok(());
        }
        let world_len = items.len();
        let fg=Foreground::load(h,c,&r.base_spec,[r.base_x,r.base_y], &["sys","st","bg"])?;
        if let Some(config) = &self.transition
            && (3..=13).contains(&config.transition_type)
            && !self.skips_retail_transition(config)
        {
            // The native worker retains the incoming pixels at the instant of each copy.
            // A mask of a changing scene cannot reproduce this history.
            let animated_background = items.len() == 1 && r.base_spec == "*"
                && items[0].asset.to_ascii_lowercase().ends_with(".sqz")
                && items[0].rect == [0, 0, canvas[0], canvas[1]] && !self.strips.active();
            let moving = self.character_active() || self.snow_active() || self.sequences_active() || self.sequence_pending()
                || self.animation_playing(-1) || self.rotation_active(false) || self.rotation_active(true)
                || self.shaking() || self.scrolling() || self.fade.is_some() || self.movie.is_some()
                || self.fg_ready.is_some() || fg.requires_frame_publication()
                || (!animated_background && items.iter().any(Sprite::requires_frame_publication));
            let incoming_face = match &fg.main {
                Some(art) => face::has_companion(h, c, art)?,
                None => false,
            };
            ensure!(!moving && !incoming_face && self.filtered_slots.is_empty(),
                "unsupported minori tile transition {} with changing or replaced stage surfaces", config.transition_type);
        }
        if let Some(top)=&fg.main {items.push(top.clone());}
        if clear_background || self.strips.active() || !overrides.is_empty() || !r.fixed_poses.is_empty() {
            let sources = items
                .iter()
                .enumerate()
                .skip(usize::from(clear_background))
                .map(|(i, s)| {
                    let slot = if i == 0 {
                        1
                    } else if i < world_len {
                        2
                    } else {
                        0
                    };
                    (i, slot, s.asset.clone())
                })
                .collect();
            let strips = self.strips.clone();
            let storage = h.storage(c)?;
            self.preparing_stage = Some(StagePreparation {
                fg,
                items,
                world_len,
                camera: [r.x, r.y],
                job: crate::worker(move || {
                    let mut images = strips.render(storage, sources, overrides, &r.fixed_poses)?;
                    if clear_background {
                        // Both the colour and strip filters preserve zero-alpha pixels.
                        let clear = crysirror_image::ImageSurface::filled(
                            canvas[0].try_into()?, canvas[1].try_into()?, [0; 4],
                        )?;
                        images.push((0, clear));
                    }
                    images.into_iter().map(|(i, s)| {
                        let image = crysirror_core::image_processing::RgbaImage::new(c, [s.width,s.height], s.rgba.into())?;
                        Ok((i, std::sync::Arc::new(image) as StageImage))
                    }).collect()
                })?,
            });
            return Ok(());
        }
        self.publish_stage(h, c, items, world_len, [r.x, r.y], fg, Vec::new())
    }

    pub fn preparing_stage(&self) -> bool {
        self.preparing_stage.is_some()
    }

    pub fn poll_stage(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        let Some(stage) = &self.preparing_stage else {
            return Ok(true);
        };
        let Some(rendered) = crate::poll(&stage.job)? else {
            return Ok(false);
        };
        let stage = self.preparing_stage.take().unwrap();
        self.publish_stage(h, c, stage.items, stage.world_len, stage.camera, stage.fg, rendered)?;
        Ok(true)
    }

    fn publish_stage(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        mut items: Vec<Sprite>,
        world_len: usize,
        camera: [i32; 2],
        fg: Foreground,
        rendered: Vec<(usize, StageImage)>,
    ) -> Result<()> {
        let mut slots = Vec::new();
        let mut inputs = std::collections::BTreeMap::new();
        for (i, image) in rendered {
            let [width, height] = image.size();
            let slot = 1000 + u32::try_from(i)?;
            items[i].asset = h
                .present_surface(c, slot, width, height, &image.rgba())?
                .relative()
                .to_owned();
            inputs.insert(items[i].asset.clone(), image);
            slots.push(slot);
        }
        for &slot in &self.filtered_slots {
            if !slots.contains(&slot) {
                h.release_surface(c, slot)?;
            }
        }
        self.filtered_slots = slots;
        let mut world = items[..world_len].to_vec();
        for art in &mut world {
            art.rect[0] = art.rect[0].saturating_add(camera[0].max(0));
            art.rect[1] = art.rect[1].saturating_add(camera[1].max(0));
        }
        let count = u32::try_from(items.len())?;
        let mut layers: Vec<_> = items
            .iter()
            .enumerate()
            .map(|(i, s)| s.layer(i as u32 + 1, 255))
            .collect();
        if let Some(main)=&fg.main { foreground::blend(&mut layers[world_len],&main.asset); }
        fg.auxiliary(world_len as u32+1,fg.main.as_ref().map_or([0;2],|m|[m.rect[0].saturating_neg(),m.rect[1].saturating_neg()]), &mut layers);
        self.characters.stage(h, c, &mut layers)?;
        layers.extend((0..self.wrap_layers).map(|i| remove(WRAPPED_WORLD + i)));
        layers.extend((count..self.stage_count).map(|i| remove(i + 1)));
        self.transition_until = None;
        let mut pending_tiles = None;
        let mut tile_frames = None;
        if let Some(config) = &self.transition {
            match config.transition_type {
                2..=13 if self.skips_retail_transition(config) => {}
                3..=13 if let Some(grid) = self.tile_grid => {
                    let mut tiles = crate::tile_transition::TileTransition::new(
                        grid, self.canvas, config.transition_type, config.duration,
                        h.now_ms(), self.random.get_or_insert_with(|| PatternRng::msvc(1)),
                    )?;
                    tiles.update(h.now_ms(), self.effect_settings.transition_interval)?;
                    let mask = &tiles.mask;
                    let path = h.present_surface(c, TILE_MASK, mask.width, mask.height, &mask.rgba)?;
                    layers.push(json!({"op":"transition","transition":{"effect":{
                        "kind":"byte_rule","asset":path.relative(),"invert":false,"soft":false,
                        "numerator":0,"denominator":1,"step_ms":1},"duration_ms":u64::MAX}}));
                    if items[0].asset.to_ascii_lowercase().ends_with(".sqz") {
                        tile_frames = Some(sequences::TileFrames {
                            current: None,
                            history: crysirror_image::ImageSurface::filled(mask.width, mask.height, [0; 4])?,
                        });
                    }
                    pending_tiles = Some(tiles);
                }
                0 => {
                    if config.duration <= 0 {
                        self.transition_until = Some(u64::MAX);
                        return Ok(());
                    }
                    let duration = 2560_u64.div_ceil(config.duration as u64) * 10;
                    layers.push(json!({"op":"transition","transition":{"effect":{"kind":"crossfade"},"duration_ms":duration}}));
                    self.transition_until = Some(h.now_ms().saturating_add(duration));
                }
                1 | 18 if config.mask.len() <= 1 => {}
                1 | 18 => {
                    let mask = asset("sys", &config.mask)?;
                    if h.storage(c)?.exists(&mask) {
                        let size = h.image_size(c, mask.relative())?;
                        ensure!(
                            size == self.canvas.map(|v| v as u32),
                            "minori transition rule dimensions"
                        );
                        // Default transitionDrawInterval is five 10 ms ticks.
                        let speed = config.duration as u32;
                        let soft = config.transition_type == 1;
                        if speed == 0 {
                            self.transition_until = Some(u64::MAX);
                            return Ok(());
                        }
                        let first_final =
                            (if soft { 5120_u64 } else { 2560_u64 }).div_ceil(5 * u64::from(speed));
                        let duration = (first_final + u64::from(!soft)).max(1) * 50;
                        layers.push(json!({"op":"transition","transition":{"effect":{
                            "kind":"byte_rule","asset":mask.relative(),"invert":true,"soft":soft,
                            "numerator":speed,"denominator":2,"step_ms":50},"duration_ms":duration}}));
                        self.transition_until = Some(h.now_ms().saturating_add(duration));
                    }
                }
                14..=17 if self.screen_abi != ScreenAbi::PersistentZoom => {
                    let axes = match config.transition_type {
                        16 => [false, true],
                        17 => [true, false],
                        _ => [true, true],
                    };
                    let extent = self.canvas[usize::from(!axes[0])] as u64;
                    let speed = config.duration as u32;
                    if speed == 0 {
                        self.transition_until = Some(u64::MAX);
                        return Ok(());
                    }
                    let duration = (2 * (extent + 1)).div_ceil(u64::from(speed)).max(1) * 50;
                    layers.push(json!({"op":"transition","transition":{"effect":{
                        "kind":"pixel_stretch","canvas":self.canvas,"axes":axes,"shrink_outgoing":config.transition_type==15,
                        "numerator":speed,"denominator":2,"step_ms":50},"duration_ms":duration}}));
                    self.transition_until = Some(h.now_ms().saturating_add(duration));
                }
                1..=18 => bail!(
                    "unsupported minori stage transition {}",
                    config.transition_type
                ),
                _ => {}
            }
        }
        if let Err(error) = submit(&mut self.published, h, c, layers) {
            if pending_tiles.is_some() { h.release_surface(c, TILE_MASK)?; }
            return Err(error);
        }
        if let Some(tiles) = pending_tiles {
            self.published.tile_frames = tile_frames;
            self.tile_transition = Some(tiles);
            self.transition_until = Some(u64::MAX);
        }
        self.release_rotations(h, c)?;
        self.filtered_inputs = inputs;
        self.fg_parts=fg;
        self.foreground = items.get(world_len).cloned();
        self.foreground_camera = self.foreground.as_ref().map_or([0, 0], |s| {
            [s.rect[0].saturating_neg(), s.rect[1].saturating_neg()]
        });
        self.wrap_layers = 0;
        self.world = world;
        let mut face_ops = Vec::new();
        self.bind_stage_face(h, c, &mut face_ops)?;
        submit(&mut self.published, h, c, face_ops)?;
        self.camera = camera;
        self.scroll = None;
        self.stage_count = count;
        Ok(())
    }

    fn voice(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        name: &str,
        volume: i32,
        pan: i32,
    ) -> Result<()> {
        self.voice_generation = self.voice_generation.wrapping_add(1);
        self.voice_meter = None;
        // The native voice command releases its occupied slot before it opens the replacement.
        if self.audio_channels.remove(&1) {
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel: 1,
                    fade_ms: 0.,
                },
            )?;
        }
        if !name.is_empty()
            && name != "*"
            && self.sound_settings.voice_enabled(name, &self.voice_actors)
        {
            if !h.storage(c)?.exists(&asset("voice", name)?) {
                return Ok(());
            }
            self.voice_meter = Some(VoiceMeter::open(h, c, name)?);
            let volume = self.sound_settings.get("volumeVoice", 100).clamp(0, 100) * volume / 100;
            let volume = if self.screen_abi == ScreenAbi::PersistentZoom {
                volume - 5
            } else {
                volume
            };
            // The native DirectSound table is in hundredths of a decibel. The
            // host balances stereo at constant power, so convert both target gains.
            let (scale, pan) = audio_fade::balance(pan);
            self.sound(
                h,
                c,
                1,
                "voice",
                name,
                AudioParameters {
                    bus: PcmBus::Voice,
                    gain: audio_fade::amplitude(volume) * scale,
                    pan,
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    }

    fn sound(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        channel: u32,
        bank: &str,
        name: &str,
        params: AudioParameters,
    ) -> Result<()> {
        if name.is_empty() {
            return Ok(());
        }
        if name == "*" {
            h.submit_audio(
                c,
                AudioRequest::Stop {
                    channel,
                    fade_ms: params.replace_fade_ms,
                },
            )?;
            return Ok(());
        }
        ensure!(
            self.audio_ticket.is_none(),
            "minori audio publication already pending"
        );
        let path = asset(bank, name)?;
        if !h.storage(c)?.exists(&path) {
            return Ok(());
        }
        self.audio_ticket = Some(h.begin_audio(
            c,
            AudioRequest::PlayAsset {
                channel,
                source: AudioAsset {
                    path,
                    frame_limit: None,
                },
                intro: None,
                parameters: params,
            },
        )?);
        self.audio_channels.insert(channel);
        Ok(())
    }

    fn set_fg(&mut self,h:&mut dyn HostServices,c:ClientId,name:&str)->Result<()> {
        let parts=Foreground::load(h,c,name,[0,self.foreground_camera[1].saturating_neg()],&["bg"])?;
        if let Some(main)=&parts.main && self.strips.active() && !main.asset.ends_with(".sqz") {
            let source=main.asset.clone();let strips=self.strips.clone();let storage=h.storage(c)?;
            let job=crate::worker(move||strips.render(storage,vec![(0,0,source)],Default::default(), &[]))?;
            self.fg_ready=Some(PendingForeground{parts,job});return Ok(());
        }
        self.publish_fg(h,c,parts,None)
    }
    pub fn foreground_pending(&self)->bool{self.fg_ready.is_some()}
    pub fn poll_foreground(&mut self,h:&mut dyn HostServices,c:ClientId)->Result<()>{
        if let Some(p)=&self.fg_ready && let Some(mut frames)=crate::poll(&p.job)? {
            let p=self.fg_ready.take().unwrap();self.publish_fg(h,c,p.parts,frames.pop().map(|(_,s)|s))?;
        }
        Ok(())
    }
    fn publish_fg(&mut self,h:&mut dyn HostServices,c:ClientId,parts:Foreground,filtered:Option<crysirror_image::ImageSurface>)->Result<()> {
        let id=self.world.len() as u32+1;
        let mut main=parts.main.clone();let mut ops=Vec::new();
        if let Some(art)=&mut main {
            let source=art.asset.clone();
            if let Some(s)=filtered {
                let slot=1000+id-1;
                art.asset=h.present_surface(c,slot,s.width,s.height,&s.rgba)?.relative().to_owned();
                let image=crysirror_core::image_processing::RgbaImage::new(c,[s.width,s.height],s.rgba.into())?;
                self.filtered_inputs.insert(art.asset.clone(),std::sync::Arc::new(image));
                if !self.filtered_slots.contains(&slot){self.filtered_slots.push(slot);}
            }
            ops.push(Foreground::draw_main(art,id,&source));
        } else {ops.push(remove(id));}
        self.foreground_camera[0]=0;
        parts.auxiliary(id,self.foreground_camera,&mut ops);
        submit(&mut self.published,h,c,ops)?;
        self.stage_count=self.world.len() as u32+u32::from(main.is_some());
        self.foreground=main;self.fg_parts=parts;
        let mut ops = Vec::new();
        self.bind_stage_face(h,c,&mut ops)?;
        submit(&mut self.published,h,c,ops)
    }
    pub fn animation_playing(&self,id:i32)->bool{self.animations.playing(id)}
    pub fn animation_loading(&self)->bool{self.animations.loading()}
    pub fn poll_animations(&mut self,h:&mut dyn HostServices,c:ClientId)->Result<()>{
        self.animations.poll(h,c,&self.characters,&mut self.published)
    }
    pub fn sequence_pending(&self) -> bool { self.published.pending() }
    pub fn sequences_active(&self) -> bool { self.published.active() }
    pub fn poll_sequences(&mut self,h:&mut dyn HostServices,c:ClientId)->Result<()> { self.published.tick(h,c) }

    pub fn scrolling(&self) -> bool {
        self.scroll.is_some()
    }
    pub fn poll_scroll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        skip: bool,
        advance: bool,
    ) -> Result<()> {
        let Some(scroll) = &self.scroll else {
            return Ok(());
        };
        let now = h.now_ms();
        let wave = matches!(scroll.motion, Motion::Wave { .. });
        let fg = matches!(scroll.motion, Motion::Foreground(_));
        let native_plane = wave || fg || matches!(scroll.motion, Motion::Accelerated { .. });
        let interruptible = !matches!(
            scroll.motion,
            Motion::Accelerated {
                interruptible: false,
                ..
            }
        );
        let stop = native_plane && (skip || (advance && interruptible));
        if native_plane
            && !stop
            && self
                .last_scroll_draw
                .is_some_and(|time| now / 10 < time + 2)
        {
            return Ok(());
        }
        let next = if stop && !wave {
            scroll.target
        } else {
            scroll.position(now)
        };
        let complete = stop || scroll.finished(now, next);
        if fg {
            if let Some(art) = &self.foreground
                && next != self.foreground_camera
                && art.rect[2] >= self.canvas[0]
                && art.rect[3] >= self.canvas[1]
            {
                let mut art = art.clone();
                art.rect[0] = next[0].saturating_neg();
                art.rect[1] = next[1].saturating_neg();
                let id=self.world.len() as u32+1;
                let name=self.fg_parts.main.as_ref().map_or(art.asset.as_str(),|m|m.asset.as_str());
                let mut ops=vec![Foreground::draw_main(&art,id,name)];
                self.fg_parts.auxiliary(id,next,&mut ops);
                self.stage_face_position = [art.rect[0], art.rect[1]];
                self.draw_stage_face(&mut ops);
                submit(&mut self.published,h,c,ops)?;
            }
            self.foreground_camera = next;
        } else if next != self.camera || wave || self.wrap_layers != 0 {
            let mut layers = Vec::new();
            let [mut x, mut y] = next.map(|v| v.max(0));
            let mut wrapped = 0;
            if wave {
                let bg = &self.world[0];
                let width = bg.rect[2];
                x = next[0].rem_euclid(width);
                y = y.min((bg.rect[3] - self.canvas[1]).max(0));
                let seam = width - x;
                if seam < self.canvas[0] {
                    for (i, art) in self.world.iter().enumerate() {
                        let mut copy = art.clone();
                        copy.rect[0] = copy.rect[0].saturating_add(seam);
                        copy.rect[1] = copy.rect[1].saturating_sub(y);
                        let mut op = copy.layer(WRAPPED_WORLD + i as u32, 255);
                        op["layer"]["order"] = json!([i as u32 + 1, 1]);
                        op["layer"]["viewport_clip"] = json!([seam, 0, width, self.canvas[1]]);
                        layers.push(op);
                        wrapped += 1;
                    }
                }
            }
            for (i, art) in self.world.iter().enumerate() {
                let mut sprite = art.clone();
                sprite.rect[0] = sprite.rect[0].saturating_sub(x);
                sprite.rect[1] = sprite.rect[1].saturating_sub(y);
                let mut op = sprite.layer(i as u32 + 1, 255);
                if wave {
                    op["layer"]["viewport_clip"] =
                        json!([0, 0, self.world[0].rect[2] - x, self.canvas[1]]);
                }
                layers.push(op);
            }
            layers.extend((wrapped..self.wrap_layers).map(|i| remove(WRAPPED_WORLD + i)));
            submit(&mut self.published, h, c, layers)?;
            self.wrap_layers = wrapped;
            self.camera = next;
        }
        self.last_scroll_draw = Some(now / 10);
        if complete {
            self.scroll = None
        }
        Ok(())
    }

    pub fn revealing(&self) -> bool {
        self.message.is_some()
    }
    pub fn poll_message(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        finish: bool,
    ) -> Result<bool> {
        let layout = self.panel_layout.filter(|_| self.panel_mode == 1);
        let message = self.message.as_mut().context("missing minori message")?;
        let first_action = self.message_actions.len();
        let (update, done) = message.frame(h.now_ms(), finish, self.audio_ticket.is_some() || (self.audio_channels.contains(&1) && h.audio_playing(c, 1)?), &mut self.message_actions);
        for action in &mut self.message_actions[first_action..] { action.generation = self.characters.generation(action.id); }
        if let Some(line) = update {
            submit(
                &mut self.published,
                h,
                c,
                vec![if let Some(layout) = layout { panel_text(layout, TEXT, self.canvas[1], line) } else { text(
                    TEXT,
                    [28, self.canvas[1] - 163, self.canvas[0] - 56, 154],
                    line,
                    false,
                ) }],
            )?
        }
        if done {
            self.auto_advance_message = message.auto_advance;
            self.message = None
        }
        Ok(done)
    }
    fn end_tile_transition(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        self.published.finish_tiles(h, c)?;
        submit(&mut self.published, h, c, vec![json!({"op":"end_transition"})])?;
        h.release_surface(c, TILE_MASK)?;
        self.tile_transition = None;
        self.transition_until = None;
        Ok(())
    }

    pub fn transition_done(&mut self, h: &mut dyn HostServices, c: ClientId, skip: bool) -> Result<bool> {
        use crate::tile_transition::Update;
        let now = h.now_ms();
        if let Some(zoom) = &mut self.zoom {
            use zoom_transition::Draw;
            match zoom.draw(now, self.effect_settings.transition_interval, skip)? {
                Draw::Hold => return Ok(false),
                Draw::Pixels(image) => {
                    let path=h.present_surface(c,ZOOM_PIXELS,image.width,image.height,&image.rgba)?;
                    if !zoom.published {
                        let mut sprite=zoom.target.clone(); sprite.asset=path.relative().to_owned();
                        submit(&mut self.published,h,c,vec![sprite.layer(1,255)])?;
                        zoom.published=true;
                    }
                    return Ok(false);
                }
                Draw::Finished => {
                    submit(&mut self.published,h,c,vec![zoom.target.layer(1,255)])?;
                    if zoom.published {h.release_surface(c,ZOOM_PIXELS)?;}
                    self.world[0]=self.zoom.take().unwrap().target;
                    self.transition_until=None;
                    return Ok(true);
                }
            }
        }
        let Some(tiles) = &mut self.tile_transition else {
            return Ok(self.transition_until.is_none_or(|until| now >= until));
        };
        let update = if skip {
            Update::Finished
        } else if let Some(frames) = &mut self.published.tile_frames {
            let Some(source) = &frames.current else { return Ok(false) };
            tiles.draw(now, self.effect_settings.transition_interval, Some((source, &mut frames.history)))?
        } else {
            tiles.update(now, self.effect_settings.transition_interval)?
        };
        match update {
            Update::Finished => { self.end_tile_transition(h, c)?; Ok(true) }
            Update::Changed => {
                self.published.publish_tiles(h, c)?;
                let mask = &tiles.mask;
                h.present_surface(c, TILE_MASK, mask.width, mask.height, &mask.rgba)?;
                Ok(false)
            }
            Update::Unchanged => Ok(false),
        }
    }

    fn panel_sprites(
        &self,
        h: &mut dyn HostServices,
        c: ClientId,
        mode: i32,
        image: &str,
    ) -> Result<Vec<Sprite>> {
        if matches!(mode, 0 | 7) {
            return Ok(Vec::new());
        }
        if mode == 10 && h.storage(c)?.exists(&asset("sys", "topmenutxt.png")?) {
            let mut art = sprite(h, c, &["sys"], "msgPanel.png")?;
            art.rect[0] = 17;
            art.rect[1] = 412;
            art.mask_color = Some(self.panel_color);
            return Ok(vec![art]);
        }
        let strip = matches!(mode, 3 | 5 | 6);
        let mut band_height = 0;
        let default = match mode {
            1 => "msgPanel.png".to_owned(),
            2 => "fullPanel.png".to_owned(),
            4 => "fullPanel2.png".to_owned(),
            3 | 5 | 6 => {
                // The installed system bank defines the native letterbox geometry.
                let width = self.canvas[0];
                let storage = h.storage(c)?;
                for height in [75, 80] {
                    if storage.exists(&asset("sys", &format!("black{width}x{height}.png"))?) {
                        band_height = height;
                        break;
                    }
                }
                ensure!(
                    band_height > 0,
                    "missing minori letterbox geometry for {width}"
                );
                let prefix = if mode == 5 { "trans" } else { "black" };
                format!("{prefix}{width}x{band_height}.png")
            }
            _ => bail!("unsupported minori panel mode {mode}"),
        };
        let mut art = sprite(
            h,
            c,
            &["sys"],
            if image.is_empty() || image == "*" {
                &default
            } else {
                image
            },
        )?;
        if mode == 1 {
            art.rect[1] = self.canvas[1] - art.rect[3];
            if self.panel_layout.is_some() { art.rect[1] += 64; }
        } else if strip && mode != 6 {
            let top = art.clone();
            art.rect[1] = self.canvas[1] - band_height;
            return Ok(vec![art, top]);
        }
        Ok(vec![art])
    }

    pub fn poll_panel(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        let fade = self
            .fade
            .as_ref()
            .context("missing minori panel transition")?;
        let ticks = h.now_ms().saturating_sub(fade.start) / 10;
        let done = ticks >= 256 / fade.speed;
        let opacity = if done {
            255
        } else {
            ticks.saturating_mul(fade.speed).min(255) as u8
        };
        let mut layers = Vec::with_capacity(5);
        for (slot, (old, new)) in [(OLD_PANEL, PANEL), (97, 98)].into_iter().enumerate() {
            layers.push(
                self.panel
                    .get(slot)
                    .filter(|_| !done)
                    .map_or_else(|| remove(old), |s| s.layer(old, 255 - opacity)),
            );
            layers.push(
                fade.next
                    .get(slot)
                    .map_or_else(|| remove(new), |s| s.layer(new, opacity)),
            );
        }
        if done && fade.next.is_empty() {
            layers.extend([remove(TEXT), remove(SPEAKER_NAME)])
        }
        submit(&mut self.published, h, c, layers)?;
        if done {
            self.panel = self.fade.take().unwrap().next
        }
        Ok(done)
    }

    pub fn poll_movie(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        activate: bool,
    ) -> Result<bool> {
        let (id, skippable) = self.movie.context("missing minori movie")?;
        let phase = h.media_info(c, id)?.phase;
        ensure!(phase != MediaPhase::Failed, "minori movie decode failed");
        if (skippable && activate) || matches!(phase, MediaPhase::Ended | MediaPhase::Stopped) {
            h.media_request(c, MediaRequest::Stop { id })?;
            submit(&mut self.published, h, c, vec![remove(MOVIE)])?;
            if self.movie_soundtrack {
                h.submit_audio(
                    c,
                    AudioRequest::Stop {
                        channel: MOVIE,
                        fade_ms: 0.0,
                    },
                )?;
                self.movie_soundtrack = false;
            }
            self.movie = None;
            return Ok(true);
        }
        Ok(false)
    }
    fn draw_choices(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<()> {
        submit(
            &mut self.published,
            h,
            c,
            self.choices
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    text(
                        CHOICE + i as u32,
                        [64, 50 + 32 * i as i32, self.canvas[0] - 128, 32],
                        &v.text,
                        i == self.selected,
                    )
                })
                .collect(),
        )
    }
    pub fn select(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        input: InputFrame,
    ) -> Result<Option<usize>> {
        let n = self.choices.len();
        ensure!(n > 0, "empty minori choices");
        let keys = input.buttons_pressed;
        if keys & buttons::UP != 0 {
            self.selected = (self.selected + n - 1) % n;
            self.draw_choices(h, c)?
        }
        if keys & buttons::DOWN != 0 {
            self.selected = (self.selected + 1) % n;
            self.draw_choices(h, c)?
        }
        let selection = if keys & buttons::MOUSE_LEFT != 0 {
            let row = input.pointer_y.saturating_sub(50).div_euclid(32);
            ((64..self.canvas[0] - 64).contains(&input.pointer_x) && row >= 0 && (row as usize) < n)
                .then_some(row as usize)
        } else {
            (keys & (buttons::SPACE | buttons::ENTER) != 0).then_some(self.selected)
        };
        if selection.is_some() {
            submit(
                &mut self.published,
                h,
                c,
                (0..n as u32).map(|i| remove(CHOICE + i)).collect(),
            )?;
            self.choices.clear()
        }
        Ok(selection)
    }

    pub fn apply(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        vm: &mut Vm,
        event: VmEvent,
    ) -> Result<()> {
        self.characters.eye_preset = vm.global("EYE_SEQ_DEFAULT");
        match event {
            VmEvent::SetForeground(name)=>self.set_fg(h,c,&name)?,
            VmEvent::Animation(action) => {
                let mut ops=Vec::new();
                self.animations.command(h,c,action,&mut ops)?;
                submit(&mut self.published,h,c,ops)?;
                self.poll_animations(h,c)?;
            }
            VmEvent::SqzFrame { name, frame } => self.published.set_frame(&name,frame)?,
            VmEvent::Rotate { foreground, action } => self.rotate(h, c, vm, foreground, action)?,
            VmEvent::Cutin(command) => {
                self.cutin_catalogue
                    .apply(h, c, &mut self.published, command)?;
            }
            VmEvent::ScriptChanged(_) => {}
            VmEvent::EndMessage => {
                self.auto_advance_message = false;
                self.poll_message_actions(h, c, true)?;
                self.message = None;
                submit(&mut self.published, h, c, vec![remove(TEXT), remove(SPEAKER_NAME)])?
            }
            VmEvent::Message(m) => {
                self.auto_advance_message = false;
                if let Some(id) = crate::message::speaker_id(&m.speaker) {
                    self.speaker = id;
                }
                if !m.voice.is_empty() {
                    self.voice(h, c, &m.voice, m.voice_volume, m.voice_pan)?;
                }
                self.poll_message_actions(h, c, true)?;
                let split = self.panel_layout == Some(PanelLayout::Columns) && self.panel_mode == 1;
                let name = crate::message::display_name(&m.speaker);
                let speaker_layer = if split && !name.is_empty() {
                    panel_text(PanelLayout::Columns, SPEAKER_NAME, self.canvas[1], name)
                } else { remove(SPEAKER_NAME) };
                submit(&mut self.published, h, c, vec![speaker_layer])?;
                self.message = Some(Message::new(
                    if split { "" } else { &m.speaker },
                    &m.text,
                    h.now_ms(),
                    self.speeds.0[usize::from(m.read)],
                )?);
                self.poll_message(h, c, false)?;
            }
            VmEvent::Select(options) => {
                ensure!(
                    !options.is_empty() && options.len() <= 16,
                    "minori choice count"
                );
                self.selected = 0;
                self.choices = options;
                self.draw_choices(h, c)?;
            }
            VmEvent::PlayVoice(v) => {
                let (name, volume, pan) = crate::vm::parse_audio_spec(&v.name);
                self.voice(h, c, &name, volume, pan)?;
            }
            VmEvent::PlayBgm(b) => {
                let (name, _, pan) = crate::vm::parse_audio_spec(&b.name);
                self.play_sound_slot(h,c,0,name,true,b.volume,pan,b.fade_in,b.fade_out)?;
            }
            VmEvent::PlayBgm2(b) => {
                let (name, _, pan) = crate::vm::parse_audio_spec(&b.name);
                self.play_sound_slot(h,c,SECOND_MUSIC_CHANNEL,name,true,b.volume,pan,b.fade_in,b.fade_out)?;
            }
            VmEvent::PlaySe(s) => {
                self.play_sound_slot(h,c,1+u32::from(s.channel),s.name,s.looped,s.volume,s.pan,s.first,s.second)?;
            }
            VmEvent::PlayMovie(m) => {
                // The script movie number identifies a native gallery event.
                // Only one scenario movie owns the scoped host player at once.
                let id = 1;
                let path = asset("mov", &m.name)?;
                if let Some((name, rate, channels)) = &m.soundtrack {
                    ensure!(
                        *rate > 0 && *channels > 0,
                        "invalid minori movie soundtrack format"
                    );
                    self.sound(
                        h,
                        c,
                        MOVIE,
                        "bgm",
                        name,
                        AudioParameters {
                            bus: PcmBus::Music,
                            ..Default::default()
                        },
                    )?;
                }
                self.movie_soundtrack = m.soundtrack.is_some();
                let image = h.media_request(
                    c,
                    MediaRequest::Play {
                        id,
                        asset: path.relative().into(),
                        looping: false,
                        audio: !self.movie_soundtrack,
                        gain: 1.,
                    },
                )?;
                submit(
                    &mut self.published,
                    h,
                    c,
                    vec![put(
                        MOVIE,
                        [m.x, m.y, m.width, m.height],
                        json!({"kind":"image","asset":image.relative()}),
                        255,
                    )],
                )?;
                self.movie = Some((id, m.skippable));
            }
            VmEvent::ApplyChar(req) => {
                let mut ops = Vec::new();
                self.characters.apply(h, c, vm, req, &mut ops)?;
                submit(&mut self.published, h, c, ops)?;
            }
            VmEvent::SetHsvFilter(filter) => self.strips.hsv = filter,
            VmEvent::SetScreen(layer) => {
                self.strips.screen = Strips::slots(
                    h.storage(c)?.as_ref(),
                    layer.screen_slots.context("minori screen operands")?,
                )?;
            }
            VmEvent::SetOverlay(layer) => {
                self.strips.overlay = Strips::slots(
                    h.storage(c)?.as_ref(),
                    layer.overlay_slots.context("minori overlay operands")?,
                )?;
            }
            VmEvent::ApplyEffect2(effect)=>self.late_effect(h,c,vm,1,effect)?,
            VmEvent::ApplyEffect(effect) if self.screen_abi==ScreenAbi::PersistentZoom=>self.late_effect(h,c,vm,0,effect)?,
            VmEvent::ApplyEffect(effect) => match effect.kind.as_str() {
                "*" => self.clear_late_effect(h, c, 0)?,
                "CrossFade2" if self.screen_abi == ScreenAbi::Early => {
                    self.late_effect(h, c, vm, 0, effect)?;
                }
                "fadeout" => {
                    if let Some(effect) = &mut self.cutin[0] {
                        effect.fade();
                    }
                    if let Some(snow) = &mut self.snow[0] {
                        snow.fade();
                    }
                }
                "end" => {
                    if let Some(fade) = &mut self.crossfades[0] { fade.retiring = true; }
                    if let Some(walk) = &mut self.stage_walk[0] {
                        walk.retiring = true;
                    }
                    if let Some(effect) = &mut self.walking_scroll {
                        effect.retiring = true;
                    }
                    if let Some(effect) = &mut self.cutin[0] {
                        effect.retiring = true;
                    }
                    if let Some(snow) = &mut self.snow[0] {
                        snow.retiring = true;
                    }
                }
                "WScroll2" if self.screen_abi == ScreenAbi::Early => {
                    self.late_effect(h, c, vm, 0, effect)?;
                }
                "Cutin" if self.screen_abi == ScreenAbi::Early => {
                    self.clear_late_effect(h, c, 0)?;
                    let path = asset("bg", &effect.resource)?;
                    // Missing art leaves the native effect object finished.
                    if h.storage(c)?.exists(&path) {
                        let art = sprite(h, c, &["bg"], &effect.resource)?;
                        self.cutin[0] = Some(cutin::Cutin::new(
                            art,
                            self.canvas,
                            &effect.resource,
                            effect.first,
                            effect.second,
                        ));
                        self.poll_cutin(h, c)?;
                    }
                }
                "Firefly" if self.screen_abi == ScreenAbi::SignedDuration => {
                    if !self
                        .snow[0]
                        .as_mut()
                        .map(|s| s.retarget(&effect.resource, effect.first, effect.second))
                        .transpose()?
                        .unwrap_or(false)
                    {
                        self.clear_snow(h, c, 0)?;
                        self.snow[0] = Some(Weather::firefly(
                            h.storage(c)?,
                            self.random.get_or_insert_with(|| PatternRng::msvc(1)),
                            self.canvas,
                            &effect.resource,
                            effect.first,
                            effect.second,
                        )?);
                        self.poll_snow(h, c)?;
                    }
                }
                "Snow" | "SnowH" if self.screen_abi == ScreenAbi::Early => {
                    if self.stage_walk[0].is_some() { self.clear_late_effect(h, c, 0)?; }
                    self.clear_walking_scroll(h, c)?;
                    self.clear_cutin(h, c, 0)?;
                    if let Some(fade) = self.crossfades[0].take() { fade.release(h, c, &mut self.published, 0)?; }
                    let horizontal = effect.kind == "SnowH";
                    if !self.snow[0].as_ref().is_some_and(|s| s.matches(horizontal)) {
                        let storage = h.storage(c)?;
                        self.clear_snow(h, c, 0)?;
                        self.snow[0] = Some(Weather::snow(
                            storage,
                            self.random.get_or_insert_with(|| PatternRng::msvc(1)),
                            self.canvas,
                            horizontal,
                            false,
                        )?);
                        self.poll_snow(h, c)?;
                    }
                }
                _ => bail!("unsupported minori effect {effect:?}"),
            },
            VmEvent::StartShake(request) => {
                self.shake_roots(h, c)?;
                self.poll_shake(h, c, true, false)?;
                self.shake = Some(Shake::new(request, self.screen_abi, h.now_ms()));
                self.poll_shake(h, c, false, false)?;
            }
            VmEvent::StartScroll(r) => {
                let motion = if r.target_x.is_some() && r.target_y.is_some() {
                    Motion::Diagonal(r.speed)
                } else {
                    Motion::Axis(r.speed)
                };
                self.scroll = Some(Scroll {
                    start: self.camera,
                    target: [
                        r.target_x.unwrap_or(self.camera[0]),
                        r.target_y.unwrap_or(self.camera[1]),
                    ],
                    clock: h.now_ms(),
                    motion,
                });
            }
            VmEvent::WaveScroll { period, speed } => {
                let bg = self
                    .world
                    .first()
                    .context("wscroll requires a prepared background")?;
                let period = period.unwrap_or(self.canvas[0]);
                let width = bg.rect[2];
                ensure!(
                    width > 0 && speed != 0 && period != 0,
                    "invalid wscroll period or speed"
                );
                let horizontal_period = (10 * width / speed) as u32;
                ensure!(horizontal_period != 0, "wscroll horizontal period is zero");
                self.scroll = Some(Scroll {
                    start: self.camera,
                    target: self.camera,
                    clock: h.now_ms(),
                    motion: Motion::Wave {
                        period,
                        speed,
                        horizontal_period,
                        amplitude: ((bg.rect[3] - self.canvas[1]) / 2).abs(),
                    },
                });
                self.last_scroll_draw = None;
            }
            VmEvent::ForegroundScroll { target, speed } => {
                self.scroll = Some(Scroll {
                    start: self.foreground_camera,
                    target,
                    clock: h.now_ms(),
                    motion: Motion::Foreground(speed),
                });
                self.last_scroll_draw = None;
            }
            VmEvent::AcceleratedScroll {
                target,
                duration,
                interruptible,
            } => {
                self.scroll = Some(Scroll {
                    start: self.camera,
                    target,
                    clock: h.now_ms(),
                    motion: Motion::Accelerated {
                        ticks: duration,
                        interruptible: self.screen_abi != ScreenAbi::PersistentZoom
                            || interruptible,
                    },
                });
                self.last_scroll_draw = None;
            }
            VmEvent::StartScrollXf(r) => {
                let a = r.values;
                self.scroll = Some(Scroll {
                    start: [a[0], a[1]],
                    target: [a[6], a[7]],
                    clock: h.now_ms(),
                    motion: Motion::Timed(a[8].max(0) as u32),
                });
                self.poll_scroll(h, c, false, false)?;
            }
            VmEvent::EndScroll { interrupt } => {
                if self.scroll.is_some() {
                    if interrupt {
                        self.poll_scroll(h, c, true, false)?;
                        self.scroll = None
                    } else {
                        vm.begin_scroll()
                    }
                }
            }
            VmEvent::SetFrameStills(images) => {
                // ef's frame records are canvas-sized still images at origin.
                ensure!(images.len() <= 32, "minori frame layer limit");
                let mut layers = Vec::new();
                for (i, name) in images.iter().enumerate() {
                    let still = sprite(h, c, &["sys", "st", "bg"], name)?;
                    ensure!(
                        still.rect[2..] == self.canvas,
                        "minori frame dimensions for {name}"
                    );
                    layers.push(still.layer(50 + i as u32, 255));
                }
                let count = images.len() as u32;
                layers.extend((count..self.frame_count).map(|i| remove(50 + i)));
                submit(&mut self.published, h, c, layers)?;
                self.frame_count = count;
            }
            VmEvent::SetTransition(cfg) => self.transition = Some(cfg),
            VmEvent::SetStage(r) => {
                self.stage(h, c, r)?;
                if self.preparing_stage() || self.transition_until.is_some() {
                    vm.begin_transition()
                } else {
                    self.finish_stage(h, c)?;
                }
            }
            VmEvent::SetPanel(p) => {
                let next = self.panel_sprites(h, c, p.mode, &p.image)?;
                self.panel_mode = p.mode;
                self.fade = Some(PanelFade {
                    next,
                    start: h.now_ms(),
                    speed: if p.duration > 0 {
                        p.duration as u64
                    } else {
                        16
                    },
                });
                if self.poll_panel(h, c)? {
                    vm.complete_panel()
                }
            }
            unresolved => bail!("unsupported minori presentation event {unresolved:?}"),
        }
        Ok(())
    }
}
