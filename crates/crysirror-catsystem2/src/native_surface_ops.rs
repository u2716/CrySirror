// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! The 7409C0 image group natives (710..737): each edits a bitmap in place. Every
//! edit here is a description for the shared Image job; pixels never live in the VM.
use crate::native_image_resource::Prepared;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput};
use serde_json::{json, Value};
use std::sync::Arc;

/// One bitmap edit: the job's input and effects, and the geometry it leaves behind.
pub(crate) struct Edit {
    pub input: ImageInput,
    pub effects: Vec<Value>,
    pub canvas: [u32; 2],
    pub origin: [i32; 2],
}
fn canvas_of(frame: &Prepared) -> [u32; 2] {
    let g = &frame.geometry;
    if g.canvas[0] != 0 && g.canvas[1] != 0 { g.canvas } else { g.size }
}
fn unchanged(frame: &Prepared, effects: Vec<Value>) -> Edit {
    Edit { input: ImageInput::Resource(frame.image.clone()), effects, canvas: canvas_of(frame), origin: frame.geometry.origin }
}
/// 68D6E0: each channel scaled by a percentage (floor, clamped).
pub(crate) fn tone(frame: &Prepared, percent: [u32; 3]) -> Edit {
    let table = |p: u32| (0..256u32).map(|v| (v * p / 100).min(255)).collect::<Vec<_>>();
    unchanged(frame, vec![json!({"kind":"rgb_lookup","spec":{"red":table(percent[0]),"green":table(percent[1]),"blue":table(percent[2]),"profile":"straight_rgba"}})])
}
/// 68C000 (1), 68C1E0 (2), 68C180 (3).
pub(crate) fn colour_mode(frame: &Prepared, mode: u32) -> Option<Edit> {
    // 0.298912, 0.586611, 0.114478 truncated one channel at a time.
    let luma = json!({"kind":"fixed_luma","spec":{"coefficients":[19589,38444,7503],"groups":[0,1,2],"fractional_bits":16,"amount256":256}});
    let shifted = |by: i32| (0..256i32).map(|v| (v + by).clamp(0, 255)).collect::<Vec<_>>();
    Some(unchanged(frame, match mode {
        1 => vec![luma],
        2 => vec![luma, json!({"kind":"rgb_lookup","spec":{"red":shifted(30),"green":shifted(5),"blue":shifted(-35),"profile":"straight_rgba"}})],
        3 => vec![json!({"kind":"invert"})],
        _ => return None,
    }))
}
/// 68C4D0: mirror, nonzero horizontally. The bitmap keeps its place on the mirrored canvas.
pub(crate) fn flip(frame: &Prepared, horizontal: bool) -> Edit {
    let canvas = canvas_of(frame);
    let (axis, size) = if horizontal { (0, frame.image.size()[0]) } else { (1, frame.image.size()[1]) };
    let mut edit = unchanged(frame, vec![json!({"kind":"flip","horizontal":horizontal,"vertical":!horizontal})]);
    edit.origin[axis] = canvas[axis] as i32 - size as i32 - frame.geometry.origin[axis];
    edit
}
/// 68CCC0: a clockwise quarter turn of bitmap and canvas; other angles change nothing.
pub(crate) fn rotate(frame: &Prepared, degrees: u32) -> Option<Edit> {
    if !matches!(degrees, 90 | 180 | 270) {
        return None;
    }
    let canvas = canvas_of(frame);
    let size = frame.image.size();
    let [x, y] = frame.geometry.origin;
    let right = canvas[0] as i32 - size[0] as i32 - x;
    let bottom = canvas[1] as i32 - size[1] as i32 - y;
    let mut edit = unchanged(frame, vec![json!({"kind":"rotate","degrees":degrees})]);
    match degrees {
        90 => { edit.canvas = [canvas[1], canvas[0]]; edit.origin = [bottom, x]; }
        180 => edit.origin = [right, bottom],
        _ => { edit.canvas = [canvas[1], canvas[0]]; edit.origin = [y, right]; }
    }
    Some(edit)
}
/// 68D150: scale so the canvas becomes `to`.
pub(crate) fn scale(frame: &Prepared, to: [u32; 2]) -> Option<Edit> {
    let canvas = canvas_of(frame);
    let size = frame.image.size();
    if to[0] == 0 || to[1] == 0 || canvas[0] == 0 || canvas[1] == 0 {
        return None;
    }
    let scaled = |value: u32, axis: usize| ((f64::from(value) * f64::from(to[axis]) / f64::from(canvas[axis])) + 0.5) as u32;
    let new = [scaled(size[0], 0).max(1), scaled(size[1], 1).max(1)];
    let mut edit = unchanged(frame, vec![json!({"kind":"resize","width":new[0],"height":new[1]})]);
    edit.canvas = to;
    edit.origin = [
        (f64::from(frame.geometry.origin[0]) * f64::from(to[0]) / f64::from(canvas[0])) as i32,
        (f64::from(frame.geometry.origin[1]) * f64::from(to[1]) / f64::from(canvas[1])) as i32,
    ];
    Some(edit)
}
/// 68D840 + 68B490: grow by the blur reach (within the canvas), then blur.
pub(crate) fn blur(frame: &Prepared, reach: [u32; 2]) -> Edit {
    let canvas = canvas_of(frame);
    let size = frame.image.size();
    let [x, y] = frame.geometry.origin;
    let left = (reach[0] as i32).min(x).max(0);
    let top = (reach[1] as i32).min(y).max(0);
    let right = (reach[0] as i32).min(canvas[0] as i32 - size[0] as i32 - x).max(0);
    let bottom = (reach[1] as i32).min(canvas[1] as i32 - size[1] as i32 - y).max(0);
    let grown = [(size[0] as i32 + left + right) as u32, (size[1] as i32 + top + bottom) as u32];
    Edit {
        input: ImageInput::Composite {
            base: Box::new(ImageInput::Solid { size: grown, color: [0; 4] }),
            layers: vec![ImageCompositeLayer { input: Box::new(ImageInput::Resource(frame.image.clone())), source: [0, 0, size[0] as i32, size[1] as i32], destination: [left, top], clip: None, opacity: 255, math: ImageCompositeMath::Copy }],
        },
        effects: vec![json!({"kind":"box_blur","area":[-(reach[0] as i32),-(reach[1] as i32),reach[0] as i32,reach[1] as i32],"alpha_weighted":true})],
        canvas,
        origin: [x - left, y - top],
    }
}
/// 64A3E0 → 68B490 → 68D840: only the growth blur uses, without the blur.
pub(crate) fn grow(frame: &Prepared, reach: [u32; 2]) -> Edit {
    let mut edit = blur(frame, reach);
    edit.effects.clear();
    edit
}
/// 68C460 -> 688440: a rectangle of one ARGB colour replaces what was there.
pub(crate) fn fill(frame: &Prepared, rect: [i32; 4], argb: u32) -> Edit {
    let color = [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8, (argb >> 24) as u8];
    let mut edit = unchanged(frame, Vec::new());
    edit.input = ImageInput::Composite {
        base: Box::new(ImageInput::Resource(frame.image.clone())),
        layers: vec![ImageCompositeLayer { input: Box::new(ImageInput::Solid { size: [rect[2].max(1) as u32, rect[3].max(1) as u32], color }), source: [0, 0, rect[2], rect[3]], destination: [rect[0], rect[1]], clip: None, opacity: 255, math: ImageCompositeMath::Copy }],
    };
    edit
}
pub(crate) fn effects_bytes(effects: &[Value]) -> Arc<[u8]> {
    Arc::from(serde_json::to_vec(effects).unwrap_or_else(|_| b"[]".to_vec()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_descriptions_parse_as_shared_effects() {
        for value in [
            json!({"kind":"rotate","degrees":90}),
            json!({"kind":"invert"}),
            json!({"kind":"fixed_luma","spec":{"coefficients":[19589,38444,7503],"groups":[0,1,2],"fractional_bits":16,"amount256":256}}),
        ] {
            let effect: crysirror_image::effects::ImageEffect = serde_json::from_value(value).unwrap();
            effect.validate().unwrap();
        }
    }
}
