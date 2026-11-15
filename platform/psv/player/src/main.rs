// SPDX-License-Identifier: MPL-2.0
//! CrySirror PS Vita player. The Vita host owns the process, display (vita2d's
//! GXM context, started by the engine renderer), controller/touch sampling and
//! the audio port; `crysirror-console-player` selects the game under
//! ux0:data/CrySirror/games and drives its engine session.
//! Structure follows xmoezzz/siglus_rs platform/vita/player at
//! 19066ea7f36b92fdecaea240a223c9bef14efd2c (MPL-2.0).
#![cfg_attr(target_os = "vita", feature(alloc_error_hook))]

#[cfg(target_os = "vita")]
mod dialog;

#[cfg(not(target_os = "vita"))]
fn main() {
    eprintln!("crysirror_vita_player must be built with cargo-vita for PS Vita");
}

/// newlib's heap is reserved whole at startup (extended memory mode). The
/// rest of user memory backs GPU buffers, staging and the textures that do
/// not fit CDRAM. Measured on Vita3K, 2026-11-15, KiriKiri (game-kr):
/// - 320 MiB left 5 MiB of user memory and GPU allocations failed;
/// - 256 MiB ran out when the title's E-mote model loaded (~171 to 256 MiB);
/// - 300 MiB peaked at 260 MiB and ran to the title.
#[cfg(target_os = "vita")]
#[used]
#[unsafe(export_name = "_newlib_heap_size_user")]
pub static NEWLIB_HEAP_SIZE_USER: u32 = 300 * 1024 * 1024;

#[cfg(target_os = "vita")]
mod vita {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::mem::MaybeUninit;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::JoinHandle;

    use crysirror_console_player::session;
    use vitasdk_sys::*;

    const WIDTH: u32 = 960;
    const HEIGHT: u32 = 544;
    const AUDIO_FRAMES: usize = 512;
    const ROOT: &str = "ux0:data/CrySirror";

    pub(super) fn log(message: &str) {
        let _ = fs::create_dir_all(ROOT);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(format!("{ROOT}/crysirror.log")) {
            let _ = writeln!(file, "{message}");
        }
    }

    /// Log hook used by the integrated Siglus Vita renderer.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn siglus_vita_log_marker(message: *const u8) {
        if message.is_null() {
            return;
        }
        let message = unsafe { std::ffi::CStr::from_ptr(message.cast()) };
        if let Ok(message) = message.to_str() {
            log(message.trim_end());
        }
    }

    struct AudioOutput {
        running: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }
    impl AudioOutput {
        fn start(pull: fn(&mut [i16])) -> Option<Self> {
            let port = unsafe {
                sceAudioOutOpenPort(SCE_AUDIO_OUT_PORT_TYPE_MAIN, AUDIO_FRAMES as i32, 48_000, SCE_AUDIO_OUT_MODE_STEREO)
            };
            if port < 0 {
                log(&format!("audio port unavailable: {port:#x}"));
                return None;
            }
            let running = Arc::new(AtomicBool::new(true));
            let flag = Arc::clone(&running);
            let worker = std::thread::Builder::new()
                .name("crysirror-vita-audio".into())
                .stack_size(128 * 1024)
                .spawn(move || {
                    let mut samples = [0i16; AUDIO_FRAMES * 2];
                    while flag.load(Ordering::Relaxed) {
                        pull(&mut samples);
                        if unsafe { sceAudioOutOutput(port, samples.as_ptr().cast()) } < 0 {
                            break;
                        }
                    }
                    unsafe { sceAudioOutReleasePort(port) };
                })
                .ok()?;
            Some(Self { running, worker: Some(worker) })
        }
    }
    impl Drop for AudioOutput {
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    /// Vita buttons onto the shared console indices (libnx order).
    const BUTTONS: [(u32, u8); 12] = [
        (SCE_CTRL_CROSS, 0),
        (SCE_CTRL_CIRCLE, 1),
        (SCE_CTRL_SQUARE, 2),
        (SCE_CTRL_TRIANGLE, 3),
        (SCE_CTRL_LTRIGGER, 6),
        (SCE_CTRL_RTRIGGER, 7),
        (SCE_CTRL_START, 10),
        (SCE_CTRL_SELECT, 11),
        (SCE_CTRL_LEFT, 12),
        (SCE_CTRL_UP, 13),
        (SCE_CTRL_RIGHT, 14),
        (SCE_CTRL_DOWN, 15),
    ];

    fn run() -> Result<(), String> {
        log("player start");
        let root = Path::new(ROOT);
        if root.join("selftest-gxm").exists() {
            log(&crysirror_console_player::selftest::gxm(root));
            return Ok(());
        }
        let mut opened = session::open(root, (WIDTH, HEIGHT)).map_err(|e| format!("{e:#}"))?;
        log(&format!("open: {} ({}) {}", opened.game.id, opened.engine, opened.session.diagnostics()));
        // Marker file: keep the audio port closed (emulator diagnosis); the
        // mixer still advances by being pulled every frame.
        let silent = root.join("disable-audio").exists();
        let _audio = if silent { None } else { AudioOutput::start(opened.session.pcm_pull()) };
        let touch_ok = unsafe { sceTouchSetSamplingState(SCE_TOUCH_PORT_FRONT, SCE_TOUCH_SAMPLING_STATE_START) } >= 0;
        let mut panel = MaybeUninit::<SceTouchPanelInfo>::zeroed();
        let panel = (touch_ok && unsafe { sceTouchGetPanelInfo(SCE_TOUCH_PORT_FRONT, panel.as_mut_ptr()) } >= 0)
            .then(|| unsafe { panel.assume_init() });
        let dump: Vec<u64> = fs::read_to_string(root.join("dump-frames"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect();
        if !dump.is_empty() {
            let _ = fs::create_dir_all(root.join("dump"));
        }
        let script = crysirror_console_player::script::Script::load(root);
        let (mut previous, mut touching, mut last) = (0u32, false, (0.0, 0.0));
        let mut silent_pcm = [0i16; 800 * 2];
        let mut frame = 0u64;
        let mut period = std::time::Instant::now();
        loop {
            let mut pad = MaybeUninit::<SceCtrlData>::zeroed();
            if unsafe { sceCtrlPeekBufferPositive(0, pad.as_mut_ptr(), 1) } > 0 {
                let buttons = unsafe { pad.assume_init() }.buttons;
                let changed = buttons ^ previous;
                for (bit, index) in BUTTONS {
                    if changed & bit != 0 {
                        opened.session.button(index, buttons & bit != 0);
                    }
                }
                previous = buttons;
            }
            if let Some(panel) = &panel {
                let mut touch = MaybeUninit::<SceTouchData>::zeroed();
                if unsafe { sceTouchPeek(SCE_TOUCH_PORT_FRONT, touch.as_mut_ptr(), 1) } > 0 {
                    let touch = unsafe { touch.assume_init() };
                    if touch.reportNum > 0 {
                        let r = touch.report[0];
                        let rx = (i32::from(panel.maxDispX) - i32::from(panel.minDispX)).max(1);
                        let ry = (i32::from(panel.maxDispY) - i32::from(panel.minDispY)).max(1);
                        let x = f64::from((i32::from(r.x) - i32::from(panel.minDispX)) * WIDTH as i32 / rx);
                        let y = f64::from((i32::from(r.y) - i32::from(panel.minDispY)) * HEIGHT as i32 / ry);
                        last = (x, y);
                        opened.session.touch(if touching { 1 } else { 0 }, x, y);
                        touching = true;
                    } else if touching {
                        opened.session.touch(2, last.0, last.1);
                        touching = false;
                    }
                }
            }
            for action in script.actions(frame) {
                match action {
                    crysirror_console_player::script::Action::Button(b, down) => opened.session.button(b, down),
                    crysirror_console_player::script::Action::Touch(phase, x, y) => opened.session.touch(phase, x, y),
                }
            }
            if dump.contains(&frame) {
                opened.session.dump_next_frame(root.join("dump").join(format!("frame-{frame}.png")));
            }
            if _audio.is_none() {
                opened.session.render_pcm(&mut silent_pcm);
            }
            if opened.session.step(16).map_err(|e| format!("engine frame: {e:#}"))? {
                break;
            }
            frame += 1;
            if frame % 60 == 0 {
                let secs = period.elapsed().as_secs_f64().max(0.001);
                let heap = unsafe { mallinfo() };
                log(&format!(
                    "frame {frame}: {:.1} fps heap used={} free={} {}",
                    60.0 / secs,
                    heap.uordblks,
                    heap.fordblks,
                    opened.session.diagnostics()
                ));
                period = std::time::Instant::now();
            }
        }
        log("player exit");
        Ok(())
    }

    /// Descriptor of `<root>/fatal.log`, opened before the session so the
    /// allocation-failure hook can report without allocating (the heap is
    /// exhausted at that point and a formatting hook would abort silently).
    static FATAL_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

    unsafe extern "C" {
        fn open(path: *const std::ffi::c_char, flags: i32, ...) -> i32;
        fn write(fd: i32, buf: *const u8, len: usize) -> isize;
        fn mallinfo() -> MallInfo;
    }
    /// newlib `struct mallinfo` (all `size_t` on ARM).
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MallInfo {
        arena: usize,
        ordblks: usize,
        smblks: usize,
        hblks: usize,
        hblkhd: usize,
        usmblks: usize,
        fsmblks: usize,
        uordblks: usize,
        fordblks: usize,
        keepcost: usize,
    }

    fn open_fatal_log() {
        const O_WRONLY: i32 = 1;
        const O_CREAT: i32 = 0x200;
        const O_TRUNC: i32 = 0x400;
        let _ = fs::create_dir_all(ROOT);
        let fd = unsafe { open(c"ux0:data/CrySirror/fatal.log".as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o666) };
        FATAL_FD.store(fd, Ordering::Relaxed);
    }

    /// Fixed-capacity formatter: never touches the heap.
    struct StackText {
        buf: [u8; 256],
        len: usize,
    }
    impl std::fmt::Write for StackText {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            let n = s.len().min(self.buf.len() - self.len);
            self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
            self.len += n;
            Ok(())
        }
    }

    /// newlib's `abort` traps without a message (C `assert`, runtime
    /// aborts). Record the caller and every text-segment address found on
    /// the stack, for symbolization against the unstripped ELF, then trap.
    #[unsafe(no_mangle)]
    pub extern "C" fn abort() -> ! {
        use std::fmt::Write as _;
        let (lr, sp): (usize, usize);
        unsafe { std::arch::asm!("mov {0}, lr", "mov {1}, sp", out(reg) lr, out(reg) sp) };
        unsafe extern "C" {
            static __executable_start: u8;
            static etext: u8;
        }
        let (lo, hi) = unsafe { (&__executable_start as *const u8 as usize, &etext as *const u8 as usize) };
        let fd = FATAL_FD.load(Ordering::Relaxed);
        let mut text = StackText { buf: [0; 256], len: 0 };
        let _ = writeln!(text, "abort: lr={lr:#x} sp={sp:#x} text={lo:#x}..{hi:#x}");
        if fd >= 0 {
            unsafe { write(fd, text.buf.as_ptr(), text.len) };
        }
        let mut found = 0;
        for i in 0..4096usize {
            let word = unsafe { std::ptr::read_volatile((sp + i * 4) as *const usize) };
            if word > lo && word < hi && word & 1 == 1 {
                let mut text = StackText { buf: [0; 256], len: 0 };
                let _ = writeln!(text, "  sp+{:#x}: {word:#x}", i * 4);
                if fd >= 0 {
                    unsafe { write(fd, text.buf.as_ptr(), text.len) };
                }
                found += 1;
                if found >= 64 {
                    break;
                }
            }
        }
        unsafe { std::arch::asm!("udf #0xfe", options(noreturn)) }
    }

    fn alloc_failed(layout: std::alloc::Layout) {
        use std::fmt::Write as _;
        let info = unsafe { mallinfo() };
        let mut text = StackText { buf: [0; 256], len: 0 };
        let _ = writeln!(
            text,
            "out of memory: {} bytes (align {}); heap arena={} used={} free={}",
            layout.size(),
            layout.align(),
            info.arena,
            info.uordblks,
            info.fordblks
        );
        let fd = FATAL_FD.load(Ordering::Relaxed);
        if fd >= 0 {
            unsafe { write(fd, text.buf.as_ptr(), text.len) };
        }
    }

    pub fn main() {
        open_fatal_log();
        crysirror_console_player::set_log_sink(log);
        std::panic::set_hook(Box::new(|info| log(&format!("panic: {info}"))));
        std::alloc::set_alloc_error_hook(alloc_failed);
        if let Err(error) = run() {
            log(&format!("error: {error}"));
            let _ = super::dialog::show(
                &format!("CrySirror could not continue.\n\n{error}\n\nLog: {ROOT}/crysirror.log"),
                false,
            );
        }
        let _: Option<PathBuf> = None;
    }
}

#[cfg(target_os = "vita")]
fn main() {
    // Integrated VM initialization has stack frames larger than newlib's
    // default entry stack (Siglus host construction alone is about 160 KiB),
    // and adopted sessions run the desktop call depth: System-NNN (MUGEN2)
    // overflowed 2 MiB during preparation (Vita3K, 2026-11-15). The Vita has
    // no guard page, so an overflow corrupts memory instead of faulting.
    // Keep the session and all GXM calls on one explicitly sized owner thread.
    match std::thread::Builder::new()
        .name("crysirror-player".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(vita::main)
    {
        Ok(player) => {
            if player.join().is_err() {
                vita::log("error: player thread panicked");
            }
        }
        Err(error) => {
            vita::log(&format!("error: could not start player thread: {error}"));
            let _ = dialog::show("CrySirror could not start its player thread.", false);
        }
    }
}
