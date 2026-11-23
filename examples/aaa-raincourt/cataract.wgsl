fn crysirror_surface(i:SurfaceInput)->SurfaceOutput {
    var o=surface_default(i);
    let u=i.uv.x; let v=i.uv.y;
    let flow=v*7.-i.time*1.6;
    let twist=.007*sin(2.*flow+u*12.)+.008*sin(flow+u*31.);
    let strand=.5+.5*sin(81.*(u+twist)+sin(27.*u+flow));
    let foam=smoothstep(.50,.96,strand)*(.42+.35*sin(flow*2.8+u*16.));
    let froth=clamp(.18+foam+.32*v+.15*sin(4.8*flow+u*19.),0.,1.);
    let torn=.035+.065*v+.036*sin(flow*1.3+v*11.);
    let edge=smoothstep(.0,torn+.02,min(u,1.-u));
    let ribbon=.68+.32*sin(u*93.+4.*sin(flow+3.*u));
    let alpha=edge*(.51+.43*ribbon)*(1.-.60*smoothstep(.88,1.,v));
    o.color=vec4<f32>(mix(vec3<f32>(.10,.23,.225),vec3<f32>(.71,.85,.82),froth),alpha);
    o.emission=vec3<f32>(.26,.33,.34)*(.20+froth);
    o.roughness=.28+.40*froth;
    return o;
}
