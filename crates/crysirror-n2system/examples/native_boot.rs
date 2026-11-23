//! Drive installed native scripts and shared services without a window/device.
//! This checks VM/resource behavior, not raster output or original-save parity.
use anyhow::{Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{buttons, HostServices, InputFrame},
    state::ClientId,
};
use crysirror_resource::{ArchiveMount, DecodeContext};
use crysirror_runtime::{AdapterRegistry, Project, Runtime};
use serde_json::Value;
use std::{path::Path, sync::Arc};

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    anyhow::ensure!(
        args.len() == 4,
        "native_boot PLAN_JSON GAME_ROOT WRITABLE_ROOT"
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
    for frame in 0..12000 {
        let mut input = InputFrame::default();
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
        if frame % 600 == 0 {
            eprintln!("native frame {frame}: {}", rt.inspect()["sessions"]);
        }
        if rt.vm_sessions_halted() {
            println!("{}", rt.inspect());
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("{}", rt.inspect());
    Ok(())
}
