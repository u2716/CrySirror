// SPDX-License-Identifier: MPL-2.0
//! Console session for the integrated Siglus runtime (Switch and Vita).
//!
//! The CrySirror console host owns the process, display, input sampling and
//! the audio device. This session owns the complete Siglus runtime and draws
//! with its native console renderer (deko3d on Horizon, GXM on Vita) into the
//! host's display. PCM is pulled by the host. Key material is resolved from
//! the game's own files, as on desktop, and saves go to the host-owned user
//! root instead of the game directory.
use crate::native_vm::{
    audio::switch_backend,
    host::{SiglusHost, SiglusHostConfig},
    render::Renderer,
    runtime::input::VmKey,
};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

/// How often (frames) the global save is checked for changes: console apps
/// are usually closed from the system menu without an engine exit.
const GLOBAL_SAVE_INTERVAL: u64 = 300;

pub struct Session {
    host: SiglusHost,
    display: (u32, u32),
    frame: u64,
    global: u64,
}

impl Session {
    /// `resource_root` is the read-only game directory; `user_root` is the
    /// host's per-game save directory (created if missing).
    pub fn open(
        resource_root: PathBuf,
        user_root: PathBuf,
        font: Option<PathBuf>,
        system_font: Option<&'static [u8]>,
        display: (u32, u32),
    ) -> Result<Self> {
        ensure!(display.0 > 0 && display.1 > 0, "console display size");
        std::fs::create_dir_all(&user_root).with_context(|| format!("create {}", user_root.display()))?;
        let config = crate::Config {
            remote_root: None,
            resource_root: resource_root.clone(),
            user_root,
            scene: "Scene.pck".into(),
            gameexe: "Gameexe.dat".into(),
            key_file: None,
            font_file: font,
            font_face_index: 0,
            font: None,
        };
        crate::adapter::install_scope_with(&config, &resource_root, system_font)?;
        let renderer = Renderer::new(display.0, display.1).context("Siglus console renderer")?;
        let mut host = SiglusHost::new_with_renderer_sync(SiglusHostConfig::new(resource_root), renderer)?;
        let (lw, lh) = host.logical_size();
        // The game keeps its own screen size; the renderer fits it into the display.
        host.resize_with_logical_viewport(display.0, display.1, 1.0, lw, lh, 0, 0, display.0, display.1);
        let global = host.persist_global_if_changed(None);
        Ok(Self { host, display, frame: 0, global })
    }

    pub fn logical_size(&self) -> (u32, u32) {
        self.host.logical_size()
    }

    /// One original-engine frame (VM, render and present). Returns true when
    /// the game ended.
    pub fn step(&mut self, dt_ms: u32) -> Result<bool> {
        self.frame += 1;
        let done = self.host.step(dt_ms)?;
        if self.frame % GLOBAL_SAVE_INTERVAL == 0 {
            self.global = self.host.persist_global_if_changed(Some(self.global));
        }
        Ok(done)
    }

    /// Shared console button index (libnx HidNpadButton bit order: 0 A,
    /// 1 B, 2 X, 3 Y, 6 L, 7 R, 8 ZL, 9 ZR, 10 Plus, 11 Minus, 12-15 D-pad
    /// left/up/right/down, 16-23 sticks). Vita hosts map Cross/Circle/...
    /// onto the same indices.
    pub fn button(&mut self, button: u8, down: bool) {
        let key = match button {
            0 => Some(VmKey::Enter),
            1 => Some(VmKey::Escape),
            2 => Some(VmKey::Space),
            3 => Some(VmKey::Tab),
            6 => Some(VmKey::Shift),
            7 | 8 => Some(VmKey::Control),
            9 => Some(VmKey::Space),
            10 => Some(VmKey::Escape),
            11 => Some(VmKey::Tab),
            12 | 16 | 20 => Some(VmKey::ArrowLeft),
            13 | 17 | 21 => Some(VmKey::ArrowUp),
            14 | 18 | 22 => Some(VmKey::ArrowRight),
            15 | 19 | 23 => Some(VmKey::ArrowDown),
            _ => None,
        };
        // Keyboard compatibility first: it switches the input family to
        // keyboard/mouse, so the raw joypad edge must follow it.
        if let Some(key) = key {
            if down {
                self.host.key_down(key);
            } else {
                self.host.key_up(key);
            }
        }
        self.host.joypad_button(button as usize, down);
    }

    /// Touch phase 0 begin, 1 move, 2 end at display pixel (x, y), mapped
    /// into the letterboxed game screen.
    pub fn touch(&mut self, phase: i32, x: f64, y: f64) {
        let (lw, lh) = self.host.logical_size();
        let (w, h) = (f64::from(self.display.0), f64::from(self.display.1));
        let scale = (w / f64::from(lw)).min(h / f64::from(lh));
        let (dw, dh) = (f64::from(lw) * scale, f64::from(lh) * scale);
        let (ox, oy) = ((w - dw) / 2.0, (h - dh) / 2.0);
        let lx = ((x - ox) * f64::from(lw) / dw).clamp(0.0, f64::from(lw) - 1.0);
        let ly = ((y - oy) * f64::from(lh) / dh).clamp(0.0, f64::from(lh) - 1.0);
        self.host.touch(phase, lx, ly);
    }

    /// Interleaved stereo i16 at 48 kHz from the runtime's own mixer.
    pub fn render_pcm(&mut self, out: &mut [i16]) {
        let frames = out.len() / 2;
        unsafe { switch_backend::render_stereo_i16(out.as_mut_ptr(), frames) }
    }

    pub fn dump_next_frame(&mut self, path: PathBuf) {
        self.host.renderer_mut().dump_next_frame(path);
    }

    /// Renderer counters since the last call (Vita: phases, draws, scenes,
    /// uploads), for the host's periodic log.
    pub fn diagnostics(&mut self) -> String {
        #[cfg(target_os = "vita")]
        return crate::native_vm::render::vita_stats::take_summary();
        #[cfg(not(target_os = "vita"))]
        String::new()
    }

    pub fn persist(&mut self) {
        self.global = self.host.persist_global_if_changed(Some(self.global));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.persist();
    }
}

/// The runtime's mixer is process-global behind its own lock: a host audio
/// thread may pull from it while the session steps on the main thread.
pub fn pull_pcm(out: &mut [i16]) {
    unsafe { switch_backend::render_stereo_i16(out.as_mut_ptr(), out.len() / 2) }
}

/// True when `dir` holds a Siglus game (the files this session opens).
pub fn is_game(dir: &std::path::Path) -> bool {
    dir.join("Scene.pck").is_file() && dir.join("Gameexe.dat").is_file()
}
