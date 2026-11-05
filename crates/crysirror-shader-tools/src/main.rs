// SPDX-License-Identifier: MPL-2.0
use anyhow::{Context,Result,ensure};
use naga::back::glsl;
use serde_json::{json,Value};
use std::{path::{Path,PathBuf},collections::{BTreeMap,BTreeSet}};

fn main()->Result<()> {
 let mut args=std::env::args().skip(1);let target=args.next().context("usage: crysirror-shader-tools switch|psv OUTPUT")?;
 let out=PathBuf::from(args.next().context("output directory")?);std::fs::create_dir_all(&out)?;
 ensure!(target=="switch"||target=="psv","unknown target");
 let mut entries=Vec::new();let mut errors=Vec::new();let mut covered=BTreeSet::new();
 for source in crysirror_shaders::native::ray_lite_sources(&target).context("native shader target")? {
  std::fs::write(out.join(source.file),source.source)?;
  entries.push(json!({"name":"ray_lite","entry":source.entry,"stage":source.stage,"file":source.file,"status":"native_source_uncompiled","scene_abi":"rg32f-stackless-bvh-v1"}));
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
   let result=if target=="switch" {export_switch(&out,&name,&single)}else{Err(anyhow::anyhow!("native raster/compute equivalent not yet ported; ray_lite source is a separate reduced profile"))};
   match result {Ok(mut item)=>{item["module"]=json!(unit.name);item["entry"]=json!(ep.name);entries.push(item);},Err(e)=>{errors.push(format!("{name}: {e:#}"));entries.push(json!({"name":name,"status":"failed","error":format!("{e:#}")}));}}
  }
 }
 for (file,_) in crysirror_shaders::wgsl::FILES {ensure!(covered.contains(*file),"shader not covered by catalog: {file}");}
 let manifest=json!({"target":target,"source_files":crysirror_shaders::wgsl::FILES.iter().map(|(n,s)|json!({"name":n,"blake3":blake3::hash(s.as_bytes()).to_hex().to_string()})).collect::<Vec<_>>(),"entries":entries,"errors":errors,"native_compiled":false,"device_execution":false});
 std::fs::write(out.join("manifest.json"),serde_json::to_vec_pretty(&manifest)?)?;
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
 let options=glsl::Options {version:glsl::Version::Desktop(460),writer_flags:glsl::WriterFlags::empty(),binding_map:map,zero_initialize_workgroup_memory:true};
 let pipeline=glsl::PipelineOptions {shader_stage:ep.stage,entry_point:ep.name.clone(),multiview:None};
 let mut source=String::new();
 let reflection={let mut writer=glsl::Writer::new(&mut source,module,&info,&options,&pipeline,Default::default())?;writer.write()?};
 let mut textures=Vec::new();for (uniform,mapping) in reflection.texture_mapping {textures.push(json!({"uniform":uniform,"texture":module.global_variables[mapping.texture].binding.map(|b|[b.group,b.binding]),"sampler":mapping.sampler.and_then(|s|module.global_variables[s].binding).map(|b|[b.group,b.binding])}));}
 let file=format!("{name}.glsl");std::fs::write(out.join(&file),source)?;
 Ok(json!({"name":name,"stage":format!("{:?}",ep.stage),"workgroup_size":ep.workgroup_size,"status":"source_translated","file":file,"bindings":bindings,"combined_samplers":textures}))
}
