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

fn cubic_weight(x: f32) -> f32 {
    let a = -0.5;
    let ax = abs(x);
    if (ax <= 1.0) {
        return (a + 2.0) * ax * ax * ax - (a + 3.0) * ax * ax + 1.0;
    }
    if (ax < 2.0) {
        return a * ax * ax * ax - 5.0 * a * ax * ax + 8.0 * a * ax - 4.0 * a;
    }
    return 0.0;
}

fn sample_bicubic(uv: vec2<f32>) -> vec4<f32> {
    let coord = uv * uniforms.tex_size - vec2<f32>(0.5, 0.5);
    let base = floor(coord);
    let frac = coord - base;
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var weight_sum = 0.0;

    for (var y: i32 = -1; y <= 2; y = y + 1) {
        let wy = cubic_weight(f32(y) - frac.y);
        for (var x: i32 = -1; x <= 2; x = x + 1) {
            let wx = cubic_weight(f32(x) - frac.x);
            let weight = wx * wy;
            let sample_coord = (base + vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5, 0.5)) / uniforms.tex_size;
            color = color + textureSample(pal_texture, pal_sampler, sample_coord) * weight;
            weight_sum = weight_sum + weight;
        }
    }

    return color / max(weight_sum, 0.000001);
}

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
    return sample_bicubic(input.tex_coord) * input.color;
}
