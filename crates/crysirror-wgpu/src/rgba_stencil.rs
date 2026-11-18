// SPDX-License-Identifier: MPL-2.0
//! Independent RGBA matte coverage without framebuffer copies or dual-source
//! blending. Attenuation and contribution stay adjacent in one ordered pass.
use super::*;

impl Renderer {
    pub(super) fn prepare_rgba_stencil(
        &self,
        p: &Primitive,
        canvas: [u32; 2],
        color: [u8; 4],
        key: String,
        filter: Option<(String, String)>,
        output: Option<(String, [u32; 2], u64)>,
        consume: bool,
        records: &mut Vec<FilterGpu>,
        vertices: &mut Vec<clip_sampling::LayerVertex>,
        draws: &mut Vec<Draw>,
        assets: &Assets,
    ) -> Result<()> {
        ensure!(filter.is_some(), "component matte sampler binding missing");
        let mode = match p.blend {
            Blend::Normal => 0,
            Blend::Add | Blend::Subtract => 1u32 << 22,
            Blend::Multiply => 1u32 << 21,
            Blend::Screen => 1u32 << 23,
            _ => anyhow::bail!("unsupported component matte blend"),
        };
        let mut record = filter_gpu(
            p.filter
                .as_ref()
                .context("component matte filter missing")?,
            self.mask_viewport,
        );
        filter_sampling(&mut record, p.sampling);
        coverage::configure(&mut record, consume);
        record.c[3] = (record.c[3] as u32 | mode) as f32;
        if p.target.as_ref().is_some_and(|t| t.spec.opaque) {
            record.i[3] = (record.i[3] as u32 | 1) as f32;
        }
        let mut feedback = p
            .target
            .as_ref()
            .filter(|t| p.output.as_ref() == Some(*t))
            .map(|t| Self::target_key(assets, t));
        for destination in [true, false] {
            let mut pass_record = record;
            if destination {
                pass_record.c[3] = (pass_record.c[3] as u32 | (1u32 << 20)) as f32;
            }
            let index = records.len() as f32;
            records.push(pass_record);
            let mut quad = clip_sampling::quad(p, canvas, color);
            for vertex in &mut quad {
                vertex.filter = index;
            }
            let first = vertices.len() as u32;
            vertices.extend_from_slice(&quad);
            draws.push(Draw {
                rgba_contribution: !destination,
                output: output.clone(),
                feedback_source: feedback.take(),
                owner: p.owner,
                coverage: p.coverage,
                key: key.clone(),
                range: first..first + 6,
                clip: p.clip,
                blend: if destination {
                    Blend::Modulate
                } else if p.blend == Blend::Subtract {
                    Blend::Subtract
                } else {
                    Blend::Add
                },
                capture_after: false,
                filter: filter.clone(),
            });
        }
        Ok(())
    }
}
