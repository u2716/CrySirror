//! Geometry/UV cropping shared by retained layers and positioned glyph draws.
use anyhow::{Result,ensure};
/// A surviving local rectangle and its corresponding source texture UVs.
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct CropRect {
    /// Local [x,y,width,height] after intersection, before projection.
    pub rect:[f32;4],
    /// Interpolated [u0,v0,u1,v1], preserving reversed/repeated source UVs.
    pub uv:[f32;4],
}
/// Finite local rectangles may be empty; they never allocate a pixel buffer.
pub fn valid_rect(rect:[f32;4])->bool {
    rect.iter().all(|v|v.is_finite()) && rect[2]>=0. && rect[3]>=0.
        && (rect[0]+rect[2]).is_finite() && (rect[1]+rect[3]).is_finite()
}
/// Intersect geometry and remap texture coordinates without reflowing content.
pub fn crop_rect(bounds:[f32;4],clip:Option<[f32;4]>,uv:[f32;4])->Result<Option<CropRect>> {
    ensure!(valid_rect(bounds) && clip.is_none_or(valid_rect) && uv.iter().all(|v|v.is_finite()),"invalid source crop rectangle/UV");
    if bounds[2]==0. || bounds[3]==0. {return Ok(None)}
    let rect=match clip {Some(clip)=>match super::intersect(bounds,clip){Some(r)=>r,None=>return Ok(None)},None=>bounds};
    let x0=(rect[0]-bounds[0])/bounds[2];let y0=(rect[1]-bounds[1])/bounds[3];
    let x1=(rect[0]+rect[2]-bounds[0])/bounds[2];let y1=(rect[1]+rect[3]-bounds[1])/bounds[3];
    let out=[uv[0]+(uv[2]-uv[0])*x0,uv[1]+(uv[3]-uv[1])*y0,uv[0]+(uv[2]-uv[0])*x1,uv[1]+(uv[3]-uv[1])*y1];
    ensure!(out.iter().all(|v|v.is_finite()),"source crop UV overflow");
    Ok(Some(CropRect{rect,uv:out}))
}
/// Extract a local subrectangle from an already projected quad. Interpolate
/// homogeneous positions before dividing, preserving perspective and shear.
pub fn subrect_quad(quad:[[f32;4];4],extent:[f32;2],rect:[f32;4])->Result<[[f32;4];4]> {
    ensure!(extent.iter().all(|v|v.is_finite() && *v>0.) && valid_rect(rect),"invalid crop quad extent/rect");
    let positions=quad.map(|[x,y,z,w]|[x*w,y*w,z*w,w]);
    let point=|x:f32,y:f32|->Result<[f32;4]> {
        let (u,v)=(x/extent[0],y/extent[1]);
        let h:[f32;4]=std::array::from_fn(|i|positions[0][i]*(1.-u)*(1.-v)+positions[1][i]*u*(1.-v)+positions[2][i]*u*v+positions[3][i]*(1.-u)*v);
        ensure!(h.iter().all(|v|v.is_finite()) && h[3].abs()>1e-6,"crop projection has invalid homogeneous coordinates");
        Ok([h[0]/h[3],h[1]/h[3],h[2]/h[3],h[3]])
    };
    let [x,y,w,h]=rect;Ok([point(x,y)?,point(x+w,y)?,point(x+w,y+h)?,point(x,y+h)?])
}
