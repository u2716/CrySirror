// SPDX-License-Identifier: MPL-2.0
//! Modal RGB selection using the shared slider, button and keyboard input paths.
use anyhow::{Context, Result};
use crysirror_core::{
    service::{EngineQuery, HostServices, buttons},
    state::ClientId,
};
use serde_json::{Value, json};

pub struct ColorPicker {
    root: u32,
    presets: Vec<[u8; 3]>,
    rgb: [u8; 3],
}

pub enum ColorSelection {
    Pending,
    Cancelled,
    Selected([u8; 3]),
}

impl ColorPicker {
    pub fn open(
        host: &mut dyn HostServices,
        client: ClientId,
        root: u32,
        canvas: [u32; 2],
        initial: [u8; 3],
        presets: &[[u8; 3]],
        font: &str,
    ) -> Result<Self> {
        anyhow::ensure!(
            presets.len() <= 16 && root <= u32::MAX - 32,
            "colour picker bounds"
        );
        let rows = presets.len().div_ceil(8);
        let height = 206 + 32 * rows;
        let rect = [
            (i64::from(canvas[0]) - 340) / 2,
            (i64::from(canvas[1]) - height as i64) / 2,
            340,
            height as i64,
        ];
        let mut ops = vec![
            json!({"op":"put","widget":{"id":root,"rect":rect,"kind":"panel",
            "modal":true,"z":i32::MAX,"color":[48,48,48,255],"keyboard_focusable":false,
            "key_event_scope":true,"key_events":[{"buttons":buttons::ESCAPE,"event":"cancel_picker"}]}}),
        ];
        for (i, c) in initial.into_iter().enumerate() {
            let y = 16 + i * 38;
            ops.push(json!({"op":"put","widget":{"id":root+4+i as u32,"parent":root,"kind":"label",
                "rect":[16,y,25,26],"font":font,"text":(["R","G","B"][i]),"keyboard_focusable":false}}));
            ops.push(
                json!({"op":"put","widget":{"id":root+1+i as u32,"parent":root,"kind":"slider",
                "rect":[49,y,270,26],"value":f32::from(c)/255.}}),
            );
        }
        for (i, &[r, g, b]) in presets.iter().enumerate() {
            ops.push(
                json!({"op":"put","widget":{"id":root+16+i as u32,"parent":root,"kind":"button",
                "rect":[16+38*(i%8),132+32*(i/8),32,26],"color":[r,g,b,255]}}),
            );
        }
        for (i, label) in ["OK", "Cancel"].into_iter().enumerate() {
            ops.push(
                json!({"op":"put","widget":{"id":root+7+i as u32,"parent":root,"kind":"button",
                "rect":[86+118*i,height-50,106,34],"font":font,"text":label}}),
            );
        }

        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"ui":ops}))?)?;
        Ok(Self {
            root,
            presets: presets.to_vec(),
            rgb: initial,
        })
    }

    pub fn poll(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<ColorSelection> {
        let events: Vec<Value> = serde_json::from_slice(
            &host.query_engine(client, EngineQuery::UiSubtreeEvents(self.root))?,
        )?;
        for event in events {
            let id = event["widget"]["id"]
                .as_u64()
                .and_then(|i| u32::try_from(i).ok());
            let offset = id.and_then(|i| i.checked_sub(self.root));
            if let Some(channel @ 1..=3) = offset {
                if event["kind"] == "change" {
                    let value = event["value"].as_f64().context("colour slider value")?;
                    self.rgb[channel as usize - 1] = (value * 255.).round().clamp(0., 255.) as u8;
                }
                continue;
            }
            let cancel = event["kind"] == "cancel_picker"
                || event["kind"] == "activate" && offset == Some(8);
            if !cancel && event["kind"] != "activate" {
                continue;
            }
            if let Some(color) = offset
                .and_then(|i| i.checked_sub(16))
                .and_then(|i| self.presets.get(i as usize))
            {
                self.rgb = *color;
                let ops: Vec<_> = color
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        json!({"op":"set_value",
                    "id":self.root+1+i as u32,"value":f32::from(v)/255.})
                    })
                    .collect();
                host.submit_engine(client, 1, &serde_json::to_vec(&json!({"ui":ops}))?)?;
                continue;
            }
            if !cancel && offset != Some(7) {
                continue;
            }
            let result = if cancel {
                ColorSelection::Cancelled
            } else {
                ColorSelection::Selected(self.rgb)
            };
            host.submit_engine(
                client,
                1,
                &serde_json::to_vec(&json!({"ui":[{"op":"remove","id":self.root}]}))?,
            )?;
            return Ok(result);
        }
        Ok(ColorSelection::Pending)
    }
}
