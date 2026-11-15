// SPDX-License-Identifier: MPL-2.0
//! Console session for the integrated BGI runtime on PS Vita.
//!
//! The CrySirror console host owns the process, input sampling and the audio
//! device. This session owns the complete BGI runtime (ADR 0102 integrated
//! mode). Every frame is composed on the GPU: the runtime's native layer
//! commands drive `native_render::Renderer::new_native` over the retained
//! original GXM driver (`native/vita/gpu.c`, ethornell-rs
//! fc0efa7586f130aa5dd29d4846aac5dc66124dad). No CPU framebuffer is composed
//! or uploaded. 48 kHz PCM goes to a host-pulled queue.
//!
//! Console newlib has no `openat`/`dirfd`, so the game directory and the save
//! root are handed to the runtime as hosted files (structure from
//! ethornell-rs `platform/console/src/files.rs`, same revision, MPL-2.0).
use crate::native_app::hosted_files::{ResourceFiles, UserStore};
use crate::native_archive::ArchiveSource;
use crate::{HostConfig, HostRuntime};
use std::mem::ManuallyDrop;
use anyhow::{Context, Result, anyhow, bail, ensure};
use std::collections::VecDeque;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

unsafe extern "C" {
    fn ethornell_vita_gpu_init() -> i32;
    fn ethornell_vita_gpu_exit();
    fn ethornell_gpu_request_dump(out: *mut u8);
}

/// Two seconds of interleaved stereo; older samples are dropped when the
/// host audio thread is not pulling (diagnostic silent runs drain it).
const PCM_LIMIT: usize = 48_000 * 2 * 2;
static PCM: Mutex<VecDeque<i16>> = Mutex::new(VecDeque::new());
static PERSIST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Host audio thread: interleaved stereo i16; silence when the queue is short.
pub fn pull_pcm(out: &mut [i16]) {
    let Ok(mut queue) = PCM.lock() else {
        out.fill(0);
        return;
    };
    for sample in out.iter_mut() {
        *sample = queue.pop_front().unwrap_or(0);
    }
}

/// Shared console button index (libnx order) to the original host's
/// keyboard descriptors (ethornell-rs Vita player mapping).
fn key_code(button: u8) -> Option<i32> {
    Some(match button {
        12 => 37,
        13 => 38,
        14 => 39,
        15 => 40,
        1 => 27,
        2 => 32,
        3 => 13,
        6 => 17,
        _ => return None,
    })
}

pub struct Session {
    /// Dropped explicitly before GXM shutdown: its renderer owns GXM textures.
    host: ManuallyDrop<HostRuntime>,
    display: (u32, u32),
    /// Engine clock in nanoseconds, advanced by the host's frame interval.
    now: u64,
    pointer: [f32; 2],
    pending_confirm: Option<u64>,
    dump: Option<PathBuf>,
    closed: bool,
}

impl Session {
    /// `resource_root` is the game directory (read in place), `user_root` the
    /// host-owned save directory, `bootstrap` the BP image detected by
    /// `crysirror-launch` (e.g. `sysprg.arc:main._bp`).
    pub fn open(resource_root: PathBuf, user_root: PathBuf, font: Option<PathBuf>, bootstrap: &str, display: (u32, u32)) -> Result<Self> {
        ensure!(display.0 > 0 && display.1 > 0, "console display size");
        let font = font.context("BGI needs a font: place one at ux0:data/CrySirror/fonts/default.ttf")?;
        std::fs::create_dir_all(&user_root).with_context(|| format!("create {}", user_root.display()))?;
        install_files(&resource_root, &user_root)?;
        let mut host = HostRuntime::create_with_bootstrap(
            HostConfig {
                resource_root,
                user_root,
                font_path: Some(font),
                font_data: None,
                font_face_index: 0,
                gbk: false,
                backend: 0,
                strict: true,
                trace: false,
                silent: false,
            },
            bootstrap,
        )
        .map_err(|e| anyhow!("BGI runtime: {e}"))?;
        host.use_virtual_display();
        let rc = unsafe { ethornell_vita_gpu_init() };
        ensure!(rc >= 0, "BGI GXM initialization failed: {rc}");
        let renderer = crate::native_render::Renderer::new_native(display.0, display.1, host.font(), unsafe { crate::vita::callbacks() })
            .map_err(|e| anyhow!("BGI GXM renderer: {e}"));
        let attached = renderer.and_then(|r| host.attach_renderer(r, display.0, display.1, 1).map_err(|e| anyhow!("BGI GXM renderer attach: {e}")));
        if let Err(error) = attached {
            drop(host);
            unsafe { ethornell_vita_gpu_exit() };
            return Err(error);
        }
        let [w, h] = host.logical_size();
        Ok(Self {
            host: ManuallyDrop::new(host),
            display,
            now: 0,
            pointer: [w as f32 / 2.0, h as f32 / 2.0],
            pending_confirm: None,
            dump: None,
            closed: false,
        })
    }

    /// The game screen inside the display: x, y, width, height in pixels.
    fn letterbox(&self) -> [f32; 4] {
        let [lw, lh] = self.host.logical_size();
        let (w, h) = (self.display.0 as f32, self.display.1 as f32);
        let scale = (w / lw as f32).min(h / lh as f32);
        let (dw, dh) = (lw as f32 * scale, lh as f32 * scale);
        [(w - dw) / 2.0, (h - dh) / 2.0, dw, dh]
    }

    /// One frame: runtime step, host events, present. True once the game ended.
    pub fn step(&mut self, dt_ms: u32) -> Result<bool> {
        self.now += u64::from(dt_ms) * 1_000_000;
        let mut readback = self.dump.take().map(|path| (path, vec![0u8; 960 * 544 * 4]));
        if let Some((_, buffer)) = readback.as_mut() {
            ensure!(self.display == (960, 544), "BGI GXM readback expects the 960x544 display");
            unsafe { ethornell_gpu_request_dump(buffer.as_mut_ptr()) };
        }
        let stepped = self.host.step(self.now).map_err(|e| anyhow!("BGI step: {e}"));
        unsafe { ethornell_gpu_request_dump(std::ptr::null_mut()) };
        stepped?;
        if let Some((path, mut rgba)) = readback {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            image::save_buffer(&path, &rgba, 960, 544, image::ColorType::Rgba8)
                .with_context(|| format!("save {}", path.display()))?;
        }
        if let Some(error) = PERSIST_ERROR.lock().ok().and_then(|mut e| e.take()) {
            bail!(error);
        }
        while let Some(event) = self.host.poll_event() {
            match event.kind {
                2 => {
                    crate_log(&format!("confirm (Cross=yes/Circle=no): {}", event.text));
                    self.host.clear_input();
                    self.pending_confirm = Some(event.request_id);
                }
                3 => self.closed = true,
                5 => bail!("BGI structured dialog unsupported on the console host: {}", event.text),
                1 | 4 => crate_log(&event.text),
                _ => {}
            }
        }
        let pcm = self.host.take_pcm();
        if let Ok(mut queue) = PCM.lock() {
            for [l, r] in pcm {
                queue.push_back((l.clamp(-1.0, 1.0) * 32767.0) as i16);
                queue.push_back((r.clamp(-1.0, 1.0) * 32767.0) as i16);
            }
            let excess = queue.len().saturating_sub(PCM_LIMIT);
            queue.drain(..excess);
        }
        // The GPU renderer attached at open presented this frame inside
        // host.step; no CPU frame exists in this mode.
        debug_assert!(self.host.take_frame().is_none());
        Ok(self.closed || self.host.is_closed())
    }

    fn mouse(&mut self, kind: u32, code: i32) {
        let [x, y] = self.pointer;
        let _ = self.host.input(kind, x, y, code, 0.0);
    }

    pub fn button(&mut self, button: u8, down: bool) {
        if let Some(request) = self.pending_confirm {
            if down && (button == 0 || button == 1) {
                self.pending_confirm = None;
                self.host.clear_input();
                let _ = self.host.scope(|h| h.complete_request(request, i32::from(button == 0)));
            }
            return;
        }
        if button == 0 {
            self.mouse(if down { 2 } else { 3 }, 1);
        } else if let Some(code) = key_code(button) {
            let _ = self.host.input(if down { 4 } else { 5 }, 0.0, 0.0, code, 0.0);
        }
    }

    /// Display-pixel touch (phase 0 begin, 1 move, 2 end) into logical game
    /// coordinates, as the crysirror-host input contract expects.
    pub fn touch(&mut self, phase: i32, x: f64, y: f64) {
        let [ox, oy, w, h] = self.letterbox();
        let [lw, lh] = self.host.logical_size();
        self.pointer = [
            ((x as f32 - ox) * lw as f32 / w).clamp(0.0, lw as f32 - 1.0),
            ((y as f32 - oy) * lh as f32 / h).clamp(0.0, lh as f32 - 1.0),
        ];
        self.mouse(1, 0);
        match phase {
            0 => self.mouse(2, 1),
            2 => self.mouse(3, 1),
            _ => {}
        }
    }

    pub fn render_pcm(&mut self, out: &mut [i16]) {
        pull_pcm(out);
    }

    /// Save the next presented GXM display frame (960x544, including the
    /// letterbox) to `path`. Reads back the GPU output; nothing is CPU-composed.
    pub fn dump_next_frame(&mut self, path: PathBuf) {
        self.dump = Some(path);
    }

    pub fn diagnostics(&self) -> String {
        self.host.diagnostics()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the host is never used after this; GXM outlives its renderer.
        unsafe { ManuallyDrop::drop(&mut self.host) };
        unsafe { ethornell_vita_gpu_exit() };
    }
}

fn crate_log(text: &str) {
    tracing::info!(target: "bgi-console", "{text}");
}

fn join(root: &Path, name: &str) -> io::Result<PathBuf> {
    if name.contains(['\\', ':', '\0']) || Path::new(name).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(root.join(name))
}

fn list(root: &Path) -> io::Result<Vec<(String, u64)>> {
    fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<(String, u64)>) -> io::Result<()> {
        if depth > 64 || out.len() > 65536 {
            return Err(io::Error::other("console file-tree budget exceeded"));
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                walk(root, &entry.path(), depth + 1, out)?;
            } else if kind.is_file() {
                let path = entry.path();
                let name = path
                    .strip_prefix(root)
                    .map_err(io::Error::other)?
                    .to_str()
                    .ok_or_else(|| io::Error::other("non-UTF8 filename"))?
                    .replace('\\', "/");
                out.push((name, entry.metadata()?.len()));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(root, root, 0, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

struct Resources {
    root: PathBuf,
    files: Vec<(String, u64)>,
}
impl ResourceFiles for Resources {
    fn list(&self) -> Vec<(String, u64)> {
        self.files.clone()
    }
    fn open(&self, path: &str) -> io::Result<Box<dyn ArchiveSource>> {
        Ok(Box::new(std::fs::File::open(join(&self.root, path)?)?))
    }
}

const TEMP_SUFFIX: &str = ".ethornell-tmp";
const BACKUP_SUFFIX: &str = ".ethornell-bak";
fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
fn remove_if_present(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
/// The console rename does not replace an existing destination: keep the old
/// file until the flushed replacement has its final name, and recover
/// interrupted moves on the next launch.
fn recover(user: &Path) -> io::Result<()> {
    for (name, _) in list(user)? {
        if let Some(original) = name.strip_suffix(BACKUP_SUFFIX) {
            let backup = join(user, &name)?;
            let target = join(user, original)?;
            if target.try_exists()? {
                std::fs::remove_file(backup)?;
            } else {
                std::fs::rename(backup, target)?;
            }
        }
    }
    Ok(())
}
struct Saves(PathBuf);
impl UserStore for Saves {
    fn persist(&self, name: &str, bytes: Option<&[u8]>) {
        let write = || -> io::Result<()> {
            let path = join(&self.0, name)?;
            let Some(bytes) = bytes else {
                remove_if_present(&sidecar(&path, BACKUP_SUFFIX))?;
                return remove_if_present(&path);
            };
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp = sidecar(&path, TEMP_SUFFIX);
            let backup = sidecar(&path, BACKUP_SUFFIX);
            {
                use std::io::Write;
                let mut file = std::fs::File::create(&tmp)?;
                file.write_all(bytes)?;
                file.sync_all()?;
            }
            let had_previous = path.try_exists()?;
            if had_previous {
                remove_if_present(&backup)?;
                std::fs::rename(&path, &backup)?;
            }
            if let Err(error) = std::fs::rename(&tmp, &path) {
                if had_previous {
                    let _ = std::fs::rename(&backup, &path);
                }
                return Err(error);
            }
            remove_if_present(&backup)
        };
        if let Err(e) = write() {
            if let Ok(mut failure) = PERSIST_ERROR.lock() {
                *failure = Some(format!("save persistence failed: {e}"));
            }
        }
    }
}

fn install_files(game: &Path, user: &Path) -> Result<()> {
    let resources = Resources { root: game.to_owned(), files: list(game)? };
    recover(user)?;
    let mut saves = Vec::new();
    let mut total = 0u64;
    for (name, size) in list(user)? {
        if name.ends_with(TEMP_SUFFIX) || name.ends_with(BACKUP_SUFFIX) {
            continue;
        }
        total = total.checked_add(size).ok_or_else(|| anyhow!("save size overflow"))?;
        ensure!(total <= 64 * 1024 * 1024, "save budget exceeded");
        saves.push((name.clone(), std::fs::read(join(user, &name)?)?));
    }
    crate::native_app::hosted_files::install(Arc::new(resources), Arc::new(Saves(user.to_owned())), saves);
    Ok(())
}
