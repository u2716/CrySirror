// SPDX-License-Identifier: MPL-2.0
//! Retire old GPU cache generations before isolated-render budget admission.
//! Cache residency follows declared dependencies, not the most recent upload.
use super::*;

impl Renderer {
    pub(super) fn scene_cache_keys(assets: &Assets, scene: &SceneState) -> Result<BTreeSet<String>> {
        let mut keys = BTreeSet::new();
        for (id, node) in scene.draw_order() {
            match &node.visual {
                Visual::Sprite(sprite) => {
                    keys.insert(Self::image_key(assets, id.client.0, sprite.asset.relative())?);
                }
                Visual::Text(text) => {
                    let mut local = text.clone();
                    local.bounds.x = 0;
                    local.bounds.y = 0;
                    keys.insert(assets.text.for_client(id.client).key(&local)?);
                }
            }
        }
        Ok(keys)
    }

    pub(super) fn frame_cache_keys(assets: &Assets, frame: &[Primitive]) -> Result<BTreeSet<String>> {
        fn collect(assets: &Assets, frame: &[Primitive], keys: &mut BTreeSet<String>) -> Result<()> {
            for p in frame {
                match &p.paint {
                    Paint::Solid { .. } => { keys.insert("solid:white".into()); }
                    Paint::Backdrop { .. } => { keys.insert(BACKDROP_KEY.into()); }
                    Paint::Image { asset, .. } => {
                        keys.insert(Renderer::image_key(assets, p.owner, asset)?);
                        keys.insert(Renderer::sampled_image_key(assets, p.owner, asset, p.sampling)?);
                    }
                    Paint::Text { text, font, size, color, layout } => {
                        keys.insert(assets.text.for_client(ClientId(p.owner)).key(&TextRun {
                            text: text.clone(), font: font.clone(), size_px: *size,
                            bounds: Rect { x: 0, y: 0, width: p.size[0], height: p.size[1] },
                            color: Color(*color), layout_profile: layout.clone(),
                        })?);
                    }
                    Paint::GlyphRun { .. } => { glyph_run::cache_keys(assets, p.owner, &p.paint, keys)?; }
                    Paint::Snapshot { size, layers } => {
                        keys.insert(Renderer::snapshot_key(assets, p.owner, *size, layers)?);
                        let mut isolated = crysirror_scene::layer::LayerSystem::default();
                        isolated.install(ClientId(p.owner), layers)?;
                        collect(assets, &isolated.frame(*size)?, keys)?;
                    }
                    Paint::Puppet { .. } | Paint::Mesh { .. } => {
                        let [w,h] = Renderer::puppet_size(p);
                        keys.insert(format!("puppet:{}:{}:{w}x{h}", p.owner, p.id));
                    }
                    Paint::ProjectedMesh { frame, .. } => {
                        keys.insert("solid:white".into());
                        keys.insert("curve:identity".into());
                        for part in &frame.parts {
                            if let Some(asset) = &part.texture {
                                keys.insert(Renderer::image_key(assets,p.owner,asset)?);
                                keys.insert(Renderer::sampled_image_key_filter(assets,p.owner,asset,p.sampling,part.linear)?);
                            }
                        }
                    }
                    Paint::LegacyMesh { .. } => anyhow::bail!("legacy mesh must be evaluated by shared layer extraction"),
                    Paint::CoverageWrite{..}|Paint::CoverageReset{..}|Paint::GlyphView{..}|Paint::Group { .. } | Paint::OrderedState { .. } => anyhow::bail!("composition command must be consumed by shared layer extraction"),
                }
                if let Some(mesh) = &p.puppet {
                    for asset in &mesh.textures {
                        keys.insert(match crysirror_core::service::frame_history_layer(asset) {
                            Some(id) => format!("history:{}:{id}", p.owner),
                            None if asset == BACKDROP_KEY => BACKDROP_KEY.into(),
                            None => Renderer::image_key(assets, p.owner, asset)?,
                        });
                    }
                }
                if let Some(filter) = &p.filter {
                    keys.insert(match &filter.mask { Some(mask) => Renderer::image_key(assets, p.owner, &mask.asset)?, None => "solid:white".into() });
                    keys.insert(match &filter.tone_curve { Some(curve) => Renderer::image_key(assets, p.owner, &curve.asset)?, None => "curve:identity".into() });
                }
                if let Some(composite) = &p.composite {
                    keys.insert(Renderer::image_key(assets, p.owner, &format!("__crysirror_composite/{}",p.id))?);
                    if let Some((rule,_)) = &composite.rule {
                        keys.insert(Renderer::image_key(assets, p.owner, &rule.asset)?);
                    }
                    collect(assets, &composite.before, keys)?;
                    collect(assets, &composite.after, keys)?;
                }
            }
            Ok(())
        }
        let mut keys = BTreeSet::new();
        collect(assets, frame, &mut keys)?;
        Ok(keys)
    }

    pub(super) fn preserve_capture_textures(&mut self, used: &mut BTreeSet<String>) {
        used.extend(self.capture_texture_keys.iter().cloned());
        used.extend(self.snapshot_readbacks.iter().cloned());
        self.snapshot_readers.retain(|key, readers| {
            readers.retain(|reader| reader.strong_count() != 0);
            if readers.is_empty() { false } else { used.insert(key.clone()); true }
        });
    }

    pub(super) fn retire_unused_textures(&mut self, assets: &Assets) -> Result<()> {
        let mut used = Self::frame_cache_keys(assets, &self.engine_frame)?;
        used.extend(self.scene_texture_keys.iter().cloned());
        self.preserve_capture_textures(&mut used);
        self.textures.retain(|key,_| used.contains(key));
        Ok(())
    }
}
