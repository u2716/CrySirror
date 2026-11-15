// SPDX-License-Identifier: MPL-2.0
// Lite software rays, ABI v1 (docs/CONSOLE_SHADERS.md): the fragment profile
// of compute ray tracing for GPUs without compute. Same arithmetic as
// native/ray_lite.cgh, whose PSV programs (ray_lite_v/ray_lite_f) the GXM
// shader index joins to this module. Scene data: crysirror-raytrace
// PackedScene in two RG32F textures.
struct RayLite {
    ray_nodes: vec4<f32>,     // width, height, node_count, 0
    ray_triangles: vec4<f32>, // width, height, triangle_count, 0
    ray_eye: vec4<f32>,       // eye.xyz, positive world-space ray bias
    ray_corner: vec4<f32>,    // unnormalized top-left ray
    ray_dx: vec4<f32>,        // ray delta over the target width
    ray_dy: vec4<f32>,        // ray delta over the target height
    ray_light: vec4<f32>,     // direction toward the sun, maximum distance
    ray_sun: vec4<f32>,
    ray_ambient: vec4<f32>,
    ray_sky_low: vec4<f32>,
    ray_sky_high: vec4<f32>,
}
@group(0) @binding(0) var<uniform> ray: RayLite;
@group(0) @binding(1) var node_texture: texture_2d<f32>;
@group(0) @binding(2) var triangle_texture: texture_2d<f32>;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOut {
    return VertexOut(vec4<f32>(position, 0.0, 1.0), uv);
}

fn load_node(index: f32) -> vec2<f32> {
    let y = floor(index / ray.ray_nodes.x);
    return textureLoad(node_texture, vec2<i32>(i32(index - y * ray.ray_nodes.x), i32(y)), 0).xy;
}
fn load_triangle(index: f32) -> vec2<f32> {
    let y = floor(index / ray.ray_triangles.x);
    return textureLoad(triangle_texture, vec2<i32>(i32(index - y * ray.ray_triangles.x), i32(y)), 0).xy;
}

struct Slab { valid: bool, near_t: f32, far_t: f32 }
fn lite_slab(o: f32, d: f32, lo: f32, hi: f32, near_t: f32, far_t: f32) -> Slab {
    var s = Slab(o >= lo && o <= hi, near_t, far_t);
    if (abs(d) >= 1.0e-20) {
        let a = (lo - o) / d;
        let b = (hi - o) / d;
        s.near_t = max(near_t, min(a, b));
        s.far_t = min(far_t, max(a, b));
        s.valid = s.near_t <= s.far_t;
    }
    return s;
}
fn lite_box(origin: vec3<f32>, direction: vec3<f32>, lo: vec3<f32>, hi: vec3<f32>, limit: f32) -> bool {
    let x = lite_slab(origin.x, direction.x, lo.x, hi.x, ray.ray_eye.w, limit);
    let y = lite_slab(origin.y, direction.y, lo.y, hi.y, x.near_t, x.far_t);
    let z = lite_slab(origin.z, direction.z, lo.z, hi.z, y.near_t, y.far_t);
    return x.valid && y.valid && z.valid;
}

struct Hit { valid: bool, distance: f32, normal: vec3<f32>, color: vec3<f32> }
fn lite_triangle(origin: vec3<f32>, direction: vec3<f32>, index: f32, limit: f32) -> Hit {
    let base = index * 6.0;
    let a0 = load_triangle(base);
    let a1 = load_triangle(base + 1.0);
    let b0 = load_triangle(base + 2.0);
    let b1 = load_triangle(base + 3.0);
    let c0 = load_triangle(base + 4.0);
    let c1 = load_triangle(base + 5.0);
    let a = vec3<f32>(a0, a1.x);
    let e1 = vec3<f32>(b0, b1.x) - a;
    let e2 = vec3<f32>(c0, c1.x) - a;
    let p = cross(direction, e2);
    let det = dot(e1, p);
    let n = cross(e1, e2);
    let area2 = dot(n, n);
    var hit = Hit(false, limit, vec3<f32>(0.0), vec3<f32>(a1.y, b1.y, c1.y));
    if (area2 >= 1.0e-30 && abs(det) >= 1.0e-7 * sqrt(area2)) {
        let inverse = 1.0 / det;
        let offset = origin - a;
        let u = dot(offset, p) * inverse;
        if (u >= 0.0 && u <= 1.0) {
            let q = cross(offset, e1);
            let v = dot(direction, q) * inverse;
            if (v >= 0.0 && u + v <= 1.0) {
                let candidate = dot(e2, q) * inverse;
                if (candidate > ray.ray_eye.w && candidate < limit) {
                    hit.valid = true;
                    hit.distance = candidate;
                    hit.normal = n / sqrt(area2);
                    if (dot(hit.normal, direction) > 0.0) {
                        hit.normal = -hit.normal;
                    }
                }
            }
        }
    }
    return hit;
}

fn lite_trace(origin: vec3<f32>, direction: vec3<f32>, any_hit: bool) -> Hit {
    var best = Hit(false, ray.ray_light.w, vec3<f32>(0.0), vec3<f32>(0.0));
    var index = 0.0;
    for (var visit = 0; visit < i32(ray.ray_nodes.z) && index < ray.ray_nodes.z && !(any_hit && best.valid); visit += 1) {
        let base = index * 4.0;
        let n0 = load_node(base);
        let n1 = load_node(base + 1.0);
        let n2 = load_node(base + 2.0);
        let n3 = load_node(base + 3.0);
        let intersects = lite_box(origin, direction, vec3<f32>(n0, n1.x), vec3<f32>(n2, n3.x), best.distance);
        let first = floor(n3.y / 5.0);
        let count = n3.y - first * 5.0;
        if (intersects && count == 0.0) {
            index += 1.0;
        } else {
            if (intersects) {
                for (var item = 0; item < 4; item += 1) {
                    if (f32(item) < count && !(any_hit && best.valid)) {
                        let candidate = lite_triangle(origin, direction, first + f32(item), best.distance);
                        if (candidate.valid) {
                            best = candidate;
                        }
                    }
                }
            }
            index = n1.y;
        }
    }
    return best;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let direction = normalize(ray.ray_corner.xyz + in.uv.x * ray.ray_dx.xyz + in.uv.y * ray.ray_dy.xyz);
    let primary = lite_trace(ray.ray_eye.xyz, direction, false);
    if (!primary.valid) {
        let sky = clamp(direction.y * 0.5 + 0.5, 0.0, 1.0);
        return vec4<f32>(ray.ray_sky_low.xyz * (1.0 - sky) + ray.ray_sky_high.xyz * sky, 1.0);
    }
    let light = normalize(ray.ray_light.xyz);
    let lambert = max(dot(primary.normal, light), 0.0);
    if (lambert <= 0.0) {
        return vec4<f32>(primary.color * ray.ray_ambient.xyz, 1.0);
    }
    let origin = ray.ray_eye.xyz + direction * primary.distance + primary.normal * (ray.ray_eye.w * 2.0);
    let occluded = lite_trace(origin, light, true).valid;
    let direct = select(ray.ray_sun.xyz * lambert, vec3<f32>(0.0), occluded);
    return vec4<f32>(primary.color * (ray.ray_ambient.xyz + direct), 1.0);
}
