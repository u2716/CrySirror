//! C ABI surface used by the platform launcher projects.
//!
//! The desktop entry point delegates to the same `run_sena` path as the
//! command-line binary. Mobile host-mode entry points are exported so the
//! launcher projects render through an RGBA frame pump. Fatal frame errors can
//! be acknowledged asynchronously through the platform native messagebox UI.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::str::FromStr;
#[cfg(target_os = "android")]
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::native_asset::Nls;

use crate::native_vm::config::TextLang;
use crate::native_vm::engine::Engine;
use crate::native_vm::event::{InputEvent, MouseButton, PalEvent};
use crate::native_vm::runtime::RuntimeStatus;
use crate::native_vm::scene::{rasterize_scene_rgba, FrameScene};

mod scene;
use scene::NativeScene;

type NativeMessageboxFn = extern "C" fn(*mut c_void, u64, i32, *const c_char, *const c_char);
type NativeMessageboxCallback = Option<NativeMessageboxFn>;

// Android submits results on its UI thread while the engine runs on another
// thread. This mailbox deliberately never dereferences a host handle. Late UI
// results after teardown are ignored once the outstanding request is removed.
fn messagebox_results() -> &'static Mutex<HashMap<usize, (u64, bool)>> {
    static RESULTS: OnceLock<Mutex<HashMap<usize, (u64, bool)>>> = OnceLock::new();
    RESULTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct SenaFfiHost {
    engine: Engine,
    rgba: Vec<u8>,
    scene_rendering: bool,
    native_scene: Option<NativeScene>,
    frame_width: u32,
    frame_height: u32,
    frame_generation: u64,
    last_status: i32,
    messagebox: NativeMessageboxState,
}

#[derive(Default)]
struct NativeMessageboxState {
    callback: NativeMessageboxCallback,
    user_data: *mut c_void,
    pending_request_id: Option<u64>,
}

impl NativeMessageboxState {
    fn register(&mut self, callback: NativeMessageboxCallback, user_data: *mut c_void) {
        self.callback = callback;
        self.user_data = user_data;
        // Hosts unregister before tearing their UI down. Do not leave the pump
        // waiting for a response that can no longer arrive.
        if callback.is_none() {
            self.pending_request_id = None;
        }
    }

    fn begin(&mut self) -> Option<(NativeMessageboxFn, *mut c_void, u64)> {
        let callback = self.callback?;
        if self.pending_request_id.is_some() {
            return None;
        }
        // Process-wide IDs prevent an old UI result from matching a new host
        // whose allocation happens to reuse the same address.
        static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        let mut request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        if request_id == 0 {
            request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        }
        self.pending_request_id = Some(request_id);
        Some((callback, self.user_data, request_id))
    }

    fn acknowledge(&mut self, request_id: u64, value: i64) {
        // Kind 0 has one OK button, whose host ABI value is 0. An old dialog
        // result must never acknowledge a later request.
        if value == 0 && self.pending_request_id == Some(request_id) {
            self.pending_request_id = None;
        }
    }
}

fn set_native_messagebox_callback(
    handle: *mut c_void,
    callback: NativeMessageboxCallback,
    user_data: *mut c_void,
) {
    if let Some(host) = host_from_handle(handle) {
        host.messagebox.register(callback, user_data);
        if callback.is_none() {
            messagebox_results().lock().unwrap().remove(&(handle as usize));
        }
    }
}

fn submit_messagebox_result(handle: *mut c_void, request_id: u64, value: i64) {
    if value != 0 {
        return;
    }
    if let Some((expected, acknowledged)) =
        messagebox_results().lock().unwrap().get_mut(&(handle as usize))
    {
        if *expected == request_id {
            *acknowledged = true;
        }
    }
}

fn report_host_error(handle: *mut c_void, message: &str) -> i32 {
    let dispatch = {
        let Some(host) = host_from_handle(handle) else {
            return 2;
        };
        host.last_status = 2;
        host.engine.input_begin_frame();
        host.messagebox.begin()
    };
    let Some((callback, user_data, request_id)) = dispatch else {
        return 2;
    };
    messagebox_results()
        .lock()
        .unwrap()
        .insert(handle as usize, (request_id, false));
    let title = c"Sena";
    let message = CString::new(message.replace('\0', " ")).expect("NULs were removed");
    // No host borrow crosses the callback: Android can submit a synchronous
    // fallback result from inside the callback if its Activity is unavailable.
    // Both strings remain valid until the callback returns; hosts copy them.
    callback(user_data, request_id, 0, title.as_ptr(), message.as_ptr());
    // Hold the terminal pump alive until the UI acknowledges the fatal error.
    // The next step returns status 2 after acknowledgement, never resumes VM.
    0
}

fn cstring_lossy(s: impl AsRef<str>) -> *mut c_char {
    let clean = s.as_ref().replace('\0', " ");
    CString::new(clean)
        .unwrap_or_else(|_| CString::new("Sena").expect("static fallback is a valid CString"))
        .into_raw()
}

unsafe fn path_from_cstr(ptr: *const c_char) -> Result<PathBuf, &'static str> {
    if ptr.is_null() {
        return Err("null path");
    }
    let s = CStr::from_ptr(ptr)
        .to_str()
        .map_err(|_| "path is not UTF-8")?;
    if s.trim().is_empty() {
        return Err("empty path");
    }
    Ok(PathBuf::from(s))
}

unsafe fn string_from_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(str::to_owned)
}

fn nls_from_cstr(ptr: *const c_char) -> Nls {
    unsafe { string_from_cstr(ptr) }
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| Nls::from_str(s).ok())
        .unwrap_or_default()
}

fn lang_from_cstr(ptr: *const c_char) -> TextLang {
    unsafe { string_from_cstr(ptr) }
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| TextLang::from_str(s).ok())
        .unwrap_or(TextLang::Jp)
}

fn runtime_status_is_terminal(status: &RuntimeStatus) -> bool {
    matches!(
        status,
        RuntimeStatus::Halted { .. }
            | RuntimeStatus::UnsupportedCommand { .. }
            | RuntimeStatus::UnsupportedExtCall { .. }
            | RuntimeStatus::Faulted { .. }
    )
}

fn host_from_handle<'a>(handle: *mut c_void) -> Option<&'a mut SenaFfiHost> {
    if handle.is_null() {
        None
    } else {
        unsafe { (handle as *mut SenaFfiHost).as_mut() }
    }
}

fn cover_candidate(root: &Path) -> Option<PathBuf> {
    const NAMES: &[&str] = &[
        "cover.png",
        "cover.jpg",
        "cover.jpeg",
        "Cover.png",
        "Cover.jpg",
        "icon.png",
        "Icon.png",
    ];
    NAMES
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("png") => Some("image/png"),
        Some(ext) if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") => {
            Some("image/jpeg")
        }
        _ => None,
    }
}

/// Releases strings returned by the Sena C ABI.
#[no_mangle]
pub extern "C" fn sena_free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

/// Alias retained for the platform launcher headers.
#[no_mangle]
pub extern "C" fn sena_string_free(ptr: *mut c_char) {
    sena_free_c_string(ptr);
}

/// Returns a display name for a game directory.
#[no_mangle]
pub unsafe extern "C" fn sena_game_name_from_dir(game_root_utf8: *const c_char) -> *mut c_char {
    match path_from_cstr(game_root_utf8) {
        Ok(path) => {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("Sena");
            cstring_lossy(name)
        }
        Err(_) => cstring_lossy("Sena"),
    }
}

/// Returns a best-effort cover image path for launcher UI previews.
#[no_mangle]
pub unsafe extern "C" fn sena_game_cover_path_from_dir(
    game_root_utf8: *const c_char,
) -> *mut c_char {
    match path_from_cstr(game_root_utf8)
        .ok()
        .and_then(|root| cover_candidate(&root))
    {
        Some(path) => cstring_lossy(path.to_string_lossy()),
        None => ptr::null_mut(),
    }
}

/// Returns the MIME type for the cover image discovered by `sena_game_cover_path_from_dir`.
#[no_mangle]
pub unsafe extern "C" fn sena_game_cover_mime_from_dir(
    game_root_utf8: *const c_char,
) -> *mut c_char {
    match path_from_cstr(game_root_utf8)
        .ok()
        .and_then(|root| cover_candidate(&root))
        .and_then(|path| mime_for_path(&path).map(str::to_owned))
    {
        Some(mime) => cstring_lossy(mime),
        None => ptr::null_mut(),
    }
}

/// Runs the desktop Sena engine with `game_root_utf8` as the game directory.
#[no_mangle]
pub unsafe extern "C" fn sena_run_entry(game_root_utf8: *const c_char) -> i32 {
    sena_run_entry_nls(game_root_utf8, ptr::null(), ptr::null())
}

/// Runs the desktop Sena engine with an explicit text/resource encoding.
#[no_mangle]
pub unsafe extern "C" fn sena_run_entry_nls(
    game_root_utf8: *const c_char,
    nls_utf8: *const c_char,
    lang_utf8: *const c_char,
) -> i32 {
    let game_root = match path_from_cstr(game_root_utf8) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("sena_run_entry: {err}");
            return 2;
        }
    };

    #[cfg(not(feature = "desktop"))]
    {
        let _ = (game_root, nls_utf8, lang_utf8);
        eprintln!("This build requires a native host using sena_host_create");
        return 2;
    }
    #[cfg(feature = "desktop")]
    match crate::native_vm::run_sena(crate::native_vm::SenaConfig {
        game_root: Some(game_root),
        nls: nls_from_cstr(nls_utf8),
        lang: lang_from_cstr(lang_utf8),
        ..Default::default()
    }) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("sena_run_entry failed: {err:?}");
            1
        }
    }
}

/// Diagnostic checkpoint APIs use the portable save format, not native PAL exports.
#[no_mangle]
pub unsafe extern "C" fn sena_host_save_checkpoint(handle: *mut c_void, slot: i32) -> i32 {
    let Some(host) = handle.cast::<SenaFfiHost>().as_mut() else { return 0; };
    match host.engine.save_checkpoint(slot) {
        Ok(_) => 1,
        Err(error) => { eprintln!("save checkpoint failed: {error:#}"); 0 }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sena_host_load_checkpoint(handle: *mut c_void, slot: i32) -> i32 {
    let Some(host) = handle.cast::<SenaFfiHost>().as_mut() else { return 0; };
    if host.frame_generation != 0 { return 0; }
    match host.engine.load_checkpoint(slot) {
        Ok(()) => 1,
        Err(error) => { eprintln!("load checkpoint failed: {error:#}"); 0 }
    }
}

/// Creates a platform-owned PAL host that renders each frame into an RGBA buffer.
#[no_mangle]
pub unsafe extern "C" fn sena_host_create(
    game_root_utf8: *const c_char,
    nls_utf8: *const c_char,
    lang_utf8: *const c_char,
    width: u32,
    height: u32,
) -> *mut c_void {
    let game_root = match path_from_cstr(game_root_utf8) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("sena_host_create: {err}");
            return ptr::null_mut();
        }
    };
    let nls = nls_from_cstr(nls_utf8);
    let lang = lang_from_cstr(lang_utf8);
    init_android_diagnostics();
    log::info!(
        "sena_host_create: root={} nls={nls:?} lang={lang:?} surface={}x{}",
        game_root.display(),
        width.max(1),
        height.max(1)
    );
    let config = crate::native_vm::EngineConfig {
        game_root: Some(game_root),
        nls,
        lang,
        ..Default::default()
    };
    // Keep the external host independent of desktop windowing dependencies.
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Engine::new(config)));
    let mut engine = match built {
        Ok(Ok(engine)) => engine,
        Ok(Err(err)) => {
            eprintln!("sena_host_create failed: {err:?}");
            return ptr::null_mut();
        }
        Err(_) => {
            // The panic hook installed above has already logged the message.
            eprintln!("sena_host_create panicked");
            return ptr::null_mut();
        }
    };
    let (surface_width, surface_height) =
        engine.window_size_from_config(width.max(1), height.max(1));
    engine.handle_event(PalEvent::Resized {
        width: surface_width,
        height: surface_height,
    });

    Box::into_raw(Box::new(SenaFfiHost {
        engine,
        rgba: Vec::new(),
        frame_width: surface_width.max(1),
        frame_height: surface_height.max(1),
        frame_generation: 0,
        last_status: 0,
        messagebox: NativeMessageboxState::default(),
        scene_rendering: false,
        native_scene: None,
    })) as *mut c_void
}

/// Advances the host by one platform frame and refreshes the RGBA framebuffer.
#[no_mangle]
pub extern "C" fn sena_host_step(handle: *mut c_void, dt_ms: u32) -> i32 {
    let acknowledged = {
        let mut results = messagebox_results().lock().unwrap();
        match results.get(&(handle as usize)).copied() {
            Some((id, true)) => {
                results.remove(&(handle as usize));
                Some(id)
            }
            _ => None,
        }
    };
    let Some(host) = host_from_handle(handle) else {
        return 2;
    };
    if let Some(request_id) = acknowledged {
        host.messagebox.acknowledge(request_id, 0);
    }
    if host.messagebox.pending_request_id.is_some() {
        host.engine.input_begin_frame();
        return 0;
    }
    if host.last_status != 0 {
        return host.last_status;
    }
    host.engine.handle_event(PalEvent::RedrawRequested);
    // Borrowed views expire at the next step. Release the previous scene before
    // script resource replacement, so it cannot pin retired full-size pixels.
    host.native_scene = None;
    let updated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        host.engine
            .update_with_delta(Duration::from_millis(dt_ms.max(1).min(250) as u64))
    }));
    let frame = match updated {
        Ok(Ok(frame)) => frame,
        Ok(Err(err)) => {
            eprintln!("sena_host_step failed: {err:?}");
            return report_host_error(handle, &format!("Engine frame failed: {err:#}"));
        }
        Err(_) => {
            // A panic mid-frame leaves the engine in an unknown state; stop the
            // run the way any other step failure does instead of aborting.
            eprintln!("sena_host_step panicked");
            return report_host_error(handle, "The engine stopped after an unexpected frame error.");
        }
    };
    host.frame_width = frame.scene.logical_width.max(1);
    host.frame_height = frame.scene.logical_height.max(1);
    #[cfg(any(target_os = "vita", target_os = "horizon"))]
    if host.frame_generation % 120 == 119 {
        eprintln!(
            "[console] frame={} status={} pc={:#x} textures={} source_bytes={} draws={}",
            host.frame_generation + 1,
            frame.runtime_status,
            host.engine.runtime().map_or(0, |runtime| runtime.pc()),
            frame.scene.textures.len(),
            frame.scene.textures.iter().map(|texture| texture.pixels.len()).sum::<usize>(),
            frame.scene.commands.len(),
        );
    }
    if host.scene_rendering {
        host.native_scene = Some(NativeScene::new(frame.scene));
        host.rgba.clear();
    } else {
        host.rgba = rasterize_scene_rgba(&frame.scene);
        host.native_scene = None;
    }
    host.frame_generation = host.frame_generation.wrapping_add(1);
    if let RuntimeStatus::Faulted { .. } = &frame.runtime_status {
        return report_host_error(handle, &frame.runtime_status.to_string());
    }
    let terminal = runtime_status_is_terminal(&frame.runtime_status);
    host.engine.input_begin_frame();
    host.last_status = if terminal { 1 } else { 0 };
    host.last_status
}

#[no_mangle]
pub extern "C" fn sena_host_resize(handle: *mut c_void, width: u32, height: u32) {
    if let Some(host) = host_from_handle(handle) {
        host.engine.handle_event(PalEvent::Resized {
            width: width.max(1),
            height: height.max(1),
        });
    }
}

#[no_mangle]
pub extern "C" fn sena_host_touch(handle: *mut c_void, phase: i32, x: f64, y: f64) {
    let Some(host) = host_from_handle(handle) else {
        return;
    };
    host.engine.set_host_touch_available(true);
    host.engine
        .handle_event(PalEvent::Input(InputEvent::CursorMoved { x, y }));
    match phase {
        0 => host
            .engine
            .handle_event(PalEvent::Input(InputEvent::MouseInput {
                button: MouseButton::Left,
                pressed: true,
            })),
        2 | 3 => host
            .engine
            .handle_event(PalEvent::Input(InputEvent::MouseInput {
                button: MouseButton::Left,
                pressed: false,
            })),
        _ => {}
    }
}

#[no_mangle]
pub unsafe extern "C" fn sena_host_key(handle: *mut c_void, key_utf8: *const c_char, pressed: i32) {
    let Some(host) = host_from_handle(handle) else {
        return;
    };
    let Some(key_name) = string_from_cstr(key_utf8) else {
        return;
    };
    host.engine
        .handle_event(PalEvent::Input(InputEvent::Keyboard {
            key_name,
            pressed: pressed != 0,
        }));
}

#[no_mangle]
pub extern "C" fn sena_host_frame_rgba(handle: *mut c_void) -> *const u8 {
    host_from_handle(handle)
        .map(|host| if host.rgba.is_empty() { ptr::null() } else { host.rgba.as_ptr() })
        .unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn sena_host_frame_width(handle: *mut c_void) -> u32 {
    host_from_handle(handle)
        .map(|host| host.frame_width)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn sena_host_frame_height(handle: *mut c_void) -> u32 {
    host_from_handle(handle)
        .map(|host| host.frame_height)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn sena_host_frame_generation(handle: *mut c_void) -> u64 {
    host_from_handle(handle)
        .map(|host| host.frame_generation)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn sena_host_destroy(handle: *mut c_void) {
    messagebox_results().lock().unwrap().remove(&(handle as usize));
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle as *mut SenaFfiHost));
        }
    }
}

/// The native-activity entry point the Android platform never calls.
///
/// `winit`'s `android-native-activity` backend links the native-activity glue,
/// and the glue is what the application library is supposed to complete: the
/// entry point belongs to the app, not to the glue. This platform drives the
/// engine from Java through JNI and a `SurfaceView`
/// (`platform/android/app/src/main/cpp/sena_jni.cpp`), so nothing ever calls
/// it -- but the symbol has to exist, because an undefined one makes the
/// dynamic linker refuse to load the library at all
/// (`dlopen failed: cannot locate symbol "android_main"`), which is what the
/// launcher saw when it asked the engine for a game's title.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn android_main(_app: *mut c_void) {
    log::warn!(
        "android_main was called, but the Android host drives the engine through JNI"
    );
}

/// Installs the logcat logger and the panic hook, once per process.
///
/// Android throws a library's stdout and stderr away, so without this a panic
/// reaches the developer as nothing but a SIGABRT tombstone and every
/// `log::warn!` in the engine is lost. The desktop hosts keep their own logger
/// (`env_logger` in the CLI), so this is Android-only.
#[cfg(target_os = "android")]
fn init_android_diagnostics() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag("sena")
                .with_max_level(log::LevelFilter::Info),
        );
        std::panic::set_hook(Box::new(|info| {
            log::error!("panic: {info}");
        }));
    });
}

#[cfg(not(target_os = "android"))]
fn init_android_diagnostics() {}

/// Registers the JNI handles the audio backend reaches the platform through.
///
/// `cpal`'s AAudio host calls `ndk_context::android_context()`, which panics
/// until the context is registered; that panic unwinds out of `extern "C"` and
/// aborts the process, so the first SOUND init took the whole player down with
/// it. `platform/android/app/src/main/cpp/sena_jni.cpp` passes the `JavaVM` and
/// a global reference to the application context.
#[no_mangle]
pub extern "C" fn sena_android_init_context(java_vm_ptr: *mut c_void, context_ptr: *mut c_void) {
    init_android_diagnostics();

    #[cfg(target_os = "android")]
    {
        if java_vm_ptr.is_null() || context_ptr.is_null() {
            log::warn!(
                "sena_android_init_context: null JavaVM or context; audio backends cannot start"
            );
            return;
        }
        // `initialize_android_context` asserts it is only called once.
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe {
            ndk_context::initialize_android_context(java_vm_ptr, context_ptr);
        });
        log::info!("sena_android_init_context: registered the Android context");
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (java_vm_ptr, context_ptr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn sena_android_create(
    _native_window_ptr: *mut c_void,
    surface_width_px: u32,
    surface_height_px: u32,
    _native_scale_factor: f64,
    game_dir_utf8: *const c_char,
    nls_utf8: *const c_char,
    lang_utf8: *const c_char,
) -> *mut c_void {
    sena_host_create(
        game_dir_utf8,
        nls_utf8,
        lang_utf8,
        surface_width_px,
        surface_height_px,
    )
}

#[no_mangle]
pub extern "C" fn sena_android_set_native_messagebox_callback(
    handle: *mut c_void,
    callback: NativeMessageboxCallback,
    user_data: *mut c_void,
) {
    set_native_messagebox_callback(handle, callback, user_data);
}

#[no_mangle]
pub extern "C" fn sena_android_submit_messagebox_result(
    handle: *mut c_void,
    request_id: u64,
    value: i64,
) {
    submit_messagebox_result(handle, request_id, value);
}

#[no_mangle]
pub extern "C" fn sena_android_step(handle: *mut c_void, dt_ms: u32) -> i32 {
    sena_host_step(handle, dt_ms)
}

#[no_mangle]
pub extern "C" fn sena_android_resize(
    handle: *mut c_void,
    surface_width_px: u32,
    surface_height_px: u32,
) {
    sena_host_resize(handle, surface_width_px, surface_height_px);
}

#[no_mangle]
pub extern "C" fn sena_android_set_surface(
    handle: *mut c_void,
    _native_window_ptr: *mut c_void,
    surface_width_px: u32,
    surface_height_px: u32,
) {
    sena_host_resize(handle, surface_width_px, surface_height_px);
}

#[no_mangle]
pub extern "C" fn sena_android_touch(handle: *mut c_void, phase: i32, x_px: f64, y_px: f64) {
    sena_host_touch(handle, phase, x_px, y_px);
}

#[no_mangle]
pub extern "C" fn sena_android_destroy(handle: *mut c_void) {
    sena_host_destroy(handle);
}

#[no_mangle]
pub extern "C" fn sena_android_frame_rgba(handle: *mut c_void) -> *const u8 {
    sena_host_frame_rgba(handle)
}

#[no_mangle]
pub extern "C" fn sena_android_frame_width(handle: *mut c_void) -> u32 {
    sena_host_frame_width(handle)
}

#[no_mangle]
pub extern "C" fn sena_android_frame_height(handle: *mut c_void) -> u32 {
    sena_host_frame_height(handle)
}

#[no_mangle]
pub extern "C" fn sena_android_frame_generation(handle: *mut c_void) -> u64 {
    sena_host_frame_generation(handle)
}

#[no_mangle]
pub unsafe extern "C" fn sena_ios_create(
    _ui_view: *mut c_void,
    surface_width: u32,
    surface_height: u32,
    _native_scale_factor: f64,
    game_root_utf8: *const c_char,
    nls_utf8: *const c_char,
    lang_utf8: *const c_char,
) -> *mut c_void {
    sena_host_create(
        game_root_utf8,
        nls_utf8,
        lang_utf8,
        surface_width,
        surface_height,
    )
}

#[no_mangle]
pub extern "C" fn sena_ios_resize_viewport(
    handle: *mut c_void,
    _surface_width: u32,
    _surface_height: u32,
    _viewport_x: u32,
    _viewport_y: u32,
    viewport_width: u32,
    viewport_height: u32,
) {
    sena_host_resize(handle, viewport_width.max(1), viewport_height.max(1));
}

#[no_mangle]
pub unsafe extern "C" fn sena_ios_logical_size(
    _handle: *mut c_void,
    width_out: *mut u32,
    height_out: *mut u32,
) {
    let (w, h) = host_from_handle(_handle)
        .map(|host| {
            host.engine.logical_size_from_config(
                FrameScene::PAL_DEFAULT_WIDTH,
                FrameScene::PAL_DEFAULT_HEIGHT,
            )
        })
        .unwrap_or((1280, 720));
    if !width_out.is_null() {
        *width_out = w;
    }
    if !height_out.is_null() {
        *height_out = h;
    }
}

#[no_mangle]
pub extern "C" fn sena_ios_set_native_messagebox_callback(
    handle: *mut c_void,
    callback: NativeMessageboxCallback,
    user_data: *mut c_void,
) {
    set_native_messagebox_callback(handle, callback, user_data);
}

#[no_mangle]
pub extern "C" fn sena_ios_submit_messagebox_result(
    handle: *mut c_void,
    request_id: u64,
    value: i64,
) {
    submit_messagebox_result(handle, request_id, value);
}

#[no_mangle]
pub extern "C" fn sena_ios_step(handle: *mut c_void, dt_ms: u32) -> i32 {
    sena_host_step(handle, dt_ms)
}

#[no_mangle]
pub extern "C" fn sena_ios_resize(handle: *mut c_void, surface_width: u32, surface_height: u32) {
    sena_host_resize(handle, surface_width, surface_height);
}

#[no_mangle]
pub extern "C" fn sena_ios_touch(handle: *mut c_void, phase: i32, x_points: f64, y_points: f64) {
    sena_host_touch(handle, phase, x_points, y_points);
}

#[no_mangle]
pub extern "C" fn sena_ios_destroy(handle: *mut c_void) {
    sena_host_destroy(handle);
}

#[no_mangle]
pub extern "C" fn sena_ios_frame_rgba(handle: *mut c_void) -> *const u8 {
    sena_host_frame_rgba(handle)
}

#[no_mangle]
pub extern "C" fn sena_ios_frame_width(handle: *mut c_void) -> u32 {
    sena_host_frame_width(handle)
}

#[no_mangle]
pub extern "C" fn sena_ios_frame_height(handle: *mut c_void) -> u32 {
    sena_host_frame_height(handle)
}

#[no_mangle]
pub extern "C" fn sena_ios_frame_generation(handle: *mut c_void) -> u64 {
    sena_host_frame_generation(handle)
}

#[repr(C)]
pub struct SenaPumpHandle {
    _private: [u8; 0],
}

#[no_mangle]
pub unsafe extern "C" fn sena_pump_create(game_root_utf8: *const c_char) -> *mut SenaPumpHandle {
    sena_pump_create_nls(game_root_utf8, ptr::null(), ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn sena_pump_create_nls(
    game_root_utf8: *const c_char,
    nls_utf8: *const c_char,
    lang_utf8: *const c_char,
) -> *mut SenaPumpHandle {
    sena_host_create(game_root_utf8, nls_utf8, lang_utf8, 1280, 720) as *mut SenaPumpHandle
}

#[no_mangle]
pub extern "C" fn sena_pump_set_native_messagebox_callback(
    handle: *mut SenaPumpHandle,
    callback: NativeMessageboxCallback,
    user_data: *mut c_void,
) {
    set_native_messagebox_callback(handle as *mut c_void, callback, user_data);
}

#[no_mangle]
pub extern "C" fn sena_pump_submit_messagebox_result(
    handle: *mut SenaPumpHandle,
    request_id: u64,
    value: i64,
) {
    submit_messagebox_result(handle as *mut c_void, request_id, value);
}

#[no_mangle]
pub extern "C" fn sena_pump_step(_handle: *mut SenaPumpHandle, _timeout_ms: u32) -> i32 {
    sena_host_step(_handle as *mut c_void, _timeout_ms.max(1))
}

#[no_mangle]
pub extern "C" fn sena_pump_destroy(_handle: *mut SenaPumpHandle) {
    sena_host_destroy(_handle as *mut c_void);
}

#[cfg(test)]
mod native_messagebox_tests {
    use super::*;

    #[derive(Default)]
    struct CapturedMessage {
        calls: usize,
        request_id: u64,
        kind: i32,
        title: String,
        message: String,
    }

    extern "C" fn capture(
        user_data: *mut c_void,
        request_id: u64,
        kind: i32,
        title: *const c_char,
        message: *const c_char,
    ) {
        let captured = unsafe { &mut *(user_data as *mut CapturedMessage) };
        captured.calls += 1;
        captured.request_id = request_id;
        captured.kind = kind;
        captured.title = unsafe { CStr::from_ptr(title) }.to_string_lossy().into_owned();
        captured.message = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
    }

    fn host() -> Box<SenaFfiHost> {
        let mut config = crate::native_vm::engine::EngineConfig::default();
        config.audio.enabled = false;
        Box::new(SenaFfiHost {
            engine: Engine::new(config).unwrap(),
            rgba: vec![],
            frame_width: 1280,
            frame_height: 720,
            frame_generation: 0,
            last_status: 0,
            messagebox: NativeMessageboxState::default(),
            scene_rendering: false,
            native_scene: None,
        })
    }

    #[test]
    fn scene_software_fallback_does_not_step_or_invalidate_scene_views() {
        use super::scene::{sena_host_scene_rasterize, sena_host_scene_clear_color,
            sena_host_set_scene_rendering};
        let mut host = host();
        let mut scene = FrameScene::boot().with_logical_size(2, 2);
        scene.clear_color = [0.25, 0.5, 0.75, 1.0];
        let expected = rasterize_scene_rgba(&scene);
        host.native_scene = Some(NativeScene::new(scene));
        host.scene_rendering = true;
        host.frame_generation = 23;
        let handle = (&mut *host as *mut SenaFfiHost).cast();
        let pixels = sena_host_scene_rasterize(handle);
        assert!(!pixels.is_null());
        assert_eq!(unsafe { std::slice::from_raw_parts(pixels, 16) }, expected);
        assert_eq!(sena_host_scene_rasterize(handle), pixels);
        assert_eq!(sena_host_frame_generation(handle), 23);
        assert!(!sena_host_scene_clear_color(handle).is_null());
        sena_host_set_scene_rendering(handle, 0);
        assert!(sena_host_scene_rasterize(handle).is_null());
    }

    #[test]
    fn fatal_error_waits_for_matching_ack_then_remains_terminal() {
        let mut host = host();
        let handle = (&mut *host as *mut SenaFfiHost).cast();
        let mut captured = CapturedMessage::default();
        sena_android_set_native_messagebox_callback(
            handle,
            Some(capture),
            (&mut captured as *mut CapturedMessage).cast(),
        );
        assert_eq!(report_host_error(handle, "Failure\0details"), 0);
        assert_eq!(captured.calls, 1);
        assert_ne!(captured.request_id, 0);
        assert_eq!(captured.kind, 0);
        assert_eq!(captured.title, "Sena");
        assert_eq!(captured.message, "Failure details");
        assert_eq!(sena_host_step(handle, 16), 0);
        sena_ios_submit_messagebox_result(handle, captured.request_id + 1, 0);
        sena_ios_submit_messagebox_result(handle, captured.request_id, 1);
        assert_eq!(sena_host_step(handle, 16), 0);
        let address = handle as usize;
        let request_id = captured.request_id;
        std::thread::spawn(move || {
            sena_pump_submit_messagebox_result(address as *mut SenaPumpHandle, request_id, 0);
        }).join().unwrap();
        assert_eq!(sena_host_step(handle, 16), 2);
        assert_eq!(sena_host_step(handle, 16), 2);
        assert_eq!(host.frame_generation, 0);
        assert_eq!(captured.calls, 1);
    }

    extern "C" fn acknowledge_immediately(
        handle: *mut c_void,
        request_id: u64,
        _kind: i32,
        _title: *const c_char,
        _message: *const c_char,
    ) {
        sena_android_submit_messagebox_result(handle, request_id, 0);
    }

    #[test]
    fn synchronous_callback_and_missing_callback_are_safe() {
        let mut host = host();
        let handle = (&mut *host as *mut SenaFfiHost).cast();
        sena_ios_set_native_messagebox_callback(handle, Some(acknowledge_immediately), handle);
        assert_eq!(report_host_error(handle, "failure"), 0);
        assert_eq!(sena_host_step(handle, 16), 2);
        sena_pump_set_native_messagebox_callback(handle.cast(), None, ptr::null_mut());
        assert_eq!(report_host_error(handle, "failure"), 2);
        assert_eq!(sena_host_step(handle, 16), 2);
        sena_android_set_native_messagebox_callback(ptr::null_mut(), None, ptr::null_mut());
        sena_ios_submit_messagebox_result(ptr::null_mut(), 1, 0);
        assert_eq!(report_host_error(ptr::null_mut(), "failure"), 2);
    }

    #[test]
    fn late_ui_ack_after_host_destroy_is_ignored() {
        let handle = Box::into_raw(host()).cast();
        let mut captured = CapturedMessage::default();
        sena_android_set_native_messagebox_callback(
            handle,
            Some(capture),
            (&mut captured as *mut CapturedMessage).cast(),
        );
        assert_eq!(report_host_error(handle, "failure"), 0);
        sena_host_destroy(handle);
        sena_android_submit_messagebox_result(handle, captured.request_id, 0);
        assert!(!messagebox_results().lock().unwrap().contains_key(&(handle as usize)));
    }

    #[test]
    fn unregister_releases_pending_error_and_stale_results_do_not_ack_new_requests() {
        let mut state = NativeMessageboxState::default();
        state.register(Some(capture), ptr::null_mut());
        let (_, _, first) = state.begin().unwrap();
        assert!(state.begin().is_none());
        state.register(None, ptr::null_mut());
        assert_eq!(state.pending_request_id, None);
        state.register(Some(capture), ptr::null_mut());
        let (_, _, second) = state.begin().unwrap();
        assert_ne!(first, second);
        state.acknowledge(first, 0);
        assert_eq!(state.pending_request_id, Some(second));
        state.acknowledge(second, 0);
        assert_eq!(state.pending_request_id, None);
    }
}

/// Create an independent audio-thread handle. The VM handle stays on its owner thread.
#[cfg(feature = "host-audio")]
#[no_mangle]
pub extern "C" fn sena_host_audio_sink(handle: *mut c_void) -> *mut c_void {
    host_from_handle(handle)
        .map(|host| Box::into_raw(Box::new(host.engine.audio().host_sink())).cast())
        .unwrap_or(ptr::null_mut())
}

/// Fill at most 16384 stereo frames at 48000 Hz. The sink may outlive its engine.
#[cfg(feature = "host-audio")]
#[no_mangle]
pub unsafe extern "C" fn sena_audio_render_i16(sink: *mut c_void, output: *mut i16, frames: usize) {
    if output.is_null() || frames == 0 || frames > 16384 {
        return;
    }
    let output = std::slice::from_raw_parts_mut(output, frames * 2);
    output.fill(0);
    if let Some(sink) = (sink as *const crate::native_vm::audio::host_backend::AudioSink).as_ref() {
        sink.render_i16(output);
    }
}

/// Stop/join the audio consumer before releasing its handle.
#[cfg(feature = "host-audio")]
#[no_mangle]
pub unsafe extern "C" fn sena_audio_sink_destroy(sink: *mut c_void) {
    if !sink.is_null() {
        drop(Box::from_raw(sink as *mut crate::native_vm::audio::host_backend::AudioSink));
    }
}

/// Mouse buttons for native controller cursors: 0 left, 1 right, 2 middle.
#[no_mangle]
pub extern "C" fn sena_host_mouse_button(handle: *mut c_void, button: u32, pressed: i32) {
    let button = match button {
        0 => MouseButton::Left,
        1 => MouseButton::Right,
        2 => MouseButton::Middle,
        _ => return,
    };
    if let Some(host) = host_from_handle(handle) {
        host.engine.handle_event(PalEvent::Input(InputEvent::MouseInput {
            button,
            pressed: pressed != 0,
        }));
    }
}
