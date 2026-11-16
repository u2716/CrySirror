// SPDX-License-Identifier: MPL-2.0
//! `crysirror showcase`: an offline recorder for a scripted 3D scene whose
//! screens sample live native game frames.
//!
//! Every game boots through the ordinary headless launch path and keeps its
//! own renderer. Each recorded frame, its composed picture is published as a
//! dynamic surface (`__surface/<slot>`) of the scene client, where the scene
//! script binds it as a PBR material texture. The scene script owns layout,
//! intro and camera; the host only reads the resulting camera and the
//! `screen:<slot>` entity poses to spatialise each game's captured PCM.
//! Audio is mixed into the recording only; no output device is opened.
use crate::{
    app::{self, Core, CoreHost as _},
    audio::Audio,
    input_track::InputTrack,
    virtual_window::VirtualWindow,
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    service::{InputFrame, InputSourcesFrame, PointerObservation},
    state::ClientId,
};
use crysirror_runtime::Runtime;
use crysirror_wgpu::{Gpu, Renderer, RenderingPath};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

const RATE: u32 = 48_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Scene project (a `crysirror-script-v1` client named `game`).
    scene: PathBuf,
    scripts: Vec<PathBuf>,
    #[serde(default = "default_width")]
    width: u32,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default = "default_fps")]
    fps: u32,
    seconds: f64,
    #[serde(default = "default_intro")]
    intro_seconds: f64,
    /// Font for the scene's captions (system or local file; never shipped).
    #[serde(default)]
    font: Option<PathBuf>,
    #[serde(default = "default_codec")]
    video_codec: String,
    #[serde(default = "default_bitrate")]
    bitrate: String,
    games: Vec<GameEntry>,
}
fn default_width() -> u32 {
    3840
}
fn default_height() -> u32 {
    2160
}
fn default_fps() -> u32 {
    60
}
fn default_intro() -> f64 {
    7.
}
fn default_codec() -> String {
    "hevc_videotoolbox".into()
}
fn default_bitrate() -> String {
    "70M".into()
}
fn one() -> f32 {
    1.
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct GameEntry {
    caption: String,
    engine: String,
    path: PathBuf,
    /// Frames run before recording (logos, boot) with no picture or sound.
    #[serde(default)]
    warmup_frames: u32,
    /// Input track indexed from the game's first frame (warm-up included).
    #[serde(default)]
    input_track: Option<PathBuf>,
    #[serde(default = "one")]
    gain: f32,
    #[serde(default)]
    nls: Option<String>,
    #[serde(default)]
    font: Option<PathBuf>,
    /// Wall-clock sleep after each warm-up tick (0..=16 ms) for integrated
    /// runtimes whose worker threads need real time between host ticks.
    #[serde(default)]
    pacing_ms: u64,
}

/// Published to the scene's save root as `showcase.json` before its scripts start.
#[derive(Serialize)]
struct SceneBrief<'a> {
    fps: u32,
    seconds: f64,
    intro_seconds: f64,
    width: u32,
    height: u32,
    screens: Vec<ScreenBrief<'a>>,
}
#[derive(Serialize)]
struct ScreenBrief<'a> {
    slot: u32,
    caption: &'a str,
    engine: &'a str,
    size: [u32; 2],
    surface: String,
}

pub struct Options {
    pub manifest: PathBuf,
    pub output: PathBuf,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub seconds: Option<f64>,
    pub start_seconds: f64,
    pub preview_dir: Option<PathBuf>,
    pub preview_every: u32,
    pub keep_work: bool,
}

struct GameRun {
    caption: String,
    core: Core,
    audio: Audio,
    renderer: Renderer,
    window: VirtualWindow,
    track: InputTrack,
    frame: u32,
    gain: f32,
    /// Previous per-channel gains, ramped across each frame.
    last_gain: [f32; 2],
    pcm: Vec<[f32; 2]>,
}

fn expand(base: &Path, path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn new_renderer(gpu: &Arc<Gpu>) -> Renderer {
    Renderer::new(gpu.clone(), wgpu::TextureFormat::Rgba8UnormSrgb)
}

impl GameRun {
    fn tick(&mut self, dt: u64) -> Result<()> {
        let input = self.track.at(self.frame);
        let rt = self.core.rt();
        self.window.begin_tick(rt);
        if rt.host.pointer_activity.is_none()
            || rt.host.pointer_observation.position != Some([input.pointer_x, input.pointer_y])
        {
            rt.host.note_pointer_activity()?;
        }
        let canvas = rt.project.canvas;
        rt.host.pointer_observation = PointerObservation {
            position: Some([input.pointer_x, input.pointer_y]),
            inside: Some(
                input.pointer_x >= 0
                    && input.pointer_y >= 0
                    && input.pointer_x < canvas[0] as i32
                    && input.pointer_y < canvas[1] as i32,
            ),
        };
        rt.host.publish_input_sources(InputSourcesFrame {
            keyboard_pointer: input,
            controller_backend: false,
            controllers: Vec::new(),
        })?;
        self.core
            .begin_tick(&mut self.audio, dt, input)
            .with_context(|| format!("{} frame {}", self.caption, self.frame))?;
        app::materialize_canvases(&mut self.renderer, self.core.rt())?;
        let start = Instant::now();
        while !self.core.poll_tick(&mut self.audio)? {
            ensure!(
                start.elapsed() < Duration::from_secs(120),
                "{}: frame {} dependencies did not complete",
                self.caption,
                self.frame
            );
            std::thread::sleep(Duration::from_micros(200));
        }
        self.frame += 1;
        Ok(())
    }

    fn picture(&mut self) -> Result<image::RgbaImage> {
        match &mut self.core {
            Core::Plain(rt) => rt.finish_vfx_offline()?,
            Core::Scripted(engine) => engine.prepare_render_offline()?,
        }
        let rt = self.core.rt();
        let previous = rt.assets.is_nonblocking();
        rt.assets.set_nonblocking(false);
        let renderer = &mut self.renderer;
        let result = (|| {
            let snapshot = rt.schedule_render_snapshot()?.wait(rt)?;
            renderer.set_scene3d(snapshot.simple.as_ref())?;
            renderer.set_pbr_scene(snapshot.pbr)?;
            renderer.set_engine_frame(snapshot.layers)?;
            renderer.set_transition(snapshot.transition)?;
            let (canvas, background) = (rt.project.canvas, rt.project.background);
            rt.assets.with_frame_surfaces(snapshot.surfaces, |assets| {
                renderer.capture(&snapshot.scene, assets, canvas, background)
            })
        })();
        rt.assets.set_nonblocking(previous);
        result
    }
}

struct Pose {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
}

fn camera_pose(rt: &Runtime) -> Option<Pose> {
    let c = rt.host.engine.world.view.camera.as_ref()?;
    let eye = Vec3::from_array(c.eye);
    let forward = (Vec3::from_array(c.target) - eye).normalize_or_zero();
    let up = Vec3::from_array(c.up).normalize_or_zero();
    let right = forward.cross(up).normalize_or_zero();
    Some(Pose { eye, forward, right })
}

/// World pose of each `screen:<slot>` entity: centre and facing (+Z).
fn screen_poses(rt: &Runtime, client: ClientId, count: usize) -> Vec<Option<(Vec3, Vec3)>> {
    let world = &rt.host.engine.world;
    let mut out = vec![None; count];
    for (id, _, spec) in world.entities(client) {
        let Some(slot) = spec
            .name
            .strip_prefix("screen:")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        if slot >= count {
            continue;
        }
        let Ok(handle) = world.handle(client, id) else { continue };
        let Ok(m) = world.global_matrix(handle) else { continue };
        let m: Mat4 = m;
        out[slot] = Some((
            m.w_axis.truncate(),
            m.z_axis.truncate().normalize_or_zero(),
        ));
    }
    out
}

/// Distance/facing attenuation and equal-power pan for one screen.
fn spatial_gain(pose: &Pose, screen: (Vec3, Vec3), base: f32) -> [f32; 2] {
    let to = screen.0 - pose.eye;
    let distance = to.length().max(0.01);
    let dir = to / distance;
    let reference = 12.0;
    let attenuation = 1.6 / (1. + (distance / reference).powi(2));
    let facing = 0.35 + 0.65 * (-dir).dot(screen.1).max(0.);
    let front = 0.7 + 0.3 * dir.dot(pose.forward).max(0.);
    let pan = dir.dot(pose.right).clamp(-1., 1.);
    let angle = (pan + 1.) * std::f32::consts::FRAC_PI_4;
    // Equal-power pan normalised so a centred source keeps unity per channel.
    let g = base * attenuation * facing * front * std::f32::consts::SQRT_2;
    [(g * angle.cos()).min(1.2), (g * angle.sin()).min(1.2)]
}

/// Procedural score for the brand intro and the drone flight. Nothing here is
/// sampled from a game.
struct Synth {
    phase: [f64; 8],
    noise: u64,
    low: [f32; 4],
    prev_speed: f32,
}
impl Synth {
    fn new() -> Self {
        Self {
            phase: [0.; 8],
            noise: 0x9e37_79b9_7f4a_7c15,
            low: [0.; 4],
            prev_speed: 0.,
        }
    }
    fn white(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 7;
        self.noise ^= self.noise << 17;
        (self.noise >> 40) as f32 / (1u64 << 23) as f32 - 1.
    }
    /// Intro sting: sub impact, rising shimmer, a bright chord at the logo
    /// reveal and a whoosh into the world.
    fn intro(&mut self, t: f64, intro: f64) -> [f32; 2] {
        let mut l = 0f32;
        let mut r = 0f32;
        let tau = std::f64::consts::TAU;
        // Rising shimmer 0.3 .. reveal.
        let reveal = (intro * 0.42).max(1.5);
        if t < reveal + 0.2 {
            let k = (t / reveal).clamp(0., 1.);
            let env = (k * k * 0.22) as f32 * (1. - ((t - reveal) / 0.2).clamp(0., 1.)) as f32;
            for (i, ratio) in [1.0, 1.5, 2.0, 2.52].iter().enumerate() {
                let f = 220. * ratio * (1. + 1.2 * k * k);
                self.phase[i] = (self.phase[i] + f / f64::from(RATE)) % 1.;
                let s = (self.phase[i] * tau).sin() as f32 * env / (1. + i as f32);
                l += s * if i % 2 == 0 { 1. } else { 0.6 };
                r += s * if i % 2 == 0 { 0.6 } else { 1. };
            }
            let n = self.white() * env * 0.25 * k as f32;
            self.low[0] += 0.08 * (n - self.low[0]);
            l += n - self.low[0];
            r += n - self.low[0];
        }
        // Impact + chord at the reveal.
        let d = t - reveal;
        if d >= 0. {
            let boom = (-d * 3.2).exp() as f32;
            let sub = (tau * 46. * d * (1. - 0.25 * (1. - (-d * 4.).exp()))).sin() as f32;
            l += sub * boom * 0.55;
            r += sub * boom * 0.55;
            let chord = [261.63, 329.63, 392.0, 523.25, 659.25, 783.99];
            let env = (1. - (-d * 30.).exp()) as f32 * (-d * 0.55).exp() as f32 * 0.07;
            for (i, f) in chord.iter().enumerate() {
                let detune = 1. + 0.0015 * (i as f64 - 2.5);
                let p = tau * f * detune * d;
                let s = (p.sin() + 0.3 * (2. * p).sin() + 0.12 * (3. * p).sin()) as f32 * env;
                let pan = i as f32 / 5.;
                l += s * (1. - 0.5 * pan);
                r += s * (0.5 + 0.5 * pan);
            }
            let crack = self.white() * (-d * 18.).exp() as f32 * 0.3;
            l += crack;
            r += crack;
        }
        // Whoosh over the last 1.6 s of the intro.
        let w = t - (intro - 1.6);
        if w >= 0. && w < 2.4 {
            let x = (w / 2.4) as f32;
            let env = (x * std::f32::consts::PI).sin().powi(2) * 0.35;
            let cutoff = 0.02 + 0.25 * x;
            let n = self.white();
            self.low[1] += cutoff * (n - self.low[1]);
            self.low[2] += cutoff * (self.low[1] - self.low[2]);
            l += self.low[2] * env * (1.2 - x);
            r += self.low[2] * env * (0.2 + x);
        }
        [l, r]
    }
    /// Quiet four-rotor whine and air rush driven by camera speed (m/s).
    fn drone(&mut self, speed: f32, fade: f32) -> [f32; 2] {
        self.prev_speed += 0.0005 * (speed - self.prev_speed);
        let s = self.prev_speed;
        let mut tone = 0f32;
        for i in 0..4 {
            let f = (150. + s * 7.5) as f64 * (1. + 0.013 * i as f64);
            self.phase[4 + i] = (self.phase[4 + i] + f / f64::from(RATE)) % 1.;
            let p = self.phase[4 + i];
            tone += (2. * p - 1.) as f32 * 0.25;
        }
        self.low[3] += 0.06 * (tone - self.low[3]);
        let n = self.white();
        self.low[0] += (0.01 + s * 0.0008).min(0.08) * (n - self.low[0]);
        let air = self.low[0] * (s / 30.).min(1.2) * 0.5;
        let motor = self.low[3] * 0.045;
        let v = (motor + air) * fade;
        [v, v]
    }
}

fn spawn_encoder(m: &Manifest, width: u32, height: u32, path: &Path) -> Result<Child> {
    Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{width}x{height}"), "-r", &m.fps.to_string(), "-i", "-"])
        .args(["-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p"])
        .args(["-c:v", &m.video_codec, "-b:v", &m.bitrate])
        .args(["-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709"])
        .args(if m.video_codec.starts_with("hevc") { vec!["-tag:v", "hvc1"] } else { vec![] })
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .context("start ffmpeg (required for showcase recording)")
}

fn write_wav(path: &Path, pcm: &[[f32; 2]]) -> Result<()> {
    let mut out = Vec::with_capacity(44 + pcm.len() * 4);
    let data = (pcm.len() * 4) as u32;
    out.extend(b"RIFF");
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 4).to_le_bytes());
    out.extend(4u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    for frame in pcm {
        for v in frame {
            out.extend(((v.clamp(-1., 1.) * 32767.).round() as i16).to_le_bytes());
        }
    }
    std::fs::write(path, out)?;
    Ok(())
}

pub fn run(options: Options) -> Result<()> {
    let manifest_path = options.manifest.canonicalize().context("showcase manifest")?;
    let base = manifest_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut m: Manifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)
        .context("showcase manifest JSON")?;
    if let Some(w) = options.width {
        m.width = w;
    }
    if let Some(h) = options.height {
        m.height = h;
    }
    if let Some(s) = options.seconds {
        m.seconds = s;
    }
    ensure!((16..=7680).contains(&m.width) && (16..=4320).contains(&m.height), "output size");
    ensure!(m.width % 2 == 0 && m.height % 2 == 0, "output size must be even");
    ensure!((1..=120).contains(&m.fps), "fps 1..=120");
    ensure!(m.seconds > 0. && m.seconds <= 1800., "seconds 0..=1800");
    ensure!(!m.games.is_empty() && m.games.len() <= 64, "1..=64 games");
    let total_frames = (m.seconds * f64::from(m.fps)).round() as u64;
    let first_frame = (options.start_seconds * f64::from(m.fps)).round() as u64;
    ensure!(first_frame < total_frames, "start is past the end");

    let output = expand(&std::env::current_dir()?, &options.output);
    let work = output.with_extension("parts");
    std::fs::create_dir_all(&work)?;
    if let Some(dir) = &options.preview_dir {
        std::fs::create_dir_all(dir)?;
    }
    let instance = wgpu::Instance::default();
    let gpu = Arc::new(pollster::block_on(Gpu::request_for(
        &instance,
        None,
        RenderingPath::Raster,
    ))?);
    eprintln!("showcase: gpu {:?}", gpu.adapter.get_info().name);

    // Games first: their canvas sizes go into the scene brief.
    let mut games = Vec::new();
    for (slot, entry) in m.games.iter().enumerate() {
        let path = expand(&base, &entry.path);
        let state = work.join(format!("state-{slot}"));
        let font = entry.font.as_ref().map(|f| expand(&base, f));
        let (mut core, _font) = crate::boot_headless_game(&path, &state, font, entry.nls.as_deref())
            .with_context(|| format!("boot {}", entry.caption))?;
        let track = InputTrack::load(entry.input_track.as_ref().map(|p| expand(&base, p)).as_deref())?;
        let window = VirtualWindow::new(core.rt().project.canvas);
        let mut renderer = new_renderer(&gpu);
        if std::env::var_os("CRYSIRROR_CPU_FRAME_HISTORY").is_none() {
            core.rt().host.mesh_frame_history = Some(renderer.mesh_frame_history_provider());
        }
        core.rt().set_canvas_image_presenter(true);
        app::settle_vm_preparations(core.rt())?;
        let _ = &mut renderer;
        games.push(GameRun {
            caption: entry.caption.clone(),
            core,
            audio: Audio::capturing(),
            renderer,
            window,
            track,
            frame: 0,
            gain: entry.gain,
            last_gain: [0.; 2],
            pcm: Vec::new(),
        });
    }
    for (game, entry) in games.iter_mut().zip(&m.games) {
        let started = Instant::now();
        ensure!(entry.pacing_ms <= 16, "pacing_ms must be 0..=16");
        for i in 0..entry.warmup_frames {
            game.tick(if i == 0 { 0 } else { 16 })?;
            if entry.pacing_ms > 0 {
                std::thread::sleep(Duration::from_millis(entry.pacing_ms));
            }
        }
        game.audio.take_captured();
        eprintln!(
            "showcase: warmed {} ({} frames, {:.1}s)",
            game.caption,
            entry.warmup_frames,
            started.elapsed().as_secs_f64()
        );
    }

    // Scene runtime at the output size; the brief is a save-root JSON file.
    let scene_path = expand(&base, &m.scene);
    let scene_state = work.join("scene-state");
    let _ = std::fs::remove_dir_all(&scene_state);
    std::fs::create_dir_all(&scene_state)?;
    let mut project = crysirror_launch::core::load_project(&scene_path)?;
    project.canvas = [m.width, m.height];
    let root = crysirror_runtime::runtime::asset_root(&scene_path, &project)?;
    let mut rt = Runtime::with_asset_root(
        project,
        &root,
        &scene_state,
        crysirror_launch::core::adapter_registry()?,
    )?;
    crate::desktop_provider::install(&mut rt, false)?;
    let client = rt.client_id("game").context("scene client 'game'")?;
    let mut brief = SceneBrief {
        fps: m.fps,
        seconds: m.seconds,
        intro_seconds: m.intro_seconds,
        width: m.width,
        height: m.height,
        screens: Vec::new(),
    };
    for (slot, (game, entry)) in games.iter_mut().zip(&m.games).enumerate() {
        let image = game.picture()?;
        let (w, h) = image.dimensions();
        rt.assets.put_surface(client, slot as u32, w, h, image.into_raw())?;
        brief.screens.push(ScreenBrief {
            slot: slot as u32,
            caption: &entry.caption,
            engine: &entry.engine,
            size: [w, h],
            surface: format!("{}{slot}", crysirror_runtime::assets::SURFACE_PREFIX),
        });
    }
    let brief_json = serde_json::to_vec_pretty(&brief)?;
    {
        use crysirror_core::service::FileSystem as _;
        let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, "showcase.json")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        rt.host
            .fs
            .write_atomic(client, &path, &brief_json)
            .map_err(|e| anyhow::anyhow!("write scene brief: {e}"))?;
    }
    let font = m.font.as_ref().map(|f| expand(&base, f));
    app::load_fonts(&mut rt, font.as_deref())?;
    rt.host.display = Some(crysirror_core::service::DisplayInfo {
        screen: [m.width, m.height],
        work: [0, 0, m.width as i32, m.height as i32],
    });
    let scripts: Vec<PathBuf> = m.scripts.iter().map(|s| expand(&base, s)).collect();
    let mut scene = crate::start_app(rt, &scripts, "game")?;
    let mut scene_renderer = new_renderer(&gpu);
    scene_renderer.set_movie_capture(true);
    app::load_fonts(scene.runtime_mut()?, font.as_deref())?;

    let video_path = work.join("video.mp4");
    let mut encoder = spawn_encoder(&m, m.width, m.height, &video_path)?;
    let mut stdin = encoder.stdin.take().context("ffmpeg stdin")?;
    let mut master: Vec<[f32; 2]> = Vec::new();
    let mut synth = Synth::new();
    let mut elapsed_ms = 0u64;
    let mut previous_eye: Option<Vec3> = None;
    let intro_frames = (m.intro_seconds * f64::from(m.fps)).round() as u64;
    let started = Instant::now();
    let mut log_lines = 0usize;
    let mut stage = [0f64; 4];
    for frame in 0..total_frames {
        let mut mark = Instant::now();
        let mut lap = |i: usize, mark: &mut Instant| {
            stage[i] += mark.elapsed().as_secs_f64();
            *mark = Instant::now();
        };
        let target_ms = (frame + 1) * 1000 / u64::from(m.fps);
        let dt = if frame == 0 { 0 } else { target_ms - elapsed_ms };
        let dt_ms = target_ms - elapsed_ms;
        elapsed_ms = target_ms;
        let samples = (dt_ms * u64::from(RATE) / 1000) as usize;
        let recording = frame >= first_frame;
        let games_live = frame >= intro_frames;
        // 1. Games advance in lock-step and publish their pictures.
        if games_live {
            for (slot, game) in games.iter_mut().enumerate() {
                game.tick(dt_ms)?;
                let mut pcm = game.audio.take_captured();
                pcm.resize(samples, [0.; 2]);
                game.pcm = pcm;
                if recording {
                    let image = game.picture()?;
                    let (w, h) = image.dimensions();
                    scene
                        .runtime_mut()?
                        .assets
                        .put_surface(client, slot as u32, w, h, image.into_raw())?;
                }
            }
        }
        lap(0, &mut mark);
        // 2. The scene script advances (layout, intro, flight).
        scene.frame(dt, InputFrame::default())?;
        lap(1, &mut mark);
        {
            let engine = scene.engine()?;
            for line in engine.log.iter().skip(log_lines) {
                eprintln!("script: {line}");
            }
            log_lines = engine.log.len();
        }
        // 3. Spatial mix against the camera the script just set.
        let rt = scene.runtime()?;
        let pose = camera_pose(rt);
        let screens = screen_poses(rt, client, games.len());
        let speed = match (&pose, previous_eye) {
            (Some(p), Some(prev)) if dt_ms > 0 => (p.eye - prev).length() * 1000. / dt_ms as f32,
            _ => 0.,
        };
        previous_eye = pose.as_ref().map(|p| p.eye);
        let mut mix = vec![[0f32; 2]; samples];
        let t0 = (target_ms - dt_ms) as f64 / 1000.;
        let fade_in = ((t0 - m.intro_seconds + 1.0) / 2.0).clamp(0., 1.) as f32;
        let fade_out = ((m.seconds - t0) / 2.5).clamp(0., 1.) as f32;
        for (i, out) in mix.iter_mut().enumerate() {
            let t = t0 + i as f64 / f64::from(RATE);
            if t < m.intro_seconds + 1.0 {
                let s = synth.intro(t, m.intro_seconds);
                out[0] += s[0];
                out[1] += s[1];
            }
            let d = synth.drone(speed, fade_in * fade_out);
            out[0] += d[0];
            out[1] += d[1];
        }
        if games_live {
            for (slot, game) in games.iter_mut().enumerate() {
                let target = match (&pose, screens[slot]) {
                    (Some(p), Some(s)) => spatial_gain(p, s, game.gain * fade_in * fade_out),
                    _ => [0.; 2],
                };
                for (i, (out, src)) in mix.iter_mut().zip(&game.pcm).enumerate() {
                    let k = (i + 1) as f32 / samples.max(1) as f32;
                    let gl = game.last_gain[0] + (target[0] - game.last_gain[0]) * k;
                    let gr = game.last_gain[1] + (target[1] - game.last_gain[1]) * k;
                    // Downmix to mono before panning: the screen is a point source.
                    let mono = 0.5 * (src[0] + src[1]);
                    let width = 0.35;
                    out[0] += gl * (mono + width * (src[0] - mono));
                    out[1] += gr * (mono + width * (src[1] - mono));
                }
                game.last_gain = target;
            }
        }
        if !recording {
            continue;
        }
        for v in mix.iter_mut().flatten() {
            *v = (*v * 1.1).tanh() * 0.92;
        }
        master.extend(mix);
        // 4. Render the scene and hand the frame to the encoder.
        scene.prepare_render_offline()?;
        let rt = scene.runtime_mut()?;
        let previous = rt.assets.is_nonblocking();
        rt.assets.set_nonblocking(false);
        let image = (|| {
            let snapshot = rt.schedule_render_snapshot()?.wait(rt)?;
            scene_renderer.set_profiler(rt.profiler.clone(), snapshot.frame);
            scene_renderer.set_scene3d(snapshot.simple.as_ref())?;
            scene_renderer.set_pbr_scene(snapshot.pbr)?;
            scene_renderer.set_engine_frame(snapshot.layers)?;
            scene_renderer.set_transition(snapshot.transition)?;
            let (canvas, background) = (rt.project.canvas, rt.project.background);
            rt.assets.with_frame_surfaces(snapshot.surfaces, |assets| {
                scene_renderer.capture(&snapshot.scene, assets, canvas, background)
            })
        })();
        rt.assets.set_nonblocking(previous);
        let image = image?;
        lap(2, &mut mark);
        ensure!(
            image.dimensions() == (m.width, m.height),
            "scene rendered {:?}, expected {}x{}",
            image.dimensions(),
            m.width,
            m.height
        );
        if let Some(dir) = &options.preview_dir {
            if options.preview_every > 0 && (frame - first_frame) % u64::from(options.preview_every) == 0 {
                image.save(dir.join(format!("frame_{frame:05}.png")))?;
            }
        }
        if let Err(e) = stdin.write_all(image.as_raw()) {
            let _ = encoder.wait();
            bail!("ffmpeg rejected video data: {e}");
        }
        lap(3, &mut mark);
        if frame % u64::from(m.fps) == 0 {
            let done = (frame + 1 - first_frame) as f64;
            let rate = done / started.elapsed().as_secs_f64().max(0.001);
            eprintln!(
                "showcase: frame {}/{} ({:.2} fps, eta {:.0}s) games {:.1}s script {:.1}s render {:.1}s encode {:.1}s",
                frame + 1,
                total_frames,
                rate,
                (total_frames - frame - 1) as f64 / rate.max(0.001),
                stage[0], stage[1], stage[2], stage[3]
            );
            let levels: Vec<String> = games
                .iter()
                .map(|g| {
                    let e = g.pcm.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f32>()
                        / (2 * g.pcm.len().max(1)) as f32;
                    format!("{:.0}dB@{:.2}", 10. * (e + 1e-12).log10(), g.last_gain[0] + g.last_gain[1])
                })
                .collect();
            eprintln!("showcase: game levels {}", levels.join(" "));
        }
    }
    drop(stdin);
    ensure!(encoder.wait()?.success(), "ffmpeg video encoding failed");
    let audio_path = work.join("audio.wav");
    write_wav(&audio_path, &master)?;
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&video_path)
        .arg("-i")
        .arg(&audio_path)
        .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "320k"])
        .args(["-movflags", "+faststart", "-shortest"])
        .arg(&output)
        .status()
        .context("start ffmpeg mux")?;
    ensure!(status.success(), "ffmpeg mux failed");
    for game in &mut games {
        let _ = game.core.shutdown(&mut game.audio);
    }
    if !options.keep_work {
        let _ = std::fs::remove_dir_all(&work);
    }
    println!(
        "{}",
        serde_json::json!({
            "output": output, "frames": total_frames - first_frame, "fps": m.fps,
            "size": [m.width, m.height], "games": m.games.iter().map(|g| &g.caption).collect::<Vec<_>>(),
            "audio_samples": master.len(), "audio_emitted": false,
            "wall_seconds": started.elapsed().as_secs_f64(),
            "stage_seconds": {"games": stage[0], "script": stage[1], "render": stage[2], "encode": stage[3]},
            "profile": scene_renderer.profile_report_offline().ok(),
        })
    );
    Ok(())
}
