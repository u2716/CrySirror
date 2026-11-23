// GPL-2.0-or-later. Native declarations/fragment/blend policy: complete pinned
// xsystem4 DrawGraph.c, draw.c and their fragment shaders (Nunuhara Cabbage).
// Rust (C)2026 CrySirror contributors. No pixel loop or renderer is imported.
use super::*;
use crysirror_core::image_blend::{
    RgbaBlendArithmetic as A, RgbaBlendComponent as C, RgbaBlendEquation as E,
    RgbaBlendFactor as F, RgbaBlendState as S, RgbaByteRounding as Q,
};
fn equation(source: F, destination: F, equation: E) -> C {
    C {
        source,
        destination,
        equation,
    }
}
fn color_only() -> S {
    S {
        write_mask: [true, true, true, false],
        ..Default::default()
    }
}
fn alpha_only() -> S {
    S {
        write_mask: [false, false, false, true],
        ..Default::default()
    }
}
fn unit(value: i32) -> Result<f32> {
    Ok(f32::from(byte(value)?) / 255.)
}
/// Only original syscall/resource policy. All fragment and pixel arithmetic is
/// owned by the shared ImageCompositeMath::Normalized worker.
pub(super) fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>> {
    let copy = matches!(
        name,
        b"CopyBright"
            | b"CopyColorReverse"
            | b"CopySprite"
            | b"SpriteCopyAMap"
            | b"CopyUseAMapUnder"
            | b"CopyUseAMapBorder"
            | b"CopyAMapMax"
            | b"CopyAMapMin"
            | b"Blend"
            | b"BlendSrcBright"
            | b"BlendAddSatur"
            | b"BlendAMap"
            | b"BlendAMapSrcOnly"
            | b"BlendAMapColor"
            | b"BlendAMapColorAlpha"
            | b"BlendAMapAlpha"
            | b"BlendAMapBright"
            | b"BlendAMapAlphaSrcBright"
            | b"BlendUseAMapColor"
            | b"BlendScreen"
            | b"BlendMultiply"
            | b"BlendScreenAlpha"
            | b"SaturDP_DPxSA"
            | b"AddDA_DAxSA"
            | b"BlendDA_DAxSA"
            | b"SubDA_DAxSA"
            | b"ScreenDA_DAxSA"
    );
    let fill = matches!(
        name,
        b"FillAlphaColor"
            | b"FillScreen"
            | b"FillMultiply"
            | b"BrightDestOnly"
            | b"FillAMapOverBorder"
            | b"FillAMapUnderBorder"
    );
    if !copy && !fill || name == b"ScreenDA_DAxSA" && !native_tables {
        return Ok(None);
    }
    let (number, input, rect, destination, state) = if copy {
        let [dst, dx, dy, src, sx, sy, w, h, rest @ ..] = values else {
            bail!("System4 normalized copy ABI");
        };
        ensure!(
            (1..=8192).contains(w) && (1..=8192).contains(h),
            "System4 normalized copy dimensions"
        );
        let mut s = color_only();
        match (name, rest) {
            (b"CopyBright", [rate]) => s.constant = [unit(*rate)?; 4],
            (b"CopyColorReverse", []) => {
                s.source_scale = [-1., -1., -1., 1.];
                s.source_bias = [1., 1., 1., 0.];
            }
            (b"CopySprite", [r, g, b]) => {
                s.source_color_key = Some([byte(*r)?, byte(*g)?, byte(*b)?])
            }
            (b"SpriteCopyAMap", [key]) => {
                s = alpha_only();
                s.source_alpha_key = Some(byte(*key)?);
            }
            (b"CopyUseAMapUnder", [threshold]) => s.source_alpha_range[1] = unit(*threshold)?,
            (b"CopyUseAMapBorder", [threshold]) => s.source_alpha_range[0] = unit(*threshold)?,
            (b"CopyAMapMax", []) => {
                s = alpha_only();
                s.alpha.equation = E::Max;
            }
            (b"CopyAMapMin", []) => {
                s = alpha_only();
                s.alpha.equation = E::Min;
            }
            (b"Blend", [alpha]) => {
                s.constant[3] = unit(*alpha)?;
                s.color = equation(F::ConstantAlpha, F::OneMinusConstantAlpha, E::Add);
            }
            (b"BlendSrcBright", [alpha, rate]) => {
                let r = unit(*rate)?;
                s.source_scale = [r, r, r, 1.];
                s.constant[3] = unit(*alpha)?;
                s.color = equation(F::ConstantAlpha, F::OneMinusConstantAlpha, E::Add);
            }
            (b"BlendAddSatur", []) => s.color = equation(F::DestinationAlpha, F::One, E::Add),
            (b"BlendAMap", []) => {
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add)
            }
            (b"BlendAMapSrcOnly", []) => s.color = equation(F::SourceAlpha, F::One, E::Add),
            (b"BlendAMapColor", [r, g, b]) | (b"BlendAMapColorAlpha", [r, g, b, _]) => {
                s.source_scale = [
                    0.,
                    0.,
                    0.,
                    if rest.len() == 4 { unit(rest[3])? } else { 1. },
                ];
                s.source_bias = [unit(*r)?, unit(*g)?, unit(*b)?, 0.];
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
            }
            (b"BlendAMapAlpha", [alpha]) => {
                s.source_scale[3] = unit(*alpha)?;
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
            }
            (b"BlendAMapBright", [rate]) => {
                let r = unit(*rate)?;
                s.source_scale = [r, r, r, 1.];
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
            }
            (b"BlendAMapAlphaSrcBright", [alpha, rate]) => {
                let r = unit(*rate)?;
                s.source_scale = [r, r, r, unit(*alpha)?];
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
            }
            (b"BlendUseAMapColor", [r, g, b, threshold]) => {
                s.source_scale = [0., 0., 0., -16.];
                s.source_bias = [
                    unit(*r)?,
                    unit(*g)?,
                    unit(*b)?,
                    1. + 16. * unit(*threshold)?,
                ];
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
            }
            (b"BlendScreen", []) => s.color = equation(F::One, F::OneMinusSourceColor, E::Add),
            (b"BlendMultiply", []) => s.color = equation(F::DestinationColor, F::Zero, E::Add),
            (b"BlendScreenAlpha", [alpha]) => {
                let a = unit(*alpha)?;
                s.source_scale = [a, a, a, 1.];
                s.color = equation(F::One, F::OneMinusSourceColor, E::Add);
            }
            (b"SaturDP_DPxSA", []) => {
                s.source_scale = [0., 0., 0., 1.];
                s.source_bias = [1., 1., 1., 0.];
                s.source_alpha_range[0] = 0.001;
                s.color = equation(F::SourceAlpha, F::One, E::Add);
            }
            (b"AddDA_DAxSA", []) => {
                s = alpha_only();
                s.alpha = equation(F::One, F::One, E::Add);
            }
            (b"BlendDA_DAxSA", []) => {
                s = alpha_only();
                s.alpha = equation(F::Zero, F::SourceAlpha, E::Add);
            }
            (b"ScreenDA_DAxSA", []) => {
                s = alpha_only();
                s.alpha = equation(F::One, F::OneMinusSourceAlpha, E::Add);
            }
            (b"SubDA_DAxSA", []) => {
                s = alpha_only();
                s.alpha = equation(F::One, F::One, E::ReverseSubtract);
            }
            _ => bail!(
                "System4 normalized original copy ABI {}",
                String::from_utf8_lossy(name)
            ),
        }
        if name == b"CopyBright" {
            s.color = equation(F::ConstantColor, F::Zero, E::Add);
        }
        (
            *dst,
            cg.sprite_input(owner, *src, bridge)?,
            [*sx, *sy, *w, *h],
            [*dx, *dy],
            s,
        )
    } else {
        let [dst, x, y, w, h, rest @ ..] = values else {
            bail!("System4 normalized fill ABI");
        };
        ensure!(
            (1..=8192).contains(w) && (1..=8192).contains(h),
            "System4 normalized fill dimensions"
        );
        let mut s = color_only();
        let size = [*w as u32, *h as u32];
        let (input, rect) = match (name, rest) {
            (b"FillAlphaColor", [r, g, b, a]) => {
                s.color = equation(F::SourceAlpha, F::OneMinusSourceAlpha, E::Add);
                (
                    ImageInput::Solid {
                        size,
                        color: [byte(*r)?, byte(*g)?, byte(*b)?, byte(*a)?],
                    },
                    [0, 0, *w, *h],
                )
            }
            (b"FillScreen", [r, g, b]) | (b"FillMultiply", [r, g, b]) => {
                s.color = if name == b"FillScreen" {
                    equation(F::One, F::OneMinusSourceColor, E::Add)
                } else {
                    equation(F::DestinationColor, F::Zero, E::Add)
                };
                (
                    ImageInput::Solid {
                        size,
                        color: [byte(*r)?, byte(*g)?, byte(*b)?, 255],
                    },
                    [0, 0, *w, *h],
                )
            }
            (b"BrightDestOnly", [rate]) => {
                s.constant = [unit(*rate)?; 4];
                s.color = equation(F::Zero, F::ConstantColor, E::Add);
                (
                    ImageInput::Solid {
                        size,
                        color: [0; 4],
                    },
                    [0, 0, *w, *h],
                )
            }
            (b"FillAMapOverBorder", [alpha, border])
            | (b"FillAMapUnderBorder", [alpha, border]) => {
                s = alpha_only();
                if name == b"FillAMapOverBorder" {
                    s.source_alpha_range[0] = unit(*border)?;
                } else {
                    s.source_alpha_range[1] = unit(*border)?;
                }
                s.source_scale[3] = 0.;
                s.source_bias[3] = unit(*alpha)?;
                (cg.sprite_input(owner, *dst, bridge)?, [*x, *y, *w, *h])
            }
            _ => bail!(
                "System4 normalized original fill ABI {}",
                String::from_utf8_lossy(name)
            ),
        };
        // `size` belongs to the explicit solid source; masked fills above use
        // the frozen original destination as source, never an in-place alias.
        (*dst, input, rect, [*x, *y], s)
    };
    let mut state = state;
    if native_tables {
        state.arithmetic = A::Byte255 {
            source: Q::Floor,
            destination: Q::Floor,
        };
        if matches!(
            name,
            b"BlendScreen" | b"BlendScreenAlpha" | b"FillScreen" | b"ScreenDA_DAxSA"
        ) {
            state.arithmetic = A::Byte255 {
                source: Q::Floor,
                destination: Q::Ceil,
            };
        }
        if name == b"BlendAddSatur" {
            state.color = equation(F::One, F::One, E::Add);
        }
    }
    state.validate()?;
    let needs_alpha = state.write_mask[3];
    Ok(Some((
        number,
        composite(
            cg.sprite_input(owner, number, bridge)?,
            input,
            rect,
            destination,
            ImageCompositeMath::Normalized(state),
        ),
        needs_alpha,
    )))
}
