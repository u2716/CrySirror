//! Native stage policy mapped to shared channel queues and affine views.
use super::*;
use crate::machine::effect_ops::{StageOperation, StageReference};
#[derive(Clone, Debug)]
pub(super) struct View {
    pub layer: u32,
    pub baseline: Option<[f32; 16]>,
    pub enabled: bool,
    pub declared: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        engine: crysirror_scene::EngineSystems,
    }
    impl crysirror_core::service::FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::Unsupported("unexpected file"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported("unexpected write"))
        }
    }
    impl HostServices for Host {
        fn submit(
            &mut self,
            _: ClientId,
            _: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported(
                "unexpected legacy command",
            ))
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn submit_engine(
            &mut self,
            client: ClientId,
            _: u32,
            bytes: &[u8],
        ) -> crysirror_core::Result<()> {
            let result = (|| -> Result<()> {
                self.engine
                    .submit(client, &serde_json::from_slice(bytes)?)?;
                Ok(())
            })();
            result.map_err(|error| crysirror_core::Error::Io(error.to_string()))
        }
        fn query_engine(
            &mut self,
            client: ClientId,
            query: EngineQuery,
        ) -> crysirror_core::Result<Vec<u8>> {
            let result = (|| -> Result<Vec<u8>> {
                let value = match query {
                    EngineQuery::Layer(id) => {
                        serde_json::to_value(self.engine.layers.get(client, id))?
                    }
                    EngineQuery::LayerSequence(id) => {
                        serde_json::to_value(self.engine.layers.sequence_status(client, id)?)?
                    }
                    EngineQuery::LayerMotion(id) => {
                        serde_json::to_value(self.engine.layers.motion_status(client, id)?)?
                    }
                    _ => bail!("unexpected query"),
                };
                Ok(serde_json::to_vec(&value)?)
            })();
            result.map_err(|error| crysirror_core::Error::Io(error.to_string()))
        }
    }
    #[test]
    fn queued_view_uses_native_activation_and_snap_without_overwriting_primary_scale() {
        let mut services = Services::default();
        services.image_extents.insert(1, [1280, 720]);
        let mut host = Host::default();
        let mut bridge = Bridge {
            services: &mut services,
            host: &mut host,
            client: ClientId(7),
            font: "test",
            input: InputFrame::default(),
        };
        let root = bridge.visual_root(0).unwrap();
        bridge.submit(vec![json!({"op":"set_transform","id":root,"transform":{"scale_x":0.8,"scale_y":0.9,"rotate":0.2,"pivot_x":16,"pivot_y":20}})],vec![]).unwrap();
        let operation = StageOperation {
            mode: 1,
            selector: 255,
            offset: [0, 0],
            zoom: 150,
            restore_scale: true,
            restore_position: true,
            apply_view: true,
        };
        bridge.stage_batch(&[operation], 4, &[]).unwrap();
        let mut samples = Vec::new();
        for _ in 0..4 {
            bridge.advance_stage_views().unwrap();
            samples.push(
                bridge
                    .effect_frame_samples()
                    .unwrap()
                    .into_iter()
                    .find(|sample| sample.object == 0)
                    .unwrap()
                    .scale[0],
            );
        }
        assert_eq!(samples, [1., 1.125, 1.25, 1.5]);
        let layer: Value = serde_json::from_slice(
            &bridge
                .host
                .query_engine(ClientId(7), EngineQuery::Layer(root))
                .unwrap(),
        )
        .unwrap();
        assert!((layer["transform"]["scale_x"].as_f64().unwrap() - 0.8).abs() < 0.00001);
        assert_eq!(layer["rect"][0], 0.);
        assert_eq!(layer["transform"]["local_matrix"][0], 1.5);
        assert!(bridge.stage_views_complete().unwrap());
    }
    #[test]
    fn immediate_restore_uses_remembered_primary_geometry_with_native_scale_units() {
        let mut services = Services::default();
        let mut host = Host::default();
        let mut bridge = Bridge {
            services: &mut services,
            host: &mut host,
            client: ClientId(7),
            font: "test",
            input: InputFrame::default(),
        };
        let operation = StageOperation {
            mode: 2,
            selector: 255,
            offset: [0; 2],
            zoom: 100,
            restore_scale: true,
            restore_position: true,
            apply_view: true,
        };
        bridge
            .stage_batch(
                &[operation],
                60,
                &[StageReference {
                    object: 10,
                    position: [200., -80.],
                    scale: [150., 120.],
                }],
            )
            .unwrap();
        let layer: Value = serde_json::from_slice(
            &bridge
                .host
                .query_engine(ClientId(7), EngineQuery::Layer(visual::object_layer(10)))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(layer["rect"][0], 200.);
        assert_eq!(layer["rect"][1], -80.);
        assert_eq!(layer["transform"]["scale_x"], 1.5);
        assert!((layer["transform"]["scale_y"].as_f64().unwrap() - 1.2).abs() < 0.00001);
    }
}
fn receipt_integer(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .context("Circus sequence receipt integer")
}
impl Bridge<'_> {
    pub(in crate::adapter) fn set_stage_texture_offset(
        &mut self,
        objects: &[u16],
        offset: [f32; 2],
    ) -> Result<()> {
        ensure!(
            objects.iter().all(|object| *object < 150)
                && offset.iter().all(|value| value.is_finite()),
            "Circus native texture translation"
        );
        let baseline = if offset == [0.; 2] {
            None
        } else {
            let mut matrix = [1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.];
            matrix[12] = offset[0];
            matrix[13] = offset[1];
            Some(matrix)
        };
        let mut commands = Vec::new();
        for object in objects {
            let id = self.visual_root(*object)?;
            let layer: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(id))?,
            )?;
            let mut transform = layer["transform"].clone();
            transform["local_matrix"] = json!(baseline);
            commands.push(json!({"op":"set_transform","id":id,"transform":transform}));
            if let Some(view) = self.services.effect_control.views.get_mut(object) {
                view.baseline = baseline;
            }
        }
        self.submit(commands, vec![])?;
        self.apply_stage_views()
    }
    fn stage_view(&mut self, object: u16) -> Result<u32> {
        if let Some(view) = self.services.effect_control.views.get(&object) {
            return Ok(view.layer);
        }
        let root = self.visual_root(object)?;
        let layer: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(root))?,
        )?;
        let baseline = layer["transform"]["local_matrix"]
            .as_array()
            .map(|values| -> Result<[f32; 16]> {
                ensure!(values.len() == 16, "Circus primary matrix extent");
                let mut matrix = [0.; 16];
                for (index, value) in values.iter().enumerate() {
                    matrix[index] = value.as_f64().context("Circus primary matrix scalar")? as f32;
                }
                Ok(matrix)
            })
            .transpose()?;
        let id = 0x3b00_0000 + u32::from(object);
        self.submit(vec![json!({"op":"put","layer":{"id":id,"rect":[640,360,1,1],"paint":{"kind":"solid","color":[0,0,0,0]},"visible":false}}),json!({"op":"begin_sequence","id":id,"spec":{"values":[1.,1.,640.,360.],"bindings":[
            {"layer":id,"channel":0,"target":{"kind":"motion","property":"scale_x"}},
            {"layer":id,"channel":1,"target":{"kind":"motion","property":"scale_y"}},
            {"layer":id,"channel":2,"target":{"kind":"motion","property":"x"}},
            {"layer":id,"channel":3,"target":{"kind":"motion","property":"y"}},
            {"layer":root,"channel":0,"target":{"kind":"finite_gate"}},
            {"layer":root,"channel":1,"target":{"kind":"finite_gate"}},
            {"layer":root,"channel":2,"target":{"kind":"finite_gate"}},
            {"layer":root,"channel":3,"target":{"kind":"finite_gate"}}
        ]}})],vec![])?;
        self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"capture","enabled":self.services.effect_control.aux_capture,"repeats":self.services.effect_control.aux_repeats,"forever":self.services.effect_control.aux_forever}})],vec![])?;
        self.services.effect_control.views.insert(
            object,
            View {
                layer: id,
                baseline,
                enabled: false,
                declared: false,
            },
        );
        Ok(id)
    }
    fn set_stage_view(
        &mut self,
        object: u16,
        scale: f32,
        center: [f32; 2],
        ticks: i32,
        animated: bool,
    ) -> Result<()> {
        let id = self.stage_view(object)?;
        self.services
            .effect_control
            .views
            .get_mut(&object)
            .unwrap()
            .enabled = scale != 1.;
        if !animated {
            self.services
                .effect_control
                .views
                .get_mut(&object)
                .unwrap()
                .declared = true;
            return self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[0,1,2,3],"values":[scale,scale,center[0],center[1]],"retained":false}})],vec![]);
        }
        let status: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::LayerSequence(id))?,
        )?;
        ensure!(status.values.len() == 4, "Circus view channel extent");
        let mut values = status.values.clone();
        for (n, fallback) in [(0, 1.), (1, 1.), (2, 640.), (3, 360.)] {
            if values[n] == 0. {
                values[n] = fallback;
            }
        }
        if values != status.values {
            self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[0,1,2,3],"values":values,"retained":false}})],vec![])?;
        }
        let current = values[0];
        let current_center = [values[2], values[3]];
        if current == scale && current_center == center {
            return Ok(());
        }
        let actual = status.lanes.get(&0);
        let bank = if status.capture {
            status.captured.get(&0).map_or(0, |records| records.len())
        } else {
            actual.map_or(0, |lane| lane.records.len())
        };
        let previous = if bank == 0 {
            None
        } else {
            let Some(lane) = actual else {
                return Ok(());
            };
            let Some(key) = lane
                .records
                .len()
                .checked_sub(1)
                .and_then(|n| u8::try_from(n).ok())
            else {
                return Ok(());
            };
            let Some(record) = lane.records.get(&key) else {
                return Ok(());
            };
            let crysirror_scene::layer::queued::SequenceTrack::Linear { target, .. } =
                &record.track
            else {
                bail!("Circus view predecessor track");
            };
            ensure!(target.len() == 4, "Circus view predecessor target extent");
            Some(target)
        };
        let center = if scale == 1. { current_center } else { center };
        if current == 1. && scale != 1. {
            self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"set","channels":[2,3],"values":center,"retained":false}})],vec![])?;
        }
        let fixed = scale == 1. || current == 1.;
        let reference = previous.unwrap_or(&values);
        let start = [
            reference[0],
            reference[0],
            if fixed { center[0] } else { reference[2] },
            if fixed { center[1] } else { reference[3] },
        ];
        self.services
            .effect_control
            .views
            .get_mut(&object)
            .unwrap()
            .declared = true;
        self.submit(vec![json!({"op":"sequence","id":id,"operation":{"kind":"enqueue","lane":0,"track":{"kind":"linear","channels":[0,1,2,3],"target":[scale,scale,center[0],center[1]],"ticks":ticks.unsigned_abs(),"velocity_divisor":ticks as f32,"references":[0,0,2,3],"loop_origins":[0,1,2,3],"delta":"explicit_start","start":start,"complete_first":false,"complete_zero":false,"skip_zero":false,"fast":true,"fast_channels":[2,3]}}})],vec![])
    }
    pub(in crate::adapter) fn restore_stage_objects(
        &mut self,
        records: &[(u16, Vec<u8>)],
    ) -> Result<()> {
        ensure!(
            records
                .iter()
                .all(|(object, record)| *object < 150 && record.len() == 960),
            "Circus auxiliary restore records"
        );
        let mut commands = Vec::new();
        for (object, record) in records {
            self.visual_root(*object)?;
            //447D20 copies184 along with geometry;444B00 tests it before
            //drawing. Preserve this saved visibility independently of crop.
            self.submit(vec![json!({"op":"set_visible","id":visual::object_layer(*object),"visible":self.native_primary_visible(*object,u32::from_le_bytes(record[176..180].try_into().unwrap())!=0)})],vec![])?;
            self.set_primary_alignment_maps(*object,crate::native_control::alignment_maps(record)?)?;
            let offset=|at:usize|f32::from_bits(u32::from_le_bytes(record[at-8..at-4].try_into().unwrap()));
            self.set_stage_texture_offset(&[*object],[offset(20),offset(24)])?;
            let id = self.stage_view(*object)?;
            let current: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(id))?,
            )?;
            let state = crate::native_control::auxiliary_from_record(record, &current)?;
            let view = self.services.effect_control.views.get_mut(object).unwrap();
            view.enabled = u32::from_le_bytes(record[732..736].try_into().unwrap()) != 0;
            view.declared = u32::from_le_bytes(record[44..48].try_into().unwrap()) != 0;
            commands.push(json!({"op":"sequence","id":id,"operation":{"kind":"restore_control","state":state,"seed_missing":[0]}}));
        }
        self.submit(commands, vec![])?;
        self.apply_stage_views()
    }
    pub(super) fn reset_stage_objects(&mut self, objects: &[u16]) -> Result<()> {
        ensure!(
            objects.iter().all(|object| *object < 150),
            "Circus auxiliary reset object"
        );
        let mut commands = Vec::new();
        for object in objects {
            if let Some(view) = self.services.effect_control.views.get_mut(object) {
                view.enabled = false;
                view.declared = false;
                commands.push(json!({"op":"sequence","id":view.layer,"operation":{"kind":"reset","values":[1.,1.,0.,0.],"retained":[0.,0.,0.,0.]}}));
            }
        }
        self.submit(commands, vec![])?;
        self.apply_stage_views()
    }
    pub(super) fn finish_stage_objects(&mut self,objects:&[u16])->Result<()> {
        ensure!(objects.iter().all(|object|*object<150),"Circus auxiliary finish objects");
        let commands=objects.iter().filter_map(|object|self.services.effect_control.views.get(object).map(|view|view.layer)).map(|id|json!({"op":"sequence","id":id,"operation":{"kind":"finish","cancel":false}})).collect();
        self.submit(commands,vec![])?;self.apply_stage_views()
    }
    pub(super) fn finish_stage_views(&mut self) -> Result<()> {
        self.native_visual(&crate::machine::visual_ops::VisualRequest::Finish)?;
        let ids = self
            .services
            .effect_control
            .views
            .values()
            .map(|view| view.layer)
            .collect::<Vec<_>>();
        if !ids.is_empty() {
            self.submit(ids.into_iter().map(|id|json!({"op":"sequence","id":id,"operation":{"kind":"finish","cancel":false}})).collect(),vec![])?;
        }
        self.apply_stage_views()
    }
    pub(super) fn stage_batch(
        &mut self,
        operations: &[StageOperation],
        ticks: i32,
        references: &[StageReference],
    ) -> Result<bool> {
        for operation in operations {
            //Mode3 declarations are carried by the typed primary sprite queue.
            if operation.mode == 3 {
                continue;
            }
            if operation.mode >= 2 {
                for reference in references {
                    let id = self.visual_root(reference.object)?;
                    if operation.restore_position
                        && ((0..=5).contains(&operation.selector) || operation.selector == 255)
                    {
                        let layer: Value = serde_json::from_slice(
                            &self
                                .host
                                .query_engine(self.client, EngineQuery::Layer(id))?,
                        )?;
                        let mut rect = layer["rect"].clone();
                        rect[0] = json!(reference.position[0]);
                        rect[1] = json!(reference.position[1]);
                        self.submit(vec![json!({"op":"set_rect","id":id,"rect":rect})], vec![])?;
                    }
                    if operation.restore_scale {
                        let divisor = if reference.scale.iter().all(|value| *value >= 1000.) {
                            10000.
                        } else {
                            100.
                        };
                        let layer: Value = serde_json::from_slice(
                            &self
                                .host
                                .query_engine(self.client, EngineQuery::Layer(id))?,
                        )?;
                        let mut transform = layer["transform"].clone();
                        transform["scale_x"] = json!(reference.scale[0] / divisor);
                        transform["scale_y"] = json!(reference.scale[1] / divisor);
                        self.submit(
                            vec![json!({"op":"set_transform","id":id,"transform":transform})],
                            vec![],
                        )?;
                    }
                }
            } else if operation.apply_view
                && ((0..=5).contains(&operation.selector) || operation.selector == 255)
            {
                let [width, height] = self
                    .services
                    .image_extents
                    .get(&1)
                    .copied()
                    .context("Circus stage zoom needs actual background dimensions")?;
                let mut percent = operation.zoom;
                if i64::from(width) * i64::from(percent) / 100 < 1280 && width != 0 {
                    percent = 128000 / i32::try_from(width)?;
                }
                let vertical =
                    if i64::from(height) * i64::from(operation.zoom) / 100 < 720 && height != 0 {
                        72000 / i32::try_from(height)?
                    } else {
                        operation.zoom
                    };
                percent = percent.max(vertical);
                let mut offset = operation.offset;
                if offset.contains(&-1) {
                    let id = self.visual_root(0)?;
                    let layer: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::Layer(id))?,
                    )?;
                    for (axis, dimension, half) in [(0, width, 640.), (1, height, 360.)] {
                        if offset[axis] == -1 {
                            let position = layer["rect"][axis]
                                .as_f64()
                                .context("Circus automatic stage center")?
                                as f32;
                            let center = -position + (dimension / 2) as f32;
                            offset[axis] = (half - f64::from(center)) as i32;
                        }
                    }
                }
                let [x, y] = offset;
                let background = [
                    (512 - x.clamp(-640, 640)) as f32,
                    (288 - y.clamp(-360, 360)) as f32,
                ];
                for object in [0, 1] {
                    self.set_stage_view(
                        object,
                        percent as f32 / 100.,
                        background,
                        ticks,
                        operation.mode != 0,
                    )?;
                }
                let characters = [(512 - 80 * x / 100) as f32, (288 - 80 * y / 100) as f32];
                for base in [10, 20, 30, 40, 50] {
                    for object in base..base + 5 {
                        self.set_stage_view(
                            object,
                            percent as f32 / 100.,
                            characters,
                            ticks,
                            operation.mode != 0,
                        )?;
                    }
                }
            }
        }
        self.apply_stage_views()?;
        Ok(true)
    }
    fn apply_stage_views(&mut self) -> Result<()> {
        let views = self
            .services
            .effect_control
            .views
            .iter()
            .map(|(object, view)| (*object, view.clone()))
            .collect::<Vec<_>>();
        let mut commands = Vec::new();
        for (object, view) in views {
            let sample: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(view.layer))?,
            )?;
            let values = &sample.values;
            ensure!(values.len() == 4, "Circus view sample extent");
            let scale = if view.declared {
                [values[0], values[1]]
            } else {
                [1.; 2]
            };
            let pivot = [values[2], values[3]];
            if !scale.iter().chain(&pivot).all(|value| value.is_finite()) {
                //447FF0/448CE0's zero/zero remains an active IEEE NaN. Its
                //bits still reach NativeLive viaeffect_frame_samples; GPU
                //geometry retains the last finite matrix until sourcefinish.
                continue;
            }
            let id = visual::object_layer(object);
            let layer: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(id))?,
            )?;
            let mut transform = layer["transform"].clone();
            let anchored = transform["object_anchor"].as_bool().unwrap_or(false);
            let mut anchor = [0.; 2];
            for (axis, name) in ["pivot_x", "pivot_y"].iter().enumerate() {
                anchor[axis] = layer["rect"][axis]
                    .as_f64()
                    .context("Circus view object position")? as f32;
                if !anchored {
                    anchor[axis] += transform[*name].as_f64().unwrap_or(0.) as f32;
                }
            }
            let matrix =
                crysirror_core::service::anchored_view_scale(view.baseline, anchor, pivot, scale)?;
            transform["local_matrix"] = if scale == [1.; 2] && view.baseline.is_none() {
                Value::Null
            } else {
                json!(matrix)
            };
            commands.push(json!({"op":"set_transform","id":id,"transform":transform}));
        }
        if !commands.is_empty() {
            self.submit(commands, vec![])?;
        }
        Ok(())
    }
    pub(in crate::adapter) fn advance_stage_objects(
        &mut self,
        objects: &[u16],
        fast: bool,
    ) -> Result<()> {
        let ids = objects
            .iter()
            .filter_map(|object| {
                self.services
                    .effect_control
                    .views
                    .get(object)
                    .map(|view| (*object, view.layer))
            })
            .collect::<Vec<_>>();
        let mut commands = Vec::new();
        for (object, id) in ids {
            let status: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(id))?,
            )?;
            let primary: Option<crysirror_scene::layer::queued::SequenceStatus> =
                serde_json::from_slice(&self.host.query_engine(
                    self.client,
                    EngineQuery::LayerSequence(0x3a04_0000 + u32::from(object)),
                )?)?;
            let rate = primary.as_ref().map_or(1., |primary| primary.values[16]);
            commands.push(json!({"op":"sequence","id":id,"operation":{"kind":"advance","generation":status.generation,"revision":status.revision.checked_add(1).context("Circus view revision overflow")?,"rate":rate,"fast":fast}}));
        }
        self.submit(commands, vec![])?;
        self.apply_stage_views()
    }
    pub(super) fn advance_stage_views(&mut self) -> Result<()> {
        let objects = self
            .services
            .effect_control
            .views
            .keys()
            .copied()
            .filter(|object|!self.services.effect_control.particle_wrapper(*object)&&!self.services.primary.paused.contains(object))
            .collect::<Vec<_>>();
        self.advance_stage_objects(&objects, self.native_effect_fast())
    }
    pub(in crate::adapter) fn stage_views_complete(&mut self) -> Result<bool> {
        let ids = self
            .services
            .effect_control
            .views
            .values()
            .map(|view| view.layer)
            .collect::<Vec<_>>();
        for id in ids {
            let status: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(id))?,
            )?;
            if !status.complete {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(in crate::adapter) fn effect_frame_samples(
        &mut self,
    ) -> Result<Vec<crate::machine::effect_ops::StageSample>> {
        let views = self
            .services
            .effect_control
            .views
            .iter()
            .map(|(object, view)| (*object, view.clone()))
            .collect::<Vec<_>>();
        let mut samples = Vec::new();
        for (object, view) in views {
            let status: crysirror_scene::layer::queued::SequenceStatus = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::LayerSequence(view.layer))?,
            )?;
            let values = &status.values;
            ensure!(values.len() == 4, "Circus sampled stage extent");
            samples.push(crate::machine::effect_ops::StageSample {
                object,
                scale: [values[0], values[1]],
                pivot: [values[2], values[3]],
                enabled: view.enabled,
                declared: view.declared,
                sequence: status,
            });
        }
        Ok(samples)
    }
}
