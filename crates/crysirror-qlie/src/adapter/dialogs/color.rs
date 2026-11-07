//! dlgs c,colour (700C20 TColorDialog): engine modal UI with red, green
//! and blue sliders. Colours are Delphi TColor values ($00BBGGRR). Results:
//! ResultInt 1/0 (accepted), ResultInt the colour, ResultStr `$%.6x`;
//! cancel returns the initial colour. Native follows with the 250 ms input
//! reset.
use super::*;

pub(in crate::adapter) struct ColorDialog {
    root: u32,
    initial: u32,
}
const ROOT: u32 = 0x40000200;

impl QlieVm {
    pub(super) fn color_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<bool> {
        if self.color_dialog.is_none() {
            ensure!(a.len() <= 1, "QLIE colour dialog ABI");
            let initial = a.first().map(Value::integer).transpose()?.unwrap_or(0) as u32 & 0xFFFFFF;
            let (w, hgt) = (360.0f32, 260.0f32);
            let x = (self.config.canvas[0] as f32 - w) / 2.;
            let y = (self.config.canvas[1] as f32 - hgt) / 2.;
            let mut ops = vec![json!({"op":"put","widget":{"id":ROOT,"rect":[x,y,w,hgt],"kind":"panel","modal":true,"z":i32::MAX,"color":[36,40,48,255],"keyboard_focusable":false}})];
            for (i, (label, shift)) in [("R", 0), ("G", 8), ("B", 16)].into_iter().enumerate() {
                let row = 24. + 52. * i as f32;
                let value = ((initial >> shift) & 255) as f32 / 255.;
                ops.push(json!({"op":"put","widget":{"id":ROOT+1+2*i as u32,"parent":ROOT,"rect":[20,row,30,32],"kind":"label","text":label,"font":"default","text_size":22,"keyboard_focusable":false}}));
                ops.push(json!({"op":"put","widget":{"id":ROOT+2+2*i as u32,"parent":ROOT,"rect":[60,row,280,32],"kind":"slider","value":value}}));
            }
            ops.push(json!({"op":"put","widget":{"id":ROOT+7,"parent":ROOT,"rect":[w-300.,hgt-60.,130,44],"kind":"button","text":"OK","font":"default","text_size":20}}));
            ops.push(json!({"op":"put","widget":{"id":ROOT+8,"parent":ROOT,"rect":[w-150.,hgt-60.,130,44],"kind":"button","text":"Cancel","font":"default","text_size":20}}));
            self.submit(h, json!({ "ui": ops }))?;
            self.color_dialog = Some(ColorDialog { root: ROOT, initial });
            return Ok(false);
        }
        let d = self.color_dialog.as_ref().unwrap();
        let events: serde_json::Value = serde_json::from_slice(
            &h.query_engine(self.client, EngineQuery::UiSubtreeEvents(d.root))?,
        )?;
        let mut done = None;
        for event in events.as_array().context("QLIE colour dialog events")? {
            let widget = event["widget"]["id"].as_u64().or_else(|| event["widget"].as_u64());
            if event["kind"] == "activate" {
                if widget == Some(u64::from(ROOT + 7)) {
                    done = Some(true);
                } else if widget == Some(u64::from(ROOT + 8)) {
                    done = Some(false);
                }
            }
        }
        let Some(accepted) = done else {
            return Ok(false);
        };
        let mut colour = d.initial;
        if accepted {
            colour = 0;
            for (i, shift) in [0u32, 8, 16].into_iter().enumerate() {
                let widget: serde_json::Value = serde_json::from_slice(&h.query_engine(
                    self.client,
                    EngineQuery::UiWidget(ROOT + 2 + 2 * i as u32),
                )?)?;
                let v = (widget["value"].as_f64().unwrap_or(0.) * 255.).round().clamp(0., 255.) as u32;
                colour |= v << shift;
            }
        }
        self.submit(h, json!({"ui":[{"op":"remove","id":ROOT}]}))?;
        self.color_dialog = None;
        m.result(0, Value::Int(i32::from(accepted)));
        m.result(1, Value::Int(colour as i32));
        m.result_text(0, format!("${colour:06x}"));
        self.block_input(h.now_ms(), 250);
        Ok(true)
    }
}
