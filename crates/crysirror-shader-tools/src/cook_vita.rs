// SPDX-License-Identifier: MPL-2.0
//! `cook-vita CAPTURE OUT`: join every captured renderer module/entry
//! (crysirror-wgpu-deko3d `Capture`) with its already compiled GXM program in
//! the PSV catalog (`crysirror-shaders/native/psv/manifest.json`) and write
//! `OUT/index.json` for crysirror-wgpu-gxm. No shader is compiled here: GXP
//! comes only from the native catalog. The key is the blake3 of the exact
//! WGSL the renderer passed to create_shader_module, as for Switch.
//!
//! Reflection follows the backend's slot contract: uniform/storage slots are
//! per-entry indices; texture slots are the program's GXM texture units.
//! Texture/sampler pairs come from naga, since the catalog records textures
//! only. Entries absent from the catalog are listed as unavailable so the
//! driver can refuse their draws visibly instead of the device panicking.
use anyhow::{Context, Result, bail, ensure};
use naga::back::glsl;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub fn run(args: Vec<String>) -> Result<()> {
    ensure!(args.len() == 2, "usage: crysirror-shader-tools cook-vita CAPTURE_DIR OUT_DIR");
    let (capture, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    std::fs::create_dir_all(&out)?;
    let manifest: Value = serde_json::from_str(crysirror_shaders::native::PSV_MANIFEST)?;
    let catalog = manifest["entries"].as_array().context("manifest entries")?;
    let mut modules = serde_json::Map::new();
    let (mut cooked, mut unavailable) = (0, Vec::new());
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&capture)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wgsl"))
        .collect();
    sources.sort();
    let mut modules_in: Vec<(String, String, BTreeSet<(String, String)>)> = Vec::new();
    for path in sources {
        let hash = path.file_stem().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&path)?;
        ensure!(blake3::hash(source.as_bytes()).to_hex()[..32] == hash, "{hash}: captured source does not match its hash");
        let wanted: BTreeSet<(String, String)> = std::fs::read_to_string(path.with_extension("entries"))?
            .lines()
            .filter_map(|l| l.split_once(' ').map(|(s, e)| (s.to_string(), e.to_string())))
            .collect();
        modules_in.push((hash, source, wanted));
    }
    // The renderer creates its box filter only when a blurred transition
    // first runs, which short captures rarely reach. Its sources are fixed
    // (composite_filter.rs: RGBA32F horizontal sums, RGBA16F or, without
    // RGBA16F storage, RGBA8 output).
    for format in ["rgba32float", "rgba16float", "rgba8unorm"] {
        let source = crysirror_shaders::wgsl::COMPOSITE_FILTER.replace("__OUTPUT_FORMAT__", format);
        let hash = blake3::hash(source.as_bytes()).to_hex()[..32].to_string();
        if !modules_in.iter().any(|m| m.0 == hash) {
            modules_in.push((hash, source, BTreeSet::from([("compute".to_string(), "cs_main".to_string())])));
        }
    }
    // Like the box filter, lite rays run only when a scene selects compute
    // ray tracing on a GPU without compute storage.
    {
        let source = crysirror_shaders::wgsl::RAY_LITE.to_string();
        let hash = blake3::hash(source.as_bytes()).to_hex()[..32].to_string();
        if !modules_in.iter().any(|m| m.0 == hash) {
            modules_in.push((hash, source, BTreeSet::from([("vertex".to_string(), "vs_main".to_string()), ("fragment".to_string(), "fs_main".to_string())])));
        }
    }
    for (hash, source, wanted) in modules_in {
        if source == crysirror_shaders::wgsl::RAY_LITE {
            modules.insert(hash, ray_lite_entries(catalog)?);
            cooked += 2;
            continue;
        }
        let module = naga::front::wgsl::parse_str(&super::lower_depth_reads(&source))
            .map_err(|e| anyhow::anyhow!("{hash}: {}", e.emit_to_string(&source)))?;
        // Without the new mask flags, the two quad revisions execute the
        // same code. Link the legacy GXP for those draws and retain each
        // program's actual source hash in the native catalogue.
        let legacy_quad = (source == crysirror_shaders::wgsl::QUAD_V2)
            .then(|| blake3::hash(crysirror_shaders::wgsl::QUAD.as_bytes()).to_hex());
        let mut entries = Vec::new();
        for (stage, entry) in &wanted {
            let st = match stage.as_str() {
                "vertex" => naga::ShaderStage::Vertex,
                "fragment" => naga::ShaderStage::Fragment,
                _ => naga::ShaderStage::Compute,
            };
            let ep = module
                .entry_points
                .iter()
                .find(|e| e.stage == st && (entry == "-" || &e.name == entry))
                .with_context(|| format!("{hash}: {stage} entry {entry}"))?;
            let box_filter = st == naga::ShaderStage::Compute && is_box_filter(&source);
            let programs: Vec<&Value> = catalog
                .iter()
                .filter(|c| {
                    c["status"] == "gxp_compiled"
                        && c["entry"] == ep.name.as_str()
                        && if box_filter {
                            // The fragment lowering drops the storage output
                            // declaration, the only line the formats differ
                            // in: every output format shares one program.
                            c["module"] == "composite_filter_rgba16float" && c["compute_lowering"].is_string()
                        } else {
                            c["wgsl_blake3"].as_str().is_some_and(|w| {
                                w.starts_with(&hash) || legacy_quad.as_ref().is_some_and(|legacy| w == legacy.as_str())
                            }) && c["stage"] == stage.as_str()
                        }
                })
                .collect();
            if legacy_quad.is_some() && matches!(ep.name.as_str(), "fs_filtered" | "fs_filtered_premultiply") {
                for sampling in 0..5 {
                    let variants = programs.iter().filter(|p| p["module"] == "quad_v2"
                        && p["distortion_variant"] == 0 && p["sampling_variant"] == sampling);
                    ensure!(variants.count() == 1, "{hash} {}: missing or duplicate mask variant {sampling}", ep.name);
                }
            }
            if programs.is_empty() {
                unavailable.push(format!("{hash} {stage} {}", ep.name));
                entries.push(json!({"entry": ep.name, "stage": stage, "unavailable": true}));
                continue;
            }
            let mut entry = entry_json(&module, ep, stage, &programs)?;
            if box_filter {
                // Storage output (group 0, binding 2) becomes the render
                // target of the driver's fragment passes.
                entry["images"] = json!([[0, 2, 0]]);
                entry["gxm"]["lowering"] = json!("box_filter");
            }
            entries.push(entry);
            cooked += 1;
        }
        modules.insert(hash, Value::Array(entries));
    }
    std::fs::write(out.join("index.json"), serde_json::to_vec_pretty(&json!({ "modules": modules }))?)?;
    println!("{cooked} entries joined with GXP, {} without a native program", unavailable.len());
    for u in &unavailable {
        println!("unavailable: {u}");
    }
    Ok(())
}

/// `ray_lite.wgsl` joined with the hand-written native lite ray programs
/// (`crysirror-shaders/native/psv/ray_lite_{v,f}.cg`), which declare the same
/// interface: attributes `position`/`uv`, bare `ray_*` uniforms in the
/// WGSL member order, node/triangle textures on units 0/1.
fn ray_lite_entries(catalog: &[Value]) -> Result<Value> {
    for program in ["ray_lite_v", "ray_lite_f"] {
        ensure!(
            catalog.iter().any(|c| c["status"] == "gxp_compiled" && c["gxp"].as_str().is_some_and(|g| g.ends_with(&format!("{program}.gxp")))),
            "{program} is not compiled in the PSV catalog"
        );
    }
    let members: Vec<Value> = ["ray_nodes", "ray_triangles", "ray_eye", "ray_corner", "ray_dx", "ray_dy", "ray_light", "ray_sun", "ray_ambient", "ray_sky_low", "ray_sky_high"]
        .iter()
        .enumerate()
        .map(|(i, name)| json!({"name": name, "offset": i * 16, "components": 4, "kind": "f32"}))
        .collect();
    Ok(json!([
        {"entry": "vs_main", "stage": "vertex", "uniforms": [], "storage": [], "textures": [], "images": [],
         "gxm": {"blocks": [], "attributes": [[0, "position"], [1, "uv"]],
                 "variants": [{"program": "ray_lite_v", "wipe": null, "distortion": null, "sampling": null, "textures": [], "blocks": []}]}},
        {"entry": "fs_main", "stage": "fragment", "uniforms": [[0, 0, 0]], "storage": [],
         "textures": [[0, 0, 1, u32::MAX, 0], [1, 0, 2, u32::MAX, 0]], "images": [],
         "gxm": {"blocks": [{"slot": 0, "storage": false, "name": "", "members": members}],
                 "variants": [{"program": "ray_lite_f", "wipe": null, "distortion": null, "sampling": null,
                               "textures": [{"slot": 0, "unit": 0, "dimensions": ""}, {"slot": 1, "unit": 1, "dimensions": ""}],
                               "blocks": [0]}]}}
    ]))
}

/// The shared renderer's separable box filter, for any storage output format.
fn is_box_filter(source: &str) -> bool {
    const FORMATS: [&str; 3] = ["rgba32float", "rgba16float", "rgba8unorm"];
    FORMATS.iter().any(|f| crysirror_shaders::wgsl::COMPOSITE_FILTER.replace("__OUTPUT_FORMAT__", f) == source)
}

fn program_name(c: &Value) -> Result<String> {
    let gxp = c["gxp"].as_str().context("gxp path")?;
    Ok(gxp.trim_start_matches("gxp/").trim_end_matches(".gxp").to_string())
}

fn entry_json(module: &naga::Module, ep: &naga::EntryPoint, stage: &str, programs: &[&Value]) -> Result<Value> {
    // Texture/sampler pairing and slots, exactly as the GLSL backend sees them.
    let mut single = module.clone();
    single.entry_points.retain(|e| e.name == ep.name && e.stage == ep.stage);
    naga::compact::compact(&mut single, naga::compact::KeepUnused::No);
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all()).validate(&single)?;
    let mut map = glsl::BindingMap::default();
    let mut slot = 0u8;
    for (_, g) in single.global_variables.iter() {
        if let Some(b) = g.binding {
            map.insert(b, slot);
            slot = slot.wrapping_add(1);
        }
    }
    let options = glsl::Options {
        version: glsl::Version::Desktop(460),
        writer_flags: glsl::WriterFlags::empty(),
        binding_map: map,
        zero_initialize_workgroup_memory: false,
    };
    let pipeline = glsl::PipelineOptions { shader_stage: ep.stage, entry_point: ep.name.clone(), multiview: None };
    let mut text = String::new();
    let reflection = glsl::Writer::new(&mut text, &single, &info, &options, &pipeline, Default::default())?.write()?;
    let mut samplers: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
    for (_, m) in reflection.texture_mapping {
        let t = single.global_variables[m.texture].binding.context("texture binding")?;
        let s = m.sampler.and_then(|s| single.global_variables[s].binding).map_or((u32::MAX, 0), |s| (s.group, s.binding));
        if let Some(previous) = samplers.insert((t.group, t.binding), s) {
            ensure!(previous == s, "texture ({},{}) sampled with two samplers", t.group, t.binding);
        }
    }
    // Specializations drop unused resources, so their texture units differ.
    // Backend slots are indices into the union of every variant's bindings;
    // each variant maps those slots onto its own units and parameters.
    let mut texture_keys: Vec<(u32, u32)> = Vec::new();
    let mut block_keys: Vec<(u32, u32)> = Vec::new();
    let mut blocks = Vec::new();
    for p in programs {
        for b in p["bindings"].as_array().context("bindings")? {
            let key = (b["group"].as_u64().context("group")? as u32, b["binding"].as_u64().context("binding")? as u32);
            if !texture_keys.contains(&key) {
                texture_keys.push(key);
            }
        }
        for u in p["uniforms"].as_array().context("uniforms")? {
            let key = (u["group"].as_u64().context("group")? as u32, u["binding"].as_u64().context("binding")? as u32);
            if block_keys.contains(&key) {
                continue;
            }
            block_keys.push(key);
            let members: Vec<Value> = u["members"]
                .as_array()
                .context("members")?
                .iter()
                .map(|m| {
                    let ty = m["type"].as_str().unwrap_or("");
                    let components = if ty.contains("Quad") { 4 } else if ty.contains("Tri") { 3 } else if ty.contains("Bi") { 2 } else { 1 };
                    let kind = if ty.contains("Uint") { "u32" } else if ty.contains("Sint") { "i32" } else { "f32" };
                    json!({"name": m["name"], "offset": m["byte_offset"], "components": components, "kind": kind})
                })
                .collect();
            // A lowered storage array (quad filters) is still bound as storage
            // by the renderer; the driver selects one record per draw.
            let is_storage = single.global_variables.iter().any(|(_, gv)| {
                gv.binding.is_some_and(|b| b.group == key.0 && b.binding == key.1) && matches!(gv.space, naga::AddressSpace::Storage { .. })
            });
            blocks.push(json!({"slot": block_keys.len() - 1, "storage": is_storage, "name": u["name"], "members": members}));
        }
    }
    let textures: Vec<Value> = texture_keys
        .iter()
        .enumerate()
        .map(|(slot, &(g, n))| {
            let (sg, sb) = samplers.get(&(g, n)).copied().unwrap_or((u32::MAX, 0));
            json!([slot, g, n, sg, sb])
        })
        .collect();
    let (mut uniforms, mut storage) = (Vec::new(), Vec::new());
    for (slot, &(g, n)) in block_keys.iter().enumerate() {
        if blocks[slot]["storage"] == true {
            storage.push(json!([g, n, slot]));
        } else {
            uniforms.push(json!([g, n, slot]));
        }
    }
    let mut variants = Vec::new();
    for p in programs {
        let mut units = Vec::new();
        for b in p["bindings"].as_array().context("bindings")? {
            let key = (b["group"].as_u64().unwrap() as u32, b["binding"].as_u64().unwrap() as u32);
            let slot = texture_keys.iter().position(|k| *k == key).unwrap();
            units.push(json!({"slot": slot, "unit": b["texture_unit"], "dimensions": b["dimensions"]}));
        }
        let mut used_blocks = Vec::new();
        for u in p["uniforms"].as_array().context("uniforms")? {
            let key = (u["group"].as_u64().unwrap() as u32, u["binding"].as_u64().unwrap() as u32);
            used_blocks.push(block_keys.iter().position(|k| *k == key).unwrap());
        }
        let mut variant = json!({
            "program": program_name(p)?,
            "wipe": p["wipe_variant"],
            "distortion": p["distortion_variant"],
            "sampling": p["sampling_variant"],
            "textures": units,
            "blocks": used_blocks,
        });
        if p["module"] == "quad_v2" {
            variant["quad_mask"] = json!(true);
        }
        variants.push(variant);
    }
    let first = programs[0];
    if variants.len() > 1 && first["wipe_variant"].is_null() && first["distortion_variant"].is_null() {
        bail!("{}: multiple programs without a variant key", ep.name);
    }
    Ok(json!({
        "entry": ep.name, "stage": stage,
        "uniforms": uniforms, "storage": storage, "textures": textures, "images": [],
        "gxm": {"blocks": blocks, "variants": variants},
    }))
}
