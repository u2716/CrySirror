// Single-leaf backlight approximation for the fixed key light. Thick midribs absorb more.
fn crysirror_surface(i: SurfaceInput) -> SurfaceOutput {
    var o = surface_default(i);
    let eye = normalize(i.view_direction);
    let to_sun = normalize(i.parameters[0].xyz);
    let face = faceForward(normalize(i.world_normal), -eye, normalize(i.world_normal));
    let back = max(0.0, dot(-face, to_sun));
    let forward = .32 + .68 * pow(max(dot(-eye, to_sun), 0.0), 2.0);
    let sheet = .30 + .70 * smoothstep(.0, .047, abs(i.uv.x - .5));
    o.emission += i.color.rgb * vec3<f32>(.83, 1.12, .51) * back * forward * sheet * i.parameters[0].w;
    return o;
}
