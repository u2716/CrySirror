// SPDX-License-Identifier: MPL-2.0
// Single-sample depth + view-space normal + motion vector prepass.
struct PreOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) cur: vec4<f32>,
    @location(3) prev: vec4<f32>,
    @location(4) world:vec3<f32>,
    @location(5) world_normal:vec3<f32>,
};
@vertex
fn vs_main(v: VertexIn) -> PreOut {
    return transform_vertex(v);
}
// Entry points cannot call each other; both vertex paths share this.
fn transform_vertex(v: VertexIn) -> PreOut {
    let world = shell_world(v.position, v.normal, v.uv, object.model, frame.view_proj);
    let prev_world = shell_world(v.position, v.normal, v.uv, object.prev_model, frame.prev_view_proj);
    var o: PreOut;
    o.clip = frame.view_proj * world;
    o.normal = (frame.view * vec4<f32>((object.normal * vec4<f32>(v.normal, 0.0)).xyz, 0.0)).xyz;
    o.uv = v.uv + object.uv.xy;
    o.world=world.xyz;
    o.world_normal=(object.normal*vec4<f32>(v.normal,0.0)).xyz;
    o.cur = frame.view_proj_nojitter * world;
    o.prev = frame.prev_view_proj * prev_world;
    return o;
}
struct PreTargets {
    @location(0) normal: vec4<f32>,
    @location(1) velocity: vec2<f32>,
};
@fragment
fn fs_main(i: PreOut, @builtin(front_facing) front: bool) -> PreTargets {
    if raster_culled(front) { discard; }
    if object.uv.w > 0.0 && !bayer_visible(1.0 - object.uv.w, vec2<u32>(vec2<f32>(i.clip.x, frame.screen.y - i.clip.y))) { discard; }
    let uv=material_uv(i.uv);
    let surface=sample_surface(uv,i.world,i.world_normal,frame.camera_pos.xyz,frame.misc.z);
    if material.surface.y == 5.0 && !shell_visible(front, vec2<u32>(vec2<f32>(i.clip.x, frame.screen.y - i.clip.y)), surface.color.a) {discard;}
    if material.line_params[0].x == 2.0 {
        let cloth = cloth_coverage(uv, frame.camera_pos.xyz, object.model[3].xyz, vec2<u32>(vec2<f32>(i.clip.x, frame.screen.y - i.clip.y)));
        if cloth.x < cloth.y { discard; }
    } else if material.alpha.x == 1.0 && surface.color.a < material.alpha.y {discard;}
    var n = normalize(i.normal);
    if !front {
        n = -n;
    }
    let cur = i.cur.xy / i.cur.w;
    let prev = i.prev.xy / i.prev.w;
    var o: PreTargets;
    // Alpha carries roughness for GGX ray generation; depth determines validity.
    o.normal = vec4<f32>(n, surface.roughness);
    // Screen-space delta in UV units (y down): current - previous.
    o.velocity = (cur - prev) * vec2<f32>(0.5, -0.5);
    return o;
}

@vertex
fn vs_skinned(v: VertexIn, s: SkinIn) -> PreOut {
    let k = skin_matrix(s, object.skin.x);
    let kp = skin_matrix(s, object.skin.y);
    var w = v;
    w.position = (k * vec4<f32>(v.position, 1.0)).xyz;
    w.normal = (k * vec4<f32>(v.normal, 0.0)).xyz;
    var o = transform_vertex(w);
    let pos = (kp * vec4<f32>(v.position, 1.0)).xyz;
    let normal = (kp * vec4<f32>(v.normal, 0.0)).xyz;
    o.prev = frame.prev_view_proj * shell_world(pos, normal, v.uv, object.prev_model, frame.prev_view_proj);
    return o;
}
