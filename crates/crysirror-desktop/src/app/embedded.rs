// SPDX-License-Identifier: MPL-2.0
//! Native view host. The session thread exclusively owns VM, renderer and audio;
//! UIKit/Android/SwiftUI submit bounded state, never touch a Runtime or GPU.
use super::{materialize_canvases, Audio, Core};
use anyhow::{Context, Result};
use crysirror_core::service::{DisplayInfo, InputFrame};
use crysirror_runtime::{Project, Runtime};
use crysirror_wgpu::{geometry::Viewport, Gpu, RenderTarget, Renderer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub game: PathBuf,
    pub state_dir: PathBuf,
    #[serde(default)]
    pub font: Option<PathBuf>,
    #[serde(default)]
    pub nls: Option<String>,
    #[serde(default)]
    pub muted: bool,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: &'static str,
    pub message: String,
    pub frames: u64,
    pub attached: bool,
    pub surface_generation: u64,
}
pub type SurfaceFactory =
    Box<dyn Fn(&wgpu::Instance) -> Result<wgpu::Surface<'static>> + Send + Sync + 'static>;
struct SurfaceRequest {
    generation: u64,
    factory: Option<SurfaceFactory>,
}
struct Mailbox {
    size: [u32; 2],
    pointer: [f64; 2],
    down: u64,
    pressed: u64,
    pointer_activity: u64,
    paused: bool,
    stop: bool,
    close: bool,
    status: Status,
    surface_request: Option<SurfaceRequest>,
    surface_revision: u64,
    surface_in_use: Option<u64>,
    worker_done: bool,
}
struct Shared {
    state: Mutex<Mailbox>,
    changed: Condvar,
}
pub struct Session {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

pub fn probe(game: &std::path::Path) -> Result<Value> {
    if game.is_file() && game.extension().is_some_and(|s| s == "json") {
        let p = crate::load_project(game)?;
        let adapter = p.clients.first().map(|c| c.adapter.as_str()).unwrap_or("");
        return Ok(
            json!({"engine":adapter,"title":p.name,"nls_options":crate::nls::options(adapter)}),
        );
    }
    let plan = crate::detect::detect(game)?;
    let adapter = plan.project["clients"][0]["adapter"].as_str().unwrap_or("");
    Ok(
        json!({"engine":plan.engine,"title":plan.project["name"],"nls_options":crate::nls::options(adapter)}),
    )
}
impl Session {
    /// The factory and its native view lease stay alive until generation 1 is
    /// detached, replaced, or the session stops. Runtime ownership is independent
    /// of Surface lifetime; replacing a view never recreates the VM.
    pub fn start(
        options: Options,
        size: [u32; 2],
        surface: impl Fn(&wgpu::Instance) -> Result<wgpu::Surface<'static>> + Send + Sync + 'static,
    ) -> Result<Self> {
        anyhow::ensure!(
            size.iter().all(|n| *n > 0 && *n <= 16384),
            "native surface size"
        );
        let shared = Arc::new(Shared {
            state: Mutex::new(Mailbox {
                size,
                pointer: [0.; 2],
                down: 0,
                pressed: 0,
                pointer_activity: 0,
                paused: false,
                stop: false,
                close: false,
                status: Status {
                    phase: "loading",
                    message: String::new(),
                    frames: 0,
                    attached: false,
                    surface_generation: 0,
                },
                surface_request: Some(SurfaceRequest {
                    generation: 1,
                    factory: Some(Box::new(surface)),
                }),
                surface_revision: 1,
                surface_in_use: None,
                worker_done: false,
            }),
            changed: Condvar::new(),
        });
        let state = shared.clone();
        let worker = thread::Builder::new()
            .name("crysirror-session".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(options, &state)));
                let mut m = state.state.lock().unwrap_or_else(|p| p.into_inner());
                // run's stack (including any active native view lease) has
                // unwound before publishing the final retirement boundary.
                m.surface_request = None;
                m.surface_in_use = None;
                m.worker_done = true;
                m.status.attached = false;
                match result {
                    Ok(Ok(())) => m.status.phase = "stopped",
                    Ok(Err(e)) => {
                        m.status.phase = "failed";
                        m.status.message = format!("{e:#}");
                    }
                    Err(_) => {
                        m.status.phase = "failed";
                        m.status.message = "Native session panicked".into();
                    }
                }
                state.changed.notify_all();
            })?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }
    pub fn attach(&self, size: [u32; 2], factory: SurfaceFactory) -> Result<u64> {
        anyhow::ensure!(
            size.iter().all(|n| *n > 0 && *n <= 16384),
            "native surface size"
        );
        let mut m = self.shared.state.lock().unwrap();
        anyhow::ensure!(!m.worker_done && !m.stop, "native session has stopped");
        let generation = m
            .surface_revision
            .checked_add(1)
            .context("surface generation exhausted")?;
        m.surface_revision = generation;
        m.size = size;
        m.down = 0;
        m.pressed = 0;
        let retired = m.surface_request.replace(SurfaceRequest {
            generation,
            factory: Some(factory),
        });
        drop(m);
        drop(retired);
        self.shared.changed.notify_all();
        Ok(generation)
    }
    /// Blocks only until this particular view is no longer used by the owner.
    /// Boot with no active surface can detach immediately. A late callback for
    /// an older generation never removes a newer pending/active view.
    pub fn detach(&self, generation: u64) -> Result<()> {
        anyhow::ensure!(generation > 0, "native surface generation");
        let mut m = self.shared.state.lock().unwrap();
        if m.surface_revision == generation && !m.worker_done {
            let revision = m
                .surface_revision
                .checked_add(1)
                .context("surface generation exhausted")?;
            m.surface_revision = revision;
            m.surface_request = Some(SurfaceRequest {
                generation: revision,
                factory: None,
            });
            m.down = 0;
            m.pressed = 0;
            self.shared.changed.notify_all();
        }
        while m.surface_in_use == Some(generation) && !m.worker_done {
            m = self.shared.changed.wait(m).unwrap();
        }
        Ok(())
    }
    pub fn status(&self) -> Status {
        self.shared.state.lock().unwrap().status.clone()
    }
    pub fn resize(&self, width: u32, height: u32) {
        let mut m = self.shared.state.lock().unwrap();
        m.size = [width.min(16384), height.min(16384)];
        self.shared.changed.notify_one();
    }
    pub fn input(&self, x: f64, y: f64, down: u64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let mut m = self.shared.state.lock().unwrap();
        if m.paused || !m.status.attached {
            return;
        }
        if m.pointer != [x, y] || m.down != down {
            m.pointer_activity = m.pointer_activity.wrapping_add(1);
        }
        m.pointer = [x, y];
        m.pressed |= down & !m.down;
        m.down = down;
        self.shared.changed.notify_one();
    }
    pub fn pause(&self, paused: bool) {
        let mut m = self.shared.state.lock().unwrap();
        m.paused = paused;
        if paused {
            m.down = 0;
            m.pressed = 0;
        }
        self.shared.changed.notify_one();
    }
    pub fn request_close(&self) {
        self.shared.state.lock().unwrap().close = true;
        self.shared.changed.notify_one();
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .stop = true;
        self.shared.changed.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn prepare(options: &Options, shared: &Shared) -> Result<Core> {
    let cancelled = || shared.state.lock().unwrap().stop;
    anyhow::ensure!(!cancelled(), "native launch cancelled");
    let game = options.game.canonicalize().context("game source")?;
    std::fs::create_dir_all(&options.state_dir)?;
    let state = options.state_dir.canonicalize()?;
    let (mut project, root, plan) =
        if game.is_file() && game.extension().is_some_and(|s| s == "json") {
            let p = crate::load_project(&game)?;
            let root = crysirror_runtime::runtime::asset_root(&game, &p)?;
            (p, root, None)
        } else {
            anyhow::ensure!(game.is_dir(), "select a game folder or project JSON");
            let plan = crate::detect::detect(&game)?;
            (
                serde_json::from_value::<Project>(plan.project.clone())?,
                game,
                Some(plan),
            )
        };
    anyhow::ensure!(!cancelled(), "native launch cancelled");
    let mut value = serde_json::to_value(&project)?;
    crate::nls::apply(&mut value, options.nls.as_deref())?;
    project = serde_json::from_value(value)?;
    crate::integrated_roots::bind(&mut project, &root, &state, options.font.as_deref())?;
    let mut rt = Runtime::with_asset_root(project, &root, &state, crate::adapter_registry()?)?;
    // Interactive OS services need a presenter-specific implementation. Do not
    // invoke desktop subprocess dialogs from a mobile application.
    #[cfg(target_os = "macos")]
    crate::desktop_provider::install(&mut rt, false)?;
    rt.assets
        .decoders
        .register("g00", Arc::new(crysirror_siglus::SiglusImageDecoder))?;
    rt.set_audio_nonblocking(true);
    let font = options.font.clone().or_else(|| {
        (!rt.project.fonts.contains_key("default"))
            .then(crate::detect::system_font)
            .flatten()
    });
    super::load_fonts(&mut rt, font.as_deref())?;
    match plan {
        Some(plan) => crate::launch::bootstrap_interruptible(rt, &plan, cancelled),
        None => Ok(Core::Plain(rt)),
    }
}
/// The native reference captured by factory must outlive surface; fields drop
/// in declaration order. The shared renderer/device survive view replacement.
struct AttachedSurface {
    surface: wgpu::Surface<'static>,
    _factory: SurfaceFactory,
    config: wgpu::SurfaceConfiguration,
}
// GPU initialization is polled by the owner so a destroyed native view can
// cancel its request before acquiring a device. No UI callback joins a blocking
// pollster future. The waker only wakes this owner, never touches engine state.
fn request_gpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    shared: &Shared,
    generation: u64,
) -> Result<Gpu> {
    use std::{
        future::Future,
        task::{Context, Poll, Wake, Waker},
    };
    struct OwnerWake(thread::Thread);
    impl Wake for OwnerWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(OwnerWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(Gpu::request(instance, Some(surface)));
    loop {
        {
            let m = shared.state.lock().unwrap();
            anyhow::ensure!(
                !m.stop && m.surface_revision == generation,
                "native Surface request cancelled"
            );
        }
        match future.as_mut().poll(&mut context) {
            Poll::Ready(result) => return result,
            Poll::Pending => thread::park_timeout(Duration::from_millis(2)),
        }
    }
}
fn attach_surface(
    instance: &wgpu::Instance,
    factory: SurfaceFactory,
    size: [u32; 2],
    renderer: &mut Option<(Renderer, wgpu::TextureFormat)>,
    core: &mut Core,
    shared: &Shared,
    generation: u64,
) -> Result<AttachedSurface> {
    let surface = factory(instance)?;
    let gpu = match renderer.as_ref() {
        Some((r, _)) => r.gpu.clone(),
        None => Arc::new(request_gpu(instance, &surface, shared, generation)?),
    };
    anyhow::ensure!(
        gpu.adapter.is_surface_supported(&surface),
        "replacement Surface is incompatible with the current GPU"
    );
    let mut config = surface
        .get_default_config(&gpu.adapter, size[0], size[1])
        .context("native surface configuration")?;
    let caps = surface.get_capabilities(&gpu.adapter);
    if let Some((_, format)) = renderer.as_ref() {
        if caps.formats.contains(format) {
            config.format = *format;
        }
    }
    if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
    }
    config.present_mode = wgpu::PresentMode::Fifo;
    surface.configure(&gpu.device, &config);
    if renderer
        .as_ref()
        .is_none_or(|(_, format)| *format != config.format)
    {
        let next = Renderer::new(gpu, config.format);
        core.rt().host.mesh_frame_history = Some(next.mesh_frame_history_provider());
        *renderer = Some((next, config.format));
    }
    core.rt().set_canvas_image_presenter(true);
    Ok(AttachedSurface {
        surface,
        _factory: factory,
        config,
    })
}
fn run(options: Options, shared: &Shared) -> Result<()> {
    let mut core = prepare(&options, shared)?;
    let mut audio = Audio::new(options.muted);
    let result = (|| {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let mut attached: Option<AttachedSurface> = None;
        let mut renderer: Option<(Renderer, wgpu::TextureFormat)> = None;
        let mut pending: Option<crysirror_runtime::frame::PendingFrame> = None;
        let mut snapshot: Option<crysirror_runtime::frame::RenderSnapshot> = None;
        let mut previous = Instant::now();
        let mut paused = true;
        let mut frames = 0u64;
        let mut pointer_activity = None;
        let mut epoch = (core.rt().assets.store_id(), core.rt().restored);
        audio.pause()?;
        loop {
            let request = {
                let mut m = shared.state.lock().unwrap();
                if m.stop {
                    break;
                }
                let request = m.surface_request.take();
                // Retirement is published only after dropping the previous
                // Surface and its lease, never merely after dequeuing a request.
                if request.is_some() {
                    m.status.attached = false;
                }
                request
            };
            if let Some(request) = request {
                drop(attached.take());
                {
                    let mut m = shared.state.lock().unwrap();
                    m.surface_in_use = None;
                    shared.changed.notify_all();
                    // A detach/attach may have overtaken this dequeued request.
                    if m.surface_revision != request.generation {
                        continue;
                    }
                    if request.factory.is_some() {
                        m.surface_in_use = Some(request.generation);
                    }
                }
                if let Some(factory) = request.factory {
                    let size = shared.state.lock().unwrap().size;
                    let binding = attach_surface(
                        &instance,
                        factory,
                        size,
                        &mut renderer,
                        &mut core,
                        shared,
                        request.generation,
                    );
                    let mut m = shared.state.lock().unwrap();
                    match binding {
                        Ok(binding) if m.surface_revision == request.generation => {
                            attached = Some(binding);
                            m.status.attached = true;
                            m.status.surface_generation = request.generation;
                            m.status.message.clear();
                        }
                        Ok(binding) => {
                            drop(binding);
                            m.surface_in_use = None;
                        }
                        Err(error) => {
                            m.surface_in_use = None;
                            if m.surface_revision == request.generation {
                                m.status.message = format!("{error:#}");
                            }
                        }
                    }
                    shared.changed.notify_all();
                }
                previous = Instant::now();
            }
            let (size, pointer, down, pressed, activity, suspend, close) = {
                let mut m = shared.state.lock().unwrap();
                if m.stop {
                    break;
                }
                core.rt().host.window_focused = Some(!m.paused && attached.is_some());
                core.rt().host.window_minimized = Some(m.paused);
                let suspend = m.paused || m.size.contains(&0) || attached.is_none();
                if suspend {
                    if !paused {
                        audio.pause()?;
                        paused = true;
                    }
                    m.status.phase = if attached.is_none() {
                        "waiting_surface"
                    } else {
                        "paused"
                    };
                    // Process replacement/close before sleeping, including when
                    // the app remains suspended throughout surface recreation.
                    if m.surface_request.is_some() {
                        continue;
                    }
                    if m.close {
                        m.close = false;
                        drop(m);
                        if !core.rt().request_close() {
                            break;
                        }
                        continue;
                    }
                    let _guard = shared
                        .changed
                        .wait_timeout(m, Duration::from_millis(100))
                        .unwrap();
                    previous = Instant::now();
                    continue;
                }
                m.status.phase = "running";
                let pressed = if !core.frame_pending(&audio) && !audio.device_opening() {
                    std::mem::take(&mut m.pressed)
                } else {
                    0
                };
                (
                    m.size,
                    m.pointer,
                    m.down,
                    pressed,
                    m.pointer_activity,
                    suspend,
                    std::mem::take(&mut m.close),
                )
            };
            if paused && !suspend {
                audio.resume()?;
                paused = false;
                previous = Instant::now();
            }
            if close && !core.rt().request_close() {
                break;
            }
            let binding = attached.as_mut().expect("attached surface while running");
            let (renderer, _) = renderer.as_mut().expect("renderer with attached surface");
            let config = &mut binding.config;
            if [config.width, config.height] != size {
                config.width = size[0];
                config.height = size[1];
                binding.surface.configure(&renderer.gpu.device, config);
            }
            core.rt().host.display = Some(DisplayInfo {
                screen: size,
                work: [0, 0, size[0] as i32, size[1] as i32],
            });
            core.rt().host.engine.ui.pointer_surface = size;
            let mode = core
                .rt()
                .host
                .engine
                .ui
                .presentation_scale
                .map(|(_, m)| m)
                .unwrap_or_default();
            renderer.set_presentation_scale(mode);
            let vp = Viewport::with_mode(core.rt().project.canvas, size, mode);
            let [x, y] = vp.pointer(pointer[0], pointer[1], core.rt().project.canvas);
            if pointer_activity != Some(activity) {
                core.rt().host.note_pointer_activity()?;
                pointer_activity = Some(activity);
            }
            core.rt().host.pointer_observation = crysirror_core::service::PointerObservation {
                position: Some([x, y]),
                inside: Some(x >= 0 && y >= 0),
            };
            materialize_canvases(renderer, core.rt())?;
            if !core.frame_pending(&audio) && !audio.device_opening() {
                let now = Instant::now();
                let ms = now.duration_since(previous).as_millis().min(100) as u64;
                previous = now;
                core.begin_tick(
                    &mut audio,
                    if frames == 0 { 0 } else { ms },
                    InputFrame {
                        buttons_down: down,
                        buttons_pressed: pressed,
                        pointer_x: x,
                        pointer_y: y,
                    },
                )?;
            }
            materialize_canvases(renderer, core.rt())?;
            if !core.poll_tick(&mut audio)? {
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            if core.should_exit() {
                break;
            }
            let rt = core.rt();
            let next_epoch = (rt.assets.store_id(), rt.restored);
            if epoch != next_epoch {
                snapshot = None;
                pending = None;
                epoch = next_epoch;
            }
            if pending.is_none() && (snapshot.is_none() || !rt.presentation_pending()) {
                pending = Some(rt.schedule_render_snapshot()?);
            }
            if let Some(p) = &mut pending {
                if let Some(s) = p.poll(rt)? {
                    snapshot = Some(s);
                    pending = None;
                }
            }
            let Some(s) = &snapshot else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            let frame = match binding.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(f)
                | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                wgpu::CurrentSurfaceTexture::Lost => {
                    // Recreate the swap-chain surface from the same retained
                    // native view; this does not replace Runtime or GPU state.
                    binding.surface = (binding._factory)(&instance)?;
                    binding.surface.configure(&renderer.gpu.device, config);
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    binding.surface.configure(&renderer.gpu.device, config);
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    thread::sleep(Duration::from_millis(8));
                    continue;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    anyhow::bail!("native surface validation failed")
                }
            };
            let view = frame.texture.create_view(&Default::default());
            renderer.set_scene3d(s.simple.as_ref())?;
            renderer.set_pbr_scene(s.pbr.clone())?;
            renderer.set_engine_frame(s.layers.clone())?;
            renderer.set_transition(s.transition.clone())?;
            rt.assets
                .with_frame_surfaces(s.surfaces.clone(), |assets| {
                    renderer.render(
                        &s.scene,
                        assets,
                        RenderTarget { view: &view, size },
                        rt.project.canvas,
                        rt.project.background,
                    )
                })?;
            renderer.gpu.queue.present(frame);
            frames += 1;
            shared.state.lock().unwrap().status.frames = frames;
        }
        // Surface drops before its retained factory, before the renderer/device.
        drop(attached);
        Ok(())
    })();
    let shutdown = core.shutdown(&mut audio);
    result.and(shutdown)
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Lease(Arc<AtomicUsize>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn factory(drops: &Arc<AtomicUsize>) -> SurfaceFactory {
        let lease = Lease(drops.clone());
        Box::new(move |_| {
            let _ = &lease;
            anyhow::bail!("test factory must never acquire a GPU")
        })
    }
    fn loading(drops: &Arc<AtomicUsize>) -> Session {
        Session {
            worker: None,
            shared: Arc::new(Shared {
                changed: Condvar::new(),
                state: Mutex::new(Mailbox {
                    size: [640, 360],
                    pointer: [0.; 2],
                    down: 0,
                    pressed: 0,
                    pointer_activity: 0,
                    paused: false,
                    stop: false,
                    close: false,
                    status: Status {
                        phase: "loading",
                        message: String::new(),
                        frames: 73,
                        attached: false,
                        surface_generation: 0,
                    },
                    surface_request: Some(SurfaceRequest {
                        generation: 1,
                        factory: Some(factory(drops)),
                    }),
                    surface_revision: 1,
                    surface_in_use: None,
                    worker_done: false,
                }),
            }),
        }
    }
    #[test]
    fn detach_during_boot_releases_view_without_waiting_for_vm_or_resetting_frames() {
        let drops = Arc::new(AtomicUsize::new(0));
        let session = loading(&drops);
        session.detach(1).unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(session.status().frames, 73);
        let m = session.shared.state.lock().unwrap();
        assert_eq!(m.surface_revision, 2);
        assert!(m.surface_request.as_ref().unwrap().factory.is_none());
    }
    #[test]
    fn replacing_pending_view_retires_only_its_lease_and_ignores_late_detach() {
        let drops = Arc::new(AtomicUsize::new(0));
        let session = loading(&drops);
        let next = session.attach([800, 600], factory(&drops)).unwrap();
        assert_eq!(next, 2);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        session.detach(1).unwrap();
        {
            let m = session.shared.state.lock().unwrap();
            assert_eq!(m.surface_revision, 2);
            assert!(m.surface_request.as_ref().unwrap().factory.is_some());
            assert_eq!(m.status.frames, 73);
        }
        drop(session);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn detach_waits_until_owner_retires_active_surface() {
        let drops = Arc::new(AtomicUsize::new(0));
        let session = loading(&drops);
        {
            let mut m = session.shared.state.lock().unwrap();
            m.surface_request = None;
            m.surface_in_use = Some(1);
        }
        let shared = session.shared.clone();
        let done = Arc::new(AtomicUsize::new(0));
        let completion = done.clone();
        let caller = thread::spawn(move || {
            session.detach(1).unwrap();
            completion.store(1, Ordering::SeqCst);
        });
        {
            let mut m = shared.state.lock().unwrap();
            while m.surface_revision == 1 {
                m = shared.changed.wait(m).unwrap();
            }
            assert_eq!(done.load(Ordering::SeqCst), 0);
            m.surface_in_use = None;
            shared.changed.notify_all();
        }
        caller.join().unwrap();
        assert_eq!(done.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn late_destroy_of_old_view_never_detaches_active_successor() {
        let drops = Arc::new(AtomicUsize::new(0));
        let session = loading(&drops);
        {
            let mut m = session.shared.state.lock().unwrap();
            m.surface_request = None;
            m.surface_revision = 5;
            m.surface_in_use = Some(5);
            m.down = 3;
        }
        session.detach(1).unwrap();
        let m = session.shared.state.lock().unwrap();
        assert_eq!(m.surface_in_use, Some(5));
        assert!(m.surface_request.is_none());
        assert_eq!(m.down, 3);
    }
}
