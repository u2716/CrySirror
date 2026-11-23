// SPDX-License-Identifier: MPL-2.0
//! Run a scripted PBR project on a real GPU without a surface or image readback.
//! The explicit device fence measures serialized frame latency, including streaming waits.
use anyhow::{ensure, Context, Result};
use crysirror_engine::{App, ScriptHost};
#[path = "../../../../crates/crysirror-launch/src/input_track.rs"]
mod input_track;
use crysirror_runtime::{AdapterRegistry, Runtime};
use crysirror_script_js::JsHost;
use crysirror_wgpu::{Gpu, RenderTarget, Renderer};
use input_track::InputTrack;
use serde_json::json;
use std::{path::PathBuf, sync::Arc, time::Instant};

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let project = PathBuf::from(args.next().context("expected project.json")?);
    let script = PathBuf::from(args.next().context("expected script file")?);
    let state = PathBuf::from(args.next().context("expected writable state directory")?);
    let track_path = PathBuf::from(args.next().context("expected input track")?);
    let frames: u32 = args
        .next()
        .context("expected frame count")?
        .to_string_lossy()
        .parse()?;
    ensure!(
        (2..=10_000).contains(&frames) && args.next().is_none(),
        "invalid benchmark arguments"
    );
    let track = InputTrack::load(Some(&track_path))?;
    ensure!(
        !track.has_text(),
        "this scene benchmark accepts button and pointer input only"
    );
    let start = Instant::now();
    let spec = crysirror_runtime::Project::load(&project)?;
    let size = spec.canvas;
    let root = crysirror_runtime::runtime::asset_root(&project, &spec)?;
    let mut runtime = Runtime::with_asset_root(spec, &root, &state, AdapterRegistry::default())?;
    runtime.assets.set_prepared_budget(1200 * 1024 * 1024)?;
    let mut app = App::new(runtime, "game")?;
    let mut js = JsHost::new("scripts");
    js.add_source(
        script
            .file_name()
            .and_then(|s| s.to_str())
            .context("script name")?,
        &std::fs::read_to_string(&script)?,
    )?;
    app.add_script_host(Box::new(js))?;
    app.init()?;
    app.start()?;
    let gpu = Arc::new(pollster::block_on(Gpu::request(
        &wgpu::Instance::default(),
        None,
    ))?);
    let format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene benchmark"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let mut renderer = Renderer::new(gpu.clone(), format);
    let setup_ms = start.elapsed().as_secs_f64() * 1000.;
    let mut latencies = Vec::new();
    let mut stages = Vec::new();
    for frame in 0..frames {
        let start = Instant::now();
        debug_assert!(track.text_at(frame).is_none());
        app.frame(if frame == 0 { 0 } else { 16 }, track.at(frame))?;
        let tick = Instant::now();
        app.prepare_render_offline()?;
        let ready = Instant::now();
        let rt = app.runtime_mut()?;
        let snapshot = rt.schedule_render_snapshot()?.wait(rt)?;
        renderer.set_profiler(rt.profiler.clone(), snapshot.frame);
        renderer.set_scene3d(snapshot.simple.as_ref())?;
        renderer.set_pbr_scene(snapshot.pbr)?;
        renderer.set_engine_frame(snapshot.layers)?;
        renderer.set_transition(snapshot.transition)?;
        rt.assets.with_frame_surfaces(snapshot.surfaces, |assets| {
            renderer.render(
                &snapshot.scene,
                assets,
                RenderTarget { view: &view, size },
                size,
                rt.project.background,
            )
        })?;
        let submitted = Instant::now();
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        let end = Instant::now();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.;
        latencies.push(ms(start, end));
        let cells: Vec<_> = rt
            .host
            .engine
            .world
            .streaming
            .iter()
            .filter(|(_, cell)| cell.loaded.is_some())
            .map(|(name, _)| name.as_str())
            .collect();
        stages.push(json!({"tick":ms(start,tick),"readiness":ms(tick,ready),"extract_encode_submit":ms(ready,submitted),
            "gpu_fence":ms(submitted,end),"cells":cells}));
    }
    let mut sorted = latencies.clone();
    sorted.sort_by(f64::total_cmp);
    let worst = |start: usize| {
        latencies
            .iter()
            .enumerate()
            .skip(start)
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(frame, ms)| json!({"frame":frame,"ms":ms}))
    };
    let percentile = |p: f64| sorted[(p * sorted.len() as f64).ceil() as usize - 1];
    println!(
        "{}",
        json!({
            "adapter": format!("{:?}",gpu.adapter.get_info()), "canvas":size,
            "setup_ms":setup_ms, "completed_frame_ms":latencies,"frame_stages_ms":stages,
            "summary":{"frames":frames,"p50_ms":percentile(0.50),"p95_ms":percentile(0.95),
                       "worst":worst(0),"worst_after_first":worst(1)},
            "profile":renderer.profile_report_offline()?,
            "prepared":app.runtime_mut()?.inspect()["tasks"]["resident_bytes"],
        })
    );
    Ok(())
}
