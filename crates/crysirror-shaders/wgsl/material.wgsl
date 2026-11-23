// SPDX-License-Identifier: MPL-2.0
struct Material {
    base_color: vec4<f32>,
    // rgb emissive factor, a: emissive strength
    emissive: vec4<f32>,
    // metallic, roughness, normal scale, occlusion strength
    params: vec4<f32>,
    // x: opaque=0/mask=1/alpha=2/additive=3/premultiplied=4, y: cutoff, z: unlit, w: opacity channel alpha
    alpha: vec4<f32>,
    uv_transform: vec4<f32>,
    // x: depth fade, y: effect kind, z: normal encoding (0 rgb, 1 rg, 2 ag), w: specular factor
    surface:vec4<f32>,
    effect_params:array<vec4<f32>,4>,
    // Toon: rgb linear grazing shadow tint, a its strength (shadow_rim).
    toon:vec4<f32>,
    // Toon: rgb linear unlit-side light, a enables the AL skin light model (shadow_light).
    toon_light:vec4<f32>,
    // Toon: rgb fixed shade colour, a shade kind (0 base HSV, 1 cloth, 2 fixed).
    toon_shade:vec4<f32>,
    shadow_tone_override:vec4<f32>,
    line_color:vec4<f32>,
    line_params:array<vec4<f32>,2>,
    raster:vec4<f32>,
    // Liquid layer: xy scale, zw offset of the blob texture.
    liquid_tiling:vec4<f32>,
    // Liquid layer: rgb linear colour, a 1 when bound.
    liquid_color:vec4<f32>,
};
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var t_base: texture_2d<f32>;
@group(1) @binding(2) var t_mr: texture_2d<f32>;
@group(1) @binding(3) var t_normal: texture_2d<f32>;
@group(1) @binding(4) var t_occlusion: texture_2d<f32>;
@group(1) @binding(5) var t_emissive: texture_2d<f32>;
@group(1) @binding(6) var s_material: sampler;
@group(1) @binding(7) var t_opacity: texture_2d<f32>;
@group(1) @binding(8) var t_specular: texture_2d<f32>;


fn raster_culled(front: bool) -> bool {
    return (material.raster.x == 1.0 && front) || (material.raster.x == 2.0 && !front);
}

// Tangent-space normal in [-1,1] from the material's encoding.
fn decode_normal(t: vec4<f32>) -> vec3<f32> {
    if material.surface.z < 0.5 {
        return t.xyz * 2.0 - vec3<f32>(1.0);
    }
    var xy = t.xy * 2.0 - vec2<f32>(1.0);
    if material.surface.z > 1.5 {
        xy = vec2<f32>(t.a, t.g) * 2.0 - vec2<f32>(1.0);
    }
    return vec3<f32>(xy, sqrt(max(1.0 - dot(xy, xy), 0.0)));
}
fn material_uv(uv:vec2<f32>)->vec2<f32>{return uv*material.uv_transform.xy+material.uv_transform.zw;}
// AL/hair_outline, forward vertex blob 3. Expand after skinning in the
// shell mesh's own coordinate system. The zero-width fallback is independent
// of the subsequent component coverage test.
fn shell_world(p: vec3<f32>, n: vec3<f32>, uv: vec2<f32>, model: mat4x4<f32>, vp: mat4x4<f32>) -> vec4<f32> {
    let world = model * vec4<f32>(p, 1.0);
    if material.surface.y == 5.0 {
        let shell = material.effect_params[0];
        let mask = textureSampleLevel(t_base, s_material, material_uv(uv), 0.0).a;
        let width = select(shell.x, 1.0, shell.x == 0.0);
        let w = (vp * world).w;
        return world + model * vec4<f32>(n * (mask * width * shell.y * min(w, shell.z)), 0.0);
    }
    return world;
}

// D3D single-sample coverage. The native dither table contains thresholds
// 1/16 through 16/16; x increases right, y increases upward.
fn bayer_visible(amount: f32, pixel: vec2<u32>) -> bool {
    let thresholds = array<u32,16>(1,9,3,11,13,5,15,7,4,12,2,10,16,8,14,6);
    let threshold = f32(thresholds[(pixel.y % 4u) * 4u + pixel.x % 4u]) / 16.0;
    return min(1.0, amount * 1.00001) >= threshold;
}
// Single-sample cutout in AL/cloth forward/depth/shadow. Width selects
// an eight-neighbour expansion attenuated by camera-to-object distance.
fn cloth_coverage(uv: vec2<f32>, camera: vec3<f32>, origin: vec3<f32>, pixel: vec2<u32>) -> vec2<f32> {
    let line = material.line_params[0];
    let settings = material.line_params[1];
    let mask = textureSample(t_normal, s_material, uv);
    let base_alpha = textureSampleBias(t_base, s_material, uv, line.w).a;
    let width = mask.r * line.y;
    let edge = clamp(4.0 * width, 0.0, 1.0) * line.z;
    let reach = width * max(1.0 - distance(camera, origin) / 11.0, 0.0);
    var coverage = base_alpha;
    if line.y > 0.0 && line.z > 0.0 {
        let step = 0.0007 * clamp(reach, 0.0, 0.6);
        let center = textureSampleLevel(t_base, s_material, uv, line.w).a;
        let offsets = array<vec2<f32>,8>(
            vec2<f32>(1,0), vec2<f32>(0,1), vec2<f32>(1,1), vec2<f32>(1,-1),
            vec2<f32>(-1,0), vec2<f32>(0,-1), vec2<f32>(-1,1), vec2<f32>(-1,-1));
        var largest = center;
        // The center uses explicit LOD; all eight neighbours use a bias.
        for (var tap = 0u; tap < 8u; tap += 1u) {
            largest = max(largest, textureSampleBias(t_base, s_material, uv + offsets[tap]*step, line.w).a);
        }
        let expanded = clamp((largest - 0.3) * 2.5, 0.0, 1.0);
        let widened = mix(center, expanded, clamp(2.0*reach,0.0,1.0));
        coverage = mix(base_alpha, widened, edge);
    }
    let stipple = select(0.0, 1.0, bayer_visible(coverage,pixel));
    coverage = min(coverage, mix(coverage,stipple,settings.x)) * settings.y;
    coverage = min(coverage, ceil(coverage-0.009)) * round(mask.b);
    let cutoff = mix(clamp(settings.z,0.011,1.0),0.999,select(0.0,settings.w,1.0-mask.r >= 0.7));
    return vec2<f32>(coverage,cutoff);
}
fn shell_visible(front: bool, pixel: vec2<u32>, mask: f32) -> bool {
    let shell = material.effect_params[0];
    let coverage = min(ceil(shell.x), min(ceil(mask), select(0.0, 1.0, bayer_visible(shell.w, pixel))));
    return !front && coverage >= material.effect_params[1].x;
}

fn opacity_channel(v:vec4<f32>)->f32{return select(v.r,v.a,material.alpha.w>0.5);}

