// Bank stones wick water above the creek. The breakup follows the sampled pores.
fn crysirror_surface(i: SurfaceInput) -> SurfaceOutput {
    var o = surface_default(i);
    let pore = dot(i.color.rgb, vec3<f32>(.21, .72, .07));
    let line = i.parameters[0].x + .10 * sin(i.world_position.x * 4.2 + i.world_position.z);
    let wet = 1.0 - smoothstep(line, line + i.parameters[0].y, i.world_position.y + pore * .12);
    o.color = vec4<f32>(i.color.rgb * mix(1.0, .43, wet), i.color.a);
    o.roughness = mix(i.roughness, max(.085, .27 * i.roughness), wet);
    return o;
}
