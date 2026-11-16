// SPDX-License-Identifier: MPL-2.0
//! C ABI for the libnx shell (platform/switch/runtime/source/main.c).
//! The session runs on the shell's main thread.
use crate::session::{self, Opened};
use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;

unsafe extern "C" {
    fn crysirror_switch_log(message: *const c_char);
    fn siglus_switch_random_fill(buffer: *mut u8, length: usize);
}

pub fn log(message: &str) {
    if let Ok(c) = CString::new(message) {
        unsafe { crysirror_switch_log(c.as_ptr()) };
    }
}

static mut LAST_ERROR: [u8; 1024] = [0; 1024];

fn set_error(message: &str) {
    log(&format!("error: {message}\n"));
    let bytes = message.as_bytes();
    let n = bytes.len().min(1023);
    unsafe {
        let buf = &mut *std::ptr::addr_of_mut!(LAST_ERROR);
        buf[..n].copy_from_slice(&bytes[..n]);
        buf[n] = 0;
    }
}

struct State {
    opened: Opened,
    frame: u64,
    dump: Vec<u64>,
    root: PathBuf,
    script: crate::script::Script,
}

pub struct Player(State);

/// The shell passes the system shared font (plGetSharedFontByType); the
/// memory stays mapped for the process lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_set_system_font(data: *const u8, size: usize) {
    if !data.is_null() && size > 0 {
        session::set_system_font(unsafe { std::slice::from_raw_parts(data, size) });
        log(&format!("system font: {size} bytes\n"));
    }
}

/// Last error text (NUL-terminated, static storage).
#[unsafe(no_mangle)]
pub extern "C" fn crysirror_console_last_error() -> *const c_char {
    std::ptr::addr_of!(LAST_ERROR).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_open(root: *const c_char, width: u32, height: u32) -> *mut Player {
    if root.is_null() {
        return std::ptr::null_mut();
    }
    let root = PathBuf::from(unsafe { CStr::from_ptr(root) }.to_string_lossy().into_owned());
    std::panic::set_hook(Box::new(|info| log(&format!("panic: {info}\n"))));
    struct ShellLog;
    impl ::log::Log for ShellLog {
        fn enabled(&self, m: &::log::Metadata) -> bool {
            m.level() <= ::log::Level::Debug && (m.target().starts_with("crysirror") || m.level() <= ::log::Level::Warn)
        }
        fn log(&self, r: &::log::Record) {
            if self.enabled(r.metadata()) {
                log(&format!("{} {}: {}\n", r.level(), r.target(), r.args()));
            }
        }
        fn flush(&self) {}
    }
    if ::log::set_logger(&ShellLog).is_ok() {
        ::log::set_max_level(::log::LevelFilter::Debug);
    }
    match std::panic::catch_unwind(|| session::open(&root, (width, height))) {
        Ok(Ok(opened)) => {
            log(&format!("open: {} ({})\n", opened.game.id, opened.engine));
            let dump = std::fs::read_to_string(root.join("dump-frames"))
                .unwrap_or_default()
                .lines()
                .filter_map(|l| l.trim().parse().ok())
                .collect::<Vec<u64>>();
            if !dump.is_empty() {
                let _ = std::fs::create_dir_all(root.join("dump"));
            }
            let script = crate::script::Script::load(&root);
            Box::into_raw(Box::new(Player(State { opened, frame: 0, dump, root, script })))
        }
        Ok(Err(e)) => {
            set_error(&format!("{e:#}"));
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error("engine startup panicked");
            std::ptr::null_mut()
        }
    }
}

/// Returns true when the session ended or failed (see last_error).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_step(player: *mut Player, dt_ms: u32) -> bool {
    let Some(Player(s)) = (unsafe { player.as_mut() }) else { return true };
    {
        if s.dump.contains(&s.frame) {
            let path = s.root.join("dump").join(format!("frame-{}.png", s.frame));
            s.opened.session.dump_next_frame(path);
        }
        for action in s.script.actions(s.frame) {
            match action {
                crate::script::Action::Button(b, down) => s.opened.session.button(b, down),
                crate::script::Action::Touch(phase, x, y) => s.opened.session.touch(phase, x, y),
            }
        }
        s.frame += 1;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.opened.session.step(dt_ms))) {
            Ok(Ok(done)) => done,
            Ok(Err(e)) => {
                set_error(&format!("{e:#}"));
                true
            }
            Err(_) => {
                set_error("engine frame panicked");
                true
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_button(player: *mut Player, button: u8, down: bool) {
    if let Some(Player(s)) = unsafe { player.as_mut() } {
        s.opened.session.button(button, down);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_touch(player: *mut Player, phase: i32, x: f64, y: f64) {
    if let Some(Player(s)) = unsafe { player.as_mut() } {
        s.opened.session.touch(phase, x, y);
    }
}

/// Fill `frames` interleaved stereo i16 samples at 48 kHz.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_render_pcm(player: *mut Player, dst: *mut i16, frames: usize) {
    if dst.is_null() {
        return;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(dst, frames * 2) };
    match unsafe { player.as_mut() } {
        Some(Player(s)) => s.opened.session.render_pcm(out),
        None => out.fill(0),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crysirror_console_close(player: *mut Player) {
    if !player.is_null() {
        drop(unsafe { Box::from_raw(player) });
    }
}

/// getrandom 0.2 `custom` backend (rsa in crysirror-resource): libnx's
/// kernel-seeded randomGet via the shell. getrandom 0.3 uses the hook the
/// integrated Siglus Horizon host already defines.
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_custom(dest: *mut u8, len: usize) -> u32 {
    if !dest.is_null() {
        unsafe { siglus_switch_random_fill(dest, len) };
    }
    0
}
