// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! layerExBTOA (wamsoft a57e7a25) province members: copyAlphaToProvince,
//! fillToProvince and fillByProvince over the layer's shared RegionMap and
//! its resident image. All three work inside the layer's clip rectangle.
use crate::natives::{Rt, clip_of, native_extra::prepare, this_of};
use crysirror_core::{
    image_processing::{ImageInput, RgbaImage},
    region_map::{LabelPlane, alpha_to_labels},
    state::ClientId,
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};
use std::sync::Arc;

fn err(e: impl std::fmt::Display) -> TjsError {
    TjsError::runtime(e.to_string())
}
fn exist(a: &[Variant], i: usize) -> Option<&Variant> {
    a.get(i).filter(|v| !matches!(v, Variant::Void))
}
/// The clip rectangle inside the image, `None` when empty.
fn clip(rt: &Rt, h: ObjectHandle) -> Result<Option<[u32; 4]>> {
    let l = rt.host().layer(h)?;
    let [x, y, w, hh] = clip_of(l);
    let (iw, ih) = (l.image_width, l.image_height);
    let (x0, y0) = (x.clamp(0, iw), y.clamp(0, ih));
    let (x1, y1) = ((x + w).clamp(x0, iw), (y + hh).clamp(y0, ih));
    Ok((x1 > x0 && y1 > y0).then_some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]))
}
/// Full resident RGBA of a drawable layer (after `prepare`).
pub(crate) fn pixels(rt: &mut Rt, h: ObjectHandle, what: &str) -> Result<([u32; 2], Arc<[u8]>)> {
    if !rt.host().layer(h)?.has_image {
        return Err(TjsError::runtime(format!("{what}: must be Layer.")));
    }
    prepare(rt, h)?;
    match crate::image_api::input(rt, h)? {
        ImageInput::Resource(r) => Ok((r.size(), r.rgba())),
        ImageInput::Solid { size, color } => {
            Ok((size, color.iter().copied().cycle().take(size[0] as usize * size[1] as usize * 4).collect()))
        }
        _ => Err(TjsError::runtime(format!("{what}: image not resident"))),
    }
}
fn region(rgba: &[u8], width: u32, rect: [u32; 4]) -> impl Iterator<Item = &[u8]> {
    let [x, y, w, h] = rect;
    (y..y + h).flat_map(move |py| {
        let start = ((py * width + x) * 4) as usize;
        rgba[start..start + w as usize * 4].chunks_exact(4)
    })
}
pub(crate) fn paint(rt: &mut Rt, h: ObjectHandle, rect: [u32; 4], labels: Vec<u8>) -> Result<()> {
    let client = rt.host().client;
    let plane = LabelPlane::new(client, [rect[2], rect[3]], labels).map_err(err)?;
    let layer = rt.host_mut().layer_mut(h)?;
    layer.regions.paint_plane(client, [rect[0].into(), rect[1].into()], Arc::new(plane)).map_err(err)?;
    layer.province_allocated = true;
    Ok(())
}

pub(crate) fn copy_alpha_to_province(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.copyAlphaToProvince";
    let h = this_of(rt, t, M)?;
    let threshold = exist(&a, 0).map(|v| v.to_integer()).transpose()?;
    let matched = exist(&a, 1).map(|v| v.to_integer()).transpose()?.unwrap_or(1);
    let otherwise = exist(&a, 2).map(|v| v.to_integer()).transpose()?.unwrap_or(0);
    let (size, rgba) = pixels(rt, h, M)?;
    let Some(rect) = clip(rt, h)? else { return Ok(Variant::Void) };
    let alpha: Vec<u8> = region(&rgba, size[0], rect).map(|p| p[3]).collect();
    let old = rt.host().layer(h)?.regions.labels(rt.host().client, rect).map_err(err)?;
    if let Some(labels) = alpha_to_labels(&alpha, &old, threshold, matched, otherwise) {
        paint(rt, h, rect, labels)?;
    }
    Ok(Variant::Void)
}

pub(crate) fn fill_to_province(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.fillToProvince";
    if a.len() < 2 {
        return Err(TjsError::bad_param_count());
    }
    let h = this_of(rt, t, M)?;
    // The native tests an unsigned colour for `< 0`, so every pixel must
    // match the RGB value.
    let color = (a[0].to_integer()? as u32) & 0xff_ffff;
    let index = a[1].to_integer()? as u8;
    let threshold = exist(&a, 2).map(|v| v.to_integer()).transpose()?.unwrap_or(64);
    let (size, rgba) = pixels(rt, h, M)?;
    let Some(rect) = clip(rt, h)? else { return Ok(Variant::Void) };
    let old = rt.host().layer(h)?.regions.labels(rt.host().client, rect).map_err(err)?;
    let (mut min, mut max) = ([i64::MAX; 2], [-1i64; 2]);
    let w = rect[2] as usize;
    let labels: Vec<u8> = region(&rgba, size[0], rect)
        .zip(old)
        .enumerate()
        .map(|(i, (p, o))| {
            let rgb = (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]);
            if rgb == color && i64::from(p[3]) >= threshold {
                let xy = [(i % w) as i64, (i / w) as i64];
                for k in 0..2 {
                    min[k] = min[k].min(xy[k]);
                    max[k] = max[k].max(xy[k]);
                }
                index
            } else {
                o as u8
            }
        })
        .collect();
    paint(rt, h, rect, labels)?;
    if max[0] < 0 {
        return Ok(Variant::Void);
    }
    let (l, tp) = (i64::from(rect[0]), i64::from(rect[1]));
    let arr = rt.alloc_array_object(vec![
        Variant::Integer(min[0] + l),
        Variant::Integer(min[1] + tp),
        Variant::Integer(max[0] + l - min[0] + 1),
        Variant::Integer(max[1] + tp - min[1] + 1),
    ]);
    Ok(Variant::Object(arr))
}

pub(crate) fn fill_by_province(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.fillByProvince";
    if a.len() < 2 {
        return Err(TjsError::bad_param_count());
    }
    let h = this_of(rt, t, M)?;
    let index = u32::from(a[0].to_integer()? as u8);
    let color = a[1].to_integer()? as u32;
    let (size, rgba) = pixels(rt, h, M)?;
    let Some(rect) = clip(rt, h)? else { return Ok(Variant::Void) };
    let labels = rt.host().layer(h)?.regions.labels(rt.host().client, rect).map_err(err)?;
    let px = [(color >> 16) as u8, (color >> 8) as u8, color as u8, (color >> 24) as u8];
    let mut out = rgba.to_vec();
    let [x, y, w, _] = rect;
    for (i, label) in labels.iter().enumerate() {
        if *label == index {
            let (cx, cy) = (x + i as u32 % w, y + i as u32 / w);
            let o = ((cy * size[0] + cx) * 4) as usize;
            out[o..o + 4].copy_from_slice(&px);
        }
    }
    let image = RgbaImage::new(ClientId(rt.host().client), size, out.into()).map_err(err)?;
    crate::image_api::queue_chain(rt, h, ImageInput::Resource(Arc::new(image)), vec![])
}
