#version 460
// SPDX-License-Identifier: MPL-2.0
// Native deko3d/UAM compute shader. GLSL is compiler input, no OpenGL runtime.
layout(local_size_x=8, local_size_y=8, local_size_z=1) in;
layout(binding=0) uniform sampler2D node_texture;
layout(binding=1) uniform sampler2D triangle_texture;
layout(binding=0, rgba16f) writeonly uniform image2D output_image;
layout(binding=0, std140) uniform RaySettings {
    vec4 ray_nodes;
    vec4 ray_triangles;
    vec4 ray_eye;
    vec4 ray_corner;
    vec4 ray_dx;
    vec4 ray_dy;
    vec4 ray_light;
    vec4 ray_sun;
    vec4 ray_ambient;
    vec4 ray_sky_low;
    vec4 ray_sky_high;
};
#define float2 vec2
#define float3 vec3
#define float4 vec4
vec2 load_node(float index) {
    int row = int(floor(index / ray_nodes.x));
    return texelFetch(node_texture, ivec2(int(index) - row * int(ray_nodes.x), row), 0).xy;
}
vec2 load_triangle(float index) {
    int row = int(floor(index / ray_triangles.x));
    return texelFetch(triangle_texture, ivec2(int(index) - row * int(ray_triangles.x), row), 0).xy;
}
// The source packer expands this include before passing source to UAM.
#include "../ray_lite.cgh"

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 extent = imageSize(output_image);
    if (any(greaterThanEqual(pixel, extent))) return;
    vec2 uv = (vec2(pixel) + 0.5) / vec2(extent);
    imageStore(output_image, pixel, lite_pixel(uv));
}
