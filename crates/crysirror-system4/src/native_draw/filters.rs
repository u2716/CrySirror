// GPL-2.0-or-later. DrawGraph policy: pinned xsystem4 DrawGraph.c/draw.c
// (Nunuhara Cabbage and kichikuou); Rust (C)2026 CrySirror contributors.
//! Native filter routing; immutable engine jobs own all pixel arithmetic.
use super::*;
use crysirror_core::image_blend::{
    RgbaBlendComponent, RgbaBlendEquation, RgbaBlendFactor, RgbaBlendState,
};

fn process(input: ImageInput, effect: serde_json::Value) -> Result<ImageInput> {
    Ok(ImageInput::Process {
        input: Box::new(input),
        effects: serde_json::to_vec(&vec![effect])?.into(),
    })
}

/// The reviewed011E0 helper clips both source and destination before selecting
/// the blur radius. Work in wide coordinates instead of overflowing native
/// signed pointer arithmetic on malformed inputs.
fn clip_copy(
    destination: &mut [i32; 2],
    source: &mut [i32; 4],
    dst_size: [u32; 2],
    src_size: [u32; 2],
) -> bool {
    for axis in 0..2 {
        let mut d = i64::from(destination[axis]);
        let mut s = i64::from(source[axis]);
        let mut n = i64::from(source[axis + 2]);
        if d < 0 {
            s -= d;
            n += d;
            d = 0;
        }
        if s < 0 {
            d -= s;
            n += s;
            s = 0;
        }
        n = n
            .min(i64::from(dst_size[axis]) - d)
            .min(i64::from(src_size[axis]) - s);
        if n <= 0 {
            return false;
        }
        destination[axis] = d as i32;
        source[axis] = s as i32;
        source[axis + 2] = n as i32;
    }
    true
}

pub(super) fn operation(
    owner: ClientId,
    name: &[u8],
    values: &[i32],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>> {
    if matches!(name,b"FillOverlay"|b"FillOverlayAlpha") {
        ensure!(native_tables,"original byte255 overlay requires the reviewed Graph/DrawGraph module profile");
        let [dst,x,y,w,h,r,g,b,rest @ ..]=values else {bail!("native overlay argument ABI")};
        ensure!(rest.len()==usize::from(name==b"FillOverlayAlpha"),"native overlay opacity arity");
        ensure!((1..=8192).contains(w)&&(1..=8192).contains(h),"native overlay extent budget");
        let color=[*r,*g,*b].map(|n|n.clamp(0,255) as u8);
        // Original BD60 reads only the blue-indexed table for all three RGB
        // lanes. Preserve the original FillOverlayAlpha export's scalar bug.
        let color=if name==b"FillOverlayAlpha" {[color[2];3]}else{color};
        let opacity=rest.first().map(|n|(*n).clamp(0,255) as u8);
        let spec=crysirror_image::rgb_lut::RgbLookup::overlay_byte255(color,opacity);
        let base=cg.sprite_input(owner,*dst,bridge)?;
        let input=process(base.clone(),serde_json::json!({"kind":"rgb_lookup","spec":spec}))?;
        return Ok(Some((*dst,composite(base,input,[*x,*y,*w,*h],[*x,*y],ImageCompositeMath::CopyRgb),false)));
    }
    if !matches!(
        name,
        b"CopyWidthBlur"
            | b"CopyHeightBlur"
            | b"CopyAMapWidthBlur"
            | b"CopyAMapHeightBlur"
            | b"CopyGrayscale"
            | b"FillAMapGradationUD"
    ) {
        return Ok(None);
    }
    let sprites = bridge
        .sact
        .as_ref()
        .and_then(|s| s.sprites.as_ref())
        .context("System4 filter before SACT initialization")?;
    sprites.require_owner(owner)?;
    if name == b"FillAMapGradationUD" {
        let [dst, x, y, w, h, up, down] = values else {
            bail!("System4 alpha gradation ABI");
        };
        let base = cg.sprite_input(owner, *dst, bridge)?;
        let target = sprites
            .graphics(*dst)
            .context("System4 gradation destination absent")?;
        let mut source = [*x, *y, *w, *h];
        let mut position = [*x, *y];
        if native_tables {
            //012F0 clips the fill before Graph's 26D0 interpolates y/h.
            let mut dummy = [0, 0, *w, *h];
            if !clip_copy(&mut position, &mut dummy, target.size, [i32::MAX as u32; 2]) {
                return Ok(Some((*dst, base, false)));
            }
            source = [0, 0, dummy[2], dummy[3]];
            if !target.alpha {
                //03210 checks Graph.HasAlpha after clipping and returns
                //without calling the primitive when the plane is absent.
                return Ok(Some((*dst, base, false)));
            }
        }
        ensure!(
            (1..=8192).contains(&source[2]) && (1..=8192).contains(&source[3]),
            "System4 gradation dimensions"
        );
        let input = process(
            ImageInput::Solid {
                size: [source[2] as u32, source[3] as u32],
                color: [0; 4],
            },
            serde_json::json!({"kind":"linear_gradient","spec":{
                "vertical":true,"start":[0,0,0,(*up).clamp(0,255)],
                "end":[0,0,0,(*down).clamp(0,255)],"write_mask":[false,false,false,true],
                "sample":if native_tables {"start_truncate"} else {"center_round"}
            }}),
        )?;
        return Ok(Some((
            *dst,
            composite(
                base,
                input,
                [0, 0, source[2], source[3]],
                position,
                ImageCompositeMath::CopyAlpha,
            ),
            true,
        )));
    }
    let [dst, dx, dy, src, sx, sy, w, h, rest @ ..] = values else {
        bail!("System4 filter copy ABI");
    };
    let grayscale = name == b"CopyGrayscale";
    ensure!(
        rest.len() == usize::from(!grayscale),
        "System4 filter parameter count"
    );
    ensure!(
        !grayscale || !native_tables,
        "CopyGrayscale is absent from this original DrawGraph export profile"
    );
    let base = cg.sprite_input(owner, *dst, bridge)?;
    let source_input = cg.sprite_input(owner, *src, bridge)?;
    let target = sprites
        .graphics(*dst)
        .context("System4 filter destination absent")?;
    let source_sprite = sprites.graphics(*src).context("System4 filter source absent")?;
    let mut source = [*sx, *sy, *w, *h];
    let mut position = [*dx, *dy];
    if native_tables && !clip_copy(&mut position, &mut source, target.size, source_sprite.size) {
        return Ok(Some((*dst, base, false)));
    }
    ensure!(
        (1..=8192).contains(&source[2]) && (1..=8192).contains(&source[3]),
        "System4 filter copy dimensions"
    );
    let size = [source[2] as u32, source[3] as u32];
    let alpha = matches!(name, b"CopyAMapWidthBlur" | b"CopyAMapHeightBlur");
    if native_tables && alpha && (!source_sprite.alpha || !target.alpha) {
        //06B90/06C80 require both alpha planes before entering their kernels.
        return Ok(Some((*dst, base, false)));
    }
    ensure!(
        !alpha || source_sprite.alpha,
        "System4 blur source alpha plane absent"
    );
    let horizontal = matches!(name, b"CopyWidthBlur" | b"CopyAMapWidthBlur");
    let math = if alpha {
        ImageCompositeMath::CopyAlpha
    } else {
        ImageCompositeMath::CopyRgb
    };
    if !grayscale && *dst == *src {
        let overlaps = (0..2).all(|axis| {
            i64::from(position[axis]) < i64::from(source[axis]) + i64::from(source[axis + 2])
                && i64::from(source[axis]) < i64::from(position[axis]) + i64::from(source[axis + 2])
        });
        if overlaps {
            ensure!(
                native_tables,
                "System4 GL blur texture feedback has no defined source profile"
            );
            let radius = native_radius(rest[0], if horizontal { source[2] } else { source[3] })?;
            return Ok(Some((
                *dst,
                process(
                    base,
                    serde_json::json!({"kind":"sequential_box_blur","spec":{
                        "source":source,"destination":position,"vertical":!horizontal,"radius":radius,
                        "write_mask":if alpha {[false,false,false,true]} else {[true,true,true,false]}
                    }}),
                )?,
                alpha,
            )));
        }
    }
    if !native_tables && !grayscale {
        //The public GL implementation scales and adds each shifted rectangle
        //separately; it neither repeats endpoints nor divides only once.
        let radius = rest[0];
        let rate = (1.0 / (2.0 * f64::from(radius) + 1.0)).clamp(0.0, 1.0) as f32;
        let one = RgbaBlendComponent {
            source: RgbaBlendFactor::ConstantColor,
            destination: RgbaBlendFactor::Zero,
            equation: RgbaBlendEquation::Add,
        };
        let add = RgbaBlendComponent {
            destination: RgbaBlendFactor::One,
            ..one
        };
        let mut state = RgbaBlendState {
            constant: [rate; 4],
            write_mask: if alpha {
                [false, false, false, true]
            } else {
                [true, true, true, false]
            },
            ..Default::default()
        };
        if alpha {
            state.alpha = one;
        } else {
            state.color = one;
        }
        let mut layers = vec![ImageCompositeLayer {
            input: Box::new(source_input.clone()),
            source,
            destination: position,
            clip: None,
            opacity: 255,
            math: ImageCompositeMath::Normalized(state),
        }];
        let axis = usize::from(!horizontal);
        let steps = radius.max(0).min(source[axis + 2] - 1);
        //The shared graph admits256 nodes, including base and initial copy.
        ensure!(steps <= 126, "System4 shifted blur graph work budget");
        if alpha {
            state.alpha = add;
        } else {
            state.color = add;
        }
        for direction in [false, true] {
            for offset in 1..=steps {
                let mut rect = source;
                let mut destination = position;
                rect[axis + 2] -= offset;
                if direction {
                    destination[axis] = destination[axis]
                        .checked_add(offset)
                        .context("System4 blur coordinate overflow")?;
                } else {
                    rect[axis] = rect[axis]
                        .checked_add(offset)
                        .context("System4 blur coordinate overflow")?;
                }
                layers.push(ImageCompositeLayer {
                    input: Box::new(source_input.clone()),
                    source: rect,
                    destination,
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Normalized(state),
                });
            }
        }
        return Ok(Some((
            *dst,
            ImageInput::Composite {
                base: Box::new(base),
                layers,
            },
            alpha,
        )));
    }
    let cropped = composite(
        ImageInput::Solid {
            size,
            color: [0; 4],
        },
        source_input,
        source,
        [0, 0],
        ImageCompositeMath::Copy,
    );
    let effect = if grayscale {
        //Reference fragment dot(rgb,[.299,.587,.114]) quantized to bytes.
        serde_json::json!({"kind":"integer_color_matrix","spec":{
            "matrix":[[299,587,114],[299,587,114],[299,587,114]],
            "offset":[500,500,500],"divisor":1000}})
    } else {
        let radius = native_radius(rest[0], if horizontal { source[2] } else { source[3] })?;
        serde_json::json!({"kind":"box_blur_clamped","spec":{
            "radii":if horizontal {[radius,0]} else {[0,radius]},"passes":1}})
    };
    let input = process(cropped, effect)?;
    Ok(Some((
        *dst,
        composite(
            base,
            input,
            [0, 0, size[0] as i32, size[1] as i32],
            position,
            math,
        ),
        alpha,
    )))
}

fn native_radius(value: i32, length: i32) -> Result<i32> {
    let radius = if value < 1 {
        1
    } else if value >= length / 2 {
        length / 2 - 1
    } else {
        value
    };
    //The original tiny-axis branches may construct a negative lookup size or
    //read beyond the clipped row. Preserve defined kernels, reject that UB.
    ensure!(
        radius >= 0 && radius * 2 + 1 <= length,
        "System4 native blur radius would read outside the clipped surface"
    );
    Ok(radius)
}
