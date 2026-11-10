// GPL-2.0-or-later. Rust(C)2026 CrySirror contributors. DrawGraph native
// 6D80/6EE0/3E80/3FA0, Graph C460/C570 and complete helpers read statically.
//! Native geometry/handle policy. Pixel visitation is shared ByteRaster work.
use super::*;
use crysirror_image::byte_raster::{ByteRaster, ByteRasterMath};
fn raster(base: ImageInput, source: ImageInput, spec: ByteRaster) -> Result<ImageInput> {
    Ok(ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(source),
        }),
        effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::ByteRaster {
            spec,
        }])?
        .into(),
    })
}
pub(super) fn operation(
    owner: ClientId,
    name: &[u8],
    args: &[Argument],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<(i32, ImageInput, bool)>> {
    if !matches!(
        name,
        b"DrawLine"
            | b"DrawLineToAMap"
            | b"CopyTextureWrap"
            | b"CopyTextureWrapAlpha"
            | b"CopyRotZoom"
            | b"CopyRotZoomAMap"
            | b"CopyRotZoomUseAMap"
            | b"CG_RotateRGB"
            | b"CG_BlendAMapBin"
            | b"CopyRotZoom2Bilinear"
            | b"CopyRotateY"
            | b"CopyRotateYUseAMap"
            | b"CopyRotateYFixL"
            | b"CopyRotateYFixR"
            | b"CopyRotateYFixLUseAMap"
            | b"CopyRotateYFixRUseAMap"
            | b"CopyRotateX"
            | b"CopyRotateXUseAMap"
            | b"CopyRotateXFixU"
            | b"CopyRotateXFixD"
            | b"CopyRotateXFixUUseAMap"
            | b"CopyRotateXFixDUseAMap"
    ) {
        return Ok(None);
    }
    ensure!(
        native_tables,
        "native byte geometry requires the reviewed Graph/DrawGraph profile"
    );
    if name.starts_with(b"CopyRotate") {
        let [
            Argument::Scalar {
                kind: 10,
                bits: dst,
            },
            Argument::Scalar {
                kind: 10,
                bits: front,
            },
            Argument::Scalar {
                kind: 10,
                bits: back,
            },
            Argument::Scalar { kind: 10, bits: sx },
            Argument::Scalar { kind: 10, bits: sy },
            Argument::Scalar { kind: 10, bits: sw },
            Argument::Scalar { kind: 10, bits: sh },
            Argument::Scalar {
                kind: 11,
                bits: angle,
            },
            Argument::Scalar {
                kind: 11,
                bits: mag,
            },
        ] = args
        else {
            bail!("native axis rotation ABI")
        };
        ensure!(
            dst != front && dst != back,
            "native axis overlapping source/output feedback profile unsupported"
        );
        let sprites = bridge
            .sact
            .as_ref()
            .and_then(|s| s.sprites.as_ref())
            .context("native axis before SACT")?;
        let output = sprites
            .graphics(*dst)
            .context("native axis missing destination")?;
        let face = sprites
            .graphics(*front)
            .context("native axis missing front face")?;
        let reverse = sprites
            .graphics(*back)
            .context("native axis missing back face")?;
        let alpha = name.ends_with(b"UseAMap");
        ensure!(
            output.alpha && (!alpha || (face.alpha && reverse.alpha)),
            "native axis alpha planes absent"
        );
        ensure!(
            face.size == reverse.size,
            "native axis front/back dimensions differ"
        );
        ensure!(
            *sx >= 0
                && *sy >= 0
                && (1..=8192).contains(sw)
                && (1..=8192).contains(sh)
                && i64::from(*sx) + i64::from(*sw) <= i64::from(face.size[0])
                && i64::from(*sy) + i64::from(*sh) <= i64::from(face.size[1]),
            "native axis clipped crop profile unsupported"
        );
        // Original oversized branches and row-margin loops use source extent
        // rather than output extent. A different output size needs a separate
        // audited partial-write profile; do not infer it from generic clipping.
        ensure!(
            output.size == [*sw as u32, *sh as u32],
            "native axis unequal output/crop extent unsupported"
        );
        use crysirror_image::axis_projection::{AxisProjection, ProjectionAnchor};
        let anchor = if name.windows(4).any(|n| n == b"FixL" || n == b"FixU") {
            ProjectionAnchor::Start
        } else if name.windows(4).any(|n| n == b"FixR" || n == b"FixD") {
            ProjectionAnchor::End
        } else {
            ProjectionAnchor::Center
        };
        let spec = AxisProjection {
            source: [*sx as u32, *sy as u32, *sw as u32, *sh as u32],
            rows: name.starts_with(b"CopyRotateX"),
            anchor,
            degrees: f32::from_bits(*angle as u32),
            scale: f32::from_bits(*mag as u32),
            source_alpha: alpha,
        };
        let number = if spec.uses_back()? { *back } else { *front };
        let input = raster(
            cg.sprite_input(owner, *dst, bridge)?,
            cg.sprite_input(owner, number, bridge)?,
            ByteRaster::AxisProjection { spec },
        )?;
        return Ok(Some((*dst, input, false)));
    }
    if name == b"CopyRotZoom2Bilinear" {
        let [
            Argument::Scalar {
                kind: 10,
                bits: dst,
            },
            Argument::Scalar { kind: 11, bits: cx },
            Argument::Scalar { kind: 11, bits: cy },
            Argument::Scalar {
                kind: 10,
                bits: src,
            },
            Argument::Scalar {
                kind: 11,
                bits: scx,
            },
            Argument::Scalar {
                kind: 11,
                bits: scy,
            },
            Argument::Scalar {
                kind: 11,
                bits: angle,
            },
            Argument::Scalar {
                kind: 11,
                bits: mag,
            },
        ] = args
        else {
            bail!("native bilinear rotation ABI")
        };
        ensure!(
            dst != src,
            "native bilinear overlapping source/output profile unsupported"
        );
        let sprites = bridge
            .sact
            .as_ref()
            .and_then(|s| s.sprites.as_ref())
            .context("native bilinear rotation before SACT")?;
        ensure!(
            sprites.graphics(*src).is_some_and(|s| s.alpha)
                && sprites.graphics(*dst).is_some_and(|s| s.alpha),
            "native bilinear rotation alpha planes absent"
        );
        let float = |n: &i32| f32::from_bits(*n as u32);
        let input = raster(
            cg.sprite_input(owner, *dst, bridge)?,
            cg.sprite_input(owner, *src, bridge)?,
            ByteRaster::BilinearRotation {
                destination_center: [float(cx), float(cy)],
                source_center: [float(scx), float(scy)],
                degrees: float(angle),
                scale: float(mag),
            },
        )?;
        return Ok(Some((*dst, input, false)));
    }
    if matches!(
        name,
        b"CopyRotZoom" | b"CopyRotZoomAMap" | b"CopyRotZoomUseAMap"
    ) {
        let [
            Argument::Scalar {
                kind: 10,
                bits: dst,
            },
            Argument::Scalar {
                kind: 10,
                bits: src,
            },
            Argument::Scalar { kind: 10, bits: sx },
            Argument::Scalar { kind: 10, bits: sy },
            Argument::Scalar { kind: 10, bits: sw },
            Argument::Scalar { kind: 10, bits: sh },
            Argument::Scalar {
                kind: 11,
                bits: angle,
            },
            Argument::Scalar {
                kind: 11,
                bits: mag,
            },
        ] = args
        else {
            bail!("native rotate/zoom ABI");
        };
        ensure!(
            dst != src,
            "native rotate overlapping source/output profile unsupported"
        );
        let sprites = bridge
            .sact
            .as_ref()
            .and_then(|s| s.sprites.as_ref())
            .context("native rotate before SACT")?;
        let source = sprites
            .graphics(*src)
            .context("native rotate missing source")?;
        let destination = sprites
            .graphics(*dst)
            .context("native rotate missing destination")?;
        ensure!(
            destination.alpha && (name == b"CopyRotZoom" || source.alpha),
            "native rotate alpha plane absent"
        );
        // Wrapper11E0 ratio/edge clipping remains an explicit unsupported
        // profile; do not silently rotate a differently sized clipped crop.
        ensure!(
            *sx >= 0
                && *sy >= 0
                && (1..=8192).contains(sw)
                && (1..=8192).contains(sh)
                && i64::from(*sx) + i64::from(*sw) <= i64::from(source.size[0])
                && i64::from(*sy) + i64::from(*sh) <= i64::from(source.size[1]),
            "native rotate clipped source profile unsupported"
        );
        let angle = f32::from_bits(*angle as u32);
        let scale = f32::from_bits(*mag as u32);
        ensure!(
            angle.is_finite() && scale.is_finite(),
            "native rotate nonfinite parameters"
        );
        let math = match name {
            b"CopyRotZoomAMap" => ByteRasterMath::CopyAlpha,
            b"CopyRotZoomUseAMap" => ByteRasterMath::CopyRgba,
            _ => ByteRasterMath::CopyRgb,
        };
        let input = raster(
            cg.sprite_input(owner, *dst, bridge)?,
            cg.sprite_input(owner, *src, bridge)?,
            ByteRaster::RotateScale {
                source: [*sx as u32, *sy as u32, *sw as u32, *sh as u32],
                radians: angle * 0.017453292f32,
                scale,
                math,
                opaque: name == b"CopyRotZoom",
            },
        )?;
        return Ok(Some((*dst, input, name == b"CopyRotZoomAMap")));
    }
    let values = args
        .iter()
        .map(|a| match a {
            Argument::Scalar { kind: 10, bits } => Ok(*bits),
            _ => bail!("native byte geometry integer ABI"),
        })
        .collect::<Result<Vec<_>>>()?;
    if name == b"CG_RotateRGB" {
        let [dst, x, y, w, h, rotation] = values.as_slice() else {
            bail!("native RGB rotation ABI")
        };
        ensure!(
            (1..=8192).contains(w) && (1..=8192).contains(h),
            "native RGB rotation extent budget"
        );
        let matrix = match rotation % 3 {
            0 => [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
            // Native bytes are BGR. RotateType1 makes R<-G,G<-B,B<-R.
            1 => [[0, 1, 0], [0, 0, 1], [1, 0, 0]],
            _ => [[0, 0, 1], [1, 0, 0], [0, 1, 0]],
        };
        let base = cg.sprite_input(owner, *dst, bridge)?;
        let changed = ImageInput::Process {
            input: Box::new(base.clone()),
            effects: serde_json::to_vec(&vec![
                crysirror_image::effects::ImageEffect::IntegerColorMatrix {
                    spec: crysirror_image::integer_color::ColorMatrixFixed {
                        matrix,
                        offset: [0; 3],
                        divisor: 1,
                    },
                },
            ])?
            .into(),
        };
        return Ok(Some((
            *dst,
            composite(
                base,
                changed,
                [*x, *y, *w, *h],
                [*x, *y],
                ImageCompositeMath::CopyRgb,
            ),
            false,
        )));
    }
    if name == b"CG_BlendAMapBin" {
        let [dst, dx, dy, src, sx, sy, w, h, border] = values.as_slice() else {
            bail!("native alpha threshold copy ABI")
        };
        ensure!(
            dst != src || (dx == sx && dy == sy),
            "native alpha threshold overlapping feedback unsupported"
        );
        ensure!(
            (1..=8192).contains(w) && (1..=8192).contains(h) && *sx >= 0 && *sy >= 0,
            "native alpha threshold copy rectangle profile"
        );
        ensure!(
            bridge
                .sact
                .as_ref()
                .and_then(|s| s.sprites.as_ref())
                .and_then(|s| s.graphics(*src))
                .is_some_and(|s| s.alpha),
            "native alpha threshold copy source alpha absent"
        );
        let input = raster(
            cg.sprite_input(owner, *dst, bridge)?,
            cg.sprite_input(owner, *src, bridge)?,
            ByteRaster::Tile {
                source: [*sx as u32, *sy as u32, *w as u32, *h as u32],
                destination: [*dx, *dy, *w, *h],
                phase: [0; 2],
                first_run: None,
                math: ByteRasterMath::AlphaThresholdRgb {
                    threshold: (*border).clamp(0, 255) as u8,
                },
            },
        )?;
        return Ok(Some((*dst, input, false)));
    }
    if matches!(name, b"DrawLine" | b"DrawLineToAMap") {
        let [dst, x0, y0, x1, y1, rest @ ..] = values.as_slice() else {
            bail!("native line argument ABI")
        };
        let alpha = name == b"DrawLineToAMap";
        let color = match (alpha, rest) {
            (false, [r, g, b]) => [
                (*r).clamp(0, 255) as u8,
                (*g).clamp(0, 255) as u8,
                (*b).clamp(0, 255) as u8,
                255,
            ],
            (true, [a]) => [0, 0, 0, (*a).clamp(0, 255) as u8],
            _ => bail!("native line colour ABI"),
        };
        if alpha {
            ensure!(
                bridge
                    .sact
                    .as_ref()
                    .and_then(|s| s.sprites.as_ref())
                    .and_then(|s| s.graphics(*dst))
                    .is_some_and(|s| s.alpha),
                "native line destination alpha absent"
            );
        }
        let input = raster(
            cg.sprite_input(owner, *dst, bridge)?,
            ImageInput::Solid {
                size: [1, 1],
                color: [0; 4],
            },
            ByteRaster::Line {
                points: [[*x0, *y0], [*x1, *y1]],
                color,
                write_mask: if alpha {
                    [false, false, false, true]
                } else {
                    [true, true, true, false]
                },
            },
        )?;
        return Ok(Some((*dst, input, alpha)));
    }
    let [dst, dx, dy, dw, dh, src, sx, sy, sw, sh, u, v, rest @ ..] = values.as_slice() else {
        bail!("native wrap argument ABI")
    };
    ensure!(dst != src, "native wrap overlapping feedback unsupported");
    ensure!(
        rest.len() == usize::from(name == b"CopyTextureWrapAlpha"),
        "native wrap opacity arity"
    );
    ensure!(
        [*dw, *dh, *sw, *sh].iter().all(|n| (1..=8192).contains(n)) && *sx >= 0 && *sy >= 0,
        "native wrap extent/source budget"
    );
    // Native C460 uses an absolute right edge for its first horizontal run;
    // subsequent runs restart at sx for sw pixels. Preserve this asymmetry.
    let mut first = i64::from(*sx) + i64::from(*u % *sw);
    if first < 0 {
        first += i64::from(*sw);
    }
    ensure!(
        first >= 0 && first < i64::from(*sw),
        "native wrap initial run would not advance"
    );
    ensure!(
        *v >= 0 || *sy == 0,
        "native wrap negative vertical phase with nonzero source origin is unsupported"
    );
    let math = rest.first().map_or(ByteRasterMath::CopyRgb, |a| {
        ByteRasterMath::ConstantRgb255 {
            opacity: (*a).clamp(0, 255) as u8,
        }
    });
    let input = raster(
        cg.sprite_input(owner, *dst, bridge)?,
        cg.sprite_input(owner, *src, bridge)?,
        ByteRaster::Tile {
            source: [*sx as u32, *sy as u32, *sw as u32, *sh as u32],
            destination: [*dx, *dy, *dw, *dh],
            phase: [*u, *v],
            first_run: Some([first as u32, (*sw as i64 - first) as u32]),
            math,
        },
    )?;
    Ok(Some((*dst, input, false)))
}
