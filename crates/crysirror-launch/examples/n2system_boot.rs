//! Drive one installed N2System game with the shared GPU offscreen host.
//! Original resources stay mounted in place. No window or audio device is opened.
use anyhow::{Context, Result};
use crysirror_wgpu::{Gpu, RenderingPath, Renderer};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{buttons, HostServices},
    state::ClientId,
};
use crysirror_resource::{ArchiveMount, DecodeContext};
use crysirror_runtime::{AdapterRegistry, Project, Runtime};
use serde_json::Value;
use std::{path::Path, sync::Arc};

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    anyhow::ensure!(
        (5..=7).contains(&args.len()),
        "n2system_boot PLAN_JSON GAME_ROOT WRITABLE_ROOT OUTPUT_PNG [INPUT_JSON] [FRAMES]"
    );
    let plan: Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut project: Project = serde_json::from_value(plan["project"].clone())?;
    // Older detector output may still name archive fonts as project fonts.
    // Move the same declarations to the post-mount font preparation boundary.
    if project.clients[0].config.get("font_sources").is_none() {
        let sources = std::mem::take(&mut project.fonts)
            .into_iter()
            .enumerate()
            .map(|(i, (name, path))| {
                (
                    if name == "default" {
                        name
                    } else {
                        format!("n2-font-{i}")
                    },
                    path,
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        project.clients[0].config["font_sources"] = serde_json::to_value(sources)?;
    }
    let config: crysirror_n2system::Config =
        serde_json::from_value(project.clients[0].config.clone())?;
    let mut registry = AdapterRegistry::default();
    crysirror_n2system::register(&mut registry)?;
    let mut rt =
        Runtime::with_asset_root(project, Path::new(&args[2]), Path::new(&args[3]), registry)?;
    let client = ClientId(1);
    let storage = rt.host.storage(client)?;
    let mut context = DecodeContext::new(client);
    if let Some(material) = &config.archive_material {
        context.keys = material.provider(client, storage.clone())?;
    }
    context.parameters.insert(
        "nitroplus/npa".into(),
        crysirror_n2system::archive::profile()["parameters"]["nitroplus/npa"].clone(),
    );
    let format = crysirror_formats_arc::legacy_archive("nitroplus/npa")
        .context("NPA format registration")?;
    for mount in plan["mounts"].as_array().context("archive mounts")? {
        let source = mount["source"].as_str().context("archive source")?;
        let archive = format.open(
            storage.open_read(&VfsPath::new(Mount::Asset, source)?)?,
            &context,
        )?;
        rt.mount_provider(
            client,
            mount["target"].as_str().context("archive target")?,
            Arc::new(ArchiveMount::new(archive, context.clone())?),
        )?;
    }
    let instance = wgpu::Instance::default();
    let gpu = Arc::new(pollster::block_on(Gpu::request_for(
        &instance, None, RenderingPath::Raster,
    ))?);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("N2System offscreen display"),
        size: wgpu::Extent3d {width:rt.project.canvas[0],height:rt.project.canvas[1],depth_or_array_layers:1},
        mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,
        usage:wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::COPY_SRC,view_formats:&[],
    });
    let view = target.create_view(&Default::default());
    let mut renderer = Renderer::new(gpu, format);
    rt.set_canvas_image_presenter(true);
    rt.host.mesh_frame_history = Some(renderer.mesh_frame_history_provider());
    crysirror_launch::core::load_fonts(&mut rt, None)?;
    crysirror_launch::core::settle_vm_preparations(&mut rt)?;
    let track = crysirror_launch::input_track::InputTrack::load(args.get(5).map(Path::new))?;
    let frames = args.get(6).map(|s| s.parse::<u32>()).transpose()?.unwrap_or(6000);
    anyhow::ensure!((1..=100_000).contains(&frames),"offscreen frame limit");
    for frame in 0..frames {
        let mut input = track.at(frame);
        if frame % 30 == 0 && rt.host.engine.ui.dialogs.get(client.0).is_some() {
            input.buttons_down = buttons::ENTER;
            input.buttons_pressed = buttons::ENTER;
        }
        if let Err(error) = rt.tick(16, input) {
            println!(
                "{}",
                serde_json::json!({"failed_frame":frame,"runtime":rt.inspect()})
            );
            return Err(error.context(format!("native frame {frame}")));
        }
        crysirror_launch::core::materialize_canvases(&mut renderer, &mut rt)?;
        if !rt.presentation_pending() {
            let snapshot = rt.schedule_render_snapshot()?.wait(&mut rt)?;
            renderer.set_engine_frame(snapshot.layers)?;
            renderer.set_transition(snapshot.transition)?;
            rt.assets.with_frame_surfaces(snapshot.surfaces,|assets| {
                renderer.render(&snapshot.scene,assets,crysirror_wgpu::RenderTarget {
                    view:&view,size:rt.project.canvas,
                },rt.project.canvas,rt.project.background)
            })?;
        }
        if frame % 600 == 0 {
            eprintln!("native frame {frame}: {}", rt.inspect()["sessions"]);
        }
        if frame % 60 == 0 {
            renderer.gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        }
        if rt.vm_sessions_halted() {
            println!("{}", rt.inspect());
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    rt.assets.set_nonblocking(false);
    let snapshot = rt.schedule_render_snapshot()?.wait(&mut rt)?;
    renderer.set_scene3d(snapshot.simple.as_ref())?;
    renderer.set_pbr_scene(snapshot.pbr)?;
    renderer.set_engine_frame(snapshot.layers)?;
    renderer.set_transition(snapshot.transition)?;
    let image = rt.assets.with_frame_surfaces(snapshot.surfaces, |assets| {
        renderer.capture(&snapshot.scene, assets, rt.project.canvas, rt.project.background)
    })?;
    image.save_with_format(&args[4], image::ImageFormat::Png)?;
    println!("{}", rt.inspect());
    Ok(())
}
