// SPDX-License-Identifier: MPL-2.0
//! `UICamera` event delivery for the ported NGUI scripts: the topmost
//! collider widget under the pointer receives the press, `UIButton.onClick`
//! delegates (`EventDelegate.mTarget` component class + `mMethodName`) name
//! the Rust handler, `UISlider` follows the dragged pointer along its bar
//! and fires `onChange`, `UIInput` takes committed text from a host text
//! input session while selected.
use crate::{
    ngui::{Hit, NodeState},
    unity_scene::{Component, Scene},
};
use crysirror_unity::managed::{Fields, Value};
use std::collections::HashMap;

/// `Class.Method` of each delegate in a serialized `List<EventDelegate>`.
pub fn delegates(scene: &Scene, list: Option<&[Value]>) -> Vec<String> {
    let mut out = Vec::new();
    for v in list.unwrap_or(&[]) {
        let Value::Struct(d) = v else { continue };
        let (Some(target), Some(method)) = (d.pptr("mTarget"), d.string("mMethodName")) else { continue };
        if target.is_null() || method.is_empty() {
            continue;
        }
        // Same-file component of some node of this scene.
        let class = scene.nodes.iter().find_map(|n| {
            n.components.iter().find_map(|(k, c)| match c {
                Component::Script { class, .. } if k.path_id == target.path_id => Some(class.clone()),
                _ => None,
            })
        });
        if let Some(class) = class {
            out.push(format!("{class}.{method}"));
        }
    }
    out
}

/// The handlers a click on `node` runs: a run-time `EventDelegate.Add`
/// first, then the serialized `UIButton.onClick` list.
pub fn click_handlers(scene: &Scene, states: &HashMap<usize, NodeState>, node: usize) -> Vec<String> {
    let mut out: Vec<String> = states.get(&node).and_then(|s| s.click.clone()).into_iter().collect();
    if let Some(b) = scene.script(node, "UIButton") {
        out.extend(delegates(scene, b.array("onClick")));
    }
    out
}

/// Topmost hit under `(x, y)` (highest panel depth, then widget depth,
/// then later in draw order).
pub fn pick(hits: &[Hit], x: f32, y: f32) -> Option<Hit> {
    hits.iter()
        .enumerate()
        .filter(|(_, h)| x >= h.rect.x0 && x < h.rect.x1 && y >= h.rect.y0 && y < h.rect.y1)
        .max_by_key(|(i, h)| (h.order, *i))
        .map(|(_, h)| *h)
}

/// The slider owning `node`: the node itself or the parent of its thumb.
pub fn slider_of(scene: &Scene, node: usize) -> Option<usize> {
    if scene.script(node, "UISlider").is_some() {
        return Some(node);
    }
    let p = scene.nodes[node].parent?;
    scene.script(p, "UISlider").map(|_| p)
}

/// `UISlider` value at pointer x over its bar rectangle (horizontal).
pub fn slider_value(rect: crate::ngui::Rect2, x: f32, fields: &Fields) -> f32 {
    let w = (rect.x1 - rect.x0).max(1.);
    let mut v = ((x - rect.x0) / w).clamp(0., 1.);
    if fields.bool("mInverted").unwrap_or(false) {
        v = 1. - v;
    }
    let steps = fields.int("numberOfSteps").unwrap_or(0);
    if steps > 1 {
        let s = (steps - 1) as f32;
        v = (v * s).round() / s;
    }
    v
}
