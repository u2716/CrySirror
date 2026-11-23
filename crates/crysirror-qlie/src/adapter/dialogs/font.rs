//! Native font-selection policy composed from the engine font catalog and
//! modal UI. Only client-authorized aliases may be accepted.
use super::*;
pub(super) struct FontPicker {
    aliases: Vec<String>,
    index: usize,
    slot: i32,
    style: i32,
    aa: i32,
    color: u32,
}
impl QlieVm {
    pub(in crate::adapter) fn font_dialog(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool> {
        if self.dialog.is_none() {
            ensure!((7..=9).contains(&a.len()), "QLIE font dialog ABI");
            let slot = a[0].integer()?;
            ensure!((0..8).contains(&slot), "QLIE font dialog slot");
            let library = h.font_library(self.client)?;
            let aliases = library.aliases()?;
            ensure!(!aliases.is_empty(), "QLIE authorized font catalog is empty");
            let old = self
                .objects
                .get("cfg")
                .and_then(|o| o.properties.get(&format!("font_alias/{slot}")))
                .and_then(|v| v.first())
                .map(Value::text)
                .unwrap_or_else(|| "default".into());
            let index = aliases.iter().position(|v| *v == old).unwrap_or(0);
            let font = aliases[index].clone();
            let setting = |name: &str, default: i32| {
                self.objects
                    .get("cfg")
                    .and_then(|o| o.properties.get(&format!("{name}/{slot}")))
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or(default)
            };
            let style = setting("fst2", 1);
            let aa = setting("faa2", 2);
            let color = setting("fsc2", 0) as u32 & 0xffffff;
            ensure!(
                (0..=3).contains(&aa),
                "unsupported QLIE font dialog antialias value {aa}"
            );
            let root = 0x40000000;
            let field = root + 3;
            let button = root + 4;
            let width = (self.config.canvas[0] as f32 - 32.).clamp(420., 720.);
            let height = (self.config.canvas[1] as f32 - 32.).clamp(460., 520.);
            let x = (self.config.canvas[0] as f32 - width) / 2.;
            let y = (self.config.canvas[1] as f32 - height) / 2.;
            let rgb = |value: i32| {
                let v = value as u32;
                [
                    (v >> 16 & 255) as u8,
                    (v >> 8 & 255) as u8,
                    (v & 255) as u8,
                    255,
                ]
            };
            let sample_color = if let Some(v) = a.get(7) {
                let v = v.integer()?;
                if v == 0x1000000 {
                    [255; 4]
                } else {
                    rgb(v)
                }
            } else {
                [255; 4]
            };
            let background = a
                .get(8)
                .map(Value::integer)
                .transpose()?
                .map(rgb)
                .unwrap_or([136, 136, 136, 255]);
            let size = a[6].integer()?.clamp(1, 200) as u16;
            let title = if a[1].text().is_empty() {
                "Font".into()
            } else {
                a[1].text()
            };
            self.submit(h,json!({"ui":[
                {"op":"put","widget":{"id":root,"kind":"panel","rect":[x,y,width,height],"modal":true,"z":i32::MAX,"color":[36,40,48,255],"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+1,"parent":root,"kind":"label","rect":[20,12,width-40.,32],"text":title,"font":"default","text_size":24,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":field,"parent":root,"kind":"text_field","rect":[64,60,width-128.,40],"font":"default","text":font,"max_length":256,"text_size":20}},
                {"op":"put","widget":{"id":root+6,"parent":root,"kind":"button","rect":[20,60,36,40],"font":"default","text":"‹"}},
                {"op":"put","widget":{"id":root+7,"parent":root,"kind":"button","rect":[width-56.,60,36,40],"font":"default","text":"›"}},
                {"op":"put","widget":{"id":root+2,"parent":root,"kind":"label","rect":[20,112,width-40.,64],"font":font,"text":a[4].text(),"text_size":size,"text_color":sample_color,"color":background,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+10,"parent":root,"kind":"toggle","rect":[20,188,(width-52.)/2.,32],"text":"Bold","font":"default","value":if style&1!=0{1.}else{0.}}},
                {"op":"put","widget":{"id":root+11,"parent":root,"kind":"toggle","rect":[(width+12.)/2.,188,(width-52.)/2.,32],"text":"Shadow","font":"default","value":if style&2!=0{1.}else{0.}}},
                {"op":"put","widget":{"id":root+12,"parent":root,"kind":"toggle","rect":[20,228,(width-52.)/2.,32],"text":"Outline","font":"default","value":if style&4!=0{1.}else{0.}}},
                {"op":"put","widget":{"id":root+13,"parent":root,"kind":"toggle","rect":[(width+12.)/2.,228,(width-52.)/2.,32],"text":"Outline extend bold","font":"default","value":if style&8!=0{1.}else{0.}}},
                {"op":"put","widget":{"id":root+14,"parent":root,"kind":"button","rect":[20,272,width-40.,32],"text":(["No AA","2× sampling","4× sampling","8× sampling"][aa as usize]),"font":"default"}},
                {"op":"put","widget":{"id":root+20,"parent":root,"kind":"slider","rect":[52,316,width-144.,24],"value":((color>>16)&255)as f32/255.,"color":[150,45,45,255]}},
                {"op":"put","widget":{"id":root+21,"parent":root,"kind":"slider","rect":[52,348,width-144.,24],"value":((color>>8)&255)as f32/255.,"color":[45,150,45,255]}},
                {"op":"put","widget":{"id":root+22,"parent":root,"kind":"slider","rect":[52,380,width-144.,24],"value":(color&255)as f32/255.,"color":[45,45,150,255]}},
                {"op":"put","widget":{"id":root+24,"parent":root,"kind":"label","rect":[20,316,28,24],"text":"R","font":"default","keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+25,"parent":root,"kind":"label","rect":[20,348,28,24],"text":"G","font":"default","keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+26,"parent":root,"kind":"label","rect":[20,380,28,24],"text":"B","font":"default","keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+23,"parent":root,"kind":"panel","rect":[width-68.,316,48,88],"color":rgb(color as i32),"keyboard_focusable":false}},
                {"op":"put","widget":{"id":root+8,"parent":root,"kind":"label","rect":[20,height-88.,width-40.,24],"font":"default","text":"","text_color":[255,128,128,255],"text_size":18,"keyboard_focusable":false}},
                {"op":"put","widget":{"id":button,"parent":root,"kind":"button","rect":[width-288.,height-52.,124,36],"text":a[2].text(),"font":"default","text_size":20}},
                {"op":"put","widget":{"id":root+5,"parent":root,"kind":"button","rect":[width-152.,height-52.,132,36],"text":a[3].text(),"font":"default","text_size":20}},
                {"op":"focus","id":field}
            ]}))?;
            self.dialog = Some(TextDialog {
                root,
                field,
                button,
                list: None,
                font: Some(FontPicker {
                    aliases,
                    index,
                    slot,
                    style,
                    aa,
                    color,
                }),
            });
            return Ok(false);
        }
        let mut dialog = self
            .dialog
            .take()
            .context("QLIE font dialog continuation")?;
        let Some(picker) = &mut dialog.font else {
            self.dialog = Some(dialog);
            bail!("QLIE font dialog continuation owner");
        };
        let events: Vec<serde_json::Value> = serde_json::from_slice(
            &h.query_engine(self.client, EngineQuery::UiSubtreeEvents(dialog.root))?,
        )?;
        let mut done = None;
        for event in events {
            let id = event["widget"]["id"]
                .as_u64()
                .or_else(|| event["widget"].as_u64())
                .unwrap_or(0) as u32;
            let kind = event["kind"].as_str().unwrap_or("");
            if id == dialog.root + 5 && kind == "activate" || id == dialog.field && kind == "cancel"
            {
                done = Some(false);
                break;
            }
            if id == dialog.button && kind == "activate" || id == dialog.field && kind == "submit" {
                done = Some(true);
                break;
            }
            if kind == "change" && (dialog.root + 10..=dialog.root + 13).contains(&id) {
                let bit = 1i32 << (id - dialog.root - 10);
                if event["value"]
                    .as_f64()
                    .context("QLIE font style checkbox value")?
                    > 0.5
                {
                    picker.style |= bit;
                } else {
                    picker.style &= !bit;
                }
            }
            if kind == "activate" && id == dialog.root + 14 {
                picker.aa = (picker.aa + 1) % 4;
                self.submit(h,json!({"ui":[{"op":"set_text","id":id,"text":(["No AA","2× sampling","4× sampling","8× sampling"][picker.aa as usize])}]}))?;
            }
            if kind == "change" && (dialog.root + 20..=dialog.root + 22).contains(&id) {
                let shift = 16 - (id - dialog.root - 20) * 8;
                let value = (event["value"]
                    .as_f64()
                    .context("QLIE font style color channel")?
                    .clamp(0., 1.)
                    * 255.)
                    .round() as u32;
                picker.color = (picker.color & !(255 << shift)) | (value << shift);
                let spec: serde_json::Value = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::UiWidget(dialog.root + 23))?,
                )?;
                let mut spec = spec;
                spec["color"] = json!([
                    (picker.color >> 16) & 255,
                    (picker.color >> 8) & 255,
                    picker.color & 255,
                    255
                ]);
                self.submit(h, json!({"ui":[{"op":"put","widget":spec}]}))?;
            }
            if kind == "activate"
                && matches!(id,value if value==dialog.root+6||value==dialog.root+7)
            {
                picker.index = if id == dialog.root + 6 {
                    (picker.index + picker.aliases.len() - 1) % picker.aliases.len()
                } else {
                    (picker.index + 1) % picker.aliases.len()
                };
                let font = &picker.aliases[picker.index];
                self.submit(h,json!({"ui":[{"op":"set_text","id":dialog.field,"text":font},{"op":"set_text","id":dialog.root+2,"text":a[4].text(),"font":font}]}))?;
            }
        }
        let Some(accepted) = done else {
            self.dialog = Some(dialog);
            return Ok(false);
        };
        if accepted {
            let spec: serde_json::Value = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiWidget(dialog.field))?,
            )?;
            let name = spec["text"]
                .as_str()
                .context("QLIE font dialog selected alias")?
                .to_owned();
            let Some(alias) = h.font_library(self.client)?.resolve_alias(&name)? else {
                self.submit(
                    h,
                    json!({"ui":[{"op":"set_text","id":dialog.root+8,"text":"Font unavailable"}]}),
                )?;
                self.dialog = Some(dialog);
                return Ok(false);
            };
            let cfg = self
                .objects
                .get_mut("cfg")
                .context("QLIE configured font state")?;
            cfg.properties.insert(
                format!("font_alias/{}", picker.slot),
                vec![Value::Text(alias)],
            );
            cfg.properties
                .insert(format!("fn2/{}", picker.slot), vec![Value::Text(name)]);
            for (key, value) in [
                ("fst2", picker.style),
                ("faa2", picker.aa),
                ("fsc2", picker.color as i32),
            ] {
                cfg.properties
                    .insert(format!("{key}/{}", picker.slot), vec![Value::Int(value)]);
            }
        }
        self.submit(h, json!({"ui":[{"op":"remove","id":dialog.root}]}))?;
        m.result(0, Value::Int(i32::from(accepted)));
        Ok(true)
    }
}
