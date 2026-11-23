// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! layerExBTOA (wamsoft a57e7a25) and shrinkCopy (wamsoft 007fbba3) Layer
//! members, also bundled in PackinOne, over `crysirror_image::layer_ex`.
//! clipAlphaRect lives in `image_api`.
use crate::natives::{Rt, need_int, object_arg, this_of, clip_of};
use crysirror_image::{effects::ImageEffect, layer_ex};
use crysirror_tjs2::{Result, TjsError, runtime::{ObjectHandle, Variant}};
use serde_json::json;

fn layer_arg(rt: &Rt, v: Option<&Variant>, what: &str) -> Result<ObjectHandle> {
    v.and_then(object_arg)
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .ok_or_else(|| TjsError::runtime(format!("{what}: src must be Layer.")))
}
fn unary(rt: &mut Rt, t: Option<ObjectHandle>, what: &str, effect: ImageEffect) -> Result<Variant> {
    let h = this_of(rt, t, what)?;
    if !rt.host().layer(h)?.has_image {
        return Err(TjsError::runtime("dest must be Layer."));
    }
    let value = serde_json::to_value(&effect).map_err(|e| TjsError::runtime(e.to_string()))?;
    crate::image_api::begin(rt, Some(h), value)
}

pub(crate) fn copy_right_blue_to_left_alpha(rt: &mut Rt, t: Option<ObjectHandle>, _: Vec<Variant>) -> Result<Variant> {
    unary(rt, t, "Layer.copyRightBlueToLeftAlpha", ImageEffect::BlueToAlpha { spec: layer_ex::BlueToAlpha { vertical: false } })
}
pub(crate) fn copy_bottom_blue_to_top_alpha(rt: &mut Rt, t: Option<ObjectHandle>, _: Vec<Variant>) -> Result<Variant> {
    unary(rt, t, "Layer.copyBottomBlueToTopAlpha", ImageEffect::BlueToAlpha { spec: layer_ex::BlueToAlpha { vertical: true } })
}
pub(crate) fn fill_alpha(rt: &mut Rt, t: Option<ObjectHandle>, _: Vec<Variant>) -> Result<Variant> {
    let h = this_of(rt, t, "Layer.fillAlpha")?;
    let rect = clip_of(rt.host().layer(h)?).map(|n| n as i32);
    unary(rt, Some(h), "Layer.fillAlpha", ImageEffect::FillAlpha { spec: layer_ex::FillAlpha { rect } })
}
pub(crate) fn overwrap_rect(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.overwrapRect";
    if a.len() < 7 {
        return Err(TjsError::bad_param_count());
    }
    let target = this_of(rt, t, M)?;
    let source = layer_arg(rt, a.get(2), M)?;
    let n = |i| need_int(&a, i, M).map(|v| v as i32);
    let source_rect = [n(3)?, n(4)?, n(5)?, n(6)?];
    if source_rect[2] <= 0 || source_rect[3] <= 0 {
        return Err(TjsError::runtime("overwrapRect: invalid parameter"));
    }
    let threshold = match a.get(7) {
        None | Some(Variant::Void) => 1,
        Some(v) => u8::try_from(v.to_integer()?).unwrap_or(1),
    };
    let clip = clip_of(rt.host().layer(target)?).map(|v| v as i32);
    let spec = layer_ex::OverwrapRect { destination: [n(0)?, n(1)?], source: source_rect, clip, threshold };
    crate::image_api::begin_pair(rt, target, source, ImageEffect::OverwrapRect { spec }, M)
}
pub(crate) fn shrink_copy(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.shrinkCopy";
    if a.len() < 9 {
        return Err(TjsError::bad_param_count());
    }
    let target = this_of(rt, t, M)?;
    let source = layer_arg(rt, a.get(4), M)?;
    let r = |i: usize| a[i].to_real();
    let n = |i| need_int(&a, i, M).map(|v| v as i32);
    let spec = layer_ex::ShrinkCopy { destination: [r(0)?, r(1)?, r(2)?, r(3)?], source: [n(5)?, n(6)?, n(7)?, n(8)?] };
    spec.validate().map_err(|_| TjsError::runtime("shrinkCopy: invalid parameter"))?;
    crate::image_api::begin_pair(rt, target, source, ImageEffect::ShrinkCopy { spec }, M)
}
pub(crate) fn shrink_copy_fast(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.shrinkCopyFast";
    if a.len() < 2 {
        return Err(TjsError::bad_param_count());
    }
    let target = this_of(rt, t, M)?;
    let source = layer_arg(rt, a.first(), M)?;
    let sx = a[1].to_integer()?;
    let sy = match a.get(2) {
        Some(v) if !matches!(v, Variant::Void) => v.to_integer()?,
        _ => 0,
    };
    let sy = if sy == 0 { sx } else { sy };
    if sx <= 0 || sy <= 0 {
        return Err(TjsError::runtime("shrinkCopyFast: invalid parameter"));
    }
    let (w, h) = {
        let l = rt.host().layer(source)?;
        if !l.has_image {
            return Err(TjsError::runtime("shrinkCopyFast: invalid parameter"));
        }
        (l.image_width, l.image_height)
    };
    let input = crate::image_api::input(rt, source)?;
    rt.call_object_method(target, "setImageSize", vec![
        Variant::Integer((w + sx - 1) / sx),
        Variant::Integer((h + sy - 1) / sy),
    ])?;
    let effect = json!({"kind": "shrink_fast", "spec": {"step": [sx, sy]}});
    crate::image_api::queue_chain(rt, target, input, vec![effect])
}

/// `Layer.stitchWrappedCopy(dx, dy, dw, dh, src, sx, sy, sw, sh, columns)`;
/// negative or void extents mean the whole image. The destination is
/// clipped by the layer's clip rectangle, the source by its image.
pub(crate) fn stitch_wrapped_copy(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    const M: &str = "Layer.stitchWrappedCopy";
    let target = this_of(rt, t, M)?;
    let source = a
        .get(4)
        .and_then(object_arg)
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .ok_or_else(|| TjsError::runtime(format!("{M}: src must be Layer")))?;
    let int = |i: usize, default: i64| -> Result<i64> {
        match a.get(i) {
            None | Some(Variant::Void) => Ok(default),
            Some(v) => v.to_integer(),
        }
    };
    let (dx, dy, dw, dh) = (int(0, 0)?, int(1, 0)?, int(2, -1)?, int(3, -1)?);
    let (sx, sy, sw, sh) = (int(5, 0)?, int(6, 0)?, int(7, -1)?, int(8, -1)?);
    let columns = match a.get(9).and_then(object_arg) {
        Some(list) => rt
            .array_elements(list)
            .unwrap_or(&[])
            .iter()
            .map(|v| v.to_integer().map(|n| n as i32))
            .collect::<Result<Vec<_>>>()?,
        None => vec![],
    };
    let intersect = |r: [i64; 4], c: [i64; 4]| -> Option<[i32; 4]> {
        let l = r[0].max(c[0]);
        let tp = r[1].max(c[1]);
        let rr = r[2].min(c[2]);
        let b = r[3].min(c[3]);
        (rr > l && b > tp).then_some([l as i32, tp as i32, rr as i32, b as i32])
    };
    let s = rt.host().layer(source)?.clone();
    let d = rt.host().layer(target)?.clone();
    let sw = if sw < 0 { s.image_width } else { sw };
    let sh = if sh < 0 { s.image_height } else { sh };
    let Some(src) = intersect([sx, sy, sx + sw, sy + sh], [0, 0, s.image_width, s.image_height]) else {
        return Ok(Variant::Void);
    };
    let dw = if dw < 0 { d.image_width } else { dw };
    let dh = if dh < 0 { d.image_height } else { dh };
    let [cl, ct, cw, ch] = clip_of(&d);
    let Some(dst) = intersect([dx, dy, dx + dw, dy + dh], [cl, ct, cl + cw, ct + ch]) else {
        return Ok(Variant::Void);
    };
    let spec = layer_ex::StitchWrappedCopy {
        destination: dst,
        source: src,
        offset: [(dx - i64::from(dst[0])) as i32, (dy - i64::from(dst[1])) as i32],
        columns,
    };
    crate::image_api::begin_pair(rt, target, source, ImageEffect::StitchWrappedCopy { spec }, M)
}
pub(crate) fn install_stw_copy(rt: &mut Rt) {
    if let Some(class) = rt.global_member("Layer").object_handle() {
        crate::natives::methods(rt, class, &[("stitchWrappedCopy", stitch_wrapped_copy)]);
    }
}

/// GlitchEffect.dll: `Layer.glitchCopy(dx, dy, src, sx, sy, sw, sh, params)`
/// and `Layer.doGlitch(src, params)` (the whole image). `params` keys and
/// defaults as the native reads them; without `seed` the native draws engine
/// random bits, here a per-call counter.
fn glitch(rt: &mut Rt, target: ObjectHandle, source: ObjectHandle, d: [i64; 2], s: [i64; 2], size: [i64; 2], params: Option<ObjectHandle>, what: &str) -> Result<Variant> {
    let sl = rt.host().layer(source)?.clone();
    let dl = rt.host().layer(target)?.clone();
    let sw = if size[0] < 0 { sl.image_width } else { size[0] };
    let sh = if size[1] < 0 { sl.image_height } else { size[1] };
    // Source rectangle inside its image, moved to the destination and
    // clipped by the destination's clip rectangle.
    let (x0, y0) = (s[0].max(0), s[1].max(0));
    let (x1, y1) = ((s[0] + sw).min(sl.image_width), (s[1] + sh).min(sl.image_height));
    let [cl, ct, cw, ch] = clip_of(&dl);
    let l = (x0 - s[0] + d[0]).max(cl);
    let t = (y0 - s[1] + d[1]).max(ct);
    let r = (x1 - s[0] + d[0]).min(cl + cw);
    let b = (y1 - s[1] + d[1]).min(ct + ch);
    if r <= l || b <= t {
        return Ok(Variant::Void);
    }
    let real = |rt: &Rt, key: &str, default: f64| -> f64 {
        match params.map(|p| rt.object_member(p, key)) {
            Some(v) if !matches!(v, Variant::Void) => v.to_real().unwrap_or(default),
            _ => default,
        }
    };
    let block = real(rt, "size", 16.) as i64;
    let seed = match params.map(|p| rt.object_member(p, "seed")) {
        Some(v) if !matches!(v, Variant::Void) => v.to_integer()? as u64,
        _ => {
            rt.host_mut().api_calls += 1;
            0x5DEE_CE66_D ^ rt.host().api_calls as u64
        }
    };
    let spec = layer_ex::GlitchCopy {
        destination: [l as i32, t as i32, r as i32, b as i32],
        source: [(l - d[0] + s[0]) as i32, (t - d[1] + s[1]) as i32],
        noise: real(rt, "noise", 4.),
        shift: [real(rt, "sft_x", 16.), real(rt, "sft_y", 8.), real(rt, "sft_col", 8.)],
        probability: [real(rt, "per_x", 0.5), real(rt, "per_y", 0.25), real(rt, "per_col", 0.05)],
        reset: real(rt, "per_reset", 0.02),
        block: if block <= 0 { 16 } else { block.min(4096) as u32 },
        seed,
        coef: real(rt, "coef", 1.),
    };
    crate::image_api::begin_pair(rt, target, source, ImageEffect::GlitchCopy { spec }, what)
}
pub(crate) fn install_glitch(rt: &mut Rt) {
    let Some(class) = rt.global_member("Layer").object_handle() else { return };
    crate::natives::methods(
        rt,
        class,
        &[
            ("glitchCopy", |rt, t, a| {
                const M: &str = "Layer.glitchCopy";
                let target = this_of(rt, t, M)?;
                let source = layer_arg(rt, a.get(2), M)?;
                let params = a.get(7).and_then(object_arg).ok_or_else(|| TjsError::runtime("glitchCopy: params must be a dictionary"))?;
                let int = |i: usize, default: i64| -> Result<i64> {
                    match a.get(i) {
                        None | Some(Variant::Void) => Ok(default),
                        Some(v) => v.to_integer(),
                    }
                };
                let (d, s, size) = ([int(0, 0)?, int(1, 0)?], [int(3, 0)?, int(4, 0)?], [int(5, -1)?, int(6, -1)?]);
                glitch(rt, target, source, d, s, size, Some(params), M)
            }),
            ("doGlitch", |rt, t, a| {
                const M: &str = "Layer.doGlitch";
                let target = this_of(rt, t, M)?;
                let source = layer_arg(rt, a.first(), M)?;
                let params = a.get(1).and_then(object_arg).ok_or_else(|| TjsError::runtime("doGlitch: params must be a dictionary"))?;
                glitch(rt, target, source, [0, 0], [0, 0], [-1, -1], Some(params), M)
            }),
        ],
    );
}

/// What a GlitchEffect transition provider (IDA 0x100053F0/0x10005310/
/// 0x10005380) starts for `beginTransition`.
pub(crate) enum GlitchStart {
    /// A divisible handler drawing for `time` ms; `exchange` is ttExchange
    /// (`glitch`), otherwise ttSimple (`fadeglitch`).
    Effect { effect: crysirror_scene::layer::SubtreeTransitionEffect, time: i64, exchange: bool },
    /// LoopGlitch: its time is -1, so the first StartProcess (0x100057A0,
    /// signed elapsed > -1) reports completion before anything is drawn.
    Immediate,
}
/// Provider options through tTVPSimpleOptionProvider: a missing or void
/// member takes the default (0x10001CD0/0x10001DD0/0x10001EE0).
fn option(rt: &mut Rt, options: ObjectHandle, name: &str) -> Result<Option<Variant>> {
    Ok(match rt.resolve_object_member(options, name)? {
        Variant::Void => None,
        v => Some(v),
    })
}
fn option_int(rt: &mut Rt, options: ObjectHandle, name: &str, default: i64) -> Result<i64> {
    option(rt, options, name)?.map_or(Ok(default), |v| v.to_integer().map(|n| n as i32 as i64))
}
fn option_real(rt: &mut Rt, options: ObjectHandle, name: &str, default: f64) -> Result<f64> {
    option(rt, options, name)?.map_or(Ok(default), |v| v.to_real())
}
pub(crate) fn glitch_transition(rt: &mut Rt, name: &str, options: ObjectHandle, has_source: bool) -> Result<GlitchStart> {
    use crysirror_scene::layer::{GlitchParams, GlitchTransition, SubtreeTransitionEffect};
    if !rt.host().loaded_plugins.contains("glitcheffect.dll") {
        return Err(crate::natives::unsupported(&format!("transition {name}"), "linked GlitchEffect provider"));
    }
    let failed = || TjsError::runtime("iTVPTransHandlerProvider::StartTransition failed");
    if name == "loopglitch" {
        return Ok(GlitchStart::Immediate);
    }
    let time = option_int(rt, options, "time", -1)?;
    if time < 0 {
        return Err(failed());
    }
    let time = time.max(2);
    let real = |rt: &mut Rt, key: &str, default: f64| option_real(rt, options, key, default);
    let params = GlitchParams {
        noise: real(rt, "noise", 4.)?,
        shift: [real(rt, "sft_x", 16.)?, real(rt, "sft_y", 8.)?, real(rt, "sft_col", 8.)?],
        probability: [real(rt, "per_x", 0.5)?, real(rt, "per_y", 0.25)?, real(rt, "per_col", 0.05)?],
        reset: real(rt, "per_reset", 0.02)?,
        block: 16,
    };
    let (mode, exchange) = if name == "glitch" {
        let gamma_in = real(rt, "gamma_in", 1.)?;
        let gamma_out = real(rt, "gamma_out", 1.)?;
        (GlitchTransition::Exchange { gamma_in, gamma_out }, true)
    } else {
        let nofade = option_int(rt, options, "nofade", 0)? != 0;
        let fade_in = option_int(rt, options, "fadein", 0)? != 0;
        let gamma = real(rt, if fade_in { "gamma_in" } else { "gamma_out" }, 1.)?;
        // nofade forces 0x808080, which is also what disables the tint.
        let color = if nofade {
            0x80_8080
        } else {
            option(rt, options, "color")?.map_or(Ok(0), |v| v.to_integer())? as u32 & 0xFF_FFFF
        };
        let color = (color != 0x80_8080).then(|| [(color >> 16) as u8, (color >> 8) as u8, color as u8]);
        // FadeGlitch reads the transition source when fading in; the native
        // handler has no source image to read without one.
        if fade_in && !has_source {
            return Err(crate::natives::unsupported("fadeglitch fadein without a source layer", "a transition source"));
        }
        (GlitchTransition::Fade { fade_in, gamma, color }, false)
    };
    let block = option_int(rt, options, "block", 16)?;
    let params = GlitchParams { block: if block <= 0 { 16 } else { block.min(4096) as u32 }, ..params };
    rt.host_mut().api_calls += 1;
    let seed = 0x5DEE_CE66_D ^ (rt.host().api_calls as u64) << 20;
    let effect = SubtreeTransitionEffect::Glitch { params, mode, seed };
    effect.validate().map_err(|e| TjsError::runtime(format!("{name}: {e}")))?;
    Ok(GlitchStart::Effect { effect, time, exchange })
}
