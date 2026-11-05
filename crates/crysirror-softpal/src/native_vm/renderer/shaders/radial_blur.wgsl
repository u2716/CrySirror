struct EffectUniforms {
    tex_size: vec2<f32>,
    tex_diff_u: vec2<f32>,
    effect_params: vec4<f32>,
};

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var pal_texture: texture_2d<f32>;
@group(0) @binding(1) var pal_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: EffectUniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // FX.dll ps_2_0 c0/c1 = texture dimensions, c2/c3 = absolute
    // center UV, c4 = level. Twelve taps span -6 through +5.
    let center = uniforms.effect_params.xy;
    let level = uniforms.effect_params.z;
    let direction = input.tex_coord - center;
    let radius = length(direction);
    if radius == 0.0 || level <= 0.0 {
        return textureSampleLevel(pal_texture, pal_sampler, input.tex_coord, 0.0) * input.color;
    }
    let divisor = radius * log2(2.5 / (radius * level)) * 0.3010300099849701;
    if abs(divisor) < 0.0000001 {
        return textureSampleLevel(pal_texture, pal_sampler, input.tex_coord, 0.0) * input.color;
    }
    let step = direction / divisor / uniforms.tex_size;
    var color = vec4<f32>(0.0);
    for (var tap = -6; tap <= 5; tap += 1) {
        let uv = clamp(input.tex_coord + step * f32(tap), vec2<f32>(0.0), vec2<f32>(1.0));
        color += textureSampleLevel(pal_texture, pal_sampler, uv, 0.0);
    }
    return color * input.color * 0.0833333358168602;
}
