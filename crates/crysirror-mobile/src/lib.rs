// SPDX-License-Identifier: MPL-2.0
//! C/JNI boundary only; engine ownership is in crysirror_player::embedded.
use crysirror_player::embedded::{self, Options, Session};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void, CStr, CString},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};
static NEXT: AtomicU64 = AtomicU64::new(1);
static SESSIONS: OnceLock<Mutex<BTreeMap<u64, Arc<Session>>>> = OnceLock::new();
fn sessions() -> &'static Mutex<BTreeMap<u64, Arc<Session>>> {
    SESSIONS.get_or_init(Default::default)
}
fn string(value: serde_json::Value) -> *mut c_char {
    CString::new(value.to_string()).unwrap().into_raw()
}
fn guarded(f: impl FnOnce() -> anyhow::Result<serde_json::Value>) -> *mut c_char {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => string(value),
        Ok(Err(e)) => string(serde_json::json!({"error":format!("{e:#}")})),
        Err(_) => string(serde_json::json!({"error":"Native host panicked"})),
    }
}
unsafe fn read<'a>(ptr: *const c_char) -> anyhow::Result<&'a str> {
    anyhow::ensure!(!ptr.is_null(), "null request");
    // SAFETY: C caller supplies a valid NUL-terminated UTF-8 string for this call.
    Ok(unsafe { CStr::from_ptr(ptr) }.to_str()?)
}
/// All returned JSON strings are owned; release with crysirror_string_free.
#[no_mangle]
pub unsafe extern "C" fn crysirror_probe(path: *const c_char) -> *mut c_char {
    guarded(|| embedded::probe(std::path::Path::new(unsafe { read(path)? })))
}
fn session(id: u64) -> anyhow::Result<Arc<Session>> {
    sessions()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unknown native session"))
}
fn insert_session(
    options: Options,
    size: [u32; 2],
    factory: embedded::SurfaceFactory,
) -> anyhow::Result<serde_json::Value> {
    let session = Session::start(options, size, factory)?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    sessions().lock().unwrap().insert(id, Arc::new(session));
    Ok(serde_json::json!({"session":id,"surface_generation":1}))
}
fn native_surface(
    instance: &wgpu::Instance,
    address: usize,
) -> anyhow::Result<wgpu::Surface<'static>> {
    #[cfg(target_vendor = "apple")]
    let target = wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(address as *mut c_void);
    #[cfg(target_os = "android")]
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(raw_window_handle::RawDisplayHandle::Android(
            raw_window_handle::AndroidDisplayHandle::new(),
        )),
        raw_window_handle: raw_window_handle::RawWindowHandle::AndroidNdk(
            raw_window_handle::AndroidNdkWindowHandle::new(
                std::ptr::NonNull::new(address as *mut c_void).unwrap(),
            ),
        ),
    };
    #[cfg(not(any(target_vendor = "apple", target_os = "android")))]
    {
        let _ = (instance, address);
        anyhow::bail!("native view host supports Apple and Android only");
    }
    #[cfg(any(target_vendor = "apple", target_os = "android"))]
    {
        // SAFETY: the factory retains the native lease (JNI) or the C caller
        // retains its view until detach for this generation or destroy returns.
        Ok(unsafe { instance.create_surface_unsafe(target) }?)
    }
}
#[no_mangle]
pub unsafe extern "C" fn crysirror_start(
    request: *const c_char,
    view: *mut c_void,
    width: u32,
    height: u32,
) -> *mut c_char {
    guarded(|| {
        let options: Options = serde_json::from_str(unsafe { read(request)? })?;
        anyhow::ensure!(!view.is_null(), "native surface is null");
        let address = view as usize;
        insert_session(
            options,
            [width, height],
            Box::new(move |instance| native_surface(instance, address)),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn crysirror_attach(
    id: u64,
    view: *mut c_void,
    width: u32,
    height: u32,
) -> *mut c_char {
    guarded(|| {
        anyhow::ensure!(!view.is_null(), "native surface is null");
        let address = view as usize;
        let generation = session(id)?.attach(
            [width, height],
            Box::new(move |instance| native_surface(instance, address)),
        )?;
        Ok(serde_json::json!({"surface_generation":generation}))
    })
}
#[no_mangle]
pub extern "C" fn crysirror_detach(id: u64, generation: u64) -> *mut c_char {
    guarded(|| {
        session(id)?.detach(generation)?;
        Ok(serde_json::json!({"ok":true}))
    })
}
/// Session handles remain valid across Surface replacement; destroyed handles fail.
#[no_mangle]
pub unsafe extern "C" fn crysirror_command(id: u64, request: *const c_char) -> *mut c_char {
    guarded(|| {
        let q: serde_json::Value = serde_json::from_str(unsafe { read(request)? })?;
        let s = session(id)?;
        match q["op"].as_str().unwrap_or("") {
            "status" => return Ok(serde_json::to_value(s.status())?),
            "resize" => s.resize(
                q["width"].as_u64().unwrap_or(0).min(16384) as u32,
                q["height"].as_u64().unwrap_or(0).min(16384) as u32,
            ),
            "input" => s.input(
                q["x"].as_f64().unwrap_or(0.),
                q["y"].as_f64().unwrap_or(0.),
                q["down"].as_u64().unwrap_or(0),
            ),
            "pause" => s.pause(q["paused"].as_bool().unwrap_or(true)),
            "close" => s.request_close(),
            _ => anyhow::bail!("unknown native host command"),
        }
        Ok(serde_json::json!({"ok":true}))
    })
}
#[no_mangle]
pub extern "C" fn crysirror_destroy(id: u64) {
    // Remove under lock, join outside it. A completed owner cannot retain its surface.
    let _ = std::panic::catch_unwind(|| {
        let session = sessions().lock().unwrap().remove(&id);
        drop(session);
    });
}
#[no_mangle]
pub unsafe extern "C" fn crysirror_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
#[cfg(target_os = "android")]
mod android;
