//! Bitmap native resource/shape policy over the same shared Canvas/Image model
//! as Layer. Detached objects never acquire a window or renderer.
use super::*;
fn bitmap(rt: &Rt, v: &Variant) -> Result<ObjectHandle> {
    let h = object_arg(v)
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .ok_or_else(|| TjsError::runtime("Bitmap argument required"))?;
    if !rt.object_class_infos(h).iter().any(|c| c == "Bitmap") {
        return Err(TjsError::runtime("Bitmap argument required"));
    }
    rt.host().layer(h)?;
    Ok(h)
}
pub(super) fn copy_to(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    let source = this_of(rt, t, "Layer.copyToBitmapFromMainImage")?;
    let target = bitmap(rt, a.first().ok_or_else(TjsError::bad_param_count)?)?;
    assign(rt, target, source, true)
}
pub(super) fn copy_from(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    let target = this_of(rt, t, "Layer.copyFromBitmapToMainImage")?;
    let source = bitmap(rt, a.first().ok_or_else(TjsError::bad_param_count)?)?;
    assign(rt, target, source, false)
}
fn assign(
    rt: &mut Rt,
    target: ObjectHandle,
    source: ObjectHandle,
    resize: bool,
) -> Result<Variant> {
    let s = rt.host().layer(source)?.clone();
    let d = rt.host().layer(target)?.clone();
    if !s.has_image || !d.has_image {
        return Err(TjsError::runtime("Not drawable bitmap"));
    }
    if !resize && [s.image_width, s.image_height] != [d.image_width, d.image_height] {
        return Err(TjsError::runtime("bitmap/main image extent mismatch"));
    }
    if target == source {
        return Ok(Variant::Void);
    }
    if resize {
        rt.host_mut()
            .reset_surface(target, s.image_width, s.image_height, [0; 4])?;
    }
    rt.host_mut().canvas_op(json!({"op":"copy","source":s.slot,"destination":d.slot,"rect":[0,0,s.image_width,s.image_height],"position":[0,0],"opacity":255,"replace":true}));
    let d = rt.host_mut().layer_mut(target)?;
    d.image_source = s.image_source;
    d.full_alpha = s.full_alpha;
    d.blank = s.blank;
    d.image_modified = true;
    d.dirty = true;
    Ok(Variant::Void)
}
pub(crate) fn install(rt: &mut Rt) {
    let c = native_class(rt, "Bitmap", |rt, t, a| {
        let h = this_of(rt, t, "Bitmap")?;
        if !rt.host().layers.contains_key(&h.0) {
            rt.host_mut().reserve_layer(h, 0)?;
            rt.host_mut().layer_mut(h)?.attached = false;
        }
        if a.first().is_some_and(|v| matches!(v, Variant::String(_))) {
            rt.host_mut().reset_surface(h, 32, 32, [255, 255, 255, 0])?;
            return layer_load_images(rt, Some(h), a);
        }
        if !a.is_empty() && a.len() < 2 {
            return Err(TjsError::bad_param_count());
        }
        if int(&a, 2, 32)? != 32 {
            return Err(unsupported("Bitmap bit depth", "32-bit shared RGBA bitmap"));
        }
        // The native allocated bitmap is uninitialised. Use deterministic zero
        // bytes rather than inheriting allocator memory in portable resources.
        rt.host_mut().reset_surface(
            h,
            int(&a, 0, 32)?,
            int(&a, 1, 32)?,
            if a.is_empty() {
                [255, 255, 255, 0]
            } else {
                [0; 4]
            },
        )?;
        Ok(Variant::Void)
    });
    methods(
        rt,
        c,
        &[
            ("finalize", |_, _, _| Ok(Variant::Void)),
            ("getPixel", |rt, t, a| {
                let h = this_of(rt, t, "Bitmap.getPixel")?;
                prepare(rt, h)?;
                layer_get_main_pixel(rt, Some(h), a)
            }),
            ("getMaskPixel", mask_pixel),
            ("setMaskPixel", set_mask_pixel),
            ("setPixel", |rt, t, a| {
                let h = this_of(rt, t, "Bitmap.setPixel")?;
                let x = need_int(&a, 0, "setPixel")?;
                let y = need_int(&a, 1, "setPixel")?;
                let color = argb(need_int(&a, 2, "setPixel")?, Some(255));
                let l = rt.host().layer(h)?;
                if x < 0 || y < 0 || x >= l.image_width || y >= l.image_height {
                    return Err(TjsError::runtime("pixel out of rectangle"));
                }
                prepare(rt, h)?;
                let input =
                    crate::image_api::input(rt, h)?.with_composite_layer(ImageCompositeLayer {
                        input: Box::new(ImageInput::Solid {
                            size: [1, 1],
                            color,
                        }),
                        source: [0, 0, 1, 1],
                        destination: [x as i32, y as i32],
                        clip: None,
                        opacity: 255,
                        math: ImageCompositeMath::CopyRgb,
                    });
                crate::image_api::queue_chain(rt, h, input, vec![])
            }),
            ("setSize", |rt, t, a| {
                let h = this_of(rt, t, "Bitmap.setSize")?;
                rt.host_mut().resize_surface(
                    h,
                    need_int(&a, 0, "setSize")?,
                    need_int(&a, 1, "setSize")?,
                )?;
                Ok(Variant::Void)
            }),
            ("copyFrom", |rt, t, a| {
                let h = this_of(rt, t, "Bitmap.copyFrom")?;
                let src = bitmap(rt, a.first().ok_or_else(TjsError::bad_param_count)?)?;
                assign(rt, h, src, true)
            }),
            ("load", layer_load_images),
            ("save", crate::image_api::save_layer_image),
            ("independ", |rt, t, a| {
                let h = this_of(rt, t, "Bitmap.independ")?;
                if a.first().is_some_and(|v| !v.is_truthy()) {
                    let l = rt.host().layer(h)?;
                    let (w, height) = (l.image_width, l.image_height);
                    rt.host_mut().reset_surface(h, w, height, [0; 4])?;
                }
                Ok(Variant::Void)
            }),
        ],
    );
    for (name, width) in [("width", true), ("height", false)] {
        rt.register_object_native_property(
            c,
            name,
            move |rt: &mut Rt, t| {
                let h = this_of(rt, t, "Bitmap extent")?;
                let l = rt.host().layer(h)?;
                Ok(Variant::Integer(if width {
                    l.image_width
                } else {
                    l.image_height
                }))
            },
            move |rt: &mut Rt, t, v| {
                let h = this_of(rt, t, "Bitmap extent")?;
                let l = rt.host().layer(h)?;
                let (mut w, mut ht) = (l.image_width, l.image_height);
                if width {
                    w = v.to_integer()?;
                } else {
                    ht = v.to_integer()?;
                }
                rt.host_mut().resize_surface(h, w, ht)?;
                Ok(())
            },
        );
    }
}
