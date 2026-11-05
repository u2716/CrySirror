//!4250B0/4226B0(1) native thumbnail selection. This is a logical shared Canvas
//! capture, never a read of the desktop or a privately composed framebuffer.
use super::*;
const LIVE: u32 = 0x3300_0003;
const FROZEN: u32 = 0x3300_0004;
#[derive(Default)]
pub(super) struct NativeThumbnail {
    //427E20 memset(this+1588,0,0xCC) initializes this[440] to null.
    surface: Option<Value>,
    unavailable: Option<String>,
    pending: bool,
}
impl Bridge<'_> {
    pub(super) fn update_native_thumbnail(&mut self) -> Result<()> {
        self.update_native_selected(false)
    }
    fn update_native_selected(&mut self, full:bool) -> Result<()> {
        if !self.effects_ready()? {
            self.services.thumbnail.pending = true;
            return Ok(());
        }
        self.services.thumbnail.pending = false;
        let scene: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(SCENE_ROOT))?,
        )?;
        let scene_visible = scene["visible"].as_bool().unwrap_or(true);
        let mut selected = Vec::new();
        for object in self
            .services
            .visual
            .roots()
            .filter(|object| *object < (if full {150} else {80}) && scene_visible)
        {
            let id = visual::object_layer(object);
            let layer: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(id))?,
            )?;
            if !layer.is_null() {
                selected.push((layer["order"].clone(), object, id));
            }
        }
        if let Some(effect) = self.services.effects.as_ref().filter(|_| scene_visible) {
            for emitter in &effect.spec.emitters {
                if let Some(screen) = &emitter.screen {
                    let depth = screen
                        .layer
                        .checked_sub(0x3900_0010)
                        .context("Circus thumbnail particle layer")?;
                    let object = match depth {
                        0 => 59,
                        1 => 39,
                        2 => 29,
                        3 => 19,
                        4 => 6,
                        _ => bail!("Circus thumbnail particle depth"),
                    };
                    let layer: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::Layer(screen.layer))?,
                    )?;
                    if !layer.is_null() {
                        selected.push((layer["order"].clone(), object, screen.layer));
                    }
                }
            }
        }
        selected.sort_by_key(|(order, object, _)| {
            (
                order[0].as_i64().unwrap_or(0),
                order[1].as_i64().unwrap_or(0),
                *object,
            )
        });
        let mut roots = selected
            .iter()
            .filter(|(_, object, _)| *object < 70)
            .map(|(_, _, id)| *id)
            .collect::<Vec<_>>();
        //70 draws the real message/window groups.2599 (group7) is explicitly
        //suppressed for a2=1, and the80/85/88 modal/profile pass is excluded.
        roots.extend(
            selected
                .iter()
                .filter(|(_, object, _)| *object == 70)
                .map(|(_, _, id)| *id),
        );
        for id in self.framebuffer_layer_ids() {
            roots.push(id);
        }
        if self.services.profile.is_some() {
            roots.push(0x3800_0000);
        }
        for group in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10]
            .into_iter()
            .filter(|group| scene_visible && (full || *group != 7))
        {
            roots.extend(
                self.services
                    .layer_ids
                    .get(&group)
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }
        if scene_visible {
            roots.extend(self.text_layer_ids());
        }
        roots.extend(
            selected
                .iter()
                .filter(|(_, object, _)| *object > 70)
                .map(|(_, _, id)| *id),
        );
        let mut existing = BTreeMap::new();
        for id in roots {
            let layer: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(id))?,
            )?;
            if !layer.is_null() {
                existing.insert(id, layer);
            }
        }
        let mut commands = vec![
            json!({"op":"create","id":LIVE,"width":1280,"height":720}),
            json!({"op":"clear","id":LIVE,"color":[0,0,0,255]}),
        ];
        // Copy each selected subtree once. A retained child is not an extra
        // paint merely because its resource ID is also listed in a checkpoint.
        let mut ordered = selected
            .iter()
            .filter(|(_, object, _)| *object <= 70)
            .map(|(_, _, id)| *id)
            .collect::<Vec<_>>();
        ordered.extend(self.framebuffer_layer_ids());
        let mut extra = existing
            .iter()
            .filter(|(id, _)| {
                !selected.iter().any(|(_, _, root)| root == *id)
                    && !self.framebuffer_layer_ids().contains(id)
                    && **id != 0x3800_0000
            })
            .map(|(id, layer)| (*id, layer["order"].clone()))
            .collect::<Vec<_>>();
        extra.sort_by_key(|(id, order)| {
            (
                order[0].as_i64().unwrap_or(0),
                order[1].as_i64().unwrap_or(0),
                *id,
            )
        });
        ordered.extend(extra.into_iter().map(|(id, _)| id));
        ordered.extend(
            selected
                .iter()
                .filter(|(_, object, _)| *object > 70)
                .map(|(_, _, id)| *id),
        );
        if self.services.profile.is_some() {
            ordered.push(0x3800_0000);
        }
        let mut seen = BTreeSet::new();
        for id in ordered {
            if !seen.insert(id) {
                continue;
            }
            let Some(layer) = existing.get(&id) else {
                continue;
            };
            let mut parent = layer["parent"].as_u64().and_then(|v| u32::try_from(v).ok());
            let mut nested = false;
            while let Some(ancestor) = parent {
                if existing.contains_key(&ancestor) {
                    nested = true;
                    break;
                }
                let value: Value = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::Layer(ancestor))?,
                )?;
                parent = value["parent"].as_u64().and_then(|v| u32::try_from(v).ok());
            }
            if !nested {
                commands.push(json!({"op":"draw_subtree","source":id,"destination":LIVE}));
            }
        }
        if self.services.profile.is_some() {
            let widgets: Vec<Value> = serde_json::from_slice(&self.host.query_engine(
                self.client,
                EngineQuery::UiSnapshot {
                    canvas: [1280, 720],
                    root: Some(0x3800_0000),
                },
            )?)?;
            for layer in widgets {
                commands.push(json!({"op":"draw","id":LIVE,"layer":layer}));
            }
        }
        self.host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"canvases":commands}))?,
        )?;
        let surface: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Canvas(LIVE))?,
        )?;
        ensure!(
            !surface.is_null(),
            "Circus native selected thumbnail capture receipt missing"
        );
        self.services.thumbnail.surface = Some(surface);
        self.services.thumbnail.unavailable = None;
        Ok(())
    }
    ///402BC0 calls4226B0(0) after the temporary history replay. This
    ///captures the newly materialized selected graph and leaves440 untouched.
    pub(super) fn begin_history_preview_image(&mut self)->Result<Option<crysirror_core::image_processing::ImageJob>> {
        let retained=std::mem::take(&mut self.services.thumbnail);
        let result=(||->Result<Option<crysirror_core::image_processing::ImageJob>> {
            self.update_native_selected(true)?;
            if self.services.thumbnail.pending {return Ok(None)}
            ensure!(self.services.thumbnail.unavailable.is_none(),"Circus history selected capture incomplete");
            let surface=self.services.thumbnail.surface.as_ref().context("Circus history selected capture missing")?;
            const HISTORY:u32=0x3300_0006;
            self.host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":[{"op":"restore","id":HISTORY,"surface":surface}]}))?)?;
            Ok(Some(self.host.begin_canvas_image(self.client,HISTORY)?))
        })();
        self.services.thumbnail=retained;
        result
    }
    pub(super) fn complete_native_thumbnail(&mut self) -> Result<()> {
        if self.services.thumbnail.pending {
            self.update_native_thumbnail()?;
        }
        Ok(())
    }
    pub(super) fn native_restore_fade_capture(
        &mut self,
    ) -> Result<Option<crysirror_core::image_processing::ImageJob>> {
        self.complete_native_thumbnail()?;
        ensure!(
            self.services.thumbnail.unavailable.is_none(),
            "Circus native restore fade source capture is incomplete"
        );
        let Some(surface) = self.services.thumbnail.surface.clone() else {
            return Ok(None);
        };
        self.host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(
                &json!({"canvases":[{"op":"restore","id":FROZEN,"surface":surface}]}),
            )?,
        )?;
        Ok(Some(self.host.begin_canvas_image(self.client, FROZEN)?))
    }
    ///421750 reads the currently retained selected snapshot, not the440
    ///that happened to exist when41DD10 captured its independent body.
    pub(super) fn current_thumbnail_source(&mut self) -> Result<Option<Vec<u8>>> {
        self.complete_native_thumbnail()?;
        if self.services.thumbnail.pending { return Ok(None); }
        ensure!(self.services.thumbnail.unavailable.is_none(), "Circus current native440 capture is incomplete");
        let mut declaration = json!({"owner":self.client.0,"has_native_thumbnail":self.services.thumbnail.surface.is_some()});
        if let Some(surface) = &self.services.thumbnail.surface {
            const WRITE: u32 = 0x3300_0005;
            self.host.submit_engine(self.client, 1, &serde_json::to_vec(&json!({"canvases":[{"op":"restore","id":WRITE,"surface":surface}]}))?)?;
            declaration["native_thumbnail_canvas"] = json!(WRITE);
            declaration["native_thumbnail_selection"] = json!("4226b0:1");
        }
        Ok(Some(serde_json::to_vec(&declaration)?))
    }
    pub(super) fn freeze_thumbnail(&mut self, resources: &mut Value) -> Result<()> {
        if let Some(error) = &self.services.thumbnail.unavailable {
            resources["has_native_thumbnail"] = Value::Null;
            resources["native_thumbnail_error"] = json!(error);
            return Ok(());
        }
        resources["has_native_thumbnail"] = json!(self.services.thumbnail.surface.is_some());
        if let Some(surface) = &self.services.thumbnail.surface {
            self.host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(
                    &json!({"canvases":[{"op":"restore","id":FROZEN,"surface":surface}]}),
                )?,
            )?;
            resources["native_thumbnail_canvas"] = json!(FROZEN);
            resources["native_thumbnail_selection"] = json!("4226b0:1");
        }
        Ok(())
    }
}
