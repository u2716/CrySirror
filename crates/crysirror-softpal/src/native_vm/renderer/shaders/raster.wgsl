// Translation of FX.dll RASTER vs_1_1/ps_2_0. See fx-bytecode.json.
struct RasterUniforms {
    params: vec4<f32>,
    transform_x: vec4<f32>,
    transform_y: vec4<f32>,
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
@group(1) @binding(0) var<uniform> uniforms: RasterUniforms;

fn wave(coordinate: vec2<f32>, phase: f32) -> vec2<f32> {
    let cycles = (coordinate * 512.0 + phase * 512.0) * 0.0019531245343387127 + 0.25;
    let angle = fract(cycles) * 6.2831854820251465 - 3.1415927410125732;
    let square = angle * angle;
    var value = square * -2.5239850742764247e-7 + 2.476090048730839e-5;
    value = square * value - 0.001388839678838849;
    value = square * value + 0.04166664183139801;
    value = square * value - 0.5;
    return square * value + 1.0;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // Local PAL clip coordinates; placement and rotation follow the effect.
    let position = input.position + wave(input.position.yx * 0.5, uniforms.params.x) * uniforms.params.yz * 0.1;
    var output: VertexOutput;
    let point = vec3<f32>(position, 1.0);
    output.position = vec4<f32>(dot(uniforms.transform_x.xyz, point), dot(uniforms.transform_y.xyz, point), 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    // Scene tint is the enclosing PAL sprite's modulation.
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.tex_coord + wave(input.tex_coord.yx, uniforms.params.x) * uniforms.params.yz * 0.1;
    return textureSample(pal_texture, pal_sampler, uv) * input.color;
}
