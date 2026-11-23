// 4 × 6 fixed-sun depth samples. Filter visibility per column to preserve hard occluders.
fn column_light(i: SurfaceInput, col: u32, h: f32) -> f32 {
    let word_index = col / 3u;
    let word = u32(i.parameters[2u + word_index / 4u][word_index % 4u] * 16777216.);
    let depth = f32((word >> (8u * (col % 3u))) & 255u) / 255.;
    return 1. - smoothstep(depth-.008, depth+.008, h);
}
fn shaft_visibility(i: SurfaceInput, uv: vec2<f32>, h:f32) -> f32 {
    let xy = clamp(uv,vec2<f32>(0.),vec2<f32>(1.))*vec2<f32>(3.,5.);
    let a = vec2<u32>(floor(xy));
    let b = min(a+vec2<u32>(1u),vec2<u32>(3u,5u));
    let w = fract(xy);
    return mix(mix(column_light(i,4u*a.y+a.x,h), column_light(i,4u*a.y+b.x,h), w.x),
               mix(column_light(i,4u*b.y+a.x,h), column_light(i,4u*b.y+b.x,h), w.x), w.y);
}
// Finite shafts under authored canopy gaps. Integrate smoothly varying mist
// within a capped cylinder, from the entry face of its conservative proxy.
fn crysirror_surface(i: SurfaceInput) -> SurfaceOutput {
    var out = surface_default(i);
    let start = i.parameters[0].xyz;
    let radius = i.parameters[0].w;
    let span = i.parameters[1].xyz - start;
    let length_m = length(span);
    let axis = span / length_m;
    let ray = -normalize(i.view_direction);
    let delta = i.world_position - start;
    let axial = dot(delta,axis);
    let rate = dot(ray,axis);
    let lateral = delta-axis*axial;
    let drift = ray-axis*rate;
    let a = max(dot(drift,drift),0.0001);
    let b = dot(lateral,drift);
    let disc = b*b-a*(dot(lateral,lateral)-radius*radius);
    let root = sqrt(max(disc,0.0));
    var lo = max(0.0,(-b-root)/a);
    var hi = (-b+root)/a;
    if abs(rate)>0.0001 {
        let u = -axial/rate;
        let v = (length_m-axial)/rate;
        lo=max(lo,min(u,v)); hi=min(hi,max(u,v));
    } else if axial<0.0 || axial>length_m {
        hi=lo;
    }
    let step_m = max(0.0,hi-lo)/8.0;
    let right = normalize(cross(axis,vec3<f32>(0.,1.,0.)));
    let up = cross(axis,right);
    var optical = 0.0;
    for(var j=0u;j<8u;j=j+1u){
        let t = lo+(f32(j)+0.5)*step_m;
        let off = lateral+t*drift;
        let h = (axial+t*rate)/length_m;
        let radial = 1.0-smoothstep(0.14,1.0,length(off)/radius);
        let aperture = 0.5+0.5*sin(4.7*off.x+6.8*off.z+0.72*sin(off.y*5.0));
        let ends = smoothstep(0.0,0.12,h)*(1.0-smoothstep(0.85,1.0,h));
        let uv = .5+.5*vec2<f32>(dot(off,right),dot(off,up))/radius;
        optical += step_m*radial*ends*(0.22+0.78*aperture)*shaft_visibility(i,uv,h);
    }
    let alpha = 1.0-exp(-optical*i.parameters[1].w);
    out.color=vec4<f32>(vec3<f32>(1.,.81,.49),alpha);
    out.emission=vec3<f32>(0.0);
    return out;
}
