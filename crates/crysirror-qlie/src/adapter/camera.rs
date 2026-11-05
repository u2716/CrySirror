//! QLIE camera priority selection; transforms and interpolation live in Layers.
use super::*;
fn number(a: &[Value], i: usize, default: i32) -> Result<i32> {
    a.get(i)
        .map(Value::integer)
        .transpose()
        .map(|v| v.unwrap_or(default))
}
impl QlieVm {
    pub(super) fn camera_parent(&self, priority: i32) -> u32 {
        self.objects
            .values()
            .find(|o| {
                o.kind == "camex" && o.created && {
                    let range = o.properties.get("pri");
                    let lo = range
                        .and_then(|a| a.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(-2000);
                    let hi = range
                        .and_then(|a| a.get(1))
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(999);
                    (lo..=hi).contains(&priority)
                }
            })
            .map(|o| o.generator.as_ref().map(|g| g.layer).unwrap_or(o.layer))
            .unwrap_or(1)
    }
    pub(super) fn camera_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let layer = self.objects[id].layer;
        if !self.objects[id].created {
            self.submit(h,json!({"layers":[{"op":"put","layer":{"id":layer,"parent":1,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"order":[-2000,0],"transform":{"pivot_x":self.config.canvas[0] as f32/2.,"pivot_y":self.config.canvas[1] as f32/2.},"paint":{"kind":"solid","color":[0,0,0,0]}}}]}))?;
            self.objects.get_mut(id).unwrap().created = true;
        }
        match op {
            "i" => {
                if self.objects[id].properties.contains_key("col") {
                    self.submit(h, json!({"layers":[{"op":"remove","id":layer+0x20000000}]}))?;
                }
                if let Some(group) = &mut self.objects.get_mut(id).unwrap().generator {
                    group.waves.clear();
                }
                self.update_generators(h)?;
                let o = self.objects.get_mut(id).unwrap();
                o.properties.clear();
                self.submit(h,json!({"layers":[{"op":"end_motion","id":layer},{"op":"set_rect","id":layer,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]]},{"op":"set_transform","id":layer,"transform":{"pivot_x":self.config.canvas[0] as f32/2.,"pivot_y":self.config.canvas[1] as f32/2.}}]}))?;
            }
            "pri" => {
                ensure!(
                    a.len() == 2 && number(a, 0, 0)? <= number(a, 1, 0)?,
                    "QLIE camera priority range"
                );
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
                self.submit(
                    h,
                    json!({"layers":[{"op":"set_order","id":layer,"order":[number(a,0,0)?,0]}]}),
                )?;
            }
            "p" => {
                ensure!((9..=11).contains(&a.len()), "QLIE camera pose ABI");
                let duration = number(a, 9, 0)?.max(0) as u64;
                let repeat = animation::repeat(&self.objects[id], "pose_repeat", number(a, 10, 0)?);
                let o = &self.objects[id];
                let mode = o
                    .properties
                    .get("mm")
                    .and_then(|a| a.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(3);
                let easing = animation::easing(mode)?;
                let properties = [
                    "x",
                    "y",
                    "z",
                    "scale_x",
                    "scale_y",
                    "scale_z",
                    "rotation_x",
                    "rotation_y",
                    "rotation_z",
                ];
                let mut commands = Vec::new();
                // Native camera matrices translate by -position and rotate by
                // -angle around the viewport centre, rather than moving sprites.
                for (i, property) in properties.into_iter().enumerate() {
                    let mut value = number(a, i, if (3..6).contains(&i) { 100 } else { 0 })? as f32;
                    if i < 3 {
                        value = -value;
                    } else if i < 6 {
                        value = value.max(1.) / 100.;
                    } else {
                        value = -value.to_radians();
                    }
                    commands.push(json!({"op":"begin_motion","id":layer,"track":{"property":property,"to":value,"duration":duration,"repeat":repeat,"easing":easing}}));
                }
                self.submit(h, json!({"layers":commands}))?;
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("pose_repeat".into(), vec![Value::Int(repeat)]);
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            "col" => {
                let alpha = number(a, 0, 0)?.clamp(0, 255);
                let c = number(a, 1, 0)? as u32;
                let duration = number(a, 2, 0)?.max(0) as u64;
                let repeat =
                    animation::repeat(&self.objects[id], "colour_repeat", number(a, 3, 0)?);
                let overlay = layer + 0x20000000;
                let priority = self.objects[id]
                    .properties
                    .get("pri")
                    .and_then(|v| v.get(1))
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(999);
                let blend = self.objects[id]
                    .properties
                    .get("bm")
                    .and_then(|v| v.first())
                    .map(Value::integer)
                    .transpose()?
                    .unwrap_or(1);
                let blend = match blend {
                    0 | 1 => "normal",
                    2 => "screen",
                    3 => "multiply",
                    7 => "add",
                    8 => "subtract",
                    _ => bail!("unsupported QLIE camera blend {blend}"),
                };
                let previous: Option<serde_json::Value> =
                    if self.objects[id].properties.contains_key("col") {
                        Some(serde_json::from_slice(
                            &h.query_engine(self.client, EngineQuery::Layer(overlay))?,
                        )?)
                    } else {
                        None
                    };
                let before = previous
                    .as_ref()
                    .and_then(|v| v["opacity"].as_u64())
                    .unwrap_or(0);
                let multiply = previous
                    .as_ref()
                    .map(|v| v["filter"]["multiply"].clone())
                    .filter(|v| v.is_array())
                    .unwrap_or(json!([1., 1., 1.]));
                let easing = animation::easing(
                    self.objects[id]
                        .properties
                        .get("mm")
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(3),
                )?;
                let mut commands = vec![
                    json!({"op":"put","layer":{"id":overlay,"parent":1,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"order":[priority,1],"opacity":before,"blend":blend,"paint":{"kind":"solid","color":[255,255,255,255]},"filter":{"multiply":multiply}}}),
                    json!({"op":"begin_motion","id":overlay,"track":{"property":"opacity","to":alpha,"duration":duration,"repeat":repeat,"easing":easing}}),
                ];
                if number(a, 1, 0)? != -1 {
                    for (property, shift) in [
                        ("multiply_red", 16),
                        ("multiply_green", 8),
                        ("multiply_blue", 0),
                    ] {
                        commands.push(json!({"op":"begin_motion","id":overlay,"track":{"property":property,"to":((c>>shift)&255) as f32/255.,"duration":duration,"repeat":repeat,"easing":easing}}));
                    }
                }
                self.submit(h, json!({"layers":commands}))?;
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("colour_repeat".into(), vec![Value::Int(repeat)]);
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            "g" => {
                self.generator_method(h, id, op, a)?;
            }
            "g2" => {
                self.generator_method(h, id, op, a)?;
            }
            "mm" | "tf" | "bm" => {
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            "?p" => {
                let values = self.objects[id]
                    .properties
                    .get("p")
                    .cloned()
                    .unwrap_or_default();
                for (i, v) in values.into_iter().enumerate() {
                    m.result(i, v);
                }
            }
            _ => bail!("unsupported QLIE camera method \\{op}"),
        }
        if matches!(op, "pri" | "i") {
            let commands=self.objects.values().filter(|o|o.created && matches!(o.kind.as_str(),"img"|"btn"|"msg"|"theora"|"simg"|"sbtn")).map(|o|json!({"op":"set_parent","id":o.layer,"parent":self.camera_parent(o.position[2])})).collect::<Vec<_>>();
            if !commands.is_empty() {
                self.submit(h, json!({"layers":commands}))?;
            }
        }
        Ok(true)
    }
}
