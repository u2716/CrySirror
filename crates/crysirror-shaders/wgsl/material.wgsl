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
fn opacity_channel(v:vec4<f32>)->f32{return select(v.r,v.a,material.alpha.w>0.5);}

