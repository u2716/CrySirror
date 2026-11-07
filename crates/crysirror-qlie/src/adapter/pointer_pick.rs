//! Point tests over the engine-held collision tables.
//!
//! - Image `?hit[,x,y]` (5D1454): when the object's input group (+BED, `in`)
//!   shares a bit with the thread's input mask, answer 1 if the point (the
//!   pointer by default) hits the TABMP collision table. Plain images have
//!   no collision regions. `bar\?hit` uses the slider's own rectangle.
//! - `inp\?hit[,x,y]` (6B03C0 with kind mask 1022): the visible objects of
//!   kinds 1..9 hit at the point, topmost first, at most 8: the count, then
//!   their names. A hit object whose `mi` byte (+BE4, vmt+40) is set stops
//!   the search below it.
//! - `inp\bs,dir` moves the pointer between the button-select points of
//!   visible img/msg/btn/simg/sbtn objects (`sp`, object-relative, +BF5;
//!   objects whose point is 0,0 do not take part). `up/down/left/right`
//!   (6B0BA4) weigh the movement axis 1 and the cross axis 10; `tabnext`,
//!   `tabprev`, `tab100next`, `tab100prev` (6B1A0C) step through the
//!   points in `tab` order (+BFD), rows being tab / 100. Points go through
//!   the object's pose (+3237 matrix): its layer affine and those of its
//!   parent layers.
use super::*;
use crysirror_core::service::EngineQuery;

const KINDS_1_TO_9: &[&str] = &["sys", "dat", "sdat", "snd", "dsnd", "img", "msg", "btn", "bar"];
const SELECTABLE: &[&str] = &["img", "msg", "btn", "simg", "sbtn"];

impl QlieVm {
    fn property_int(&self, id: &str, key: &str, index: usize) -> Option<i32> {
        self.objects[id]
            .properties
            .get(key)
            .and_then(|v| v.get(index))
            .and_then(|v| v.integer().ok())
    }
    fn input_mask(&self, thread: ThreadKind) -> i32 {
        self.objects
            .get(&QlieVm::builtin("sys", thread))
            .and_then(|s| s.properties.get("in"))
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0xFFFFFFF)
    }
    fn group_admitted(&self, id: &str) -> bool {
        let o = &self.objects[id];
        self.property_int(id, "in", 0).unwrap_or(255) & self.input_mask(o.thread) != 0
    }
    /// Maps an object-local point to canvas pixels through its layer chain.
    fn layer_point(&self, h: &mut dyn HostServices, id: &str, point: [f32; 2]) -> Result<[f32; 2]> {
        let o = &self.objects[id];
        if !o.created {
            return Ok([o.position[0] as f32 + point[0], o.position[1] as f32 + point[1]]);
        }
        let mut layer = Some(o.layer);
        let mut point = point;
        for _ in 0..64 {
            let Some(id) = layer else { break };
            let spec: Option<crysirror_scene::layer::LayerSpec> =
                serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(id))?)?;
            let Some(spec) = spec else { break };
            point = crysirror_scene::layer::local_to_parent(&spec, point);
            layer = spec.parent;
        }
        Ok(point)
    }
    fn collision_hit(&self, h: &mut dyn HostServices, id: &str, x: i32, y: i32) -> Result<bool> {
        let o = &self.objects[id];
        if o.kind == "bar" {
            return Ok(o.ui_rect.is_some_and(|r| {
                (x as f32) >= r[0] && (y as f32) >= r[1] && (x as f32) < r[0] + r[2] && (y as f32) < r[1] + r[3]
            }));
        }
        let Some(bounds) = o.hit_bounds else {
            return Ok(false);
        };
        let rect = if o.hit_world_space {
            bounds
        } else {
            let origin = self.layer_point(h, id, [0., 0.])?;
            [bounds[0] + origin[0], bounds[1] + origin[1], bounds[2], bounds[3]]
        };
        Ok(match &o.hit_mask {
            Some(mask) => mask.hit(x, y, rect),
            None => {
                let (x, y) = (x as f32, y as f32);
                x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
            }
        })
    }
    fn query_point(&self, a: &[Value]) -> Result<[i32; 2]> {
        let given = |i: usize| a.get(i).is_some_and(|v| !v.text().is_empty());
        Ok(if given(0) && given(1) {
            [a[0].integer()?, a[1].integer()?]
        } else {
            [self.input.pointer_x, self.input.pointer_y]
        })
    }
    /// Image-family and bar `?hit`.
    pub(super) fn object_hit_query(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        a: &[Value],
    ) -> Result<()> {
        ensure!(a.len() <= 2, "QLIE hit query ABI");
        let [x, y] = self.query_point(a)?;
        if self.group_admitted(id) && self.collision_hit(h, id, x, y)? {
            m.result(0, Value::Int(1));
        }
        Ok(())
    }
    /// `inp\?hit`.
    pub(super) fn input_hit_query(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        a: &[Value],
    ) -> Result<()> {
        ensure!(a.len() <= 2, "QLIE input hit query ABI");
        let [x, y] = self.query_point(a)?;
        let mut candidates: Vec<(i32, u32, String)> = self
            .objects
            .iter()
            .filter(|(_, o)| o.visible && KINDS_1_TO_9.contains(&o.kind.as_str()))
            .map(|(id, o)| (o.position[2], o.layer, id.clone()))
            .collect();
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let mut hits = Vec::new();
        for (_, _, id) in candidates {
            if hits.len() == 8 {
                break;
            }
            if self.group_admitted(&id) && self.collision_hit(h, &id, x, y)? {
                let stops = self.property_int(&id, "mi", 0).unwrap_or(0) != 0;
                hits.push(id);
                if stops {
                    break;
                }
            }
        }
        m.result(0, Value::Int(hits.len() as i32));
        for name in hits {
            m.result_text(0, name);
        }
        Ok(())
    }
    fn select_points(&self, h: &mut dyn HostServices) -> Result<Vec<([i32; 2], i32)>> {
        let canvas = self.config.canvas;
        let mut ids: Vec<(u32, String)> = self
            .objects
            .iter()
            .filter(|(_, o)| o.visible && SELECTABLE.contains(&o.kind.as_str()))
            .map(|(id, o)| (o.layer, id.clone()))
            .collect();
        ids.sort();
        let mut points = Vec::new();
        for (_, id) in ids {
            let sp = [
                self.property_int(&id, "sp", 0).unwrap_or(0),
                self.property_int(&id, "sp", 1).unwrap_or(0),
            ];
            if sp == [0, 0] || !self.group_admitted(&id) {
                continue;
            }
            let mapped = self.layer_point(h, &id, [sp[0] as f32, sp[1] as f32])?;
            let point = [(mapped[0] + 0.5).trunc() as i32, (mapped[1] + 0.5).trunc() as i32];
            if point[0] >= 0 && point[1] >= 0 && point[0] <= canvas[0] as i32 && point[1] <= canvas[1] as i32 {
                points.push((point, self.property_int(&id, "tab", 0).unwrap_or(0)));
            }
        }
        Ok(points)
    }
    /// `inp\bs,dir`.
    pub(super) fn button_select(&mut self, h: &mut dyn HostServices, direction: &str) -> Result<()> {
        let pointer = [self.input.pointer_x, self.input.pointer_y];
        let points = self.select_points(h)?;
        let target = match direction.to_ascii_lowercase().as_str() {
            "up" => directional(&points, pointer, [0, -1]),
            "down" => directional(&points, pointer, [0, 1]),
            "left" => directional(&points, pointer, [-1, 0]),
            "right" => directional(&points, pointer, [1, 0]),
            "tabnext" => tab_step(&points, pointer, 1),
            "tabprev" => tab_step(&points, pointer, -1),
            "tab100next" => tab_step(&points, pointer, 100),
            "tab100prev" => tab_step(&points, pointer, -100),
            _ => return Ok(()),
        };
        self.submit(h, json!({"ui":[{"op":"warp_pointer","position":target}]}))
    }
}

/// 6B0BA4.
fn directional(points: &[([i32; 2], i32)], p: [i32; 2], d: [i32; 2]) -> [i32; 2] {
    if points.is_empty() {
        return p;
    }
    let weight = [if d[0] != 0 { 1 } else { 10 }, if d[1] != 0 { 1 } else { 10 }];
    let cost = |c: [i32; 2]| weight[0] * (c[0] - p[0]).abs() + weight[1] * (c[1] - p[1]).abs();
    let (mut best, mut score) = (p, 0x10000);
    for (c, _) in points {
        let (dx, dy) = (c[0] - p[0], c[1] - p[1]);
        if dx.abs() < 4 && dy.abs() < 4 {
            continue;
        }
        let (ax, ay) = (d[0] * dx, d[1] * dy);
        if ax < 0 || ay < 0 || (ax + ay).abs() < 4 {
            continue;
        }
        if (d[0] != 0 && dy.abs() < 3) || (d[1] != 0 && dx.abs() < 3) {
            let s = (dy.abs() + dx.abs()) / 4;
            if s < score {
                (best, score) = (*c, s);
            }
        }
        if cost(*c) < score {
            (best, score) = (*c, cost(*c));
        }
    }
    if score == 0x10000 {
        for (c, _) in points {
            if cost(*c) < score {
                (best, score) = (*c, cost(*c));
            }
        }
        if score < 50 {
            best = p;
        }
    }
    best
}

/// 6B1A0C; points are visited in ascending tab order.
fn tab_step(points: &[([i32; 2], i32)], p: [i32; 2], delta: i32) -> [i32; 2] {
    if points.is_empty() {
        return p;
    }
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by_key(|i| points[*i].1);
    let n = order.len() as i32;
    let at = |k: i32| &points[order[k.rem_euclid(n) as usize]];
    let current = order
        .iter()
        .position(|i| (points[*i].0[0] - p[0]).abs() < 2 && (points[*i].0[1] - p[1]).abs() < 2);
    let Some(current) = current.filter(|_| n != 1).map(|c| c as i32) else {
        let mut best = (p, 0xFFFFFF);
        for (c, _) in points {
            let (dx, dy) = (f64::from(c[0] - p[0]), f64::from(c[1] - p[1]));
            let d = (dx * dx + dy * dy).sqrt().trunc() as i32;
            if d < best.1 {
                best = (*c, d);
            }
        }
        return best.0;
    };
    if delta.abs() != 100 {
        return at(current + delta).0;
    }
    let tab = at(current).1;
    let (row, column) = (tab - tab % 100, tab % 100);
    let mut k = current;
    let mut steps = 0;
    while steps < n {
        k = (k + delta.signum()).rem_euclid(n);
        let t = at(k).1;
        if t - t % 100 != row {
            break;
        }
        steps += 1;
    }
    if steps == n {
        return at(current + delta.signum()).0;
    }
    let row = at(k).1 / 100;
    let mut best = (p, 0xFFFFFF);
    for (c, t) in points {
        if t / 100 == row && (column - t % 100).abs() < best.1 {
            best = (*c, (column - t % 100).abs());
        }
    }
    best.0
}

#[cfg(test)]
mod tests {
    #[test]
    fn directional_prefers_the_movement_axis() {
        let points = [([100, 0], 0), ([0, 100], 0), ([100, 100], 0)];
        assert_eq!(super::directional(&points, [0, 0], [1, 0]), [100, 0]);
        assert_eq!(super::directional(&points, [0, 0], [0, 1]), [0, 100]);
    }
    #[test]
    fn tab_steps_wrap_in_tab_order() {
        let points = [([10, 10], 2), ([20, 20], 1), ([30, 30], 3)];
        assert_eq!(super::tab_step(&points, [20, 20], 1), [10, 10]);
        assert_eq!(super::tab_step(&points, [20, 20], -1), [30, 30]);
    }
}
