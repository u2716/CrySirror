//! TImoScrollBar native policy. Resource decoding, interpolation, hit testing
//! and dragging are provided by CrySirror's image/layer/UI services.
use super::*;

impl QlieVm {
    pub(in crate::adapter) fn bar_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| {
            a.get(i)
                .map(Value::integer)
                .transpose()
                .map(|v| v.unwrap_or(0))
        };
        let text = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        let get = |o: &Object, key: &str, default: i32| {
            o.properties
                .get(key)
                .and_then(|v| v.first())
                .and_then(|v| v.integer().ok())
                .unwrap_or(default)
        };
        match op {
            "i" => {
                ensure!(a.is_empty(), "QLIE scrollbar initialize ABI");
                let old = self.objects.remove(id).context("QLIE scrollbar")?;
                let (layer, thread) = (old.layer, old.thread);
                self.free_object(h, old)?;
                self.objects.insert(
                    id.into(),
                    Object {
                        kind: "bar".into(),
                        layer,
                        thread,
                        gain: 1.,
                        ..Default::default()
                    },
                );
            }
            "l" | "l2" => {
                ensure!(
                    a.len() == if op == "l2" { 4 } else { 3 },
                    "QLIE scrollbar load ABI"
                );
                let signature = format!("{op}:{a:?}");
                let files: Vec<Value> = a.iter().filter(|v| matches!(v, Value::Text(_))).cloned().collect();
                if !self.objects[id]
                    .properties
                    .get("bar/loading")
                    .and_then(|v| v.first())
                    .is_some_and(|v| v.text() == signature)
                {
                    let old = self.objects.remove(id).context("QLIE scrollbar")?;
                    let (layer, thread) = (old.layer, old.thread);
                    self.free_object(h, old)?;
                    let mut root = Object {
                        kind: "bar".into(),
                        layer,
                        thread,
                        gain: 1.,
                        ..Default::default()
                    };
                    root.properties
                        .insert("bar/loading".into(), vec![Value::Text(signature)]);
                    root.properties.insert("bar/files".into(), files);
                    root.properties
                        .insert("bar/max".into(), vec![Value::Int(100)]);
                    root.properties
                        .insert("bar/value".into(), vec![Value::Int(0)]);
                    root.properties.insert(
                        "bar/vertical".into(),
                        vec![Value::Int(n(if op == "l2" { 3 } else { 2 })?)],
                    );
                    for (index, part) in ["thumb", "track", "fill"].into_iter().enumerate() {
                        let name = if index == 2 && op == "l" {
                            String::new()
                        } else {
                            text(index)
                        };
                        if name.is_empty() {
                            continue;
                        }
                        let path = self.resolve(m, &name)?;
                        let child_id = format!("{id}/bar/{part}");
                        let layer = self.next_layer;
                        self.next_layer = self
                            .next_layer
                            .checked_add(1)
                            .context("QLIE scrollbar layer budget")?;
                        let mut child = Object {
                            kind: "img".into(),
                            layer,
                            thread,
                            gain: 1.,
                            ..Default::default()
                        };
                        child
                            .properties
                            .insert("bar/owner".into(), vec![Value::Text(id.into())]);
                        child
                            .properties
                            .insert("bar/part".into(), vec![Value::Text(part.into())]);
                        child
                            .properties
                            .insert("bar/path".into(), vec![Value::Text(path)]);
                        root.properties.insert(
                            format!("bar/{part}_id"),
                            vec![Value::Text(child_id.clone())],
                        );
                        m.object_types.insert(child_id.clone(), "img".into());
                        self.objects.insert(child_id, child);
                    }
                    self.objects.insert(id.into(), root);
                }
                // One owner/resource continuation at a time: completed children
                // must not restart while a later child is waiting on decoding.
                for part in ["thumb", "track", "fill"] {
                    let Some(child) = self.objects[id]
                        .properties
                        .get(&format!("bar/{part}_id"))
                        .and_then(|v| v.first())
                        .map(Value::text)
                    else {
                        continue;
                    };
                    if self.objects[&child].properties.contains_key("bar/loaded") {
                        continue;
                    }
                    let path = self.objects[&child].properties["bar/path"][0].text();
                    if !self.load_image(h, &child, &path)? {
                        return Ok(false);
                    }
                    self.objects
                        .get_mut(&child)
                        .unwrap()
                        .properties
                        .insert("bar/loaded".into(), vec![Value::Int(1)]);
                }
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .remove("bar/loading");
                self.put_parent(h, id)?;
            }
            "e" | "d" => {
                ensure!(a.is_empty(), "QLIE scrollbar visibility ABI");
                self.objects.get_mut(id).unwrap().visible = op == "e";
                self.put_parent(h, id)?;
            }
            "?e" => m.result(0, Value::Int(i32::from(self.objects[id].visible))),
            "mv" => {
                ensure!(a.len() == 1, "QLIE scrollbar maximum ABI");
                if n(0)? >= 0 {
                    let max = n(0)?;
                    let o = self.objects.get_mut(id).unwrap();
                    o.properties.insert("bar/max".into(), vec![Value::Int(max)]);
                    let value = get(o, "bar/value", 0).min(max);
                    o.properties
                        .insert("bar/value".into(), vec![Value::Int(value)]);
                }
            }
            "v" | "vi" => {
                ensure!((1..=2).contains(&a.len()), "QLIE scrollbar value ABI");
                let max = get(&self.objects[id], "bar/max", 100);
                let value = if op == "vi" {
                    max.saturating_sub(n(0)?)
                } else {
                    n(0)?
                }
                .clamp(0, max);
                let o = self.objects.get_mut(id).unwrap();
                let old = get(o, "bar/value", 0);
                o.properties
                    .insert("bar/value".into(), vec![Value::Int(value)]);
                if old != value {
                    o.properties
                        .insert("bar/value_from".into(), vec![Value::Int(old)]);
                    let duration = if n(1)? == 0 {
                        get(o, "bar/vt", 0).max(0)
                    } else {
                        0
                    };
                    o.properties
                        .insert("bar/value_duration".into(), vec![Value::Int(duration)]);
                    o.properties.insert(
                        "bar/value_started".into(),
                        vec![Value::Text(h.now_ms().to_string())],
                    );
                }
                if o.ui_registered {
                    let layer = o.layer;
                    self.submit(h,json!({"ui":[{"op":"set_value","id":layer,"value":if max==0 {0.} else {value as f32/max as f32}}]}))?;
                }
            }
            "?v" | "?vi" | "?mv" | "?vt" => {
                ensure!(a.is_empty(), "QLIE scrollbar value query ABI");
                let o = &self.objects[id];
                let value = match op {
                    "?mv" => get(o, "bar/max", 100),
                    "?vt" => get(o, "bar/vt", 0),
                    "?vi" => get(o, "bar/max", 100) - get(o, "bar/value", 0),
                    _ => get(o, "bar/value", 0),
                };
                m.result(0, Value::Int(value));
            }
            "vt" => {
                ensure!(a.len() == 1, "QLIE scrollbar transition time ABI");
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("bar/vt".into(), vec![Value::Int(n(0)?)]);
            }
            "p" => {
                ensure!((3..=4).contains(&a.len()), "QLIE scrollbar position ABI");
                let o = self.objects.get_mut(id).unwrap();
                o.position = [n(0)?, n(1)?, n(2)?];
                o.properties.insert("bar/x".into(), vec![a[0].clone()]);
                o.properties.insert("bar/y".into(), vec![a[1].clone()]);
                self.position(h, id, n(3)?.max(0) as u64)?;
            }
            "?p" => {
                for (i, value) in self.objects[id].position.into_iter().enumerate() {
                    m.result(i, Value::Int(value));
                }
            }
            "a" => {
                ensure!((1..=2).contains(&a.len()), "QLIE scrollbar alpha ABI");
                self.put_parent(h, id)?;
                let layer = self.objects[id].layer;
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("bar/alpha".into(), vec![a[0].clone()]);
                let alpha = n(0)?.clamp(0, 255);
                if n(1)? > 0 {
                    self.submit(h,json!({"layers":[{"op":"begin_motion","id":layer,"track":{"property":"opacity","to":alpha,"duration":n(1)?,"easing":"linear"}}]}))?;
                } else {
                    self.submit(
                        h,
                        json!({"layers":[{"op":"set_opacity","id":layer,"opacity":alpha}]}),
                    )?;
                }
            }
            "?a" => m.result(0, Value::Int(get(&self.objects[id], "bar/alpha", 255))),
            "lk" => {
                ensure!(a.len() == 1, "QLIE scrollbar lock ABI");
                self.objects.get_mut(id).unwrap().locked = n(0)? == 1;
                self.put_parent(h, id)?;
            }
            "?lk" => m.result(0, Value::Int(i32::from(self.objects[id].locked))),
            "us" | "ui" => {
                ensure!(a.len() <= 4, "QLIE scrollbar user data ABI");
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), a.to_vec());
            }
            "?us" | "?ui" => {
                let data = self.objects[id]
                    .properties
                    .get(&op[1..])
                    .cloned()
                    .unwrap_or_default();
                for i in 0..4 {
                    if op == "?us" {
                        m.result_text(i, data.get(i).map(Value::text).unwrap_or_default());
                    } else {
                        m.result(i, data.get(i).cloned().unwrap_or(Value::Int(0)));
                    }
                }
            }
            // ?dc/?sc answer the stored sound file names, ?mm the mode.
            "?hit" => self.object_hit_query(h, m, id, a)?,
            "?dc" | "?sc" => {
                let value = self.objects[id]
                    .properties
                    .get(&op[1..])
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_default();
                m.result_text(0, value);
            }
            "?mm" => {
                let value = self.objects[id]
                    .properties
                    .get("mm")
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or(Value::Int(0));
                m.result(0, value);
            }
            // ?l/?l2 answer the load arguments' image names.
            "?l" | "?l2" => {
                let signature = self.objects[id]
                    .properties
                    .get("bar/files")
                    .cloned()
                    .unwrap_or_default();
                for (i, v) in signature.into_iter().enumerate() {
                    m.result_text(i, v.text());
                }
            }
            // anc,name (+150/+154/+158) sets the anchor of the bar's
            // images; ?anc answers it.
            "anc" => {
                ensure!(a.len() == 1, "QLIE scrollbar anchor ABI");
                let anchor = a[0].text();
                self.objects.get_mut(id).unwrap().properties.insert("bar/anc".into(), vec![Value::Text(anchor)]);
            }
            "?anc" => {
                let value = self.objects[id].properties.get("bar/anc").and_then(|v| v.first()).map(Value::text);
                m.result_text(0, value.unwrap_or_default());
            }
            // skp,ms (6A3DD0 -> 6A3520) advances the bar's value
            // transition by ms; this host completes it.
            "skp" => {
                ensure!(a.len() == 1, "QLIE scrollbar skip ABI");
                if n(0)? >= 1 {
                    let mut commands = Vec::new();
                    for part in ["thumb", "track", "fill"] {
                        let child = self.objects[id]
                            .properties
                            .get(&format!("bar/{part}_id"))
                            .and_then(|v| v.first())
                            .map(Value::text);
                        if let Some(layer) = child.and_then(|c| self.objects.get(&c)).map(|o| o.layer) {
                            commands.push(json!({"op":"end_motion","id":layer}));
                        }
                    }
                    if !commands.is_empty() {
                        self.submit(h, json!({ "layers": commands }))?;
                    }
                }
            }
            "vtn" | "dc" | "sc" | "in" | "out" | "mi" | "mm" => {
                ensure!(
                    !a.is_empty() || matches!(op, "in" | "out"),
                    "QLIE scrollbar control ABI"
                );
                let values = if a.is_empty() {
                    vec![Value::Int(255)]
                } else {
                    a.to_vec()
                };
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert(op.into(), values.clone());
                for part in ["thumb", "track", "fill"] {
                    if let Some(child) = self.objects[id]
                        .properties
                        .get(&format!("bar/{part}_id"))
                        .and_then(|v| v.first())
                        .map(Value::text)
                    {
                        self.objects
                            .get_mut(&child)
                            .unwrap()
                            .properties
                            .insert(op.into(), values.clone());
                    }
                }
            }
            "?in" | "?out" | "?mi" | "?vtn" => {
                m.result(
                    0,
                    self.objects[id]
                        .properties
                        .get(&op[1..])
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or(Value::Int(if matches!(op, "?in" | "?out") {
                            255
                        } else {
                            0
                        })),
                );
            }
            _ => bail!("unsupported QLIE scrollbar method \\{op}"),
        }
        Ok(true)
    }
}
