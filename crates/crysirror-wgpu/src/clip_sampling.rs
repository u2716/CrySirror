//! Exact fragment clipping and isolated sampling for clipped atlas views.
use crate::geometry;
use crysirror_scene::{layer::TextureAddressMode, Primitive};

const UNBOUNDED: [f32; 4] = [-1e20, -1e20, 1e20, 1e20];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LayerVertex {
    position: [f32; 4],
    uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) filter: f32,
    sample_bounds: [f32; 4],
    clip_rect: [f32; 4],
}

fn sample_bounds(p: &Primitive) -> [f32; 4] {
    if p.sampling.address_u != TextureAddressMode::ClampToEdge
        || p.sampling.address_v != TextureAddressMode::ClampToEdge
        || p.filter.as_ref().is_some_and(|f| f.distortion.is_some())
    {
        return UNBOUNDED;
    }
    let mut bounds = [
        p.uv[0].min(p.uv[2]),
        p.uv[1].min(p.uv[3]),
        p.uv[0].max(p.uv[2]),
        p.uv[1].max(p.uv[3]),
    ];
    let Some(c) = p.clip else { return bounds };
    // A rectangular image view remains rectangular in source UV space for
    // unrotated/flipped and quarter-turn layers. Other transforms retain their
    // ordinary sampler and use exact fragment clipping below.
    if p.quad.iter().any(|q| (q[3] - 1.).abs() > 1e-5) {
        return bounds;
    }
    let origin = [p.quad[0][0], p.quad[0][1]];
    let axes = [
        [p.quad[1][0] - origin[0], p.quad[1][1] - origin[1]],
        [p.quad[3][0] - origin[0], p.quad[3][1] - origin[1]],
    ];
    let axis_index = |d: [f32; 2]| {
        let epsilon = (d[0].abs().max(d[1].abs()) * 1e-5).max(1e-4);
        if d[0].abs() > epsilon && d[1].abs() <= epsilon {
            Some(0)
        } else if d[1].abs() > epsilon && d[0].abs() <= epsilon {
            Some(1)
        } else {
            None
        }
    };
    let (Some(a), Some(b)) = (axis_index(axes[0]), axis_index(axes[1])) else {
        return bounds;
    };
    if a == b {
        return bounds;
    }
    for (uv_axis, screen_axis) in [a, b].into_iter().enumerate() {
        let delta = axes[uv_axis][screen_axis];
        let t0 = (c[screen_axis] - origin[screen_axis]) / delta;
        let t1 = (c[screen_axis] + c[screen_axis + 2] - origin[screen_axis]) / delta;
        let lo = t0.min(t1).clamp(0., 1.);
        let hi = t0.max(t1).clamp(0., 1.);
        let uv0 = p.uv[uv_axis];
        let duv = p.uv[uv_axis + 2] - uv0;
        let x = uv0 + lo * duv;
        let y = uv0 + hi * duv;
        bounds[uv_axis] = x.min(y);
        bounds[uv_axis + 2] = x.max(y);
    }
    bounds
}

pub(crate) fn quad(p: &Primitive, canvas: [u32; 2], color: [u8; 4]) -> [LayerVertex; 6] {
    let sample_bounds = sample_bounds(p);
    let clip_rect = p.clip.map_or(UNBOUNDED, |c| {
        [
            c[0] / canvas[0] as f32,
            c[1] / canvas[1] as f32,
            (c[0] + c[2]) / canvas[0] as f32,
            (c[1] + c[3]) / canvas[1] as f32,
        ]
    });
    geometry::layer_quad(p, canvas, color).map(|v| LayerVertex {
        position: v.position,
        uv: v.uv,
        color: v.color,
        filter: v.filter,
        sample_bounds,
        clip_rect,
    })
}

pub(crate) fn mesh_vertex(
    p: &Primitive, canvas: [u32;2], size: [f32;2],
    v: &crysirror_scene::projected_mesh::Vertex, tint: [f32;4], filter: f32,
) -> LayerVertex {
    let u = v.position[0]/size[0];
    let t = v.position[1]/size[1];
    let weights = [(1.-u)*(1.-t),u*(1.-t),u*t,(1.-u)*t];
    let q: [f32;4] = std::array::from_fn(|axis| {
        (0..4).map(|i| weights[i] * if axis < 3 {
            p.quad[i][axis]*p.quad[i][3]
        } else { p.quad[i][3] }).sum()
    });
    let w = q[3]*v.position[3];
    let clip_rect = p.clip.map_or(UNBOUNDED,|c|[
        c[0]/canvas[0] as f32,c[1]/canvas[1] as f32,
        (c[0]+c[2])/canvas[0] as f32,(c[1]+c[3])/canvas[1] as f32]);
    LayerVertex {
        position: [(2.*q[0]/canvas[0] as f32-q[3])*v.position[3],
            (q[3]-2.*q[1]/canvas[1] as f32)*v.position[3],
            q[2]*v.position[3]+v.position[2]*w,w],
        uv:v.uv,
        color:std::array::from_fn(|i|v.color[i]*tint[i]),
        filter, sample_bounds:UNBOUNDED, clip_rect,
    }
}
