// SPDX-License-Identifier: MPL-2.0
//! Direct ordered mesh rasterization with native clip-W perspective sampling.
use super::*;
impl Renderer {
    pub(super) fn prepare_projected_mesh(
        &mut self,
        p: &Primitive,
        assets: &mut Assets,
        canvas: [u32; 2],
        records: &mut Vec<FilterGpu>,
        vertices: &mut Vec<clip_sampling::LayerVertex>,
        draws: &mut Vec<Draw>,
        filter_keys: Option<(String, String)>,
        coverage_active:bool,
    ) -> Result<()> {
        let Paint::ProjectedMesh { frame, tint } = &p.paint else {
            unreachable!()
        };
        frame.validate()?;
        if filter_keys
            .as_ref()
            .is_some_and(|(a, b)| !self.textures.contains_key(a) || !self.textures.contains_key(b))
        {
            return Ok(());
        }
        if !self.textures.contains_key("solid:white") {
            self.upload("solid:white".into(), 1, 1, &[255; 4])?;
        }
        if p.coverage==crysirror_scene::layer::coverage::Operation::Write {
            //The ordered writer is admitted before whole-face near/cull
            //rejection, even when projection produces no surviving triangles.
            draws.push(Draw{owner:p.owner,coverage:p.coverage,key:"solid:white".into(),range:0..0,clip:None,blend:Blend::Normal,capture_after:false,filter:None});
        }
        let mut tint = geometry::tint(*tint);
        tint[3] *= f32::from(p.opacity) / 255.;
        for part in &frame.parts {
            let key = if let Some(asset) = &part.texture {
                let key = Self::sampled_image_key_filter(
                    assets,
                    p.owner,
                    asset,
                    p.sampling,
                    part.linear,
                )?;
                if !self.ensure_sampled_image_filter(
                    assets,
                    p.owner,
                    asset,
                    &key,
                    p.sampling,
                    part.linear,
                )? {
                    continue;
                }
                key
            } else {
                "solid:white".into()
            };
            let mut filter = p.filter.clone().unwrap_or_default();
            for axis in 0..3 {
                filter.add[axis] = (filter.add[axis] + part.add[axis]).clamp(-1., 1.);
            }
            let filter_index = records.len() as f32;
            let mut record = filter_gpu(&filter);
            let mut sampling=p.sampling;
            sampling.linear=part.linear;
            filter_sampling(&mut record,sampling);
            stored_blend::configure(&mut record, part.blend.unwrap_or(p.blend),self.blend_origin,p.opacity);
            coverage::configure(&mut record,coverage_active);
            records.push(record);
            let start = vertices.len() as u32;
            for index in &part.indices {
                vertices.push(clip_sampling::mesh_vertex(
                    p,
                    canvas,
                    frame.size,
                    &part.vertices[*index as usize],
                    tint,
                    filter_index,
                ));
            }
            let blend=part.blend.unwrap_or(p.blend);
            let end=vertices.len() as u32;
            // A destination-reading blend must see prior overlapping faces,
            // including faces batched into one part. Preserve raster order by
            // taking a fresh destination before each triangle. Ordinary fixed
            // function blends retain their single draw and hardware ordering.
            let step=if matches!(blend,Blend::StoredByte{..}) {3}else{(end-start).max(1)};
            for first in (start..end).step_by(step as usize) {
                draws.push(Draw {
                    owner:p.owner,coverage:p.coverage,
                    key:key.clone(),range:first..(first+step).min(end),clip:p.clip,blend,
                    capture_after:false,filter:filter_keys.clone(),
                });
            }
        }
        Ok(())
    }
}
