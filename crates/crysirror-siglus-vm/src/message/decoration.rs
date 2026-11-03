// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native four-mode text policy; generic coverage kernels live in Text.
use super::*;
use crysirror_scene::layer::{GlyphBlur, GlyphCoverageKernel};

fn settings(ctx: &CommandContext) -> Result<(bool, i32)> {
    let configured = |op, key: &str| -> Result<i32> {
        if let Some(value) = ctx.globals.syscom.config_int.get(&op) {
            Ok(i32::try_from(*value)?)
        } else {
            config::scalar(
                ctx.tables
                    .gameexe
                    .as_ref()
                    .and_then(|cfg| cfg.get_unquoted(key)),
                0,
            )
        }
    };
    let (bold_override, shadow_override) = if ctx.globals.syscom.msg_back_open {
        (-1, -1)
    } else if ctx.excall_state.ex_call_flag {
        (ctx.excall_state.font_bold, ctx.excall_state.font_shadow)
    } else {
        (ctx.globals.script.font_bold, ctx.globals.script.font_shadow)
    };
    let bold = if bold_override >= 0 {
        i32::try_from(bold_override)?
    } else {
        configured(
            crate::runtime::forms::codes::syscom_op::GET_FONT_BOLD,
            "CONFIG.FONT.FUTOKU",
        )?
    };
    let mode = if shadow_override >= 0 {
        i32::try_from(shadow_override)?
    } else {
        configured(
            crate::runtime::forms::codes::syscom_op::GET_FONT_DECORATION,
            "CONFIG.FONT.SHADOW",
        )?
    };
    // Native queries retain the raw override; only rendering clamps the four
    // decoration modes. Bold follows the original nonzero boolean rule.
    Ok((bold != 0, mode.clamp(0, 3)))
}
pub(super) fn validate(ctx: &CommandContext) -> Result<()> {
    settings(ctx)?;
    Ok(())
}
fn effect_color(ctx: &CommandContext, name: bool, edge: bool) -> Result<[u8; 4]> {
    let Some(cfg) = ctx.tables.gameexe.as_ref() else {
        return Ok([0, 0, 0, 255]);
    };
    let field = match (name, edge) {
        (false, false) => "SHADOW_COLOR",
        (false, true) => "FUCHI_COLOR",
        (true, false) => "NAME_SHADOW_COLOR",
        (true, true) => "NAME_FUCHI_COLOR",
    };
    let own = config::scalar(cfg.get_indexed_field("MWND", 0, field), -1)?;
    let number = if own < 0 {
        config::scalar(
            cfg.get_unquoted(if edge {
                "MWND.FUCHI_COLOR"
            } else {
                "MWND.SHADOW_COLOR"
            }),
            1,
        )?
    } else {
        own
    };
    config::color(cfg, number)
}
fn face(layer: &mut LayerSpec, extent: u8) {
    let Paint::GlyphRun { bold, blur, .. } = &mut layer.paint else {
        unreachable!()
    };
    *bold = false;
    *blur = (extent > 0).then_some(GlyphBlur {
        radius: extent,
        level: 256,
        kernel: GlyphCoverageKernel::SquareFace,
    });
}
pub(super) fn layers(
    ctx: &CommandContext,
    name: bool,
    mut body: LayerSpec,
) -> Result<Vec<LayerSpec>> {
    let (bold, mode) = settings(ctx)?;
    let mut out = Vec::with_capacity(3);
    let size = match &body.paint {
        Paint::GlyphRun { size, .. } => *size,
        _ => unreachable!(),
    };
    for edge in [false, true] {
        if (!edge && !matches!(mode, 1 | 3)) || (edge && !matches!(mode, 2 | 3)) {
            continue;
        }
        let mut layer = body.clone();
        layer.id = MESSAGE_LAYER + 4 + u32::from(name) * 2 + u32::from(edge);
        let color = effect_color(ctx, name, edge)?;
        if let Paint::GlyphRun { color: c, .. } = &mut layer.paint {
            *c = color;
        }
        let extent = if edge || mode == 3 {
            2 + u8::from(bold)
        } else {
            u8::from(bold)
        };
        face(&mut layer, extent);
        let t = (size / 32.).clamp(0., 1.);
        let offset = if edge {
            -1.
        } else if mode == 3 {
            (0.5 + t - 1.).round()
        } else {
            (0.5 + 1.5 * t).round().max(1.)
        };
        layer.rect[0] += offset;
        layer.rect[1] += offset;
        let order = if edge {
            ctx.tables.mwnd_render.fuchi_layer_rep
        } else {
            ctx.tables.mwnd_render.shadow_layer_rep
        };
        layer.order[1] = i32::try_from(
            order
                .checked_add(i64::from(name))
                .context("native text effect order overflow")?,
        )?;
        out.push(layer);
    }
    face(&mut body, u8::from(bold));
    out.push(body);
    Ok(out)
}
