//! Frozen group alpha is applied once after its children are composed.
//! This is engine-owned logical isolation; the renderer uses ordinary nested
//! immutable snapshot resources, without a VM rasterizer or CPU framebuffer.
use super::*;

pub(super) fn isolate(layers: &mut Vec<LayerSpec>) -> Result<()> {
    loop {
        let parents: BTreeSet<_> = layers.iter().filter_map(|layer| layer.parent).collect();
        let by_id: BTreeMap<_, _> = layers.iter().map(|layer| (layer.id, layer)).collect();
        let mut groups = Vec::new();
        for layer in layers
            .iter()
            .filter(|layer| layer.opacity != 255 && parents.contains(&layer.id) && !matches!(layer.paint,Paint::Group{..}))
        {
            let mut depth = 0;
            let mut parent = layer.parent;
            while let Some(id) = parent {
                depth += 1;
                ensure!(depth < 30, "snapshot isolated group depth budget");
                parent = by_id
                    .get(&id)
                    .context("snapshot isolated group parent missing")?
                    .parent;
            }
            groups.push((depth, layer.id));
        }
        let Some((_, id)) = groups.into_iter().max() else {
            return Ok(());
        };
        let root = by_id[&id].clone();
        let size = [root.rect[2].ceil() as u32, root.rect[3].ceil() as u32];
        ensure!(
            size.iter().all(|n| (1..=8192).contains(n)),
            "snapshot isolated group extent budget"
        );
        // The declared local clip bounds all child pixels. Refuse an unbounded
        // group instead of silently cropping descendants to the texture extent.
        if root.clip.is_none() {
            let selected = descendants_of(layers, id);
            // Fully transparent group: no descendant pixel can survive the
            // group alpha, so the frozen subtree is exactly empty.
            if root.opacity == 0 {
                layers.retain(|layer| layer.id == id || !selected.contains(&layer.id));
                continue;
            }
            // A non-painting container around one normal painted layer has no
            // overlap to isolate: group alpha equals that layer's own alpha.
            let painted: Vec<_> = layers
                .iter()
                .filter(|layer| layer.id != id && selected.contains(&layer.id) && paints(&layer.paint))
                .map(|layer| layer.id)
                .collect();
            let foldable = match painted[..] {
                [only] if !paints(&root.paint) && root.blend == Blend::Normal && root.filter.is_none() => by_id
                    .get(&only)
                    .is_some_and(|child| child.blend == Blend::Normal && child.filter.as_ref().is_none_or(opacity_neutral))
                    .then_some(only),
                _ => None,
            };
            if let Some(only) = foldable {
                let group_alpha = u16::from(root.opacity);
                for layer in layers.iter_mut() {
                    if layer.id == only {
                        layer.opacity = ((u16::from(layer.opacity) * group_alpha + 127) / 255) as u8;
                    } else if layer.id == id {
                        layer.opacity = 255;
                    }
                }
                continue;
            }
        }
        let clip = root
            .clip
            .with_context(|| format!("snapshot isolated group {id:#x} requires bounded local clip"))?;
        ensure!(
            clip[0] >= 0.
                && clip[1] >= 0.
                && clip[0] + clip[2] <= size[0] as f32
                && clip[1] + clip[3] <= size[1] as f32,
            "snapshot isolated group clip exceeds local extent"
        );
        let selected = descendants_of(layers, id);
        let mut input: Vec<_> = layers
            .iter()
            .filter(|layer| selected.contains(&layer.id))
            .cloned()
            .collect();
        if input.iter().any(|layer| layer.viewport_clip.is_some()) {
            let inverse = super::viewport_clip::inverse_to_root(id, &by_id)?;
            super::viewport_clip::rebase(&mut input, inverse)?;
        }
        let input_root = input.iter_mut().find(|layer| layer.id == id).unwrap();
        input_root.parent = None;
        input_root.rect[0] = 0.;
        input_root.rect[1] = 0.;
        input_root.transform = SpriteTransform::default();
        input_root.opacity = 255;
        ensure!(
            snapshot_nesting(&input) < MAX_SNAPSHOT_NESTING,
            "snapshot isolated group nesting budget"
        );
        // Keep the source paint's UV/filter inside. The outer node transforms
        // the whole isolated surface and owns the original group alpha.
        let mut output = root;
        output.paint = Paint::Snapshot {
            size,
            layers: input.into(),
        };
        output.uv = None;
        output.sampling = ImageSampling::default();
        output.filter = None;
        layers.retain(|layer| !selected.contains(&layer.id));
        layers.push(output);
    }
}

/// A filter whose only setting is encoded (gamma-space) texel interpolation
/// runs before tint/alpha, so scaling the layer's opacity commutes with it.
fn opacity_neutral(filter: &LayerFilter) -> bool {
    let mut base = filter.clone();
    base.encoded_sampling = false;
    base == LayerFilter::default()
}

/// Whether a frozen layer's own paint can produce any pixel.
fn paints(paint: &Paint) -> bool {
    match paint {
        Paint::Solid { color } => color[3] != 0,
        Paint::Group { .. } => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn specs() -> Vec<LayerSpec> {
        serde_json::from_value(serde_json::json!([
            {"id":1,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[0,0,0,0]}},
            {"id":2,"parent":1,"rect":[0,0,4,4],"clip":[0,0,4,4],"opacity":128,"paint":{"kind":"solid","color":[0,0,0,0]}},
            {"id":3,"parent":2,"rect":[0,0,3,4],"paint":{"kind":"solid","color":[255,0,0,255]}},
            {"id":4,"parent":2,"rect":[1,0,3,4],"paint":{"kind":"solid","color":[255,0,0,255]}}
        ])).unwrap()
    }
    #[test]
    fn frozen_overlap_is_composed_before_group_alpha_and_keeps_source_state() {
        let mut system = LayerSystem::default();
        system.install(ClientId(1), &specs()).unwrap();
        let frozen = system.snapshot_subtree_specs(ClientId(1), 1).unwrap();
        assert_eq!(frozen.len(), 2);
        let group = frozen.iter().find(|l| l.id == 2).unwrap();
        assert_eq!(group.opacity, 128);
        let Paint::Snapshot { size, layers } = &group.paint else {
            panic!("group must be isolated")
        };
        assert_eq!(*size, [4, 4]);
        assert_eq!(layers.iter().find(|l| l.id == 2).unwrap().opacity, 255);
        assert!(
            layers
                .iter()
                .filter(|l| l.id == 3 || l.id == 4)
                .all(|l| l.opacity == 255)
        );
        validate_snapshot([4, 4], &frozen).unwrap();
        assert_eq!(system.get(ClientId(1), 2).unwrap().opacity, 128);
        assert!(system.get(ClientId(1), 4).is_some());
    }
    #[test]
    fn unbounded_group_is_rejected_instead_of_cropping_children() {
        let mut input = specs();
        input[1].clip = None;
        let mut system = LayerSystem::default();
        system.install(ClientId(1), &input).unwrap();
        assert!(
            system
                .snapshot_subtree_specs(ClientId(1), 1)
                .unwrap_err()
                .to_string()
                .contains("bounded local clip")
        );
    }
    #[test]
    fn unbounded_transparent_group_freezes_empty() {
        let mut input = specs();
        input[1].clip = None;
        input[1].opacity = 0;
        let mut system = LayerSystem::default();
        system.install(ClientId(1), &input).unwrap();
        let frozen = system.snapshot_subtree_specs(ClientId(1), 1).unwrap();
        assert_eq!(frozen.iter().map(|l| l.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(frozen.iter().find(|l| l.id == 2).unwrap().opacity, 0);
    }
    #[test]
    fn unbounded_container_of_one_layer_folds_group_alpha() {
        let mut input = specs();
        input[1].clip = None;
        input.truncate(3);
        input[2].opacity = 200;
        input[2].filter = Some(LayerFilter { encoded_sampling: true, ..Default::default() });
        let mut system = LayerSystem::default();
        system.install(ClientId(1), &input).unwrap();
        let frozen = system.snapshot_subtree_specs(ClientId(1), 1).unwrap();
        assert_eq!(frozen.iter().find(|l| l.id == 2).unwrap().opacity, 255);
        // 200 * 128 / 255, rounded.
        assert_eq!(frozen.iter().find(|l| l.id == 3).unwrap().opacity, 100);
        assert!(frozen.iter().all(|l| !matches!(l.paint, Paint::Snapshot { .. })));
        assert_eq!(system.get(ClientId(1), 3).unwrap().opacity, 200);
    }
    #[test]
    fn unbounded_container_with_other_filter_still_fails() {
        let mut input = specs();
        input[1].clip = None;
        input.truncate(3);
        input[2].filter = Some(LayerFilter { mono: 1., ..Default::default() });
        let mut system = LayerSystem::default();
        system.install(ClientId(1), &input).unwrap();
        assert!(system.snapshot_subtree_specs(ClientId(1), 1).unwrap_err().to_string().contains("bounded local clip"));
    }
}
