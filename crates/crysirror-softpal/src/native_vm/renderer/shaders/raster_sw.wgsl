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

const PI: f32 = 3.141592;

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
    let offset = uniforms.effect_params.x;
    let undulate_x = uniforms.effect_params.y;
    let undulate_y = uniforms.effect_params.z;
    let undulate = vec2<f32>(undulate_y, undulate_x);
    let val = (input.tex_coord * 180.0) + ((vec2<f32>(360.0, 360.0) * (vec2<f32>(1.0, 1.0) - undulate)) * offset);
    let denominator = vec2<f32>(180.0, 180.0) - (vec2<f32>(180.0, 180.0) * undulate);
    let safe_denominator = select(denominator, vec2<f32>(1.0), denominator == vec2<f32>(0.0));
    let wave = select(sin(val * PI / safe_denominator) * (0.1 * undulate), vec2<f32>(0.0), denominator == vec2<f32>(0.0));
    let tex_coord_yx = input.tex_coord.yx + wave;
    let tex_coord = tex_coord_yx.yx;
    // The embedded HLSL declares COLOR as a scalar float.
    return textureSample(pal_texture, pal_sampler, tex_coord) * input.color.x;
}
