// SPDX-License-Identifier: MPL-2.0
//! Canonical engine shader sources and assembly, independent of graphics devices.
pub mod native;
pub mod wgsl {include!(concat!(env!("OUT_DIR"),"/wgsl.rs"));}
#[derive(Clone)]
pub struct Module {pub name:String,pub source:String,pub files:Vec<String>,pub hardware_rt:bool}
fn assembled(name:&str,parts:&[&str])->Module {
 Module {name:name.into(),source:parts.iter().map(|name|file(name)).collect::<Vec<_>>().join("\n"),files:parts.iter().map(|s|s.to_string()).collect(),hardware_rt:false}
}
pub fn file(name:&str)->&'static str {wgsl::FILES.iter().find(|(n,_)|*n==name).unwrap_or_else(||panic!("unknown shader {name}")).1}
pub fn surface_common(hook:&str)->String {format!("{}\n{}\n{}\n{hook}\n{}",wgsl::PBR_COMMON,wgsl::MATERIAL,wgsl::SURFACE_ABI,wgsl::SURFACE_SAMPLE)}
pub fn material_modules(hook:&str)->[String;3] {
 let common=surface_common(hook);
 [format!("{common}\n{}",wgsl::PBR),format!("{common}\n{}",wgsl::PREPASS),format!("{}\n{}\n{hook}\n{}\n{}",wgsl::MATERIAL.replace("@group(1)","@group(2)"),wgsl::SURFACE_ABI,wgsl::SURFACE_SAMPLE,wgsl::SHADOW)]
}
pub fn modules()->Vec<Module> {
 let mut result=Vec::new();
 for name in ["quad","mesh","present","puppet","wipe","ordered_dither","composite","vfx_render","raytrace","gi_project","vsm_mark","lod_select","vfx_compute","vfx_cull","vfx_sort","rt_geometry","rt_denoise"] {result.push(assembled(name,&[&format!("{name}.wgsl")]));}
 for name in ["ssao","taa","ssr","brdf","prefilter","tonemap","bloom","copy_hdr","exposure","probe"] {result.push(assembled(name,&["fullscreen.wgsl",&format!("{name}.wgsl")]));}
 result.push(assembled("particle",&["pbr_common.wgsl","particle.wgsl"]));
 let sources=material_modules(wgsl::SURFACE_PRESETS);
 for (name,source) in ["pbr","prepass","shadow"].into_iter().zip(sources) {
  result.push(Module {name:name.into(),source,files:vec!["pbr_common.wgsl","material.wgsl","surface_abi.wgsl","surface_presets.wgsl","surface_sample.wgsl"].into_iter().map(str::to_owned).chain(std::iter::once(format!("{name}.wgsl"))).collect(),hardware_rt:false});
 }
 result.push(Module {name:"sky".into(),source:format!("{}\n{}",surface_common(wgsl::SURFACE_PRESETS),wgsl::SKY),files:vec!["pbr_common.wgsl".into(),"material.wgsl".into(),"surface_abi.wgsl".into(),"surface_presets.wgsl".into(),"surface_sample.wgsl".into(),"sky.wgsl".into()],hardware_rt:false});
 result.push(Module {name:"rt_alpha_bake".into(),source:format!("{}\n{}\n{}\n{}\n{}",wgsl::MATERIAL.replace("@group(1)","@group(0)"),wgsl::SURFACE_ABI,wgsl::SURFACE_PRESETS,wgsl::SURFACE_SAMPLE,wgsl::RT_ALPHA_BAKE),files:vec!["material.wgsl","surface_abi.wgsl","surface_presets.wgsl","surface_sample.wgsl","rt_alpha_bake.wgsl"].into_iter().map(str::to_owned).collect(),hardware_rt:false});
 for format in ["rgba8unorm","rgba16float"] {result.push(Module {name:format!("composite_filter_{format}"),source:wgsl::COMPOSITE_FILTER.replace("__OUTPUT_FORMAT__",format),files:vec!["composite_filter.wgsl".into()],hardware_rt:false});}
 for (name,parts) in [("hardware_rt",vec!["hardware_rt.wgsl"]),("rt_shadow",vec!["rt_common.wgsl","rt_pbr_shadow.wgsl"]),("rt_reflect",vec!["rt_common.wgsl","rt_lighting.wgsl","rt_reflect.wgsl"]),("rt_gi",vec!["rt_common.wgsl","rt_lighting.wgsl","rt_gi.wgsl"])] {let mut m=assembled(name,&parts);m.hardware_rt=true;result.push(m);}
 result
}
