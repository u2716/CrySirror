// SPDX-License-Identifier: MPL-2.0
// Ported from xmoezzz/siglus_rs 5f358ef1360b0439e92aed2b749e8f25d8721954.
// Source and changes recorded in ../UPSTREAM.md.
pub use crate::canvas_lens::CanvasLens;
use crate::layer::SpriteTransform as Sprite;
use serde::{Deserialize,Serialize};

/// Explicit geometric secondary ordering, evaluated before ordered
/// composition transforms. It is independent of projection or GPU Z testing.
#[derive(Clone,Copy,Debug,PartialEq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryOrder {
    pub aggregate:DepthAggregate,
    pub xy_weight:f32,
    pub quantize:f32,
    #[serde(default)]
    pub negative_truncation_bias:bool,
    /// A logical image/text carrier can share its computed key with payload
    /// descendants, independently of those descendants' glyph/mesh bounds.
    #[serde(default)]
    pub apply_to_descendants:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum DepthAggregate{Average,Minimum,Maximum}
impl GeometryOrder {
    pub fn validate(&self)->anyhow::Result<()> {
        anyhow::ensure!(self.xy_weight.is_finite()&&self.xy_weight.abs()<=1_000_000.
            &&self.quantize.is_finite()&&self.quantize>0.&&self.quantize<=1_000_000.,"geometry order parameters");
        Ok(())
    }
    pub fn key(&self,points:impl IntoIterator<Item=[f32;3]>)->anyhow::Result<i32> {
        let points=points.into_iter().map(|p|p[2]+self.xy_weight*(p[0].abs()+p[1].abs())).collect::<Vec<_>>();
        anyhow::ensure!(!points.is_empty()&&points.iter().all(|v|v.is_finite()),"geometry order points");
        let value=match self.aggregate {
            DepthAggregate::Average=>{
                let factor=1./points.len()as f32;
                (points.iter().map(|v|f64::from(*v)).sum::<f64>()*f64::from(factor))as f32
            },
            DepthAggregate::Minimum=>points.into_iter().fold(f32::INFINITY,f32::min),
            DepthAggregate::Maximum=>points.into_iter().fold(f32::NEG_INFINITY,f32::max),
        }*self.quantize;
        let quantized=if self.negative_truncation_bias&&value<0.{f64::from(value.trunc())-1.}else{f64::from(value).floor()};
        anyhow::ensure!(quantized>=i32::MIN as f64&&quantized<=i32::MAX as f64,"geometry order overflow");
        Ok(quantized as i32)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectedPoint {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    /// Homogeneous clip-space W. 2D paths use 1.0; the 3D camera path uses
    /// camera-space Z, matching D3DX perspective projection. The renderer
    /// keeps this value so UV/world varyings remain perspective-correct even
    /// though the quad's screen position is prepared on the CPU.
    pub clip_w: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SpriteQuadGeometry {
    pub projected: [ProjectedPoint; 4],
    pub world: [[f32; 3]; 4],
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }

    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    fn normalize(self) -> Self {
        let len2 = self.dot(self);
        if len2 <= f32::EPSILON {
            return Self::new(0.0, 0.0, 0.0);
        }
        let inv = len2.sqrt().recip();
        Self::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

fn rotate_x(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
}

fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

fn rotate_z(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
}

fn uses_3d(sprite: &Sprite) -> bool {
    !sprite.matrix_is_2d()
        || sprite.billboard
        || sprite.camera_enabled
        || sprite.z.abs() > f32::EPSILON
        || sprite.pivot_z.abs() > f32::EPSILON
        || (sprite.scale_z - 1.0).abs() > 1e-6
        || sprite.rotate_x.abs() > f32::EPSILON
        || sprite.rotate_y.abs() > f32::EPSILON
}
fn apply_local_matrix(sprite: &Sprite, p: Vec3) -> Vec3 {
    if let Some(matrix) = sprite.local_matrix {
        let p =
            glam::Mat4::from_cols_array(&matrix).transform_point3(glam::Vec3::new(p.x, p.y, p.z));
        Vec3::new(p.x, p.y, p.z)
    } else {
        p
    }
}

/// Affine model transform before camera projection. Keeping it in world
/// space allows complete 3D ancestor chains without projecting a child twice.
pub fn sprite_model_matrix(sprite: &Sprite, dst_x: f32, dst_y: f32) -> glam::Mat4 {
    let (s, offset) = sprite.resolved();
    let pivot = glam::Vec3::new(s.pivot_x, s.pivot_y, s.pivot_z);
    let anchor = glam::Vec3::new(dst_x + offset[0], dst_y + offset[1], s.z + s.pivot_z)
        + if s.object_anchor { glam::Vec3::ZERO } else { glam::Vec3::new(s.pivot_x, s.pivot_y, 0.) };
    let centre = pivot + if s.object_anchor {
        glam::Vec3::new(s.texture_center_x, s.texture_center_y, 0.)
    } else { glam::Vec3::ZERO };
    glam::Mat4::from_translation(anchor)
        * s.local_matrix.map_or(glam::Mat4::IDENTITY, |m| glam::Mat4::from_cols_array(&m))
        * glam::Mat4::from_rotation_y(s.rotate_y)
        * glam::Mat4::from_rotation_x(s.rotate_x)
        * glam::Mat4::from_rotation_z(s.rotate)
        * glam::Mat4::from_scale(glam::Vec3::new(s.scale_x, s.scale_y, s.scale_z))
        * glam::Mat4::from_translation(-centre)
}

/// Ancestor contribution for ordinary versus position-only inheritance.
/// The latter transforms the anchor through the full parent matrix and keeps
/// an identity linear basis, preserving the node's own local pose.
pub fn sprite_parent_matrix(sprite: &Sprite, dst_x: f32, dst_y: f32, parent: glam::Mat4) -> glam::Mat4 {
    let (s, offset) = sprite.resolved();
    if s.inherit_parent_basis { return parent; }
    let anchor = glam::Vec3::new(dst_x + offset[0], dst_y + offset[1], s.z + s.pivot_z)
        + if s.object_anchor { glam::Vec3::ZERO } else { glam::Vec3::new(s.pivot_x, s.pivot_y, 0.) };
    glam::Mat4::from_translation(parent.transform_point3(anchor) - anchor)
}

/// Project a point already transformed into world space. An inherited camera
/// supplies only the view/projection; its model pose is part of the ancestry.
pub fn project_world_point(camera: &Sprite, point: [f32; 3], canvas: [f32; 2]) -> Option<ProjectedPoint> {
    project_point(camera, Vec3::new(point[0], point[1], point[2]), canvas[0], canvas[1])
}
/// Homogeneous projection for GPU-clipped polygons. Behind-camera vertices
/// remain present with negative clip W so triangles crossing a camera plane
/// are clipped rather than discarded as a whole.
pub fn project_world_point_unclipped(camera:&Sprite,point:[f32;3],canvas:[f32;2])->Option<ProjectedPoint> {
    if let Some(lens)=camera.canvas_projection{return lens.point(point,canvas)}
    if !camera.camera_enabled{return project_world_point(camera,point,canvas)}
    let (eye,forward,right,up)=camera_basis(camera);
    let p=Vec3::new(point[0],point[1],point[2]).sub(eye);
    let mut w=p.dot(forward);
    // The Layer representation stores screen XY plus clip W. A signed tiny W
    // preserves finite homogeneous coordinates for a vertex exactly on W=0;
    // near-plane clipping removes it before any fragment is evaluated.
    if w.abs()<1e-20{w=1e-20f32.copysign(w)}
    let tan=(camera.camera_view_angle_deg.to_radians()*0.5).tan().max(1e-3);
    let x=(p.dot(right)/(w*tan*canvas[0]/canvas[1])+1.)*0.5*canvas[0];
    let y=(1.-p.dot(up)/(w*tan))*0.5*canvas[1];
    let depth=10000./9999.-10000./(9999.*w);
    [x,y,depth,w].iter().all(|v|v.is_finite()).then_some(ProjectedPoint{x,y,depth,clip_w:w})
}

/// Geometry under a full affine3D parent, with projection performed once after
/// model composition. Used by retained layers and immutable captures alike.
pub fn sprite_quad_geometry_in_hierarchy(
    sprite: &Sprite, dst: [f32; 2], local: [f32; 4], canvas: [f32; 2],
    parent: glam::Mat4, inherited_camera: Option<&Sprite>, inherited_3d: bool,
) -> Option<SpriteQuadGeometry> {
    let (s, offset) = sprite.resolved();
    //The owner frame decides whether a collapsed ordinary leaf is visible.
    //Ordered coverage commands still need an admission point for a degenerate
    //quad, even though it contributes no rasterized pixels.
    let camera = if s.camera_enabled||s.canvas_projection.is_some() { &s } else { inherited_camera.unwrap_or(&s) };
    let corners = [[local[0],local[1]], [local[2],local[1]], [local[2],local[3]], [local[0],local[3]]];
    let parent = sprite_parent_matrix(sprite, dst[0], dst[1], parent);
    let model = parent * sprite_model_matrix(sprite, dst[0], dst[1]);
    let world = corners.map(|p| {
        let q = if s.billboard {
            let mut facing=s.clone();
            facing.camera_eye=camera.camera_eye;
            facing.camera_target=camera.camera_target;
            facing.camera_up=camera.camera_up;
            let q=transform_billboard_point(&facing,p[0],p[1],dst[0]+offset[0],dst[1]+offset[1]);
            parent.transform_point3(glam::Vec3::new(q.x,q.y,q.z))
        } else { model.transform_point3(glam::Vec3::new(p[0],p[1],0.)) };
        q.to_array()
    });
    let mut projected=[ProjectedPoint{x:0.,y:0.,depth:0.,clip_w:1.};4];
    for i in 0..4 {
        projected[i]=project_world_point_unclipped(camera,world[i],canvas)?;
        if !inherited_3d && !uses_3d(&s) { projected[i].depth=0.; }
    }
    if s.culling && signed_area((projected[0].x,projected[0].y),(projected[1].x,projected[1].y),(projected[2].x,projected[2].y))<=0. { return None; }
    Some(SpriteQuadGeometry{projected,world})
}

fn transform_local_point(sprite: &Sprite, px: f32, py: f32, dst_x: f32, dst_y: f32) -> Vec3 {
    let (anchor_x, anchor_y, mut p) = if sprite.object_anchor {
        (
            dst_x,
            dst_y,
            Vec3::new(
                px - sprite.texture_center_x - sprite.pivot_x,
                py - sprite.texture_center_y - sprite.pivot_y,
                -sprite.pivot_z,
            ),
        )
    } else {
        (
            dst_x + sprite.pivot_x,
            dst_y + sprite.pivot_y,
            Vec3::new(px - sprite.pivot_x, py - sprite.pivot_y, -sprite.pivot_z),
        )
    };
    p.x *= sprite.scale_x;
    p.y *= sprite.scale_y;
    p.z *= sprite.scale_z;
    // D3DXMatrixRotationYawPitchRoll(yaw, pitch, roll) applies roll,
    // then pitch, then yaw to row vectors.
    p = rotate_z(p, sprite.rotate);
    p = rotate_x(p, sprite.rotate_x);
    p = rotate_y(p, sprite.rotate_y);
    p = apply_local_matrix(sprite, p);
    p.add(Vec3::new(anchor_x, anchor_y, sprite.z + sprite.pivot_z))
}

fn camera_basis(sprite: &Sprite) -> (Vec3, Vec3, Vec3, Vec3) {
    let eye = Vec3::new(
        sprite.camera_eye[0],
        sprite.camera_eye[1],
        sprite.camera_eye[2],
    );
    let target = Vec3::new(
        sprite.camera_target[0],
        sprite.camera_target[1],
        sprite.camera_target[2],
    );
    let up = Vec3::new(
        sprite.camera_up[0],
        sprite.camera_up[1],
        sprite.camera_up[2],
    );
    let forward = target.sub(eye).normalize();
    let right = up.cross(forward).normalize();
    let up2 = forward.cross(right).normalize();
    (eye, forward, right, up2)
}

fn transform_billboard_point(sprite: &Sprite, px: f32, py: f32, dst_x: f32, dst_y: f32) -> Vec3 {
    let (_, _, right, up) = camera_basis(sprite);
    let lx = (px - sprite.pivot_x) * sprite.scale_x;
    // Texture-space Y grows downward, while WORLD Y grows upward.
    let ly = -(py - sprite.pivot_y) * sprite.scale_y;
    let (s, c) = sprite.rotate.sin_cos();
    let rx = lx * c - ly * s;
    let ry = lx * s + ly * c;
    let anchor = Vec3::new(
        dst_x + sprite.pivot_x,
        dst_y + sprite.pivot_y,
        sprite.z + sprite.pivot_z,
    );
    anchor.add(Vec3::new(
        right.x * rx + up.x * ry,
        right.y * rx + up.y * ry,
        right.z * rx + up.z * ry,
    ))
}

fn project_point(sprite: &Sprite, p: Vec3, win_w: f32, win_h: f32) -> Option<ProjectedPoint> {
    if let Some(lens)=sprite.canvas_projection{return lens.point([p.x,p.y,p.z],[win_w,win_h])}
    if !sprite.camera_enabled {
        let depth = (0.5 - p.z / 100000.0).clamp(0.0, 1.0);
        return Some(ProjectedPoint {
            x: p.x,
            y: p.y,
            depth,
            clip_w: 1.0,
        });
    }

    let (eye, zaxis, xaxis, yaxis) = camera_basis(sprite);

    let rel = p.sub(eye);
    let cx = rel.dot(xaxis);
    let cy = rel.dot(yaxis);
    let cz = rel.dot(zaxis);
    if cz <= 1e-3 {
        return None;
    }

    let aspect = if win_h.abs() > f32::EPSILON {
        win_w / win_h
    } else {
        1.0
    };
    let vfov = sprite
        .camera_view_angle_deg
        .to_radians()
        .clamp(1e-3, std::f32::consts::PI - 1e-3);
    let tan_half_v = (vfov * 0.5).tan().max(1e-3);
    let tan_half_h = (tan_half_v * aspect.max(1e-3)).max(1e-3);

    let x_ndc = cx / (cz * tan_half_h);
    let y_ndc = cy / (cz * tan_half_v);

    let sx = (x_ndc + 1.0) * 0.5 * win_w;
    let sy = (1.0 - y_ndc) * 0.5 * win_h;
    let near = 1.0;
    let far = 10000.0;
    let depth = (far / (far - near) - near * far / ((far - near) * cz)).clamp(0.0, 1.0);
    Some(ProjectedPoint {
        x: sx,
        y: sy,
        depth,
        clip_w: cz,
    })
}

fn signed_area(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

pub fn sprite_quad_geometry_rect(
    sprite: &Sprite,
    dst_x: f32,
    dst_y: f32,
    local_left: f32,
    local_top: f32,
    local_right: f32,
    local_bottom: f32,
    win_w: f32,
    win_h: f32,
) -> Option<SpriteQuadGeometry> {
    let (resolved, offset) = sprite.resolved();
    let sprite = &resolved;
    let dst_x = dst_x + offset[0];
    let dst_y = dst_y + offset[1];
    if sprite.collapsed() {
        return None;
    }
    let xf = if sprite.billboard {
        transform_billboard_point as fn(&Sprite, f32, f32, f32, f32) -> Vec3
    } else {
        transform_local_point as fn(&Sprite, f32, f32, f32, f32) -> Vec3
    };

    let w0 = xf(sprite, local_left, local_top, dst_x, dst_y);
    let w1 = xf(sprite, local_right, local_top, dst_x, dst_y);
    let w2 = xf(sprite, local_right, local_bottom, dst_x, dst_y);
    let w3 = xf(sprite, local_left, local_bottom, dst_x, dst_y);
    let world = [
        [w0.x, w0.y, w0.z],
        [w1.x, w1.y, w1.z],
        [w2.x, w2.y, w2.z],
        [w3.x, w3.y, w3.z],
    ];

    if !uses_3d(sprite) {
        return Some(SpriteQuadGeometry {
            projected: [
                ProjectedPoint {
                    x: w0.x,
                    y: w0.y,
                    depth: 0.0,
                    clip_w: 1.0,
                },
                ProjectedPoint {
                    x: w1.x,
                    y: w1.y,
                    depth: 0.0,
                    clip_w: 1.0,
                },
                ProjectedPoint {
                    x: w2.x,
                    y: w2.y,
                    depth: 0.0,
                    clip_w: 1.0,
                },
                ProjectedPoint {
                    x: w3.x,
                    y: w3.y,
                    depth: 0.0,
                    clip_w: 1.0,
                },
            ],
            world,
        });
    }

    let p0 = project_point(sprite, w0, win_w, win_h)?;
    let p1 = project_point(sprite, w1, win_w, win_h)?;
    let p2 = project_point(sprite, w2, win_w, win_h)?;
    let p3 = project_point(sprite, w3, win_w, win_h)?;

    if sprite.culling && signed_area((p0.x, p0.y), (p1.x, p1.y), (p2.x, p2.y)) <= 0.0 {
        return None;
    }

    Some(SpriteQuadGeometry {
        projected: [p0, p1, p2, p3],
        world,
    })
}

pub fn sprite_quad_points_rect(
    sprite: &Sprite,
    dst_x: f32,
    dst_y: f32,
    local_left: f32,
    local_top: f32,
    local_right: f32,
    local_bottom: f32,
    win_w: f32,
    win_h: f32,
) -> Option<[ProjectedPoint; 4]> {
    sprite_quad_geometry_rect(
        sprite,
        dst_x,
        dst_y,
        local_left,
        local_top,
        local_right,
        local_bottom,
        win_w,
        win_h,
    )
    .map(|quad| quad.projected)
}

pub fn sprite_quad_points(
    sprite: &Sprite,
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    win_w: f32,
    win_h: f32,
) -> Option<[ProjectedPoint; 4]> {
    sprite_quad_points_rect(sprite, dst_x, dst_y, 0.0, 0.0, dst_w, dst_h, win_w, win_h)
}

pub fn project_model_point(
    sprite: &Sprite,
    local: [f32; 3],
    anchor_x: f32,
    anchor_y: f32,
    win_w: f32,
    win_h: f32,
) -> Option<ProjectedPoint> {
    let mut p = Vec3::new(
        local[0] - sprite.pivot_x,
        local[1] - sprite.pivot_y,
        local[2] - sprite.pivot_z,
    );
    p.x *= sprite.scale_x;
    p.y *= sprite.scale_y;
    p.z *= sprite.scale_z;
    p = rotate_z(p, sprite.rotate);
    p = rotate_x(p, sprite.rotate_x);
    p = rotate_y(p, sprite.rotate_y);
    p = apply_local_matrix(sprite, p);
    p = p.add(Vec3::new(
        anchor_x + sprite.pivot_x,
        anchor_y + sprite.pivot_y,
        sprite.z + sprite.pivot_z,
    ));
    project_point(sprite, p, win_w, win_h)
}

#[cfg(test)]
mod tests {
    use super::{project_point, rotate_x, rotate_y, rotate_z, sprite_quad_points, Vec3};
    use crate::layer::SpriteTransform as Sprite;

    fn camera_sprite() -> Sprite {
        Sprite {
            camera_enabled: true,
            camera_eye: [0.0, 0.0, 0.0],
            camera_target: [0.0, 0.0, 1.0],
            camera_up: [0.0, 1.0, 0.0],
            camera_view_angle_deg: 60.0,
            ..Default::default()
        }
    }

    #[test]
    fn yaw_pitch_roll_matches_d3dx_application_order() {
        let quarter = std::f32::consts::FRAC_PI_2;
        let mut point = Vec3::new(1.0, 2.0, 3.0);
        point = rotate_z(point, quarter);
        point = rotate_x(point, quarter);
        point = rotate_y(point, quarter);
        assert!((point.x - 1.0).abs() < 1e-5);
        assert!((point.y + 3.0).abs() < 1e-5);
        assert!((point.z - 2.0).abs() < 1e-5);
    }

    #[test]
    fn billboard_texture_top_projects_above_its_bottom() {
        let mut sprite = camera_sprite();
        sprite.billboard = true;
        sprite.z = 100.0;
        let quad = sprite_quad_points(&sprite, 0.0, 0.0, 32.0, 64.0, 640.0, 480.0).unwrap();
        assert!(quad[0].y < quad[3].y);
    }

    #[test]
    fn world_view_angle_is_vertical_and_does_not_change_with_window_width() {
        let sprite = camera_sprite();
        let point = Vec3::new(0.0, 1.0, 10.0);
        let narrow = project_point(&sprite, point, 640.0, 480.0).unwrap();
        let wide = project_point(&sprite, point, 1280.0, 480.0).unwrap();
        assert!((narrow.y - wide.y).abs() < 1e-4);
    }

    #[test]
    fn world_depth_matches_d3d_lh_near_and_far_planes() {
        let sprite = camera_sprite();
        let near = project_point(&sprite, Vec3::new(0.0, 0.0, 1.0), 640.0, 480.0).unwrap();
        let far = project_point(&sprite, Vec3::new(0.0, 0.0, 10000.0), 640.0, 480.0).unwrap();
        assert!(near.depth.abs() < 1e-6);
        assert!((far.depth - 1.0).abs() < 1e-6);
    }
}

/// Device-free retained ancestor state for projected geometry consumers such
/// as input masks. It composes the same transforms and camera as Layer output.
#[derive(Clone,Debug)]
pub struct HierarchyFrame {
    model: glam::Mat4,
    camera: Option<Sprite>,
    uses_3d: bool,
}
impl Default for HierarchyFrame {
    fn default()->Self {Self{model:glam::Mat4::IDENTITY,camera:None,uses_3d:false}}
}
impl HierarchyFrame {
    pub fn push(&mut self, transform:&Sprite, destination:[f32;2])->anyhow::Result<()> {
        transform.validate()?;
        self.model=sprite_parent_matrix(transform,destination[0],destination[1],self.model)
            *sprite_model_matrix(transform,destination[0],destination[1]);
        self.uses_3d|=uses_3d(transform);
        if transform.camera_enabled||transform.canvas_projection.is_some() {self.camera=Some(transform.clone());}
        Ok(())
    }
    pub fn quad(&self, transform:&Sprite, destination:[f32;2], local:[f32;4], canvas:[f32;2])->Option<[[f32;2];4]> {
        sprite_quad_geometry_in_hierarchy(transform,destination,local,canvas,
            self.model,self.camera.as_ref(),self.uses_3d)
            .map(|geometry|geometry.projected.map(|point|[point.x,point.y]))
    }
}
