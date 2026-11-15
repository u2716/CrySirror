// SPDX-License-Identifier: MPL-2.0
use anyhow::{Context,Result,ensure};
use naga::back::glsl;
use serde_json::{json,Value};
use std::{path::{Path,PathBuf},collections::{BTreeMap,BTreeSet}};
mod cook;
mod cook_vita;
mod psv;

fn main()->Result<()> {
 if std::env::args().nth(1).as_deref()==Some("cook-switch") {return cook::run(std::env::args().skip(2).collect());}
 if std::env::args().nth(1).as_deref()==Some("cook-vita") {return cook_vita::run(std::env::args().skip(2).collect());}
 if std::env::args().nth(1).as_deref()==Some("import-psv") {return psv::import(std::env::args().skip(2).collect());}
 let mut args=std::env::args().skip(1);let target=args.next().context("usage: crysirror-shader-tools switch|psv OUTPUT | cook-switch CAPTURE OUT")?;
 let out=PathBuf::from(args.next().context("output directory")?);std::fs::create_dir_all(&out)?;
 ensure!(target=="switch"||target=="psv","unknown target");
 let mut entries=Vec::new();let mut errors=Vec::new();let mut covered=BTreeSet::new();
 if target=="psv" {
  for source in crysirror_shaders::native::psv_lite_sources().map_err(|e|anyhow::anyhow!(e))? {
   std::fs::write(out.join(source.file),source.source)?;
   entries.push(json!({"name":source.file.trim_end_matches(".cg"),"entry":source.entry,"stage":source.stage,"file":source.file,"status":"native_source_uncompiled","profile":"psv_tier"}));
  }
 }
 for source in crysirror_shaders::native::ray_lite_sources(&target).context("native shader target")? {
  std::fs::write(out.join(source.file),source.source)?;
  entries.push(json!({"name":"ray_lite","entry":source.entry,"stage":source.stage,"file":source.file,"status":"native_source_uncompiled","scene_abi":"rg32f-stackless-bvh-v1","compiler_optimization":if target=="psv" && source.stage=="fragment" {1}else{3}}));
 }
 for unit in crysirror_shaders::modules() {
  covered.extend(unit.files.iter().cloned());
  let lowered=lower_depth_reads(&unit.source);
  let module=match naga::front::wgsl::parse_str(&lowered) {Ok(m)=>m,Err(e)=>{errors.push(format!("{}: {}",unit.name,e.emit_to_string(&unit.source)));continue;}};
  if let Err(e)=naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module) {errors.push(format!("{}: {}",unit.name,e.emit_to_string(&unit.source)));continue;}
  for ep in &module.entry_points {
   let mut single=module.clone();single.entry_points.retain(|e|e.name==ep.name&&e.stage==ep.stage);
   naga::compact::compact(&mut single,naga::compact::KeepUnused::No);
   let name=format!("{}-{}",unit.name,ep.name);
   if unit.hardware_rt {entries.push(json!({"name":name,"module":unit.name,"entry":ep.name,"stage":format!("{:?}",ep.stage),"status":"requires_hardware_ray_queries","reduced_profile":"ray_lite","equivalent_replacement":false}));continue;}
   if target=="psv" {
    if psv::is_shared_2d(&unit.name) {
     entries.extend(psv::export(&out,&unit.name,&unit.source,&ep.name,ep.stage)?);
     continue;
    }
    let (status,detail)=psv_route(&unit.name,&ep.name);
    if status.ends_with("_pending") {errors.push(format!("{name}: {status}: {detail}"));}
    entries.push(json!({"name":name,"module":unit.name,"entry":ep.name,"stage":format!("{:?}",ep.stage),"status":status,"psv":detail}));
    continue;
   }
   let result=export_switch(&out,&name,&single);
   match result {Ok(mut item)=>{item["module"]=json!(unit.name);item["entry"]=json!(ep.name);entries.push(item);},Err(e)=>{errors.push(format!("{name}: {e:#}"));entries.push(json!({"name":name,"status":"failed","error":format!("{e:#}")}));}}
  }
 }
 for (file,_) in crysirror_shaders::wgsl::FILES {ensure!(covered.contains(*file),"shader not covered by catalog: {file}");}
 let manifest=json!({"target":target,"source_files":crysirror_shaders::wgsl::FILES.iter().map(|(n,s)|json!({"name":n,"blake3":blake3::hash(s.as_bytes()).to_hex().to_string()})).collect::<Vec<_>>(),"entries":entries,"errors":errors,"native_compiled":false,"device_execution":false});
 std::fs::write(out.join("manifest.json"),serde_json::to_vec_pretty(&manifest)?)?;
 if target=="psv" {
  let programs=manifest["entries"].as_array().unwrap().iter().filter_map(|e|e["file"].as_str()).filter(|f|f.ends_with(".cg")).collect::<Vec<_>>().join("\n");
  std::fs::write(out.join("programs.txt"),format!("{programs}\n"))?;
 }
 println!("{} entries, {} failures",manifest["entries"].as_array().unwrap().len(),manifest["errors"].as_array().unwrap().len());
 for error in manifest["errors"].as_array().unwrap() {eprintln!("{}",error.as_str().unwrap());}
 ensure!(manifest["errors"].as_array().unwrap().is_empty(),"shader source export incomplete");Ok(())
}
fn lower_depth_reads(source:&str)->String {
 let regex=regex::Regex::new(r"var\s+(\w+)\s*:\s*texture_depth_(2d|2d_array)\s*;").unwrap();
 let mut result=source.to_owned();
 let names=regex.captures_iter(source).map(|c|(c[1].to_owned(),c[2].to_owned())).collect::<Vec<_>>();
 for (name,kind) in names {
  let comparison=regex::Regex::new(&format!(r"\btextureSampleCompare(?:Level)?\s*\(\s*{}\b",regex::escape(&name))).unwrap();
  if comparison.is_match(source) {continue;}
  result=regex.replace_all(&result,|c:&regex::Captures|if c[1]==name {format!("var {name}: texture_{kind}<f32>;")}else{c[0].to_owned()}).into_owned();
  for function in ["textureLoad","textureSample","textureSampleLevel"] {
   let pattern=regex::Regex::new(&format!(r"\b{function}\s*\(\s*{}\s*,",regex::escape(&name))).unwrap();
   let starts=pattern.find_iter(&result).map(|m|m.start()).collect::<Vec<_>>();
   for start in starts.into_iter().rev() {
    let open=result[start..].find('(').unwrap()+start;let mut level=0;let mut end=None;
    for (i,ch) in result[open..].char_indices() {if ch=='(' {level+=1;}if ch==')' {level-=1;if level==0 {end=Some(open+i+1);break;}}}
    if let Some(end)=end {result.insert_str(end,".x");}
   }
  }
 }
 result
}
fn export_switch(out:&Path,name:&str,module:&naga::Module)->Result<Value> {
 let ep=&module.entry_points[0];
 let info=naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(module)?;
 let mut map=glsl::BindingMap::default();let mut count:BTreeMap<&str,u8>=BTreeMap::new();let mut bindings=Vec::new();
 for (_,global) in module.global_variables.iter() {
  let Some(binding)=global.binding else{continue};
  let kind=match global.space {
   naga::AddressSpace::Uniform=>"uniform",naga::AddressSpace::Storage{..}=>"storage",
   _=>match module.types[global.ty].inner {naga::TypeInner::Sampler{..}=>"sampler",naga::TypeInner::Image{class:naga::ImageClass::Storage{..},..}=>"image",_=>"texture"}
  };
  let slot=count.entry(kind).or_default();map.insert(binding,*slot);
  bindings.push(json!({"name":global.name,"group":binding.group,"binding":binding.binding,"kind":kind,"slot":*slot}));*slot+=1;
 }
 ensure!(count.get("uniform").copied().unwrap_or(0)<=16,"deko3d uniform buffer limit");
 ensure!(count.get("storage").copied().unwrap_or(0)<=16,"deko3d storage buffer limit");
 ensure!(count.get("texture").copied().unwrap_or(0)<=32,"deko3d texture limit");
 ensure!(count.get("image").copied().unwrap_or(0)<=8,"deko3d image limit");
 let options=glsl::Options {version:glsl::Version::Desktop(460),//WGSL instance_index includes the base instance: GLSL 460 gl_BaseInstance,
 //not a driver constant buffer uniform (UAM rejects c[0x1] uniforms).
 writer_flags:glsl::WriterFlags::DRAW_PARAMETERS,binding_map:map,zero_initialize_workgroup_memory:true};
 let pipeline=glsl::PipelineOptions {shader_stage:ep.stage,entry_point:ep.name.clone(),multiview:None};
 let mut source=String::new();
 let reflection={let mut writer=glsl::Writer::new(&mut source,module,&info,&options,&pipeline,Default::default())?;writer.write()?};
 let mut textures=Vec::new();for (uniform,mapping) in reflection.texture_mapping {textures.push(json!({"uniform":uniform,"texture":module.global_variables[mapping.texture].binding.map(|b|[b.group,b.binding]),"sampler":mapping.sampler.and_then(|s|module.global_variables[s].binding).map(|b|[b.group,b.binding])}));}
 let file=format!("{name}.glsl");std::fs::write(out.join(&file),source)?;
 Ok(json!({"name":name,"stage":format!("{:?}",ep.stage),"workgroup_size":ep.workgroup_size,"status":"source_translated","file":file,"bindings":bindings,"combined_samplers":textures}))
}

/// Where each desktop entry goes on the PSV tier (crysirror-render-tier
/// Tier::Vita). `*_pending` entries are required work, never fallbacks.
fn psv_route(module:&str,entry:&str)->(&'static str,&'static str) {
 match (module,entry) {
  (_,"vs_main") if matches!(module,"ssao"|"taa"|"ssr"|"brdf"|"prefilter"|"tonemap"|"bloom"|"copy_hdr"|"exposure"|"probe")=>("tier_program","lite_fullscreen_v.cg"),
  ("pbr","vs_main")=>("tier_program","lite_mesh_v.cg"),
  ("pbr","vs_skinned")=>("tier_program","lite_skinned_v.cg"),
  ("pbr","fs_main")=>("tier_program","lite_mesh_f.cg"),
  ("shadow","vs_main")=>("tier_program","lite_shadow_v.cg"),
  ("shadow","vs_skinned")=>("tier_program","lite_shadow_skinned_v.cg"),
  ("shadow","fs_main")=>("tier_program","lite_shadow_f.cg"),
  ("sky","vs_main")=>("tier_program","lite_fullscreen_v.cg"),
  ("sky","fs_main")=>("tier_program","lite_sky_f.cg"),
  ("bloom","fs_bloom_down")=>("tier_program","lite_bloom_down_f.cg"),
  ("bloom","fs_bloom_up")=>("tier_program","lite_bloom_up_f.cg"),
  ("tonemap","fs_tonemap")=>("tier_program","lite_tonemap_f.cg"),
  ("exposure","fs_exposure")=>("tier_program","lite_luminance_f.cg + lite_reduce_f.cg + lite_exposure_f.cg"),
  ("prefilter","fs_prefilter")=>("tier_program","lite_prefilter_f.cg (64 samples, CPU Hammersley)"),
  ("probe","fs_cube_to_equirect")=>("tier_program","lite_probe_capture_f.cg (6x1 face strip)"),
  ("copy_hdr","fs_copy")=>("tier_program","lite_copy_f.cg"),
  ("particle","vs_main")=>("tier_program","lite_particle_v.cg"),
  ("particle","fs_add")=>("tier_program","lite_particle_add_f.cg"),
  ("particle","fs_alpha")=>("tier_program","lite_particle_alpha_f.cg"),
  ("mesh","vs_main")=>("tier_program","lite_legacy_mesh_v.cg (exact)"),
  ("mesh","fs_main")=>("tier_program","lite_legacy_mesh_f.cg (exact)"),
  ("brdf","fs_brdf")=>("tier_cpu","crysirror_render_tier::derived::brdf_lut (512 samples, exact)"),
  ("ssao",_)=>("tier_substituted","AmbientOcclusion -> MaterialOcclusion"),
  ("ssr",_)=>("tier_substituted","ScreenSpaceReflections -> ReflectionProbes"),
  ("taa",_)=>("tier_substituted","TemporalAa -> Msaa"),
  ("prepass",_)=>("tier_substituted","no depth prepass consumer (SSAO/SSR/TAA substituted)"),
  ("vsm_mark",_)=>("tier_substituted","VirtualShadows -> CascadedShadows"),
  ("lod_select",_)=>("tier_substituted","GeometryLod GPU selection -> CPU cluster selection"),
  ("vfx_compute"|"vfx_cull"|"vfx_sort",_)=>("tier_substituted","GpuVfx -> CpuParticles (deterministic CPU simulation)"),
  ("raytrace",_)=>("tier_substituted","compute ray tracing -> ray_lite fragment profile"),
  ("ray_lite",_)=>("tier_program","ray_lite_v.cg / ray_lite_f.cg (the same ABI v1 profile, hand-written native programs)"),
  ("rt_geometry"|"rt_denoise"|"rt_alpha_bake",_)=>("tier_substituted","hardware ray-tracing support pass; no RT path on this tier"),
  ("gi_project",_)=>("tier_cpu","crysirror_render_tier::derived::project_probe + sample_volume (no visibility moments)"),
  ("vfx_render","vs_sprite")=>("tier_program","lite_vfx_sprite_v.cg (CPU-expanded corners)"),
  ("vfx_render","vs_mesh")=>("tier_program","lite_vfx_mesh_v.cg (GXM instance streams)"),
  ("vfx_render","vs_ribbon")=>("tier_program","lite_vfx_ribbon_v.cg"),
  ("vfx_render","fs_alpha")=>("tier_program","lite_vfx_alpha_f.cg (no soft-particle depth fade)"),
  ("vfx_render","fs_premultiplied")=>("tier_program","lite_vfx_premultiplied_f.cg (no soft-particle depth fade)"),
  ("vfx_render","fs_additive")=>("tier_program","lite_vfx_additive_f.cg (no soft-particle depth fade)"),
  ("vfx_render","fs_multiply")=>("tier_program","lite_vfx_multiply_f.cg (no soft-particle depth fade)"),
  _=>("unclassified_pending","no PSV route recorded"),
 }
}
