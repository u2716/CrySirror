// SPDX-License-Identifier: MPL-2.0
//! Canonical WGSL -> native GXM Cg. Naga translates arithmetic/control flow;
//! this pass lowers resource and stage interfaces, never a GL runtime.
use anyhow::{bail, ensure, Context, Result};
use naga::back::hlsl;
use regex::Regex;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

pub fn is_shared_2d(module: &str) -> bool {
    matches!(
        module,
        "quad"
            | "present"
            | "puppet"
            | "wipe"
            | "ordered_dither"
            | "composite"
            | "composite_filter_rgba8unorm"
            | "composite_filter_rgba16float"
    )
}
const WIPE_KINDS: &[i32] = &[
    -1, 0, 1, 2, 50, 200, 210, 211, 212, 213, 214, 215, 220, 221, 230, 231, 240, 241, 242, 243,
    900, 901,
];
// GXM has no compute stage. Accumulate the original ordered taps per pixel.
// Each horizontal sum is an exact integer <= 255 * 257 (16 bits). Two
// horizontal passes write RG (extra.y=0) and BA (extra.y=1) sums as high/low
// bytes into U8x4 targets, the colour format every GXM implementation writes
// exactly. The vertical pass divides the exact 2D sums in f32 steps (SGX has
// no integer divide).
const BOX_FRAGMENT: &str = r#"
fn box_fix(q:ptr<function,vec4<f32>>,r:ptr<function,vec4<f32>>,a:f32) {
 let under=*r<vec4<f32>(0.);*q=select(*q,*q-1.,under);*r=select(*r,*r+a,under);
 let over=*r>=vec4<f32>(a);*q=select(*q,*q+1.,over);*r=select(*r,*r-a,over);
}
@fragment fn fs_main(@builtin(position) position:vec4<f32>)->@location(0) vec4<f32> {
 let pixel=vec2<i32>(position.xy);let center=vec2<f32>(pixel)+0.5;
 if(any(center<params.rect.xy)||any(center>=params.rect.xy+params.rect.zw)){return vec4<f32>(0.);}
 let radius=params.config.y;
 if(params.config.x==0u){
  var sum=vec4<f32>(0.);
  for(var n=0u;n<2u*radius+1u;n+=1u){sum+=input_value(pixel+vec2<i32>(i32(n)-i32(radius),0));}
  let pair=select(sum.xy,sum.zw,params.extra.y!=0u);
  let high=floor(pair/256.);let low=pair-high*256.;
  return vec4<f32>(high.x,low.x,high.y,low.y)/255.;
 }
 // Sum the high and low bytes separately (each <= 255 * 257), then divide
 // t = high * 256 + low by the area in base-256/16/16 steps whose operands
 // stay below 2^24: exact in f32, with no integer instruction.
 // Coordinates stay in f32 (exact small integers): the integer min/max
 // lowering of clamp() lost edge texels in Vita3K 0.2.1.
 let size=vec2<f32>(textureDimensions(source));
 let lo=clamp(floor(params.rect.xy),vec2<f32>(0.),size-1.);
 let hi=max(lo,clamp(ceil(params.rect.xy+params.rect.zw)-1.,vec2<f32>(0.),size-1.));
 var high=vec4<f32>(0.);var low=vec4<f32>(0.);
 for(var n=0u;n<2u*radius+1u;n+=1u){
  let p=vec2<i32>(clamp(center-0.5+vec2<f32>(0.,f32(n)-f32(radius)),lo,hi)+0.5);
  let rg=floor(textureLoad(source,p,0)*255.+0.5);let ba=floor(textureLoad(horizontal_ba,p,0)*255.+0.5);
  high+=vec4<f32>(rg.x,rg.z,ba.x,ba.z);low+=vec4<f32>(rg.y,rg.w,ba.y,ba.w);
 }
 let a=f32(2u*radius+1u)*f32(2u*params.config.z+1u);
 let low_high=floor(low/256.);let low_low=low-low_high*256.;
 let nibble_high=floor(low_low/16.);let nibble_low=low_low-nibble_high*16.;
 var q=floor((high+low_high)/a);var r=high+low_high-q*a;box_fix(&q,&r,a);
 let e=r*16.+nibble_high;var qe=floor(e/a);var re=e-qe*a;box_fix(&qe,&re,a);
 let f=re*16.+nibble_low;var qf=floor(f/a);var rf=f-qf*a;box_fix(&qf,&rf,a);
 var average=(q*256.+qe*16.+qf)/255.;
 if(params.extra.x!=0u){average=vec4<f32>(vec3<f32>(1.-average.r),1.);}
 return average;
}
"#;
fn lower_wgsl(module: &str, source: &str) -> Result<String> {
    let mut text = source.to_owned();
    if module == "quad" {
        let declaration = "var<storage, read> filters: array<LayerFilter>;";
        ensure!(
            text.contains(declaration) && text.contains("filters[in.filter_index]"),
            "quad filter contract changed"
        );
        // Flat per-draw values: split batches by slot, bind one original record.
        text = text
            .replace(declaration, "var<uniform> selected_filter: LayerFilter;")
            .replace("filters[in.filter_index]", "selected_filter");
        text = text
            .replace("filter_index: u32", "filter_index: f32")
            .replace("u32(max(filter_slot, 0.0))", "max(filter_slot, 0.0)");
        // Keep the exact sample counts, but supply them through a mandatory
        // uniform so SceShaccCg cannot duplicate the entire sampling tree 10x.
        text = text
            .replace("i < 10", "i < i32(cg_loop_limits.x)")
            .replace("y<=1", "y<i32(cg_loop_limits.y)-1")
            .replace("x<=1", "x<i32(cg_loop_limits.z)-1");
        text.push_str("\n@group(3) @binding(0) var<uniform> cg_loop_limits:vec4<u32>;\n");
    }
    if module == "wipe" {
        // The three admitted fullscreen indices are 0, 1, 2. Generate the
        // same triangle/UVs arithmetically: Vita3K's native indexed local-array
        // path drops this vertex program, while the equivalent arithmetic
        // path has been verified against the original pixel pattern.
        text = text.replace(
            "positions[index]",
            "vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - vec2<f32>(1.0)",
        );
        text = text.replace(
            "uvs[index]",
            "vec2<f32>(f32((index << 1u) & 2u), 1.0 - f32(index & 2u))",
        );
        text = text.replace("i < 16", "i < i32(cg_loop_limits.w)");
        text.push_str("\n@group(3) @binding(0) var<uniform> cg_loop_limits:vec4<u32>;\n");
        // The random centre is uniform for the entire draw. Evaluate its
        // original GPU formula once per fullscreen vertex, outside the SGX
        // fragment loop; SceShaccCg fails on sin/frac feeding that loop.
        text = text.replace("@location(0) uv: vec2<f32>,", "@location(0) uv: vec2<f32>,\n    @location(1) @interpolate(flat) random_center: vec2<f32>,");
        text = text.replace("out.uv = uvs[index];", "out.uv = uvs[index];\n    let seed = wipe.option3.w;\n    let random_x = sin(seed * 12.9898) * 43758.5453;\n    let random_y = sin(seed * 78.233) * 43758.5453;\n    out.random_center = vec2<f32>(random_x - floor(random_x), random_y - floor(random_y));");
        text = text.replace(
            "fn affected_color(uv: vec2<f32>)",
            "fn affected_color(uv: vec2<f32>, random_center: vec2<f32>)",
        );
        text = text.replace(
            "\n            center = fract(vec2<f32>(sin(seed * 12.9898), sin(seed * 78.233)) * 43758.5453);",
            "\n            center = random_center;",
        );
        text = text.replace("affected_color(uv)", "affected_color(uv, in.random_center)");
    }
    if module.starts_with("composite_filter_") {
        text.truncate(text.find("@compute").context("box compute entry")?);
        text = Regex::new(r"(?m)^@group\(0\) @binding\(2\).*\n")?
            .replace_all(&text, "")
            .into_owned();
        text = Regex::new(r"(?m)^var<workgroup>.*\n")?
            .replace_all(&text, "")
            .into_owned();
        text=text.replace("fn input_value(pixel:vec2<i32>)->vec4<f32> {", "@group(0) @binding(3) var horizontal_ba:texture_2d<f32>;\nfn input_value(pixel:vec2<i32>)->vec4<f32> {");

        text.push_str(BOX_FRAGMENT);
    }
    Ok(text)
}
fn arguments(text: &str) -> Vec<String> {
    let (mut level, mut start) = (0i32, 0);
    let mut result = Vec::new();
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' => level += 1,
            ')' | ']' | '}' => level -= 1,
            ',' if level == 0 => {
                result.push(text[start..i].trim().into());
                start = i + 1;
            }
            _ => {}
        }
    }
    if !text[start..].trim().is_empty() {
        result.push(text[start..].trim().into());
    }
    result
}
fn close(text: &str, open: usize) -> Result<usize> {
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        if c == '(' {
            depth += 1;
        }
        if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Ok(open + i);
            }
        }
    }
    bail!("unterminated shader call")
}
fn calls(
    mut text: String,
    pattern: &Regex,
    mut lower: impl FnMut(&regex::Captures<'_>, Vec<String>) -> Result<String>,
) -> Result<String> {
    let starts: Vec<_> = pattern.find_iter(&text).map(|m| m.start()).collect();
    for start in starts.into_iter().rev() {
        let cap = pattern.captures(&text[start..]).context("shader call")?;
        let open = start + cap.get(0).unwrap().end() - 1;
        let end = close(&text, open)?;
        let replacement = lower(&cap, arguments(&text[open + 1..end]))?;
        text.replace_range(start..end + 1, &replacement);
    }
    Ok(text)
}
fn switches(mut text: String) -> Result<String> {
    let pattern = Regex::new(r"\bswitch\s*\(")?;
    let labels = Regex::new(r"\b(?:case\s+([^:]+)|default)\s*:")?;
    let mut serial = 0;
    while let Some(start) = pattern.find_iter(&text).last().map(|m| m.start()) {
        let open = text[start..].find('(').unwrap() + start;
        let end = close(&text, open)?;
        let brace = text[end..].find('{').context("switch body")? + end;
        let mut depth = 0;
        let mut finish = None;
        for (i, c) in text[brace..].char_indices() {
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
                if depth == 0 {
                    finish = Some(brace + i);
                    break;
                }
            }
        }
        let finish = finish.context("switch end")?;
        let body = &text[brace + 1..finish];
        let mut positions = Vec::new();
        for cap in labels.captures_iter(body) {
            let m = cap.get(0).unwrap();
            let nesting = body[..m.start()].chars().fold(0i32, |d, c| {
                d + if c == '{' {
                    1
                } else if c == '}' {
                    -1
                } else {
                    0
                }
            });
            if nesting == 0 {
                positions.push((
                    m.start(),
                    m.end(),
                    cap.get(1).map(|v| v.as_str().trim().to_owned()),
                ));
            }
        }
        let value = format!("cg_switch_{serial}");
        serial += 1;
        let mut replacement = format!("{{ int {value}={};\n", &text[open + 1..end]);
        let mut pending = Vec::new();
        let mut count = 0;
        for (i, (_, begin, label)) in positions.iter().enumerate() {
            if let Some(l) = label {
                pending.push(l.clone());
            }
            let stop = positions.get(i + 1).map_or(body.len(), |p| p.0);
            let branch = body[*begin..stop].trim();
            if branch.is_empty() {
                continue;
            }
            ensure!(
                branch.starts_with('{') && branch.ends_with('}'),
                "switch case layout changed"
            );
            let branch = Regex::new(r"\bbreak;\s*\}\s*$")?.replace(branch, "}");
            if label.is_none() {
                replacement.push_str(if count == 0 { "" } else { "else " });
                replacement.push_str(&branch);
            } else {
                replacement.push_str(if count == 0 { "if(" } else { "else if(" });
                replacement.push_str(
                    &pending
                        .iter()
                        .map(|l| format!("{value}=={l}"))
                        .collect::<Vec<_>>()
                        .join("||"),
                );
                replacement.push(')');
                replacement.push_str(&branch);
            }
            replacement.push('\n');
            pending.clear();
            count += 1;
        }
        replacement.push('}');
        text.replace_range(start..finish + 1, &replacement);
    }
    Ok(text)
}
const HELPERS: &str = r#"
// SGX has no general integer divide. Bias a full-width float estimate
// below the quotient, refine its small remainder, then correct in integers.
// The 2^-20 bias dominates f32 conversion/division error (all inputs normal).
// The refinement quotient is below 2048, so three corrections are sufficient.
int cg_div_positive(int a,int b){
 if(b==1)return a;
 bool negative=a<0;
 int n=negative?-(a+1):a;
 float estimate=float(n)/float(b);
 int q=int(max(floor(estimate-max(estimate*0.00000095367431640625,2.0)),0.0));
 int r=n-q*b;
 int correction=max(int(floor(float(r)/float(b)))-2,0);
 q+=correction;r-=correction*b;
 if(r>=b){r-=b;q+=1;}
 if(r>=b){r-=b;q+=1;}
 if(r>=b){r-=b;q+=1;}
 if(negative)return -q-int(r==b-1);
 return q;
}
int cg_div(int a,int b){
 if(b==0)return a;
 if(b==(-2147483647-1))return a==b?1:0;
 if(a==(-2147483647-1)&&b==-1)return a;
 return b<0?-cg_div_positive(a,-b):cg_div_positive(a,b);
}
int2 cg_div(int2 a,int2 b){return int2(cg_div(a.x,b.x),cg_div(a.y,b.y));}
int3 cg_div(int3 a,int3 b){return int3(cg_div(a.x,b.x),cg_div(a.y,b.y),cg_div(a.z,b.z));}
int4 cg_div(int4 a,int4 b){return int4(cg_div(a.x,b.x),cg_div(a.y,b.y),cg_div(a.z,b.z),cg_div(a.w,b.w));}
float4 cg_load(sampler2D tex,int3 p,float4 size){
 if(any(p.xy<0)||any(p.xy>=int2(size.xy))||p.z!=0)return float4(0,0,0,0);
 return tex2Dlod(tex,float4((float2(p.xy)+0.5)*size.zw,0,0));
}
// Naga uses these for signed/unsigned integer wrapping, never float bitcasts.
int asint(uint v){return int(v);} int2 asint(uint2 v){return int2(v);}
int3 asint(uint3 v){return int3(v);} int4 asint(uint4 v){return int4(v);}
uint asuint(int v){return uint(v);} uint2 asuint(int2 v){return uint2(v);}
uint3 asuint(int3 v){return uint3(v);} uint4 asuint(int4 v){return uint4(v);}
"#;
fn replace_function(text: &mut String, name: &str, replacement: &str) -> Result<()> {
    let pattern = Regex::new(&format!(
        r"(?m)^\w+ {}\([^\n]*\)\s*\{{",
        regex::escape(name)
    ))?;
    let matched = pattern
        .find(text)
        .with_context(|| format!("native function {name}"))?;
    let start = matched.start();
    let brace = matched.end() - 1;
    let mut depth = 0;
    let mut end = None;
    for (i, c) in text[brace..].char_indices() {
        if c == '{' {
            depth += 1;
        }
        if c == '}' {
            depth -= 1;
            if depth == 0 {
                end = Some(brace + i + 1);
                break;
            }
        }
    }
    text.replace_range(start..end.context("native function body")?, replacement);
    Ok(())
}

fn optimize_sampling(text: &mut String, mode: u32) -> Result<()> {
    const WRAP: &str = r#"
int cg_wrap(int pixel,int size){
 int value=pixel-cg_div(pixel,size)*size;
 return value<0?value+size:value;
}
"#;
    if mode == 1 || mode == 2 {
        replace_function(
            text,
            "encoded_load",
            &format!(
                r#"{WRAP}
float4 cg_encoded_at(int2 p){{
 float4 c=cg_load(image,int3(p,0),cg_size_image);
 return float4(to_gamma(c.rgb),c.a);
}}
float4 encoded_load(int2 pixel,int2 size){{
 return cg_encoded_at(int2(cg_wrap(pixel.x,size.x),cg_wrap(pixel.y,size.y)));
}}"#
            ),
        )?;
    }
    if mode == 2 {
        // Wrap the base once, then derive its adjacent texels. Native f2i32
        // clamps below INT_MAX, so base+1 cannot overflow.
        replace_function(
            text,
            "sample_color",
            r#"float4 sample_color(float2 uv){
 int2 size=int2(cg_size_image.xy);
 float2 p=uv*float2(size)-0.5;
 int2 base=naga_f2i32(floor(p));
 float2 phase=frac(p);
 int2 a=int2(cg_wrap(base.x,size.x),cg_wrap(base.y,size.y));
 int2 b=int2(a.x+1==size.x?0:a.x+1,a.y+1==size.y?0:a.y+1);
 return lerp(lerp(cg_encoded_at(a),cg_encoded_at(int2(b.x,a.y)),phase.x),
             lerp(cg_encoded_at(int2(a.x,b.y)),cg_encoded_at(b),phase.x),phase.y);
}"#,
        )?;
    }
    if mode == 3 || mode == 4 {
        replace_function(
            text,
            "sample_axis",
            &format!(
                r#"{WRAP}
int sample_axis(int pixel,int size,bool repeat,bool mirror){{
 int value=clamp(pixel,0,size-1);
 if(mirror||repeat){{
  int period=mirror?2*size:size;
  int p=cg_wrap(pixel,period);
  value=mirror&&p>=size?period-1-p:p;
 }}
 return value;
}}"#
            ),
        )?;
        replace_function(
            text,
            "encoded_byte_load",
            r#"
struct CgBytePixel {float3 rgb;int alpha;};
CgBytePixel cg_byte_at(int2 p){
 float4 c=cg_load(image,int3(p,0),cg_size_image);
 CgBytePixel result;
 result.rgb=round(to_gamma(c.rgb)*255.0);
 result.alpha=int(clamp(round(c.a*255.0),-2147483600.0,2147483500.0));
 return result;
}"#,
        )?;
        if mode == 3 {
            replace_function(
                text,
                "sample_color",
                r#"float4 sample_color(float2 uv){
 int2 size=int2(cg_size_image.xy);int flags=naga_f2u32(selected_filter.c.w);
 int2 p=naga_f2i32(floor(uv*float2(size)));
 p=int2(sample_axis(p.x,size.x,(flags&256)!=0,(flags&512)!=0),
        sample_axis(p.y,size.y,(flags&1024)!=0,(flags&2048)!=0));
 CgBytePixel c=cg_byte_at(p);return float4(c.rgb,float(c.alpha))/255.0;
}"#,
            )?;
        } else {
            // RGB bytes are 0..255; every product is <=32640 in magnitude.
            // F32 represents those integers and /128 exactly. floor(x/128)
            // equals the original signed >>7. Alpha retains the wide path.
            replace_function(
                text,
                "sample_color",
                r#"float4 sample_color(float2 uv){
 int2 size=int2(cg_size_image.xy);int flags=naga_f2u32(selected_filter.c.w);
 float2 p=uv*float2(size)-0.5;int2 base=naga_f2i32(floor(p));
 float2 weight=floor((floor(frac(p)*65536.0)+1.0)/512.0);
 int x0=sample_axis(base.x,size.x,(flags&256)!=0,(flags&512)!=0);
 int x1=sample_axis(base.x+1,size.x,(flags&256)!=0,(flags&512)!=0);
 int y0=sample_axis(base.y,size.y,(flags&1024)!=0,(flags&2048)!=0);
 int y1=sample_axis(base.y+1,size.y,(flags&1024)!=0,(flags&2048)!=0);
 CgBytePixel a=cg_byte_at(int2(x0,y0));CgBytePixel b=cg_byte_at(int2(x1,y0));
 CgBytePixel c=cg_byte_at(int2(x0,y1));CgBytePixel d=cg_byte_at(int2(x1,y1));
 float3 top=a.rgb+floor((b.rgb-a.rgb)*weight.x/128.0);
 float3 bottom=c.rgb+floor((d.rgb-c.rgb)*weight.x/128.0);
 float3 rgb=top+floor((bottom-top)*weight.y/128.0);
 int wx=int(weight.x);int wy=int(weight.y);
 int at=a.alpha+(((b.alpha-a.alpha)*wx)>>7);
 int ab=c.alpha+(((d.alpha-c.alpha)*wx)>>7);
 int alpha=at+(((ab-at)*wy)>>7);
 return float4(rgb,float(alpha))/255.0;
}"#,
            )?;
        }
    }
    Ok(())
}

fn cg_source(
    mut text: String,
    entry: &str,
    stage: naga::ShaderStage,
    sampling: Option<u32>,
) -> Result<(String, Vec<Value>)> {
    text = format!("{HELPERS}\n{text}");
    let mut bindings = Vec::new();
    text = Regex::new(r"(?s)uint2 Naga(?:Mip)?Dimensions2D\([^\n]*\)\n\{.*?\n\}\n")?
        .replace_all(&text, "")
        .into_owned();
    // Remove HLSL sampler heaps/global aliases, not resource function arguments.
    text=Regex::new(r"(?m)^(?:SamplerState naga|SamplerComparisonState naga|StructuredBuffer<uint> naga|static const SamplerState ).*\n")?.replace_all(&text,"").into_owned();
    let textures = Regex::new(r"Texture2D<float4> (\w+) : register\(t(\d+)(?:, space(\d+))?\);")?;
    let mut replacements = Vec::new();
    for cap in textures.captures_iter(&text) {
        let name = cap[1].to_owned();
        let slot = bindings.len();
        bindings.push(json!({"name":name,"group":cap.get(3).map_or(0,|m|m.as_str().parse::<u32>().unwrap()),"binding":cap[2].parse::<u32>()?,"texture_unit":slot,"dimensions":format!("cg_size_{name}")}));
        replacements.push((
            cap[0].to_owned(),
            format!("uniform sampler2D {name} : TEXUNIT{slot};\nuniform float4 cg_size_{name};"),
        ));
    }
    for (old, new) in replacements {
        text = text.replace(&old, &new);
    }
    text = Regex::new(r"cbuffer \w+ : register\([^)]*\) \{ (\w+) (\w+); \}")?
        .replace_all(&text, "uniform $1 $2;")
        .into_owned();
    let signatures = Regex::new(r"(?m)^(\w+) (\w+)\(([^\n]*)\)\n\{")?;
    let mut functions = BTreeMap::new();
    let mut replacements = Vec::new();
    let mut filter_aliases: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for cap in signatures.captures_iter(&text) {
        let args = arguments(&cap[3]);
        if !args
            .iter()
            .any(|a| a.contains("Texture2D<") || a.starts_with("LayerFilter "))
        {
            continue;
        }
        let kinds: Vec<u8> = args
            .iter()
            .map(|a| {
                if a.contains("Texture2D<") {
                    1
                } else if a.starts_with("SamplerState ") {
                    2
                } else if a.starts_with("LayerFilter ") {
                    3
                } else {
                    0
                }
            })
            .collect();
        let mut lowered = Vec::new();
        for (a, k) in args.iter().zip(&kinds) {
            match k {
                1 => {
                    let n = a.split_whitespace().last().unwrap();
                    lowered.push(format!("sampler2D {n}"));
                    lowered.push(format!("float4 cg_size_{n}"));
                }
                2 => {}
                3 => filter_aliases
                    .entry(cap[2].to_owned())
                    .or_default()
                    .push(a.split_whitespace().last().unwrap().to_owned()),
                _ => lowered.push(a.clone()),
            }
        }
        functions.insert(cap[2].to_owned(), kinds);
        replacements.push((
            cap[0].to_owned(),
            format!("{} {}({})\n{{", &cap[1], &cap[2], lowered.join(", ")),
        ));
    }
    for (old, new) in replacements {
        text = text.replace(&old, &new);
    }
    for (function, kinds) in functions {
        text = calls(
            text,
            &Regex::new(&format!(r"\b{}\(", regex::escape(&function)))?,
            |_, args| {
                if args.iter().any(|a| {
                    Regex::new(r"^(sampler2D|float[234]?|u?int[234]?|bool|Out) \w+$")
                        .unwrap()
                        .is_match(a)
                }) {
                    return Ok(format!("{function}({})", args.join(", ")));
                }
                ensure!(args.len() == kinds.len(), "{function}: argument count");
                let mut new = Vec::new();
                for (a, k) in args.iter().zip(&kinds) {
                    match k {
                        1 => {
                            new.push(a.clone());
                            new.push(format!("cg_size_{a}"));
                        }
                        2 | 3 => {}
                        _ => new.push(a.clone()),
                    }
                }
                Ok(format!("{function}({})", new.join(", ")))
            },
        )?;
    }
    // Every LayerFilter argument is the same immutable selected record. Hoist
    // it instead of expanding a 36-float copy through every sampling helper.
    // Naga reuses local names between functions. Resolve each filter alias in
    // its own body, otherwise a colour named `f` in another function can shadow
    // the uniform after a global substitution.
    let aliases = Regex::new(r"\b(?:const )?LayerFilter (\w+) = selected_filter;")?;
    let bodies = Regex::new(r"(?m)^\w+ (\w+)\([^\n]*\)(?: : \w+)?\s*\{")?;
    let ranges: Vec<_> = bodies
        .captures_iter(&text)
        .map(|c| (c.get(0).unwrap().end() - 1, c[1].to_owned()))
        .collect();
    for (brace, name) in ranges.into_iter().rev() {
        let mut depth = 0;
        let end = text[brace..]
            .char_indices()
            .find_map(|(i, c)| {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                }
                (depth == 0).then_some(brace + i)
            })
            .context("filter function body")?;
        let body = &text[brace..=end];
        let mut names = filter_aliases.remove(&name).unwrap_or_default();
        names.extend(aliases.captures_iter(body).map(|c| c[1].to_owned()));
        let mut lowered = aliases.replace_all(body, "").into_owned();
        for alias in names {
            lowered = Regex::new(&format!(r"\b{}\b", regex::escape(&alias)))?
                .replace_all(&lowered, "selected_filter")
                .into_owned();
        }
        text.replace_range(brace..=end, &lowered);
    }
    text = calls(
        text,
        &Regex::new(r"\bNaga(?:Mip)?Dimensions2D\(")?,
        |_, args| {
            ensure!(args.len() == 1 || args.len() == 2, "dimensions arguments");
            if args.len() == 2 {
                ensure!(args[1] == "int(0)", "nonzero mip query");
            }
            Ok(format!("uint2(cg_size_{}.xy)", args[0]))
        },
    )?;
    text = calls(
        text,
        &Regex::new(r"\b(\w+)\.(SampleLevel|Sample|Load)\(")?,
        |cap, args| {
            let tex = &cap[1];
            match &cap[2] {
                "Load" => {
                    ensure!(args.len() == 1, "load arguments");
                    Ok(format!("cg_load({tex},{},cg_size_{tex})", args[0]))
                }
                "Sample" => {
                    ensure!(args.len() == 2, "sample arguments");
                    Ok(format!("tex2D({tex},{})", args[1]))
                }
                "SampleLevel" => {
                    ensure!(args.len() == 3, "sample-level arguments");
                    Ok(format!("tex2Dlod({tex},float4({},0,{}))", args[1], args[2]))
                }
                _ => unreachable!(),
            }
        },
    )?;
    let array_fn = Regex::new(
        r"(?s)typedef [^;]+ ret_Construct\w+\[\d+\];\nret_Construct\w+ (Construct\w+)\([^\n]*\) \{.*?\n\}\n",
    )?;
    let names: Vec<_> = array_fn
        .captures_iter(&text)
        .map(|c| c[1].to_owned())
        .collect();
    text = array_fn.replace_all(&text, "").into_owned();
    for name in names {
        text = calls(
            text,
            &Regex::new(&format!(r"\b{}\(", regex::escape(&name)))?,
            |_, args| Ok(format!("{{ {} }}", args.join(", "))),
        )?;
    }
    // All flat values must be identical within a native draw (batch split).
    text = text.replace("nointerpolation ", "");
    // Cg reserves these words, while the canonical WGSL/HLSL names may use them.
    for word in ["row", "emit", "fixed", "packed"] {
        text = Regex::new(&format!(r"\b{word}\b"))?
            .replace_all(&text, format!("cg_{word}"))
            .into_owned();
    }
    text = Regex::new(r"\bLOC(\d+)\b")?
        .replace_all(&text, "TEXCOORD$1")
        .into_owned();
    text = text.replace("SV_Target0", "COLOR");
    text = text.replace(
        "SV_Position",
        if stage == naga::ShaderStage::Vertex {
            "POSITION"
        } else {
            "WPOS"
        },
    );
    if stage == naga::ShaderStage::Vertex {
        let id = Regex::new(r"uint (\w+) : SV_VertexID")?;
        if let Some(c) = id.captures(&text) {
            let n = c[1].to_owned();
            text = id
                .replace_all(&text, "float cg_vertex_index : ATTR0")
                .into_owned();
            let marker = Regex::new(&format!(
                r"({}\(float cg_vertex_index : ATTR0\)[^\n]*\n\{{)",
                regex::escape(entry)
            ))?;
            ensure!(marker.is_match(&text), "vertex index entry changed");
            text = marker
                .replace(&text, |c: &regex::Captures| {
                    format!("{}\n uint {n}=uint(cg_vertex_index);", &c[1])
                })
                .into_owned();
        }
        let sig = Regex::new(&format!(r"(?m)^(\w+) {}\(([^\n]*)\)", regex::escape(entry)))?;
        text = sig
            .replace_all(&text, |c: &regex::Captures| {
                format!("{} {}({})", &c[1], entry, c[2].replace("TEXCOORD", "ATTR"))
            })
            .into_owned();
    }
    text = Regex::new(&format!(r"\b{}\(", regex::escape(entry)))?
        .replace_all(&text, "main(")
        .into_owned();
    // SceShaccCg exposes signed 32-bit int/intN, but no HLSL uint types.
    // Shared 2D unsigned values are bounded: dimensions/flags, 16-bit sample
    // weights and box sums <= 255*257*257, all below INT_MAX. Signed math is
    // therefore exact here; no floating-point integer emulation is used.
    text = Regex::new(r"\buint([234]?)\b")?
        .replace_all(&text, "int$1")
        .into_owned();
    text = Regex::new(r"\b(\d+)u\b")?
        .replace_all(&text, "$1")
        .into_owned();
    text = text.replace("4294967000.0", "2147483500.0");
    // Cg has float/half/fixed min/max overloads, which make integer-vector
    // calls ambiguous. Keep these operations in the original integer domain.
    let mut integer_helpers = String::new();
    for suffix in ["", "2", "3", "4"] {
        let t = format!("int{suffix}");
        integer_helpers.push_str(&format!("{t} min({t} a,{t} b){{return a<b?a:b;}}\n{t} max({t} a,{t} b){{return a>b?a:b;}}\n{t} clamp({t} a,{t} lo,{t} hi){{return min(max(a,lo),hi);}}\n"));
    }
    text = format!("{integer_helpers}\n{text}");
    let modulo = Regex::new(r"(?s)(int[234]?) naga_mod\(int[234]? lhs, int[234]? rhs\) \{.*?\n\}")?;
    text=modulo.replace_all(&text,|c:&regex::Captures|{
  let t=&c[1];format!("{t} naga_mod({t} lhs,{t} rhs) {{\n {t} overflow={t}(lhs=={t}(-2147483647-1)) & {t}(rhs=={t}(-1));\n {t} invalid=overflow | {t}(rhs=={t}(0));\n {t} divisor=invalid?{t}(1):rhs;\n return lhs-cg_div(lhs,divisor)*divisor;\n}}")
 }).into_owned();
    let division =
        Regex::new(r"(?s)(int[234]?) naga_div\(int[234]? lhs, int[234]? rhs\) \{.*?\n\}")?;
    text = division
        .replace_all(&text, |c: &regex::Captures| {
            format!(
                "{} naga_div({} lhs,{} rhs){{return cg_div(lhs,rhs);}}",
                &c[1], &c[1], &c[1]
            )
        })
        .into_owned();
    text = switches(text)?;
    if let Some(mode) = sampling {
        optimize_sampling(&mut text, mode)?;
    }
    for forbidden in [
        "Texture2D<",
        "SamplerState",
        "ByteAddressBuffer",
        "register(",
        "SV_",
        ".Load(",
        ".Sample(",
        "GetDimensions(",
        "Constructarray",
    ] {
        ensure!(
            !text.contains(forbidden),
            "unlowered Cg construct: {forbidden}"
        );
    }
    Ok((
        format!(
            "// SPDX-License-Identifier: MPL-2.0\n// Generated from canonical WGSL by crysirror-shader-tools.\n{text}"
        ),
        bindings,
    ))
}
fn prune_constants(block: &mut naga::Block, expressions: &naga::Arena<naga::Expression>) {
    let old = std::mem::take(block);
    for (mut statement, span) in old.span_into_iter() {
        match &mut statement {
            naga::Statement::If {
                condition,
                accept,
                reject,
            } => {
                prune_constants(accept, expressions);
                prune_constants(reject, expressions);
                if let naga::Expression::Literal(naga::Literal::Bool(value)) =
                    expressions[*condition]
                {
                    let selected = if value {
                        std::mem::take(accept)
                    } else {
                        std::mem::take(reject)
                    };
                    let terminal = matches!(
                        selected.last(),
                        Some(
                            naga::Statement::Return { .. }
                                | naga::Statement::Kill
                                | naga::Statement::Break
                                | naga::Statement::Continue
                        )
                    );
                    block.extend_block(selected);
                    if terminal {
                        break;
                    }
                    continue;
                }
            }
            naga::Statement::Block(inner) => prune_constants(inner, expressions),
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                prune_constants(body, expressions);
                prune_constants(continuing, expressions);
            }
            naga::Statement::Switch { cases, .. } => {
                for case in cases {
                    prune_constants(&mut case.body, expressions);
                }
            }
            _ => {}
        }
        let terminal = matches!(
            statement,
            naga::Statement::Return { .. }
                | naga::Statement::Kill
                | naga::Statement::Break
                | naga::Statement::Continue
        );
        block.push(statement, span);
        if terminal {
            break;
        }
    }
}
pub fn export(
    out: &Path,
    module_name: &str,
    source: &str,
    entry: &str,
    stage: naga::ShaderStage,
) -> Result<Vec<Value>> {
    let filtered = module_name == "quad"
        && matches!(
            entry,
            "fs_filtered" | "fs_filtered_premultiply" | "fs_stored"
        );
    let mut result = Vec::new();
    if filtered {
        for distortion in 0..11 {
            for sampling in 0..5 {
                result.push(export_one(
                    out,
                    module_name,
                    source,
                    entry,
                    stage,
                    Some(distortion),
                    Some(sampling),
                    None,
                )?);
            }
        }
    } else if module_name == "wipe" && entry == "fs_main" {
        for &kind in WIPE_KINDS {
            result.push(export_one(
                out,
                module_name,
                source,
                entry,
                stage,
                None,
                None,
                Some(kind),
            )?);
        }
    } else {
        result.push(export_one(
            out,
            module_name,
            source,
            entry,
            stage,
            None,
            None,
            None,
        )?);
    }
    Ok(result)
}
fn export_one(
    out: &Path,
    module_name: &str,
    source: &str,
    original_entry: &str,
    original_stage: naga::ShaderStage,
    distortion: Option<u32>,
    sampling: Option<u32>,
    wipe_kind: Option<i32>,
) -> Result<Value> {
    let mut lowered = lower_wgsl(module_name, source)?;
    if let Some(kind) = wipe_kind {
        let declaration = "let kind = i32(round(wipe.kind_progress.x));";
        ensure!(
            lowered.matches(declaration).count() == 2,
            "wipe selector changed"
        );
        lowered = lowered.replace(declaration, &format!("let kind = {kind}i;"));
        let start = lowered
            .find("fn affected_color(")
            .context("wipe effect function")?;
        let mut body = lowered[start..].to_owned();
        let old_to_new = (5..200).contains(&kind) || matches!(kind, 900 | 901 | 220 | 221);
        body = Regex::new(r"let old_to_new_family\s*=\s*[^;]+;")?
            .replace(&body, format!("let old_to_new_family = {old_to_new};"))
            .into_owned();
        // Prune whole branch families before Naga retains their short-circuit
        // temporaries. Merely replacing the kind declaration leaves all
        // unreachable helpers in the compiler input and can crash SceShaccCg.
        for (test, value) in [
            (
                "kind == 0 || kind == 1 || kind == 2",
                matches!(kind, 0 | 1 | 2),
            ),
            ("kind == 210 || kind == 211", matches!(kind, 210 | 211)),
            ("kind == 220 || kind == 221", matches!(kind, 220 | 221)),
            ("kind == 230 || kind == 231", matches!(kind, 230 | 231)),
            ("kind >= 240 && kind <= 243", (240..=243).contains(&kind)),
            (
                "kind == 900 || (kind >= 5 && kind < 200)",
                kind == 900 || (5..200).contains(&kind),
            ),
        ] {
            body = body.replace(test, if value { "true" } else { "false" });
        }
        body = Regex::new(r"\bkind\s*(==|<=|>=|<|>)\s*(\d+)")?
            .replace_all(&body, |c: &regex::Captures| {
                let rhs = c[2].parse::<i32>().unwrap();
                let value = match &c[1] {
                    "==" => kind == rhs,
                    "<=" => kind <= rhs,
                    ">=" => kind >= rhs,
                    "<" => kind < rhs,
                    ">" => kind > rhs,
                    _ => unreachable!(),
                };
                value.to_string()
            })
            .into_owned();
        lowered.replace_range(start.., &body);
    }
    if let Some(kind) = distortion {
        let start = lowered
            .find("fn distorted(")
            .context("distortion function")?;
        let end = lowered[start..]
            .find("\nfn flag(")
            .context("distortion function end")?
            + start;
        let mut body = lowered[start..end].to_owned();
        let kind = kind as f32;
        for (test, value) in [
            ("kind > 8.5 && kind < 9.5", kind > 8.5 && kind < 9.5),
            ("kind > 9.5 && kind < 10.5", kind > 9.5 && kind < 10.5),
            ("kind > 0.5 && kind < 1.5", kind > 0.5 && kind < 1.5),
            ("kind > 1.5 && kind < 3.5", kind > 1.5 && kind < 3.5),
            ("kind > 3.5 && kind < 4.5", kind > 3.5 && kind < 4.5),
            ("kind > 2.5", kind > 2.5),
            ("kind > 7.5", kind > 7.5),
            ("kind > 6.5", kind > 6.5),
            ("kind > 5.5", kind > 5.5),
            ("kind > 4.5", kind > 4.5),
        ] {
            body = body.replace(test, if value { "true" } else { "false" });
        }
        lowered.replace_range(start..end, &body);
    }
    if let Some(mode) = sampling {
        let start = lowered
            .find("fn sample_color(")
            .context("sampling function")?;
        let end = lowered[start..]
            .find("\nfn tap(")
            .context("sampling function end")?
            + start;
        let mut body = lowered[start..end].to_owned();
        for (test, value) in [
            ("(flags&128u)!=0u", mode >= 3),
            ("(flags&64u)!=0u", mode == 1 || mode == 3),
            ("(u32(f.c.w)&32u)!=0u", mode == 1 || mode == 2),
            ("(u32(f.c.w)&64u)!=0u", mode == 1 || mode == 3),
        ] {
            body = body.replace(test, if value { "true" } else { "false" });
        }
        lowered.replace_range(start..end, &body);
    }
    let mut module = naga::front::wgsl::parse_str(&lowered)
        .map_err(|e| anyhow::anyhow!(e.emit_to_string(&lowered)))?;
    let native_entry = if original_stage == naga::ShaderStage::Compute {
        "fs_main"
    } else {
        original_entry
    };
    module.entry_points.retain(|e| e.name == native_entry);
    ensure!(
        module.entry_points.len() == 1,
        "native entry {native_entry}"
    );
    for (_, function) in module.functions.iter_mut() {
        prune_constants(&mut function.body, &function.expressions);
        function.named_expressions.clear();
    }
    for ep in &mut module.entry_points {
        prune_constants(&mut ep.function.body, &ep.function.expressions);
        ep.function.named_expressions.clear();
    }
    naga::compact::compact(&mut module, naga::compact::KeepUnused::No);
    let ep = &module.entry_points[0];
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;
    let uniforms:Vec<_>=module.global_variables.iter().filter(|(_,g)|g.space==naga::AddressSpace::Uniform).map(|(_,g)|{
  let members=match &module.types[g.ty].inner {
   naga::TypeInner::Struct{members,..}=>members.iter().map(|m|json!({"name":m.name,"byte_offset":m.offset,"type":format!("{:?}",module.types[m.ty].inner)})).collect::<Vec<_>>(),
   _=>Vec::new(),
  };
  json!({"name":g.name,"group":g.binding.as_ref().map(|b|b.group),"binding":g.binding.as_ref().map(|b|b.binding),"members":members})
 }).collect();
    let options = hlsl::Options {
        restrict_indexing: false,
        force_loop_bounding: false,
        ..Default::default()
    };
    let pipeline = hlsl::PipelineOptions {
        entry_point: Some((ep.stage, ep.name.clone())),
    };
    let mut intermediate = String::new();
    let reflection = hlsl::Writer::new(&mut intermediate, &options, &pipeline).write(
        &module,
        &info,
        hlsl::FragmentEntryPoint::new(&module, &ep.name).as_ref(),
    )?;
    ensure!(
        reflection.entry_point_names.iter().all(|n| n.is_ok()),
        "unresolved entry point"
    );
    let (cg, bindings) = cg_source(intermediate, native_entry, ep.stage, sampling)?;
    let stage = if ep.stage == naga::ShaderStage::Vertex {
        "vertex"
    } else {
        "fragment"
    };
    let suffix = if stage == "vertex" { "v" } else { "f" };
    let mut variant = distortion.map_or(String::new(), |d| format!("_s{}_d{d}", sampling.unwrap()));
    if let Some(kind) = wipe_kind {
        variant = format!(
            "_k{}",
            if kind < 0 {
                "default".to_owned()
            } else {
                kind.to_string()
            }
        );
    }
    let file = format!("shared_{module_name}_{original_entry}{variant}_{suffix}.cg");
    std::fs::write(out.join(&file), cg)?;
    // Level 3 produces unsupported Vita3K predicates/branches in the full
    // filtered fragments. Keep the stored integer blend family at level 3:
    // level 1 exceeds the bounded native compiler invocation for that family.
    // Pixel acceptance remains scoped in the validation note, not implied by
    // this compilation policy.
    Ok(
        json!({"name":format!("{module_name}-{original_entry}{variant}"),"module":module_name,"entry":original_entry,"distortion_variant":distortion,"sampling_variant":sampling,"compiler_optimization":if distortion.is_some() && original_entry!="fs_stored"{1}else{3},"wipe_variant":wipe_kind,"native_entry":"main","stage":stage,"status":"native_source_uncompiled","file":file,"bindings":bindings,"uniforms":uniforms,"wgsl_blake3":blake3::hash(source.as_bytes()).to_hex().to_string(),"filter_record":"one LayerFilter per draw, split batches by flat filter index","position":"vertex attributes / fragment WPOS, top-left pixel centers","loop_limits":[10,3,3,16],"compute_lowering":if original_stage==naga::ShaderStage::Compute{Some("three fragment passes: U8x4 high/low-byte RG/BA horizontal sums (extra.y=0/1), exact f32 long division vertically")}else{None}}),
    )
}

/// Admit only GXP programs accompanied by their exact compiler input and
/// options. Selections can be compiled in separate processes to bound memory.
pub fn import(args: Vec<String>) -> Result<()> {
    ensure!(
        args.len() == 3,
        "usage: crysirror-shader-tools import-psv EXPORTED_DIR COMPILER_OUTPUT CRYSIRROR_SHADERS_CRATE"
    );
    let (export, compiled, root) = (
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
    );
    let mut manifest: Value =
        serde_json::from_slice(&std::fs::read(export.join("manifest.json"))?)?;
    ensure!(
        manifest["target"] == "psv"
            && manifest["errors"]
                .as_array()
                .context("source errors")?
                .is_empty(),
        "incomplete PSV source export"
    );
    let module_hash = std::fs::read_to_string(compiled.join("compiler-module.sha256"))?;
    let module_hash = module_hash.trim();
    ensure!(
        module_hash.len() == 64 && module_hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "compiler module SHA-256"
    );
    manifest["compiler_module_sha256"] = json!(module_hash);
    let mut programs = Vec::new();
    let mut quad_names = BTreeMap::new();
    let mut wipe_names = BTreeMap::new();
    let mut cg_files = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for entry in manifest["entries"].as_array_mut().context("entries")? {
        let Some(file) = entry["file"]
            .as_str()
            .filter(|s| s.ends_with(".cg"))
            .map(str::to_owned)
        else {
            continue;
        };
        ensure!(!file.contains(['/', '\\']), "native shader filename");
        let name = file.strip_suffix(".cg").unwrap();
        ensure!(names.insert(name.to_owned()), "duplicate shader");
        let source = std::fs::read(export.join(&file))?;
        ensure!(
            std::fs::read(compiled.join(&file))? == source,
            "{name}: compiler input is stale"
        );
        let receipt = std::fs::read_to_string(compiled.join(format!("{name}.receipt")))?;
        let options: BTreeMap<_, _> = receipt
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        for key in ["fastmath", "fastprecision", "fastint"] {
            ensure!(
                options.get(key).map(String::as_str) == Some("0"),
                "{name}: full precision required"
            );
        }
        ensure!(
            options
                .get("optimization")
                .and_then(|v| v.parse::<u64>().ok())
                == Some(entry["compiler_optimization"].as_u64().unwrap_or(3)),
            "{name}: optimization level"
        );
        ensure!(
            options.get("compiler").is_some_and(|v| !v.is_empty()),
            "{name}: compiler version"
        );
        let bytes = std::fs::read(compiled.join(format!("{name}.gxp")))?;
        ensure!(
            bytes.len() >= 64 && &bytes[..4] == b"GXP\0",
            "{name}: invalid GXP container"
        );
        ensure!(
            options
                .get("program_bytes")
                .context("program size")?
                .parse::<usize>()?
                == bytes.len(),
            "{name}: GXP byte count"
        );
        entry["status"] = json!("gxp_compiled");
        entry["gxp"] = json!(format!("gxp/{name}.gxp"));
        entry["gxp_blake3"] = json!(blake3::hash(&bytes).to_hex().to_string());
        entry["cg_blake3"] = json!(blake3::hash(&source).to_hex().to_string());
        entry["compiler_options"] = json!(options);
        if let (Some(d), Some(s)) = (
            entry["distortion_variant"].as_u64(),
            entry["sampling_variant"].as_u64(),
        ) {
            quad_names.insert(
                (
                    entry["entry"].as_str().context("quad entry")?.to_owned(),
                    d,
                    s,
                ),
                name.to_owned(),
            );
        }
        if let Some(kind) = entry["wipe_variant"].as_i64() {
            wipe_names.insert(kind as i32, name.to_owned());
        }
        programs.push((
            name.to_owned(),
            entry["stage"].as_str().context("native stage")?.to_owned(),
            bytes,
        ));
        if name.starts_with("shared_") {
            cg_files.push((file, source));
        }
    }
    ensure!(!programs.is_empty(), "no native programs");
    // Validate the entire batch before changing any checked-in output.
    let mut binary_rs = String::from(
        "// SPDX-License-Identifier: MPL-2.0\n// Generated by crysirror-shader-tools import-psv.\n&[\n",
    );
    programs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut quad_indices = Vec::new();
    for entry in ["fs_filtered", "fs_filtered_premultiply", "fs_stored"] {
        for d in 0..11 {
            for s in 0..5 {
                let name = quad_names
                    .get(&(entry.to_owned(), d, s))
                    .context("missing quad permutation")?;
                quad_indices.push(
                    programs
                        .iter()
                        .position(|p| &p.0 == name)
                        .context("quad program index")?,
                );
            }
        }
    }
    let wipe_indices: Vec<_> = WIPE_KINDS
        .iter()
        .map(|kind| {
            let name = wipe_names.get(kind).context("missing wipe permutation")?;
            Ok((
                *kind,
                programs
                    .iter()
                    .position(|p| &p.0 == name)
                    .context("wipe program index")?,
            ))
        })
        .collect::<Result<_>>()?;
    let native = root.join("native/psv");
    std::fs::create_dir_all(native.join("gxp"))?;
    std::fs::create_dir_all(native.join("shared"))?;
    for (name, stage, bytes) in &programs {
        std::fs::write(native.join(format!("gxp/{name}.gxp")), bytes)?;
        binary_rs.push_str(&format!("Program {{ name: {name:?}, stage: {stage:?}, gxp: &AlignedGxp(*include_bytes!(\"../native/psv/gxp/{name}.gxp\")).0 }},\n"));
    }
    binary_rs.push_str("]\n");
    std::fs::write(root.join("src/psv_programs.rs"), binary_rs)?;
    std::fs::write(root.join("src/psv_quad_index.rs"),format!("// SPDX-License-Identifier: MPL-2.0\n// Generated by crysirror-shader-tools import-psv.\nconst PSV_QUAD_INDEX: [usize;165] = {quad_indices:?};\n"))?;
    std::fs::write(root.join("src/psv_wipe_index.rs"), format!("// SPDX-License-Identifier: MPL-2.0\n// Generated by crysirror-shader-tools import-psv.\nconst PSV_WIPE_INDEX: &[(i32,usize)] = &{wipe_indices:?};\n"))?;
    let mut source_rs = String::from(
        "// SPDX-License-Identifier: MPL-2.0\n// Generated by crysirror-shader-tools import-psv.\npub const PSV_SHARED_PROGRAMS: &[(&str,&str)] = &[\n",
    );
    for (file, source) in &cg_files {
        std::fs::write(native.join("shared").join(file), source)?;
        source_rs.push_str(&format!(
            "({file:?},include_str!(\"../native/psv/shared/{file}\")),\n"
        ));
    }
    source_rs.push_str("];\n");
    std::fs::write(root.join("src/psv_shared.rs"), source_rs)?;
    manifest["native_compiled"] = json!(true);
    manifest["device_execution"] = json!(false);
    manifest["native_program_count"] = json!(programs.len());
    std::fs::write(
        native.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!(
        "{} GXP programs imported ({} shared 2D sources), no renderer/device behavior claimed",
        programs.len(),
        cg_files.len()
    );
    Ok(())
}
