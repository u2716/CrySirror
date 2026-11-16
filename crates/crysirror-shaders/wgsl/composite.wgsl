@group(0) @binding(0) var before: texture_2d<f32>;
@group(0) @binding(1) var after: texture_2d<f32>;
struct Params {
    // x progress; y vague (glitch: packed 0xRRGGBB tint or -1);
    // z 0=crossfade/1=rule/2=blur/3=dim/4=glitch; w=dim phase or glitch block.
    a: vec4<f32>,
    // Destination root rect in canvas pixels [x, y, w, h].
    rect: vec4<f32>,
}
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var rule: texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
fn gamma(c: vec3<f32>) -> vec3<f32> {
    let x=clamp(c,vec3<f32>(0.0),vec3<f32>(1.0));
    return select(1.055*pow(x,vec3<f32>(1.0/2.4))-0.055,x*12.92,x<=vec3<f32>(0.0031308));
}
fn linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c+0.055)/1.055,vec3<f32>(2.4)),c/12.92,c<=vec3<f32>(0.04045));
}
// KiriKiri universal transition weight of the new image (krkrz fd5c4baa
// TVPInitUnivTransBlendTable): phase = progress*(255+vague), table value
// 255 below phase-vague, 0 at/above phase, linear between; blend uses /256.
fn rule_weight(pos: vec2<f32>) -> f32 {
    let progress = params.a.x;
    if (progress >= 1.0) { return 1.0; }
    let uv = (pos - params.rect.xy) / params.rect.zw;
    if (any(uv < vec2<f32>(0.0)) || any(uv >= vec2<f32>(1.0))) { return progress; }
    let dims = vec2<f32>(textureDimensions(rule));
    var rp = vec2<i32>(clamp(floor(uv * dims), vec2<f32>(0.0), dims - 1.0));
    if(params.a.z>2.5) {rp=vec2<i32>(clamp(floor(pos),vec2<f32>(0.),dims-1.));}
    // Rule textures are sRGB-decoded on load; recover the stored 8-bit level.
    let value=textureLoad(rule,rp,0).rgb;
    let g = gamma(value);
    var level = floor((g.r + g.g + g.b) / 3.0 * 255.0 + 0.5);
    if(params.a.z>2.5) { level=floor(value.r*255.+0.5); }
    let vague = params.a.y;
    var phasemax = floor(progress * (255.0 + vague));
    if(params.a.z>2.5) { phasemax=params.a.w; }
    let phase = phasemax - vague;
    var t = 0.0;
    if (level < phase) {
        t = 255.0;
    } else if (level < phasemax) {
        t = clamp(255.0 - floor((level - phase) * 255.0 / vague), 0.0, 255.0);
    }
    return t / 256.0;
}
// GlitchEffect frame (crysirror-image layer_ex::GlitchFrame): the rule slot
// holds 24-bit biased offsets, rows 3v..3v+2 per block column (x shift with
// the new-input flag in alpha, y shift, channel split). Reads clamp to the
// root rectangle's linear pixel span, as the native copy does.
fn glitch_value(x: i32, y: i32) -> f32 {
    let t = floor(textureLoad(rule, vec2<i32>(x, y), 0) * 255.0 + 0.5);
    return t.r + t.g * 256.0 + t.b * 65536.0 - 8388608.0;
}
fn glitch_bytes(p: vec4<f32>) -> vec4<f32> {
    let rgb = gamma(p.rgb / max(p.a, 0.000001));
    return vec4<f32>(floor(rgb * 255.0 + 0.5), floor(p.a * 255.0 + 0.5));
}
fn glitch_source(index: i32, width: i32, use_new: bool) -> vec4<f32> {
    let p = vec2<i32>(params.rect.xy) + vec2<i32>(index % width, index / width);
    return glitch_bytes(select(textureLoad(before, p, 0), textureLoad(after, p, 0), use_new));
}
fn glitch(pos: vec2<f32>, a: vec4<f32>) -> vec4<f32> {
    let size = vec2<i32>(params.rect.zw);
    let local = vec2<i32>(floor(pos - params.rect.xy));
    if (any(local < vec2<i32>(0)) || any(local >= size)) {
        return vec4<f32>(a.rgb / max(a.a, 0.000001), a.a);
    }
    let block = max(i32(params.a.w), 1);
    let bx = local.x / block;
    let u = bx * block;
    let k = local.x - u;
    let len = min(block, size.x - u);
    let row = local.y * 3;
    let ox = i32(glitch_value(bx, row));
    let oy = i32(glitch_value(bx, row + 1));
    let c = glitch_value(bx, row + 2);
    let use_new = textureLoad(rule, vec2<i32>(bx, row), 0).a > 0.5;
    let hi = max(size.x * size.y - len, 0);
    let base = (local.y + oy) * size.x + u + ox;
    let split = i32(c) * 4;
    let centre = glitch_source(clamp(base, 0, hi) + k, size.x, use_new);
    let minus = glitch_source(clamp(base - split, 0, hi) + k, size.x, use_new);
    let plus = glitch_source(clamp(base + split, 0, hi) + k, size.x, use_new);
    // Native BGRA picks by split & 3 (two's complement), as RGB.
    let m = c - 4.0 * floor(c / 4.0);
    var rgb = centre.rgb;
    if (c != 0.0) {
        if (m < 0.5) {
            rgb = vec3<f32>(plus.r, centre.g, minus.b);
        } else if (m < 1.5) {
            rgb = vec3<f32>(plus.b, centre.g, minus.r);
        } else if (m < 2.5) {
            rgb = vec3<f32>(plus.r, centre.b, minus.g);
        } else {
            rgb = vec3<f32>(plus.g, centre.r, minus.b);
        }
    }
    let tint = params.a.y;
    if (tint >= 0.0) {
        let r = floor(tint / 65536.0);
        let g = floor((tint - r * 65536.0) / 256.0);
        let b = tint - r * 65536.0 - g * 256.0;
        rgb = clamp(rgb + 2.0 * (vec3<f32>(r, g, b) - 128.0), vec3<f32>(0.0), vec3<f32>(255.0));
    }
    return vec4<f32>(linear(rgb / 255.0), centre.a / 255.0);
}
@fragment fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    var a=textureLoad(before,vec2<i32>(pos.xy),0);
    var b=textureLoad(after,vec2<i32>(pos.xy),0);
    if (params.a.z > 3.5) { return glitch(pos.xy, a); }
    var w = params.a.x;
    if(params.a.z>1.5 && params.a.z<2.5) {
        // Recover exact bytes from the half-float carrier before integer blend.
        a=floor(a*255.+0.5)/255.;b=floor(b*255.+0.5)/255.;
        w=select(floor(w*255.)/256.,1.,w>=1.);
    } else if (params.a.z > 0.5) { w = rule_weight(pos.xy); }
    let alpha=mix(a.a,b.a,w);
    // Inputs are renderer-composited premultiplied linear pixels. Mix the
    // isolated gamma-space colors and output straight alpha for normal layers.
    var color=mix(gamma(a.rgb/max(a.a,0.000001))*a.a,gamma(b.rgb/max(b.a,0.000001))*b.a,w);
    if(params.a.z>1.5 && params.a.z<2.5) { color=floor(mix(a.rgb,b.rgb,w)*255.+0.0001)/255.; }
    return vec4<f32>(linear(color/max(alpha,0.000001)),alpha);
}
