// SPDX-License-Identifier: MPL-2.0
//! Engine dispatch for one console session.
use crate::games::{self, Game};
use anyhow::{Context, Result, bail};
#[cfg(target_os = "horizon")]
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static SYSTEM_FONT: OnceLock<&'static [u8]> = OnceLock::new();

/// Register the platform's system font (e.g. the Switch shared font in
/// read-only shared memory). Used after game-bundled and owner fonts; never
/// copied to storage.
pub fn set_system_font(bytes: &'static [u8]) {
    let _ = SYSTEM_FONT.set(bytes);
}
pub fn system_font() -> Option<&'static [u8]> {
    SYSTEM_FONT.get().copied()
}

pub enum Session {
    /// The installed-game library (`launcher.rs`); replaced in place by the
    /// chosen game's session.
    Launcher(Box<Launcher>),
    /// Between the launcher and the game session (never stepped).
    Switching,
    Siglus(crysirror_siglus_integrated::console::Session),
    #[cfg(target_os = "vita")]
    Bgi(crysirror_bgi_integrated::console::Session),
    Adopted(Box<crate::adopted::Adopted>, u64),
}

pub struct Opened {
    pub game: Game,
    pub engine: &'static str,
    pub session: Session,
}

/// Select the game under `root`, detect its engine and open it at the
/// display size. Unknown layouts fail visibly.
pub fn open(root: &Path, display: (u32, u32)) -> Result<Opened> {
    if root.join("heartbeat").exists() {
        let _ = std::thread::Builder::new().name("crysirror-heartbeat".into()).spawn(|| {
            let t = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                crate::log(&format!("heartbeat {:?}", t.elapsed()));
            }
        });
    }
    if root.join("js-probe").exists() {
        js_probe();
    }
    let installed = games::list(root).unwrap_or_default();
    if crate::launcher::wanted(root, &installed) {
        return open_launcher(root, installed, display);
    }
    let opened = open_game(root, games::select(root)?, display)?;
    set_pull(opened.session.pcm_pull());
    Ok(opened)
}

pub struct Launcher {
    adopted: crate::adopted::Adopted,
    previous: u64,
    root: PathBuf,
    state: PathBuf,
    display: (u32, u32),
    games: Vec<Game>,
}

/// The launcher project on the shared renderer (see `launcher.rs`).
fn open_launcher(root: &Path, installed: Vec<Game>, display: (u32, u32)) -> Result<Opened> {
    let font_path = font(root);
    let font_bytes = match &font_path {
        Some(path) => Some(std::fs::read(path)?),
        None => system_font().map(<[u8]>::to_vec),
    };
    let style = crysirror_library::library::Style {
        confirm: if cfg!(target_os = "vita") { "" } else { "A" },
        cross: cfg!(target_os = "vita"),
        add_hint: format!("Copy each game folder to {}", root.join("games").display()),
    };
    let dir = crate::launcher::prepare(root, &installed, display, font_bytes.as_deref(), &style)?;
    drop(font_bytes);
    let state = root.join("saves").join("launcher");
    let adopted = crate::adopted::Adopted::open(&dir, &state, font_path, shared_gpu()?)?;
    crate::log(&format!("launcher: {} games", installed.len()));
    set_pull(silence);
    Ok(Opened {
        game: Game { id: "launcher".into(), dir },
        engine: "Launcher",
        session: Session::Launcher(Box::new(Launcher { adopted, previous: 0, root: root.to_path_buf(), state, display, games: installed })),
    })
}

/// The shared renderer's device for adopted engines and the launcher.
fn shared_gpu() -> Result<crysirror_wgpu_deko3d::Gpu> {
    #[cfg(target_os = "horizon")]
    {
        let driver = crysirror_wgpu_deko3d::driver::Deko3d::new().context("deko3d driver")?;
        let library = crysirror_wgpu_deko3d::library::Cooked::open("romfs:/wgsl").map_err(|e| anyhow::anyhow!(e))?;
        return Ok(crysirror_wgpu_deko3d::Gpu::new(Arc::new(driver), Arc::new(library)));
    }
    #[cfg(target_os = "vita")]
    {
        crysirror_wgpu_gxm::gxm::set_log(crate::log);
        return crysirror_wgpu_gxm::create().map_err(|e| anyhow::anyhow!("GXM renderer: {e}"));
    }
    #[allow(unreachable_code)]
    {
        bail!("no shared renderer on this platform")
    }
}

/// PCM pull of the current session. The Vita audio thread captures one
/// function at start; this indirection lets it follow the launcher's switch
/// to the chosen game.
static PULL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn silence(out: &mut [i16]) {
    out.fill(0);
}
fn set_pull(pull: fn(&mut [i16])) {
    PULL.store(pull as usize, std::sync::atomic::Ordering::Release);
}
fn current_pull(out: &mut [i16]) {
    let raw = PULL.load(std::sync::atomic::Ordering::Acquire);
    if raw == 0 {
        return out.fill(0);
    }
    // SAFETY: PULL only ever holds values stored by `set_pull` from a
    // `fn(&mut [i16])`.
    let pull: fn(&mut [i16]) = unsafe { std::mem::transmute::<usize, fn(&mut [i16])>(raw) };
    pull(out)
}

/// Diagnostic (`<root>/js-probe`): build a bare JS context and evaluate one
/// expression, logging the time of each step.
pub(crate) fn js_probe() {
    let t = std::time::Instant::now();
    crate::log("js-probe: build");
    let mut ctx = boa_engine::Context::default();
    crate::log(&format!("js-probe: built in {:?}", t.elapsed()));
    let v = ctx.eval(boa_engine::Source::from_bytes("1 + 1"));
    crate::log(&format!("js-probe: eval {:?} in {:?}", v.map(|v| v.as_number()), t.elapsed()));
}

pub fn open_game(root: &Path, game: Game, display: (u32, u32)) -> Result<Opened> {
    if crysirror_siglus_integrated::console::is_game(&game.dir) {
        let saves = games::save_root(root, "siglus", &game);
        let font = font(root);
        let session = crysirror_siglus_integrated::console::Session::open(game.dir.clone(), saves, font, system_font(), display)?;
        return Ok(Opened { game, engine: "Siglus", session: Session::Siglus(session) });
    }
    #[cfg(target_os = "vita")]
    if let Some((resources, bootstrap)) = bgi_game(&game.dir)? {
        let saves = games::save_root(root, "bgi", &game);
        let session = crysirror_bgi_integrated::console::Session::open(resources, saves, font(root), &bootstrap, display)?;
        return Ok(Opened { game, engine: "BGI", session: Session::Bgi(session) });
    }
    // Adopted engines: the shared runtime and unchanged shared renderer on
    // the wgpu custom backend (deko3d on Switch, GXM on Vita).
    let _ = display;
    let gpu = shared_gpu()?;
    let saves = games::save_root(root, "adopted", &game);
    let adopted = crate::adopted::Adopted::open(&game.dir, &saves, font(root), gpu)?;
    let engine: &'static str = Box::leak(adopted.engine.clone().into_boxed_str());
    Ok(Opened { game, engine, session: Session::Adopted(Box::new(adopted), 0) })
}

/// BGI resource root and BP bootstrap, as `crysirror-launch` detects them
/// on desktop (including disc layouts with an `Install/` resource root).
#[cfg(target_os = "vita")]
fn bgi_game(dir: &Path) -> Result<Option<(PathBuf, String)>> {
    let has_arc = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("arc")) || e.file_name().eq_ignore_ascii_case("Install"));
    if !has_arc {
        return Ok(None);
    }
    let plan = crysirror_launch::detect::detect(dir).with_context(|| format!("detect {}", dir.display()))?;
    if plan.engine != "BGI" {
        return Ok(None);
    }
    let client = &plan.project["clients"][0];
    let bootstrap = client["config"]["script"].as_str().context("BGI plan bootstrap")?.to_owned();
    let resources = match client["asset_prefix"].as_str() {
        Some(prefix) => dir.join(prefix),
        None => dir.to_path_buf(),
    };
    Ok(Some((resources, bootstrap)))
}

/// Optional owner-supplied font at `<root>/fonts/default.ttf` (never bundled).
fn font(root: &Path) -> Option<PathBuf> {
    ["ttf", "otf", "ttc"]
        .iter()
        .map(|ext| root.join("fonts").join(format!("default.{ext}")))
        .find(|p| p.is_file())
}

impl Session {
    pub fn step(&mut self, dt_ms: u32) -> Result<bool> {
        if let Self::Launcher(l) = self {
            if !l.adopted.step(dt_ms, &mut l.previous)? {
                return Ok(false);
            }
            let Some(game) = crate::launcher::choice(&l.state, &l.games) else { return Ok(true) };
            let (root, display) = (l.root.clone(), l.display);
            // Release the launcher's renderer before the game creates its own.
            *self = Self::Switching;
            let mut opened = open_game(&root, game, display)?;
            crate::log(&format!("open: {} ({}) {}", opened.game.id, opened.engine, opened.session.diagnostics()));
            set_pull(opened.session.pcm_pull());
            *self = opened.session;
            return Ok(false);
        }
        match self {
            Self::Launcher(..) | Self::Switching => Ok(false),
            Self::Siglus(s) => s.step(dt_ms),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.step(dt_ms),
            Self::Adopted(a, prev) => a.step(dt_ms, prev),
        }
    }
    pub fn button(&mut self, button: u8, down: bool) {
        match self {
            Self::Launcher(l) => l.adopted.button(button, down),
            Self::Switching => {}
            Self::Siglus(s) => s.button(button, down),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.button(button, down),
            Self::Adopted(a, _) => a.button(button, down),
        }
    }
    pub fn touch(&mut self, phase: i32, x: f64, y: f64) {
        match self {
            Self::Launcher(l) => l.adopted.touch(phase, x, y),
            Self::Switching => {}
            Self::Siglus(s) => s.touch(phase, x, y),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.touch(phase, x, y),
            Self::Adopted(a, _) => a.touch(phase, x, y),
        }
    }
    pub fn render_pcm(&mut self, out: &mut [i16]) {
        match self {
            Self::Launcher(..) | Self::Switching => out.fill(0),
            Self::Siglus(s) => s.render_pcm(out),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.render_pcm(out),
            Self::Adopted(..) => out.fill(0),
        }
    }
    /// Thread-safe PCM pull for hosts whose audio device runs on its own
    /// thread (Vita): the runtime's mixer behind its own lock.
    pub fn pcm_pull(&self) -> fn(&mut [i16]) {
        match self {
            Self::Launcher(..) | Self::Switching => current_pull,
            Self::Siglus(_) => crysirror_siglus_integrated::console::pull_pcm,
            #[cfg(target_os = "vita")]
            Self::Bgi(_) => crysirror_bgi_integrated::console::pull_pcm,
            Self::Adopted(..) => |out: &mut [i16]| out.fill(0),
        }
    }
    /// Engine/renderer diagnostics for the host's periodic log.
    pub fn diagnostics(&mut self) -> String {
        match self {
            Self::Launcher(..) | Self::Switching => String::new(),
            Self::Siglus(s) => s.diagnostics(),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.diagnostics(),
            Self::Adopted(..) => String::new(),
        }
    }
    pub fn dump_next_frame(&mut self, path: PathBuf) {
        #[cfg(target_os = "vita")]
        if matches!(self, Self::Adopted(..) | Self::Launcher(..)) {
            crysirror_wgpu_gxm::gxm::dump_images(path.with_extension("images"));
        }
        match self {
            Self::Launcher(l) => {
                let _ = l.adopted.capture_png(&path);
            }
            Self::Switching => {}
            Self::Siglus(s) => s.dump_next_frame(path),
            #[cfg(target_os = "vita")]
            Self::Bgi(s) => s.dump_next_frame(path),
            Self::Adopted(a, _) => {
                let _ = a.capture_png(&path);
            }
        }
    }
}
