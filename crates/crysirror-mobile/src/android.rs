// SPDX-License-Identifier: MPL-2.0
use super::*;
use jni::{
    objects::{JClass, JObject, JString},
    sys::{jint, jlong, jstring},
    JNIEnv,
};
// Application context (not Activity) is retained for this library's process lifetime.
// CPAL's device enumeration uses ndk-context even with a Java-owned SurfaceView.
static CONTEXT: Mutex<Option<jni::objects::GlobalRef>> = Mutex::new(None);
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_initialize(
    mut env: JNIEnv,
    _class: JClass,
    context: JObject,
) -> jstring {
    let ptr = guarded(|| {
        let mut slot = CONTEXT.lock().unwrap();
        if slot.is_none() {
            let vm = env.get_java_vm()?;
            let global = env.new_global_ref(context)?;
            // SAFETY: serialized exactly-once initialization before any session
            // starts; JavaVM and retained application context outlive all workers.
            unsafe {
                ndk_context::initialize_android_context(
                    vm.get_java_vm_pointer().cast(),
                    global.as_obj().as_raw().cast(),
                )
            };
            *slot = Some(global);
        }
        Ok(serde_json::json!({"ok":true}))
    });
    reply(&mut env, ptr)
}
struct WindowLease(usize);
impl WindowLease {
    fn address(&self) -> usize {
        self.0
    }
}
impl Drop for WindowLease {
    fn drop(&mut self) {
        // SAFETY: exactly the NDK reference acquired by window_factory; the
        // owner drops its Surface before dropping this factory-held lease.
        unsafe { ndk_sys::ANativeWindow_release(self.0 as *mut ndk_sys::ANativeWindow) };
    }
}
fn window_factory(env: &JNIEnv, surface: JObject) -> anyhow::Result<embedded::SurfaceFactory> {
    // SAFETY: surface is a live JNI local reference during conversion. NDK
    // returns an acquired native reference, independent of the Java local ref.
    let window =
        unsafe { ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface.as_raw()) };
    anyhow::ensure!(!window.is_null(), "Android Surface has no native window");
    let lease = WindowLease(window as usize);
    Ok(Box::new(move |instance| {
        native_surface(instance, lease.address())
    }))
}
fn reply(env: &mut JNIEnv, ptr: *mut c_char) -> jstring {
    // SAFETY: only consumes strings allocated by this crate's guarded entry points.
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { crysirror_string_free(ptr) };
    env.new_string(text)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_probe(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    let ptr = guarded(|| {
        let path: String = env.get_string(&path)?.into();
        embedded::probe(std::path::Path::new(&path))
    });
    reply(&mut env, ptr)
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_start(
    mut env: JNIEnv,
    _class: JClass,
    request: JString,
    surface: JObject,
    width: jint,
    height: jint,
) -> jstring {
    let ptr = guarded(|| {
        anyhow::ensure!(
            CONTEXT.lock().unwrap().is_some(),
            "Android host context is not initialized"
        );
        let request: String = env.get_string(&request)?.into();
        let options: Options = serde_json::from_str(&request)?;
        anyhow::ensure!(width > 0 && height > 0, "native surface size");
        insert_session(
            options,
            [width as u32, height as u32],
            window_factory(&env, surface)?,
        )
    });
    reply(&mut env, ptr)
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_command(
    mut env: JNIEnv,
    _class: JClass,
    id: jlong,
    request: JString,
) -> jstring {
    let ptr = guarded(|| {
        let request: String = env.get_string(&request)?.into();
        let request = CString::new(request)?;
        let ptr = unsafe { crysirror_command(id as u64, request.as_ptr()) };
        let text = unsafe { CString::from_raw(ptr) };
        Ok(serde_json::from_slice(text.as_bytes())?)
    });
    reply(&mut env, ptr)
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_attach(
    mut env: JNIEnv,
    _class: JClass,
    id: jlong,
    surface: JObject,
    width: jint,
    height: jint,
) -> jstring {
    let ptr = guarded(|| {
        anyhow::ensure!(width > 0 && height > 0, "native surface size");
        let generation = session(id as u64)?.attach(
            [width as u32, height as u32],
            window_factory(&env, surface)?,
        )?;
        Ok(serde_json::json!({"surface_generation":generation}))
    });
    reply(&mut env, ptr)
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_detach(
    mut env: JNIEnv,
    _class: JClass,
    id: jlong,
    generation: jlong,
) -> jstring {
    let ptr = crysirror_detach(id as u64, generation as u64);
    reply(&mut env, ptr)
}
#[no_mangle]
pub extern "system" fn Java_org_crysirror_player_Native_destroy(
    _env: JNIEnv,
    _class: JClass,
    id: jlong,
) {
    crysirror_destroy(id as u64);
}
