// SPDX-License-Identifier: MPL-2.0
//! `cook-switch CAPTURE OUT`: compile every captured renderer module/entry
//! (crysirror-wgpu-deko3d `Capture`) to DKSH with devkitPro's UAM, and write
//! `OUT/index.json` for the backend's `Cooked` library. The index key is the
//! blake3 of the exact source the renderer passed to create_shader_module.
use anyhow::{Context, Result, bail, ensure};
use naga::back::glsl;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn uam() -> PathBuf {
    std::env::var_os("UAM").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/opt/devkitpro/tools/bin/uam"))
}

pub fn run(args: Vec<String>) -> Result<()> {
    ensure!(args.len() == 2, "usage: crysirror-shader-tools cook-switch CAPTURE_DIR OUT_DIR");
    let (capture, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    std::fs::create_dir_all(&out)?;
    let mut modules = serde_json::Map::new();
    let mut failures = Vec::new();
    let mut count = 0;
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&capture)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wgsl"))
        .collect();
    sources.sort();
    for path in sources {
        let hash = path.file_stem().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&path)?;
        ensure!(blake3::hash(source.as_bytes()).to_hex()[..32] == hash, "{hash}: captured source does not match its hash");
        let wanted: BTreeSet<(String, String)> = std::fs::read_to_string(path.with_extension("entries"))?
            .lines()
            .filter_map(|l| l.split_once(' ').map(|(s, e)| (s.to_string(), e.to_string())))
            .collect();
        let lowered = super::lower_depth_reads(&source);
        let module = match naga::front::wgsl::parse_str(&lowered) {
            Ok(m) => m,
            Err(e) => {
                failures.push(format!("{hash}: {}", e.emit_to_string(&source)));
                continue;
            }
        };
        let mut entries = Vec::new();
        for (stage, entry) in &wanted {
            let st = match stage.as_str() {
                "vertex" => naga::ShaderStage::Vertex,
                "fragment" => naga::ShaderStage::Fragment,
                _ => naga::ShaderStage::Compute,
            };
            let candidates: Vec<&naga::EntryPoint> = module.entry_points.iter().filter(|e| e.stage == st && (entry == "-" || &e.name == entry)).collect();
            let ep = match candidates.as_slice() {
                [one] => *one,
                _ => {
                    failures.push(format!("{hash}: {stage} entry {entry} not found uniquely"));
                    continue;
                }
            };
            match compile(&out, &hash, &module, ep) {
                Ok(v) => {
                    count += 1;
                    entries.push(v)
                }
                Err(e) => failures.push(format!("{hash}-{}: {e:#}", ep.name)),
            }
        }
        modules.insert(hash, Value::Array(entries));
    }
    std::fs::write(out.join("index.json"), serde_json::to_vec_pretty(&json!({ "modules": modules }))?)?;
    println!("{count} entries cooked, {} failures", failures.len());
    for f in &failures {
        eprintln!("{f}");
    }
    if !failures.is_empty() {
        bail!("cook incomplete");
    }
    Ok(())
}

fn compile(out: &Path, hash: &str, module: &naga::Module, ep: &naga::EntryPoint) -> Result<Value> {
    let mut single = module.clone();
    single.entry_points.retain(|e| e.name == ep.name && e.stage == ep.stage);
    naga::compact::compact(&mut single, naga::compact::KeepUnused::No);
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all()).validate(&single)?;
    let mut map = glsl::BindingMap::default();
    let mut slots: BTreeMap<&str, u8> = BTreeMap::new();
    let (mut uniforms, mut storage, mut images) = (Vec::new(), Vec::new(), Vec::new());
    let mut texture_slot = BTreeMap::new();
    for (_, global) in single.global_variables.iter() {
        let Some(b) = global.binding else { continue };
        let kind = match global.space {
            naga::AddressSpace::Uniform => "uniform",
            naga::AddressSpace::Storage { .. } => "storage",
            _ => match single.types[global.ty].inner {
                naga::TypeInner::Sampler { .. } => "sampler",
                naga::TypeInner::Image { class: naga::ImageClass::Storage { .. }, .. } => "image",
                _ => "texture",
            },
        };
        let slot = slots.entry(kind).or_default();
        map.insert(b, *slot);
        match kind {
            "uniform" => uniforms.push(json!([b.group, b.binding, *slot])),
            "storage" => storage.push(json!([b.group, b.binding, *slot])),
            "image" => images.push(json!([b.group, b.binding, *slot])),
            "texture" => {
                texture_slot.insert((b.group, b.binding), *slot);
            }
            _ => {}
        }
        *slot += 1;
    }
    ensure!(slots.get("uniform").copied().unwrap_or(0) <= 14, "deko3d uniform buffer limit");
    ensure!(slots.get("storage").copied().unwrap_or(0) <= 16, "deko3d storage buffer limit");
    ensure!(slots.get("texture").copied().unwrap_or(0) <= 32, "deko3d texture limit");
    ensure!(slots.get("image").copied().unwrap_or(0) <= 8, "deko3d image limit");
    let options = glsl::Options {
        version: glsl::Version::Desktop(460),
        writer_flags: glsl::WriterFlags::DRAW_PARAMETERS,
        binding_map: map,
        zero_initialize_workgroup_memory: true,
    };
    let pipeline = glsl::PipelineOptions { shader_stage: ep.stage, entry_point: ep.name.clone(), multiview: None };
    let mut text = String::new();
    let reflection = glsl::Writer::new(&mut text, &single, &info, &options, &pipeline, Default::default())?.write()?;
    let mut textures = Vec::new();
    let mut used_slots = BTreeMap::new();
    for (_, mapping) in reflection.texture_mapping {
        let tb = single.global_variables[mapping.texture].binding.context("texture binding")?;
        let slot = *texture_slot.get(&(tb.group, tb.binding)).context("texture slot")?;
        let sampler = mapping.sampler.and_then(|s| single.global_variables[s].binding);
        if let Some(previous) = used_slots.insert(slot, sampler) {
            ensure!(previous == sampler, "texture ({},{}) is sampled with two samplers", tb.group, tb.binding);
            continue;
        }
        let (sg, sb) = sampler.map_or((u32::MAX, 0), |s| (s.group, s.binding));
        textures.push(json!([slot, tb.group, tb.binding, sg, sb]));
    }
    let (stage, uam_stage) = match ep.stage {
        naga::ShaderStage::Vertex => ("vertex", "vert"),
        naga::ShaderStage::Fragment => ("fragment", "frag"),
        _ => ("compute", "comp"),
    };
    let name = format!("{hash}-{}", ep.name);
    let glsl_path = out.join(format!("{name}.glsl"));
    std::fs::write(&glsl_path, &text)?;
    let dksh = format!("{name}.dksh");
    let result = std::process::Command::new(uam())
        .arg(format!("--stage={uam_stage}"))
        .arg(format!("--out={}", out.join(&dksh).display()))
        .arg(&glsl_path)
        .output()
        .context("run uam")?;
    ensure!(result.status.success(), "uam: {}{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
    std::fs::remove_file(&glsl_path)?;
    Ok(json!({
        "entry": ep.name, "stage": stage, "file": dksh,
        "uniforms": uniforms, "storage": storage, "textures": textures, "images": images,
    }))
}
