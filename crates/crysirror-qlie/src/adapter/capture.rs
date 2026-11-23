//! Native capture policy uses engine snapshots and immutable media leases.
use super::*;
use crysirror_core::service::EngineQuery;
impl QlieVm {
    pub(super) fn capture_scene(
        &mut self,
        h: &mut dyn HostServices,
        commands: Vec<serde_json::Value>,
        priority: i32,
    ) -> Result<bool> {
        let mut sources = BTreeMap::new();
        for o in self
            .objects
            .values()
            .filter(|o| o.thread == self.host_thread && o.visible && o.position[2] <= priority)
        {
            for media in o.embedded_media.values() {
                let Some(frame) = h.media_frame(self.client, media.id)? else {
                    return Ok(false);
                };
                sources.insert(media.path.clone(), frame);
            }
            if let Some(id) = o.media {
                if let Some(path) = o
                    .properties
                    .get("media_image")
                    .and_then(|v| v.first())
                    .map(Value::text)
                {
                    let Some(frame) = h.media_frame(self.client, id)? else {
                        return Ok(false);
                    };
                    sources.insert(path, frame);
                }
            }
        }
        ensure!(sources.len() <= 64, "QLIE captured media budget");
        let mut paints = Vec::new();
        for o in self.objects.values().filter(|o| {
            o.created && o.thread == self.host_thread && o.visible && o.position[2] <= priority
        }) {
            for id in &o.children {
                let layer: serde_json::Value =
                    serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(*id))?)?;
                if let Some(path) = layer["paint"]["asset"]
                    .as_str()
                    .filter(|p| sources.contains_key(*p))
                {
                    paints.push((*id, path.to_owned(), layer["paint"].clone()));
                }
            }
        }
        // Shared MediaFrame retains decoded storage. The adapter changes only
        // resource bindings; it never copies movie pixels or composes a screen.
        let mut frozen = BTreeMap::new();
        for (_, path, _) in &paints {
            if !frozen.contains_key(path) {
                self.next_image = self
                    .next_image
                    .checked_add(1)
                    .context("QLIE capture image id")?;
                frozen.insert(
                    path.clone(),
                    h.publish_image_resource(
                        self.client,
                        self.next_image,
                        sources[path].as_image(self.client),
                    )?,
                );
            }
        }
        let mut bind = Vec::new();
        let mut restore = Vec::new();
        let mut rebound = BTreeMap::new();
        for (id, path, original) in paints {
            let mut paint = original.clone();
            paint["asset"] = json!(frozen[&path].path.relative());
            rebound.insert(id, (paint, original));
        }
        // QLIE multiplies object/state alpha into each draw, before blend
        // (5A7BA4), rather than isolating the whole object's composed screen.
        // Express that native capture policy with ordinary engine operations:
        // temporarily move inherited alpha from structural nodes to each
        // paint. Canvas still freezes/renders every actual paint.
        let mut tree = BTreeMap::<u32, serde_json::Value>::new();
        let mut ids = vec![self.root_layer_for(self.host_thread)];
        for o in self
            .objects
            .values()
            .filter(|o| o.thread == self.host_thread && o.created)
        {
            ids.push(o.layer);
            ids.extend(o.children.iter().copied());
            if let Some(message) = o.message.as_ref().filter(|m| m.created) {
                ids.push(message.layer);
            }
        }
        while let Some(id) = ids.pop() {
            if tree.contains_key(&id) {
                continue;
            }
            let layer: serde_json::Value =
                serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(id))?)?;
            if let Some(parent) = layer["parent"].as_u64() {
                ids.push(u32::try_from(parent)?);
            }
            tree.insert(id, layer);
            ensure!(tree.len() <= 4096, "QLIE capture node budget");
        }
        let parents: BTreeSet<u32> = tree
            .values()
            .filter_map(|l| l["parent"].as_u64().map(|v| v as u32))
            .collect();
        for (&id, layer) in &tree {
            let original = layer["opacity"]
                .as_u64()
                .context("QLIE capture layer alpha")?;
            let mut chain = Vec::new();
            let mut parent = layer["parent"].as_u64();
            while let Some(id) = parent {
                ensure!(chain.len() < 30, "QLIE capture alpha ancestor budget");
                let p = tree
                    .get(&(id as u32))
                    .context("QLIE capture alpha parent")?;
                chain.push(p["opacity"].as_u64().context("QLIE ancestor alpha")?);
                parent = p["parent"].as_u64();
            }
            // Match the engine's root-to-leaf byte multiplication order.
            let mut inherited = 255;
            for p in chain.into_iter().rev() {
                inherited = inherited * p / 255;
            }
            let effective = inherited * original / 255;
            if parents.contains(&id) {
                // A structural node passes alpha to its leaves. When it also
                // draws its own paint, that draw receives the same product
                // through the paint's own colour/tint alpha byte.
                if effective != 255 {
                    let (current, saved) = rebound
                        .remove(&id)
                        .unwrap_or_else(|| (layer["paint"].clone(), layer["paint"].clone()));
                    let scaled = scale_paint_alpha(current, effective)?;
                    rebound.insert(id, (scaled, saved));
                }
                if original != 255 {
                    bind.push(json!({"op":"set_opacity","id":id,"opacity":255}));
                    restore.push(json!({"op":"set_opacity","id":id,"opacity":original}));
                }
            } else if effective != original {
                bind.push(json!({"op":"set_opacity","id":id,"opacity":effective}));
                restore.push(json!({"op":"set_opacity","id":id,"opacity":original}));
            }
        }
        for (id, (paint, original)) in rebound {
            bind.push(json!({"op":"set_paint","id":id,"paint":paint}));
            restore.push(json!({"op":"set_paint","id":id,"paint":original}));
        }
        // Capture is below the native priority plane. Object selection is VM
        // policy; shared Layers/Canvas still own all sampling and composition.
        for o in self.objects.values().filter(|o| {
            o.thread == self.host_thread
                && o.created
                && o.visible
                && o.position[2] > priority
                && matches!(
                    o.kind.as_str(),
                    "img" | "btn" | "sbtn" | "simg" | "msg" | "theora"
                )
        }) {
            bind.push(json!({"op":"set_visible","id":o.layer,"visible":false}));
            restore.push(json!({"op":"set_visible","id":o.layer,"visible":true}));
        }
        if !bind.is_empty() {
            self.submit(h, json!({"layers":bind}))?;
        }
        let captured = self.submit(h, json!({"canvases":commands}));
        let restored = if restore.is_empty() {
            Ok(())
        } else {
            self.submit(h, json!({"layers":restore}))
        };
        let mut retired = Ok(());
        for image in frozen.values() {
            if let Err(e) = h.release_image_resource(self.client, image.path.relative()) {
                retired = Err(e)
            }
        }
        captured?;
        restored?;
        retired?;
        Ok(true)
    }
}

/// Multiply one retained paint's own alpha byte. Structural groups draw no
/// raster; an unknown paint kind cannot silently lose its inherited alpha.
fn scale_paint_alpha(mut paint: serde_json::Value, alpha: u64) -> Result<serde_json::Value> {
    let field = match paint["kind"].as_str().unwrap_or("") {
        "group" => return Ok(paint),
        "solid" | "text" | "glyph_run" => "color",
        "image" | "projected_mesh" => "tint",
        kind => bail!("QLIE capture alpha for paint kind {kind}"),
    };
    if paint[field].is_null() {
        paint[field] = json!([255, 255, 255, 255]);
    }
    let byte = paint[field][3].as_u64().context("QLIE capture paint alpha")?;
    paint[field][3] = json!(byte * alpha / 255);
    Ok(paint)
}
