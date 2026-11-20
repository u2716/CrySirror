// SPDX-License-Identifier: MPL-2.0
//! CVNS primary DIB and the ordered damage list at 0x41ec80.
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const SURFACE: u32 = 0x0356_8000;
type Rect = [i32; 4];

#[derive(Default)]
pub(super) struct Primary {
    created: bool,
    size: [u32; 2],
    damage: Vec<Rect>,
    previous: BTreeMap<u32, Value>,
}

impl Primary {
    fn full(size: [u32; 2]) -> Rect {
        [0, 0, size[0] as i32, size[1] as i32]
    }

    pub(super) fn invalidate(&mut self, size: [u32; 2], mut r: Rect) {
        let full = Self::full(size);
        if r[2] < 0 || r[0] > full[2] || r[3] < 0 || r[1] > full[3] {
            return;
        }
        r[0] = r[0].max(0);
        r[1] = r[1].max(0);
        r[2] = r[2].min(full[2]);
        r[3] = r[3].min(full[3]);
        // 0x41eab0 walks forward once. Contact at an edge merges too.
        let mut i = 0;
        while i < self.damage.len() {
            let d = self.damage[i];
            if d[0] <= r[0] && d[1] <= r[1] && d[2] >= r[2] && d[3] >= r[3] {
                return;
            }
            if r[0] > d[2] || r[2] < d[0] || r[1] > d[3] || r[3] < d[1] {
                i += 1;
                continue;
            }
            r = [
                r[0].min(d[0]),
                r[1].min(d[1]),
                r[2].max(d[2]),
                r[3].max(d[3]),
            ];
            self.damage.remove(i);
        }
        self.damage.push(r);
    }

    pub(super) fn invalidate_all(&mut self, size: [u32; 2]) {
        self.invalidate(size, Self::full(size));
    }
    pub(super) fn reset_damage(&mut self) {
        self.invalidate_all(self.size);
    }
    pub(super) fn capture(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<crysirror_core::image_processing::ImageJob> {
        Ok(if self.created {
            host.begin_canvas_image(client, SURFACE)?
        } else {
            host.begin_frame_capture(client)?
        })
    }
    pub(super) fn cancel(&mut self) {
        self.damage.clear();
    }

    fn initialize(&mut self, size: [u32; 2], ops: &mut Vec<Value>) {
        if !self.created {
            ops.push(json!({"op":"create","id":SURFACE,"width":size[0],"height":size[1]}));
            ops.push(json!({"op":"clear","id":SURFACE,"color":[0,0,0,255]}));
            ops.push(json!({"op":"present","id":SURFACE,"layer":SURFACE}));
            self.created = true;
        } else if self.size != size {
            ops.push(json!({"op":"resize","id":SURFACE,"width":size[0],"height":size[1]}));
            self.invalidate_all(size);
        }
        self.size = size;
    }

    pub(super) fn fill(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        size: [u32; 2],
        color: u32,
    ) -> Result<()> {
        let mut ops = Vec::new();
        self.initialize(size, &mut ops);
        let [r, g, b] = super::rgb(color);
        ops.push(json!({"op":"clear","id":SURFACE,"color":[r,g,b,255]}));
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"canvases":ops}))?)?;
        self.invalidate_all(size);
        Ok(())
    }

    pub(super) fn paint(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        size: [u32; 2],
        layers: Vec<Value>,
        background: Option<u32>,
    ) -> Result<()> {
        let mut ops = Vec::new();
        self.initialize(size, &mut ops);
        let mut current = BTreeMap::new();
        for mut command in layers {
            let layer = command["layer"].take();
            current.insert(layer["id"].as_u64().unwrap() as u32, layer);
        }
        let previous = std::mem::take(&mut self.previous);
        for (id, layer) in &current {
            if *id != super::FADE_LAYER && previous.get(id) != Some(layer) {
                if let Some(old) = previous.get(id) {
                    self.invalidate(size, bounds(old));
                }
                self.invalidate(size, bounds(layer));
            }
        }
        for (id, layer) in &previous {
            if *id != super::FADE_LAYER && !current.contains_key(id) {
                self.invalidate(size, bounds(layer));
            }
        }
        let mut ordered: Vec<_> = current.values().collect();
        ordered.sort_by_key(|l| {
            (
                l["order"][0].as_i64(),
                l["order"][1].as_i64(),
                l["id"].as_u64(),
            )
        });
        for mut d in self.damage.drain(..) {
            // 0x410e10 schedules exactly height / 12 complete scan bands.
            d[3] = d[3].min((size[1] / 12 * 12) as i32);
            let rect = [d[0], d[1], d[2] - d[0], d[3] - d[1]];
            if rect[2] <= 0 || rect[3] <= 0 {
                continue;
            }
            if let Some(color) = background {
                let [r, g, b] = super::rgb(color);
                if d == Self::full(size) {
                    ops.push(json!({"op":"clear","id":SURFACE,"color":[r,g,b,255]}));
                } else {
                    ops.push(json!({"op":"fill","id":SURFACE,"rect":rect,"color":[r,g,b,255],"replace":true}));
                }
            }
            for &layer in &ordered {
                let b = bounds(layer);
                if d[2] <= b[0] || b[2] <= d[0] || d[3] <= b[1] || b[3] <= d[1] {
                    continue;
                }
                let mut clipped = layer.clone();
                clipped["viewport_clip"] = json!(rect);
                ops.push(json!({"op":"draw","id":SURFACE,"layer":clipped}));
            }
        }
        if !ops.is_empty() {
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"canvases":ops}))?)?;
        }
        self.previous = current;
        Ok(())
    }
}

fn bounds(layer: &Value) -> Rect {
    let rect = &layer["rect"];
    let num = |v: &Value, fallback| v.as_f64().unwrap_or(fallback);
    let x = num(&rect[0], 0.);
    let y = num(&rect[1], 0.);
    let w = num(&rect[2], 0.);
    let h = num(&rect[3], 0.);
    let t = &layer["transform"];
    let px = num(&t["pivot_x"], 0.);
    let py = num(&t["pivot_y"], 0.);
    let sx = num(&t["scale_x"], 1.);
    let sy = num(&t["scale_y"], 1.);
    let (sin, cos) = num(&t["rotate"], 0.).sin_cos();
    let mut edges = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (dx, dy) in [(0., 0.), (w, 0.), (0., h), (w, h)] {
        let (a, b) = ((dx - px) * sx, (dy - py) * sy);
        let (cx, cy) = (x + px + a * cos - b * sin, y + py + a * sin + b * cos);
        edges = [
            edges[0].min(cx),
            edges[1].min(cy),
            edges[2].max(cx),
            edges[3].max(cy),
        ];
    }
    [
        edges[0].floor() as i32,
        edges[1].floor() as i32,
        edges[2].ceil() as i32,
        edges[3].ceil() as i32,
    ]
}
