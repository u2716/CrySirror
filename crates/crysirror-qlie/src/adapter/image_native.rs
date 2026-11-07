//! Image-object methods of 6F1370 beyond the original adapter set.
use super::*;

impl QlieVm {
    /// Returns false when `op` is not handled here.
    pub(super) fn image_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        match op {
            // p2,x,y,pri,sx,sy,time[,repeat] and p3,x,y,pri,sx,sy,rot,time
            // [,repeat] (5D1E7C): the 2D pose with scale percent and
            // rotation degrees; p is the same call with 100/100/0.
            "p2" | "p3" => {
                let base = if op == "p2" { 6 } else { 7 };
                ensure!((base..=base + 1).contains(&a.len()), "QLIE image 2D pose ABI");
                let (sx, sy) = (n(3)?, n(4)?);
                let rotation = if op == "p3" { n(5)? } else { 0 };
                let time = n(base - 1)?.max(0) as u64;
                let repeat = animation::repeat(&self.objects[id], "pose_repeat", n(base)?);
                self.objects.get_mut(id).unwrap().position = [n(0)?, n(1)?, n(2)?];
                self.position_repeated(h, id, time, repeat)?;
                let easing = self.property_easing(id, "mm")?;
                let layer = self.objects[id].layer;
                let tracks = [
                    ("scale_x", sx as f32 / 100.),
                    ("scale_y", sy as f32 / 100.),
                    ("rotation_z", (rotation as f32).to_radians()),
                ];
                let commands: Vec<_> = tracks
                    .into_iter()
                    .map(|(property, to)| json!({"op":"begin_motion","id":layer,"track":{"property":property,"to":to,"duration":time,"repeat":repeat,"easing":easing}}))
                    .collect();
                self.submit(h, json!({ "layers": commands }))?;
                let o = self.objects.get_mut(id).unwrap();
                o.properties.insert("pose_repeat".into(), vec![Value::Int(repeat)]);
                o.properties.insert(
                    "pose2d".into(),
                    vec![Value::Int(sx), Value::Int(sy), Value::Int(rotation), Value::Int(n(base)?)],
                );
            }
            // ?p2: x, y, priority, scale x/y, the repeat value; ?p3 adds the
            // rotation before the last value.
            "?p2" | "?p3" => {
                let o = &self.objects[id];
                let pose = o.properties.get("pose2d").cloned().unwrap_or_default();
                let get = |i: usize, d: i32| pose.get(i).cloned().unwrap_or(Value::Int(d));
                let mut values = vec![
                    Value::Int(o.position[0]),
                    Value::Int(o.position[1]),
                    Value::Int(o.position[2]),
                    get(0, 100),
                    get(1, 100),
                ];
                if op == "?p3" {
                    values.push(get(2, 0));
                }
                values.push(get(3, 0));
                for (i, v) in values.into_iter().enumerate() {
                    m.result(i, v);
                }
            }
            // a,alpha,time (5D026C): the colour animation's alpha alone,
            // with the same linear calculator as col.
            "a" => {
                ensure!((1..=3).contains(&a.len()), "QLIE image alpha ABI");
                self.put_parent(h, id)?;
                let layer = self.objects[id].layer;
                let alpha = n(0)?.clamp(0, 255);
                let duration = n(1)?.max(0);
                let repeat = animation::repeat(&self.objects[id], "colour_repeat", n(2)?);
                let easing = crysirror_scene::layer::Easing::Linear;
                self.submit(h, json!({"layers":[{"op":"begin_motion","id":layer,"track":{"property":"opacity","to":alpha,"duration":duration,"repeat":repeat,"easing":easing}}]}))?;
                self.objects.get_mut(id).unwrap().properties.insert("alpha".into(), vec![Value::Int(alpha)]);
            }
            "?a" => {
                let o = &self.objects[id];
                let alpha = o
                    .properties
                    .get("alpha")
                    .or_else(|| o.properties.get("col"))
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or(Value::Int(255));
                m.result(0, alpha);
            }
            // alr,v / ?alr (+3113) are kept with the anchor settings.
            "alr" => {
                ensure!(a.len() == 1, "QLIE anchor rotation ABI");
                self.objects.get_mut(id).unwrap().properties.insert("alr".into(), vec![Value::Int(n(0)?)]);
            }
            "?alg" => {
                let alg = self.objects[id].properties.get("alg").cloned().unwrap_or_default();
                let get = |i: usize| alg.get(i).and_then(|v| v.integer().ok()).unwrap_or(0);
                for (i, v) in [get(0) / 50, get(1) / 50, get(0), get(1)].into_iter().enumerate() {
                    m.result(i, Value::Int(v));
                }
            }
            "?anc" => {
                let anchor = self.objects[id].properties.get("anc").and_then(|v| v.first()).map(Value::text);
                m.result_text(0, anchor.unwrap_or_default());
            }
            "?col" => {
                let col = self.objects[id].properties.get("col").cloned().unwrap_or_default();
                let defaults = [255, -1, 0, 0, 0];
                for (i, d) in defaults.into_iter().enumerate() {
                    m.result(i, col.get(i).cloned().unwrap_or(Value::Int(d)));
                }
            }
            "?bm" => {
                let o = &self.objects[id];
                let bm = o.properties.get("native_bm").and_then(|v| v.first()).cloned();
                m.result(0, bm.unwrap_or(Value::Int(0)));
            }
            "?mm" => {
                let mm = self.objects[id].properties.get("mm").cloned().unwrap_or_default();
                m.result(0, mm.first().cloned().unwrap_or(Value::Int(3)));
                m.result(1, mm.get(1).cloned().unwrap_or(Value::Int(100)));
            }
            "?in" | "?out" | "?ps" | "?sp" | "?tf" | "?tab" | "?vtn" | "?alr" => {
                let default = match op {
                    "?in" | "?out" => 255,
                    "?ps" => 100,
                    _ => 0,
                };
                let value = self.objects[id].properties.get(&op[1..]).and_then(|v| v.first()).cloned();
                m.result(0, value.unwrap_or(Value::Int(default)));
            }
            "?ost" => {
                let ost = self.objects[id].properties.get("ost").cloned().unwrap_or_default();
                for i in 0..ost.len().max(1) {
                    m.result(i, ost.get(i).cloned().unwrap_or(Value::Int(0)));
                }
            }
            "?v" => {
                let gain = self.objects[id].gain;
                m.result(0, Value::Int((gain * 256.).round() as i32));
            }
            "?l" => {
                let name = self.objects[id].asset.clone().unwrap_or_default();
                m.result_text(0, name);
            }
            // ?bc (6F10A8): 1 once the button was clicked, the counter then
            // held at 1; bc (6A1B64) clicks it at its own position.
            "?bc" => {
                let o = self.objects.get_mut(id).unwrap();
                let count = o.properties.get("native/bc").and_then(|v| v.first()).and_then(|v| v.integer().ok()).unwrap_or(0);
                if count > 0 {
                    o.properties.insert("native/bc".into(), vec![Value::Int(1)]);
                }
                m.result(0, Value::Int(i32::from(count > 0)));
            }
            "bc" => {
                let o = self.objects.get_mut(id).unwrap();
                let count = o.properties.get("native/bc").and_then(|v| v.first()).and_then(|v| v.integer().ok()).unwrap_or(0);
                o.properties.insert("native/bc".into(), vec![Value::Int(count.saturating_add(1))]);
                if o.kind == "sbtn" {
                    ensure!(self.button_callbacks.len() < 256, "QLIE pending button callback budget");
                    self.button_callbacks.push_back(id.to_owned());
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
