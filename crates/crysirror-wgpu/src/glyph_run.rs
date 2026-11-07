// SPDX-License-Identifier: MPL-2.0
//! GPU extraction of retained positioned glyphs. Each glyph is a resource and
//! a quad; no source engine or whole-run framebuffer participates.
use super::Renderer;
use anyhow::{Context, Result, ensure};
use crysirror_runtime::assets::Assets;
use crysirror_scene::{
    Primitive,
    layer::{Paint, PositionedGlyph},
};
use crysirror_text::glyph::GlyphRaster;

pub(super) struct GlyphBitmap {
    pub origin: [i32; 2],
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
fn phase(baseline: [f32; 2]) -> [f32; 2] {
    baseline.map(|v| {
        let p = v - v.floor();
        if p == 0. { 0. } else { p }
    })
}
pub(super) fn key(assets: &Assets, owner: u32, paint: &Paint) -> Result<String> {
    let text = assets
        .text
        .for_client(crysirror_core::state::ClientId(owner));
    let Paint::GlyphRun {
        font,
        size,
        bold,
        glyphs,
        blur,
        outline,
        ..
    } = paint
    else {
        anyhow::bail!("expected extracted glyph paint");
    };
    ensure!(glyphs.len() == 1, "glyph draw must have one glyph");
    let g = &glyphs[0];
    let font = g.font.as_deref().unwrap_or(font);
    let request = GlyphRaster {
        glyph_id: text.glyph_id(font, g.character)?,
        size: *size,
        baseline: phase(g.baseline),
    };
    Ok(format!(
        "{}:{}:raster={:?}",
        text.glyph_cell_key(font, request)?,
        style_key(
            *bold,
            blur,
            outline,
            horizontal_scale(&text, font, g, *size)?
        ),
        g.style.map_or(Default::default(),|s|s.raster)
    ))
}
fn horizontal_scale(
    text: &crysirror_text::TextSystem,
    font: &str,
    glyph: &PositionedGlyph,
    size: f32,
) -> Result<f32> {
    let width = glyph
        .font_width
        .map(|width| text.font_width_scale(font, glyph.size.unwrap_or(size), width))
        .transpose()?
        .unwrap_or(1.);
    let scale = width * glyph.horizontal_scale.unwrap_or(1.);
    ensure!(
        scale.is_finite() && (0.03125..=32.).contains(&scale),
        "combined glyph horizontal scale"
    );
    Ok(scale)
}
fn style_key(
    bold: bool,
    blur: &Option<crysirror_scene::layer::GlyphBlur>,
    outline: &Option<crysirror_scene::layer::GlyphOutline>,
    scale: f32,
) -> String {
    format!(
        "bold={bold}:x={:08x}{}:outline={:?}",
        scale.to_bits(),
        blur_key(blur),
        outline.map(|o| (o.radius, o.kernel))
    )
}
fn blur_key(blur: &Option<crysirror_scene::layer::GlyphBlur>) -> String {
    blur.map_or(String::new(), |b| {
        format!(":coverage={:?},{},{}", b.kernel, b.radius, b.level)
    })
}
pub(super) fn cache_keys(assets: &Assets, owner: u32, paint: &Paint, needed: &mut std::collections::BTreeSet<String>) -> Result<()> {
            let text = assets
                .text
                .for_client(crysirror_core::state::ClientId(owner));
            if let Paint::GlyphRun {
                font,
                size,
                bold,
                glyphs,
                blur,
                outline,
                ..
            } = paint
            {
                paint.validate()?;
                let visible = paint.glyph_reveal_status().expect("glyph run").visible;
                for g in glyphs.iter().take(visible) {
                    if g.crop.is_some_and(|r| r[2] == 0. || r[3] == 0.) {
                        continue;
                    }
                    if let Some(image) = &g.image {
                        needed.insert(super::Renderer::image_key(assets, owner, &image.asset)?);
                        needed.insert(super::Renderer::sampled_image_key(
                            assets,
                            owner,
                            &image.asset,
                            Default::default(),
                        )?);
                        continue;
                    }
                    let font = g.font.as_deref().unwrap_or(font);
                    let request = GlyphRaster {
                        glyph_id: text.glyph_id(font, g.character)?,
                        size: g.size.unwrap_or(*size),
                        baseline: phase(g.baseline),
                    };
                    let scale = horizontal_scale(&text, font, g, *size)?;
                    let outline=g.style.map_or(*outline,|s|s.outline);
                    for pass in outline
                        .filter(|o| o.radius > 0)
                        .map(Some)
                        .into_iter()
                        .chain(std::iter::once(None))
                    {
                        needed.insert(format!(
                            "{}:{}:raster={:?}",
                            text.glyph_cell_key(font, request)?,
                            style_key(g.bold.unwrap_or(g.style.map_or(*bold,|s|s.bold)), blur, &pass, scale),
                            g.style.map_or(Default::default(),|s|s.raster)
                        ));
                    }
                }
            }
    Ok(())
}
impl Renderer {
    pub(super) fn expand_glyph_runs(
        &mut self,
        frame: Vec<Primitive>,
        assets: &Assets,
    ) -> Result<Vec<Primitive>> {
        // Drop glyphs unused by this frame before budgeting fresh resources;
        // changing a complete paragraph must not count both paragraphs forever.
        let mut needed = std::collections::BTreeSet::new();
        for p in &frame { cache_keys(assets, p.owner, &p.paint, &mut needed)?; }
        self.glyph_cells.retain(|key, _| needed.contains(key));
        let mut expanded = Vec::new();
        let mut count = 0usize;
        let mut bytes: usize = self.glyph_cells.values().map(|c| c.rgba.len()).sum();
        for p in frame {
            let text = assets
                .text
                .for_client(crysirror_core::state::ClientId(p.owner));
            let Paint::GlyphRun {
                font,
                size,
                color,
                bold,
                glyphs,
                blur,
                outline,
                ..
            } = &p.paint
            else {
                expanded.push(p);
                continue;
            };
            p.paint.validate()?;
            ensure!(
                p.uv == [0., 0., 1., 1.],
                "glyph runs do not take image UV crops"
            );
            let visible = p.paint.glyph_reveal_status().expect("glyph run").visible;
            if p.coverage==crysirror_scene::layer::coverage::Operation::Write {
                //Even an empty imported bitmap admits its ordered coverage
                //scope. Preserve a zero-alpha marker before expanding glyphs.
                let mut marker=p.clone();marker.paint=Paint::Solid{color:[0;4]};
                marker.opacity=0;
                expanded.push(marker);
            }
            for g in glyphs.iter().take(visible) {
                if g.crop.is_some_and(|r| r[2] == 0. || r[3] == 0.) {
                    continue;
                }
                if let Some(image) = &g.image {
                    if image.rect[2] <= 0. || image.rect[3] <= 0. {
                        continue;
                    }
                    let Some(crop) =
                        crysirror_scene::layer::crop::crop_rect(image.rect, g.crop, image.uv)?
                    else {
                        continue;
                    };
                    count = count.checked_add(1).context("glyph frame count overflow")?;
                    ensure!(count <= 196_608, "positioned glyph face/outline frame budget");
                    let mut draw = p.clone();
                    draw.quad = local_quad(&p, crop.rect)?;
                    draw.uv = crop.uv;
                    draw.size = [
                        (crop.rect[2].ceil() as u32).clamp(1, 16384),
                        (crop.rect[3].ceil() as u32).clamp(1, 16384),
                    ];
                    draw.extent = [crop.rect[2].max(1e-3), crop.rect[3].max(1e-3)];
                    draw.sampling = Default::default();
                    draw.paint = Paint::Image {
                        asset: image.asset.clone(),
                        tint: image.tint,
                    };
                    expanded.push(draw);
                    continue;
                }
                let font = g.font.as_ref().unwrap_or(font);
                let size = g.size.unwrap_or(*size);
                let fill_color = g.color.unwrap_or(*color);
                let bold = g.bold.unwrap_or(g.style.map_or(*bold,|s|s.bold));
                let raster=g.style.map_or(Default::default(),|s|s.raster);
                let outline=g.style.map_or(*outline,|s|s.outline);
                let shadow=g.style.and_then(|s|s.shadow);
                let horizontal_scale = horizontal_scale(&text, font, g, size)?;
                let center = g
                    .cell_width
                    .map(|width| {
                        let metrics =
                            text.glyph_metrics(font, text.glyph_id(font, g.character)?, size)?;
                        Ok::<f32, anyhow::Error>(
                            ((f32::from(width) - (metrics.advance * horizontal_scale).round())
                                / 2.)
                                .trunc(),
                        )
                    })
                    .transpose()?
                    .unwrap_or(0.);
                for (outline_pass,pass_color,pass_offset) in shadow
                    .map(|s|(None,s.color,s.offset)).into_iter()
                    .chain(outline.filter(|o|o.radius>0).map(|o|(Some(o),o.color,[0.;2])))
                    .chain(std::iter::once((None,fill_color,[0.;2])))
                {
                    count = count.checked_add(1).context("glyph frame count overflow")?;
                    ensure!(
                        count <= 196_608,
                        "positioned glyph face/outline frame budget"
                    );
                    let color = pass_color;
                    let request = GlyphRaster {
                        glyph_id: text.glyph_id(font, g.character)?,
                        size,
                        baseline: phase(g.baseline),
                    };
                    let paint = Paint::GlyphRun {
                        outline: outline_pass,
                        font: font.clone(),
                        size,
                        color,
                        bold,
                        glyphs: vec![PositionedGlyph {
                image: None,
                            style: Some(crysirror_scene::layer::PositionedGlyphStyle{raster,..Default::default()}), font: None,
                            horizontal_scale: Some(horizontal_scale),
                            font_width: None,
                            cell_width: None,
                            crop: None,
                            character: g.character,
                            baseline: request.baseline,
                            size: None,
                            color: None,
                            bold: None,
                        }],
                        reveal: None,
                        blur: *blur,
                    };
                    let key = key(assets, p.owner, &paint)?;
                    if !self.glyph_cells.contains_key(&key) {
                        let cell =
                            text.rasterize_glyph_cell_quality(font, request, horizontal_scale, raster)?;
                        let width = cell.width + u32::from(bold && cell.width > 0);
                        let length = width as usize * cell.height as usize * 4;
                        ensure!(
                            bytes
                                .checked_add(length)
                                .is_some_and(|n| n <= 64 * 1024 * 1024),
                            "positioned glyph resource budget exceeds 64 MiB"
                        );
                        bytes += length;
                        let mut rgba = vec![0u8; length];
                        for y in 0..cell.height as usize {
                            for x in 0..cell.width as usize {
                                let alpha = (cell.coverage[y * cell.width as usize + x]
                                    .clamp(0., 1.)
                                    * 255.)
                                    .round() as u8;
                                let at = (y * width as usize + x) * 4;
                                rgba[at..at + 4].copy_from_slice(&[255, 255, 255, alpha]);
                            }
                            if bold && width > 0 {
                                for x in (1..width as usize).rev() {
                                    let at = (y * width as usize + x) * 4;
                                    rgba[at..at + 3].fill(255);
                                    rgba[at + 3] = rgba[at + 3].max(rgba[at - 1]);
                                }
                            }
                        }
                        let mut bitmap = GlyphBitmap {
                            origin: cell.origin,
                            width,
                            height: cell.height,
                            rgba,
                        };
                        if let Some(b) = blur {
                            let coverage: Vec<u8> =
                                bitmap.rgba.chunks_exact(4).map(|p| p[3]).collect();
                            use crysirror_scene::layer::GlyphCoverageKernel;
                            let (out, w, h, origin) = match b.kernel {
                                GlyphCoverageKernel::WeightedBlur => {
                                    let (out, w, h) = crysirror_text::glyph_blur::blur_coverage(
                                        &coverage,
                                        bitmap.width as usize,
                                        bitmap.height as usize,
                                        u32::from(b.radius),
                                        i32::from(b.level),
                                    );
                                    (out, w, h, [-i32::from(b.radius); 2])
                                }
                                GlyphCoverageKernel::ObliqueBoxSum {
                                    numerator,
                                    denominator,
                                    radius_x,
                                    radius_y,
                                } => {
                                    let cell = crysirror_text::glyph_coverage::oblique_box_sum(
                                        &coverage,
                                        bitmap.width as usize,
                                        bitmap.height as usize,
                                        numerator,
                                        denominator,
                                        radius_x,
                                        radius_y,
                                    )?;
                                    (cell.pixels, cell.size[0], cell.size[1], cell.origin)
                                }
                                GlyphCoverageKernel::BoxSumRect { radius_x, radius_y } => {
                                    let cell = crysirror_text::glyph_coverage::box_sum_rect(
                                        &coverage,
                                        bitmap.width as usize,
                                        bitmap.height as usize,
                                        radius_x,
                                        radius_y,
                                    )?;
                                    (cell.pixels, cell.size[0], cell.size[1], cell.origin)
                                }
                                mode => {
                                    let apply = match mode {
                                        GlyphCoverageKernel::DiskOutline => {
                                            crysirror_text::glyph_coverage::disk_outline
                                        }
                                        GlyphCoverageKernel::SquareFace => {
                                            crysirror_text::glyph_coverage::square_face
                                        }
                                        GlyphCoverageKernel::BoxSum => {
                                            crysirror_text::glyph_coverage::box_sum
                                        }
                                        _ => unreachable!(),
                                    };
                                    let cell = apply(
                                        &coverage,
                                        bitmap.width as usize,
                                        bitmap.height as usize,
                                        b.radius,
                                    )?;
                                    (cell.pixels, cell.size[0], cell.size[1], cell.origin)
                                }
                            };
                            bytes += (w * h * 4).saturating_sub(bitmap.rgba.len());
                            ensure!(
                                bytes <= 64 * 1024 * 1024,
                                "positioned glyph resource budget exceeds 64 MiB"
                            );
                            bitmap = GlyphBitmap {
                                origin: [
                                    bitmap.origin[0] + origin[0],
                                    bitmap.origin[1] + origin[1],
                                ],
                                width: w as u32,
                                height: h as u32,
                                rgba: out.iter().flat_map(|&a| [255, 255, 255, a]).collect(),
                            };
                        }
                        if let Some(outline) = outline_pass {
                            let coverage: Vec<u8> =
                                bitmap.rgba.chunks_exact(4).map(|p| p[3]).collect();
                            let apply = match outline.kernel {
                                crysirror_scene::layer::GlyphOutlineKernel::Disk => {
                                    crysirror_text::glyph_coverage::disk_outline
                                }
                                crysirror_scene::layer::GlyphOutlineKernel::Square => {
                                    crysirror_text::glyph_coverage::square_outline
                                }
                            };
                            let edge = apply(
                                &coverage,
                                bitmap.width as usize,
                                bitmap.height as usize,
                                outline.radius,
                            )?;
                            bytes +=
                                (edge.size[0] * edge.size[1] * 4).saturating_sub(bitmap.rgba.len());
                            ensure!(
                                bytes <= 64 * 1024 * 1024,
                                "positioned glyph outline resource budget exceeds64MiB"
                            );
                            bitmap = GlyphBitmap {
                                origin: [
                                    bitmap.origin[0] + edge.origin[0],
                                    bitmap.origin[1] + edge.origin[1],
                                ],
                                width: edge.size[0] as u32,
                                height: edge.size[1] as u32,
                                rgba: edge
                                    .pixels
                                    .iter()
                                    .flat_map(|alpha| [255, 255, 255, *alpha])
                                    .collect(),
                            };
                        }
                        self.glyph_cells.insert(key.clone(), bitmap);
                    }
                    let cell = &self.glyph_cells[&key];
                    if cell.width == 0 || cell.height == 0 {
                        continue;
                    }
                    let origin = [
                        g.baseline[0].floor() + center + cell.origin[0] as f32 + pass_offset[0],
                        g.baseline[1].floor() + cell.origin[1] as f32 + pass_offset[1],
                    ];
                    let mut draw = p.clone();
                    let Some(crop) = crysirror_scene::layer::crop::crop_rect(
                        [origin[0], origin[1], cell.width as f32, cell.height as f32],
                        g.crop,
                        [0., 0., 1., 1.],
                    )?
                    else {
                        continue;
                    };
                    draw.quad = local_quad(&p, crop.rect)?;
                    draw.uv = crop.uv;
                    draw.size = [cell.width, cell.height];
                    draw.extent = [crop.rect[2], crop.rect[3]];
                    draw.paint = paint;
                    expanded.push(draw);
                }
            }
        }
        Ok(expanded)
    }
}

/// Subrect extraction interpolates homogeneous positions, preserving parent
/// scale/rotation/projected camera transforms rather than treating text as a
/// screen-space overlay. Primitive stores divided xyz plus homogeneous w.
fn local_quad(p: &Primitive, rect: [f32; 4]) -> Result<[[f32; 4]; 4]> {
    crysirror_scene::layer::crop::subrect_quad(p.quad, p.extent, rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gpu;
    use crysirror_core::state::{ClientId, SceneState};
    use crysirror_scene::layer::{
        GlyphReveal, GlyphRevealUnit, LayerCommand, LayerSpec, LayerSystem, rectangle,
    };
    use serde_json::json;
    use std::sync::Arc;
    #[test]
    fn retained_glyph_style_keys_distinguish_coverage_but_share_border_tints() {
        use crysirror_scene::layer::{GlyphOutline, GlyphOutlineKernel};
        let black = GlyphOutline {
            radius: 1,
            color: [0, 0, 0, 255],
            kernel: GlyphOutlineKernel::Square,
        };
        let red = GlyphOutline {
            color: [255, 0, 0, 255],
            ..black
        };
        assert_eq!(
            style_key(false, &None, &Some(black), 1.),
            style_key(false, &None, &Some(red), 1.)
        );
        assert_ne!(
            style_key(false, &None, &Some(black), 1.),
            style_key(false, &None, &None, 1.)
        );
        assert_ne!(
            style_key(false, &None, &None, 1.),
            style_key(false, &None, &None, 2.)
        );
        assert_ne!(
            style_key(false, &None, &Some(black), 1.),
            style_key(
                false,
                &None,
                &Some(GlyphOutline {
                    kernel: GlyphOutlineKernel::Disk,
                    ..black
                }),
                1.
            )
        );
    }
    #[test]
    #[ignore = "requires offscreen GPU and caller-selected CRYSIRROR_TEST_FONT; no window or bundled font"]
    fn outline_and_requested_font_width_render_as_independent_shared_coverage() {
        let font = std::env::var_os("CRYSIRROR_TEST_FONT").expect("supply a local outline font");
        let root = tempfile::tempdir().unwrap();
        let mut assets = Assets::new(root.path()).unwrap();
        assets
            .text
            .add_font("test", std::fs::read(font).unwrap())
            .unwrap();
        let mut layer:LayerSpec=serde_json::from_value(json!({"id":1,"rect":[0,0,240,80],"paint":{"kind":"glyph_run","font":"test","size":22,"color":[255,0,0,255],"outline":{"radius":1,"color":[0,255,0,255],"kernel":"square"},"glyphs":[{"character":"A","baseline":[20,50],"font_width":44}]}})).unwrap();
        let mut layers = LayerSystem::default();
        layers.install(ClientId(1), &[layer.clone()]).unwrap();
        let instance = wgpu::Instance::default();
        let gpu = Arc::new(pollster::block_on(Gpu::request(&instance, None)).unwrap());
        let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
        renderer
            .set_engine_frame(layers.frame([240, 80]).unwrap())
            .unwrap();
        let (_, draws) = renderer
            .prepare(&SceneState::default(), &mut assets, [240, 80])
            .unwrap();
        assert_eq!(draws.len(), 2);
        assert_eq!(renderer.glyph_cells.len(), 2);
        let keys = renderer.glyph_cells.keys().cloned().collect::<Vec<_>>();
        let pixels = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [240, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(pixels.pixels().any(|p| p.0[0] > 0 && p.0[1] == 0));
        assert!(pixels.pixels().any(|p| p.0[1] > 0 && p.0[0] == 0));
        if let Paint::GlyphRun { outline, .. } = &mut layer.paint {
            outline.as_mut().unwrap().color = [0, 0, 255, 255];
        }
        layers
            .apply_batch(ClientId(1), &[LayerCommand::Put { layer }])
            .unwrap();
        renderer
            .set_engine_frame(layers.frame([240, 80]).unwrap())
            .unwrap();
        renderer
            .prepare(&SceneState::default(), &mut assets, [240, 80])
            .unwrap();
        assert_eq!(
            renderer.glyph_cells.keys().cloned().collect::<Vec<_>>(),
            keys,
            "outline tint never duplicates font coverage resources"
        );
    }

    #[test]
    fn glyph_subrects_preserve_homogeneous_and_fractional_local_coordinates() {
        let mut p = rectangle(
            1,
            1,
            0,
            [0., 0., 100., 100.],
            Paint::Solid { color: [255; 4] },
            None,
            [0, 0],
        );
        p.quad = [
            [10., 20., 0., 2.],
            [30., 20., 0., 1.],
            [30., 60., 0., 1.],
            [10., 60., 0., 2.],
        ];
        let q = local_quad(&p, [50., 50., 25., 25.]).unwrap();
        assert!((q[0][0] - 25. / 1.5).abs() < 1e-5);
        assert!((q[0][1] - 40.).abs() < 1e-5);
        assert!((q[0][3] - 1.5).abs() < 1e-5);
        let p = rectangle(
            1,
            1,
            0,
            [10., 20., 100.5, 50.5],
            Paint::Solid { color: [255; 4] },
            None,
            [0, 0],
        );
        let q = local_quad(&p, [10.25, 5.25, 20., 10.]).unwrap();
        assert!((q[0][0] - 20.25).abs() < 1e-5);
        assert!((q[0][1] - 25.25).abs() < 1e-5);
    }

    #[test]
    #[ignore = "requires a real GPU and CRYSIRROR_TEST_FONT pointing to a runtime system font; no font is bundled"]
    fn retained_glyphs_are_individual_cached_draws_with_shared_state_and_clip() {
        let font =
            std::env::var_os("CRYSIRROR_TEST_FONT").expect("supply a local system font path");
        let root = tempfile::tempdir().unwrap();
        let mut assets = Assets::new(root.path()).unwrap();
        assets
            .text
            .add_font("test", std::fs::read(font).unwrap())
            .unwrap();
        let spec: LayerSpec = serde_json::from_value(json!({"id":1,"rect":[0.,0.,140.,80.],
            "clip":[0.,0.,90.,80.],"paint":{"kind":"glyph_run","font":"test","size":36.,
            "color":[255,0,0,255],"glyphs":[{"character":"A","baseline":[5.,45.]},
                {"character":"B","baseline":[75.,45.],"color":[0,255,0,255],"bold":true}]}}))
        .unwrap();
        let mut layers = LayerSystem::default();
        layers.install(ClientId(1), &[spec.clone()]).unwrap();
        let instance = wgpu::Instance::default();
        let gpu = Arc::new(pollster::block_on(Gpu::request(&instance, None)).unwrap());
        let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let (vertices, draws) = renderer
            .prepare(&SceneState::default(), &mut assets, [140, 80])
            .unwrap();
        assert_eq!(draws.len(), 2, "two glyphs must not become one run texture");
        assert_eq!(vertices.len(), 12);
        assert_eq!(renderer.glyph_cells.len(), 2);
        assert!(draws.iter().all(|d| d.key.starts_with("glyph:")));
        let cached: Vec<_> = renderer.glyph_cells.keys().cloned().collect();
        let pixels = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [140, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(
            pixels
                .enumerate_pixels()
                .any(|(x, _, p)| x < 60 && p.0[0] > 0 && p.0[1] == 0)
        );
        assert!(
            pixels
                .enumerate_pixels()
                .any(|(x, _, p)| (75..90).contains(&x) && p.0[1] > 0 && p.0[0] == 0)
        );
        assert!(
            pixels
                .enumerate_pixels()
                .filter(|(x, _, _)| *x >= 90)
                .all(|(_, _, p)| p.0 == [0, 0, 0, 255]),
            "retained clip must apply to every glyph"
        );
        if let Some(path) = std::env::var_os("CRYSIRROR_GLYPH_CAPTURE") {
            pixels.save(path).unwrap();
        }
        let mut cropped = spec.clone();
        cropped.source_crop = Some([13., 0., 12., 80.]);
        layers
            .apply_batch(ClientId(1), &[LayerCommand::Put { layer: cropped }])
            .unwrap();
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let cropped_pixels = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [140, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        for (x, y, pixel) in cropped_pixels.enumerate_pixels() {
            let expected = if (13..25).contains(&x) {
                pixels.get_pixel(x, y).0
            } else {
                [0, 0, 0, 255]
            };
            assert_eq!(pixel.0, expected, "source crop at {x},{y}");
        }
        assert!(
            renderer.glyph_cells.keys().all(|key| cached.contains(key)),
            "cropping must reuse the original immutable coverage keys; fully hidden glyphs may be retired"
        );
        layers
            .apply_batch(
                ClientId(1),
                &[LayerCommand::Put {
                    layer: spec.clone(),
                }],
            )
            .unwrap();
        let mut moved = layers.frame([140, 80]).unwrap();
        for point in &mut moved[0].quad {
            point[0] += 10.;
        }
        renderer.set_engine_frame(moved).unwrap();
        let (_, draws) = renderer
            .prepare(&SceneState::default(), &mut assets, [140, 80])
            .unwrap();
        assert_eq!(draws.len(), 2);
        assert_eq!(
            renderer.glyph_cells.keys().cloned().collect::<Vec<_>>(),
            cached,
            "integer translation reuses glyph resources"
        );
        let mut delayed = spec;
        if let Paint::GlyphRun { reveal, .. } = &mut delayed.paint {
            *reveal = Some(GlyphReveal {
                unit: GlyphRevealUnit::UnicodeScalar,
                interval_ms: 20,
                rate_per_second: None,
                initial_visible: 0,
                prefix_steps: None,
                elapsed_ms: 0,
                forced_complete: false,
                paused: false,
            });
        }
        layers
            .apply_batch(ClientId(1), &[LayerCommand::Put { layer: delayed }])
            .unwrap();
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let (_, draws) = renderer
            .prepare(&SceneState::default(), &mut assets, [140, 80])
            .unwrap();
        assert!(
            draws.is_empty(),
            "unrevealed glyphs must not create GPU draws"
        );
        layers.tick(20);
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let first = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [140, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(first.pixels().any(|p| p.0[0] > 0));
        assert!(
            first.pixels().all(|p| p.0[1] == 0),
            "the second green glyph is not visible yet"
        );
        layers
            .apply_batch(ClientId(1), &[LayerCommand::CompleteReveal { id: 1 }])
            .unwrap();
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let (_, draws) = renderer
            .prepare(&SceneState::default(), &mut assets, [140, 80])
            .unwrap();
        assert_eq!(
            draws.len(),
            2,
            "completion must show all glyph draws without a VM timer"
        );
        layers.set_visible(ClientId(1), 1, false).unwrap();
        renderer
            .set_engine_frame(layers.frame([140, 80]).unwrap())
            .unwrap();
        let blank = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [140, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(blank.pixels().all(|p| p.0 == [0, 0, 0, 255]));
        assert!(renderer.glyph_cells.is_empty());
    }

    #[test]
    #[ignore = "requires GPU and CRYSIRROR_TEST_FONT; reads runtime font only"]
    fn shared_face_outline_keeps_individual_resources_and_reveal() {
        let font = std::env::var_os("CRYSIRROR_TEST_FONT").expect("system font path");
        let root = tempfile::tempdir().unwrap();
        let mut assets = Assets::new(root.path()).unwrap();
        assets
            .text
            .add_font("test", std::fs::read(font).unwrap())
            .unwrap();
        let body: LayerSpec =
            serde_json::from_value(json!({"id":1,"rect":[0,0,160,80],"order":[0,2],
            "paint":{"kind":"glyph_run","font":"test","size":40,"color":[255,0,0,255],
            "glyphs":[{"character":"A","baseline":[20,55]},{"character":"B","baseline":[90,55]}],
            "reveal":{"interval_ms":20,"initial_visible":0,"elapsed_ms":0}}}))
            .unwrap();
        let mut edge = body.clone();
        edge.id = 2;
        edge.order = [0, 1];
        edge.rect[0] -= 1.;
        edge.rect[1] -= 1.;
        if let Paint::GlyphRun { color, blur, .. } = &mut edge.paint {
            *color = [0, 255, 0, 255];
            *blur = Some(crysirror_scene::layer::GlyphBlur {
                radius: 2,
                level: 256,
                kernel: crysirror_scene::layer::GlyphCoverageKernel::SquareFace,
            });
        }
        let mut layers = LayerSystem::default();
        layers.install(ClientId(1), &[body, edge]).unwrap();
        let instance = wgpu::Instance::default();
        let gpu = Arc::new(pollster::block_on(Gpu::request(&instance, None)).unwrap());
        let mut renderer = Renderer::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb);
        renderer
            .set_engine_frame(layers.frame([160, 80]).unwrap())
            .unwrap();
        let hidden = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [160, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(hidden.pixels().all(|p| p.0 == [0, 0, 0, 255]));
        layers.tick(20);
        renderer
            .set_engine_frame(layers.frame([160, 80]).unwrap())
            .unwrap();
        let first = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [160, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(first.pixels().any(|p| p.0[0] > 0 && p.0[1] == 0), "body");
        assert!(
            first.pixels().any(|p| p.0[1] > 0 && p.0[0] == 0),
            "engine-owned outline"
        );
        assert!(
            first
                .enumerate_pixels()
                .filter(|(x, _, _)| *x >= 80)
                .all(|(_, _, p)| p.0 == [0, 0, 0, 255]),
            "second glyph remains hidden in both passes"
        );
        assert_eq!(
            renderer.glyph_cells.len(),
            2,
            "one glyph, separate body and face kernels"
        );
        layers.tick(20);
        renderer
            .set_engine_frame(layers.frame([160, 80]).unwrap())
            .unwrap();
        let full = renderer
            .capture(
                &SceneState::default(),
                &mut assets,
                [160, 80],
                [0, 0, 0, 255],
            )
            .unwrap();
        assert!(
            full.enumerate_pixels()
                .any(|(x, _, p)| x > 80 && p.0[0] > 0)
        );
        assert!(
            full.enumerate_pixels()
                .any(|(x, _, p)| x > 80 && p.0[1] > 0)
        );
        assert_eq!(
            renderer.glyph_cells.len(),
            4,
            "two independently cached decorated glyphs"
        );
        if let Some(path) = std::env::var_os("CRYSIRROR_GLYPH_CAPTURE") {
            full.save(path).unwrap();
        }
    }
}
