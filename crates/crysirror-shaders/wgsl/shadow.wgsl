// SPDX-License-Identifier: MPL-2.0
struct Light {view_proj:mat4x4<f32>,camera_time:vec4<f32>};
@group(0) @binding(0) var<uniform> light:Light;
struct Object {model:mat4x4<f32>,normal:mat4x4<f32>,prev_model:mat4x4<f32>,skin:vec4<u32>,uv:vec4<f32>,liquid:vec4<f32>};
@group(1) @binding(0) var<uniform> object:Object;
@group(1) @binding(1) var<storage,read> palettes:array<mat4x4<f32>>;
struct Out {
    @builtin(position) position:vec4<f32>,
    @location(0) uv:vec2<f32>,
    @location(1) world:vec3<f32>,
    @location(2) normal:vec3<f32>,
};
fn shadow_vertex(position:vec3<f32>,normal:vec3<f32>,uv:vec2<f32>)->Out {
    let world=object.model*vec4<f32>(position,1.0);
    var o:Out; o.position=light.view_proj*world; o.uv=uv+object.uv.xy; o.world=world.xyz;
    o.normal=(object.normal*vec4<f32>(normal,0.0)).xyz; return o;
}
@vertex fn vs_main(@location(0) position:vec3<f32>,@location(1) normal:vec3<f32>,@location(2) uv:vec2<f32>)->Out {
    return shadow_vertex(position,normal,uv);
}
@vertex fn vs_skinned(@location(0) position:vec3<f32>,@location(1) normal:vec3<f32>,@location(2) uv:vec2<f32>,@location(4) joints:vec4<u32>,@location(5) weights:vec4<f32>)->Out {
    let b=object.skin.x;
    let k=palettes[b+joints.x]*weights.x+palettes[b+joints.y]*weights.y+palettes[b+joints.z]*weights.z+palettes[b+joints.w]*weights.w;
    return shadow_vertex((k*vec4<f32>(position,1.0)).xyz,(k*vec4<f32>(normal,0.0)).xyz,uv);
}
@fragment fn fs_main(i:Out, @builtin(front_facing) front: bool) {
    if raster_culled(front) { discard; }
    // These dedicated shells have Forward and DepthOnly passes, no ShadowCaster.
    if material.surface.y == 5.0 { discard; }
    // Shadow cascade/atlas extents and tile origins are multiples of four.
    // Convert the downward pixel row to the upward Bayer phase.
    let pixel = vec2<u32>(i.position.xy);
    if object.uv.w > 0.0 && !bayer_visible(1.0 - object.uv.w, vec2<u32>(pixel.x, 3u - (pixel.y % 4u))) { discard; }
    let uv=material_uv(i.uv);
    let surface=sample_surface(uv,i.world,i.normal,light.camera_time.xyz,light.camera_time.w);
    if material.line_params[0].x == 2.0 {
        let cloth = cloth_coverage(uv, light.camera_time.xyz, object.model[3].xyz, vec2<u32>(pixel.x, 3u - (pixel.y % 4u)));
        let cutoff = select(cloth.y, material.alpha.y, material.alpha.x == 2.0);
        if cloth.x < cutoff { discard; }
    } else if material.alpha.x==1.0 && surface.color.a<material.alpha.y {discard;}
}
