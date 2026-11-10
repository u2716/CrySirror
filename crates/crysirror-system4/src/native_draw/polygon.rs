// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! DrawGraph B0E0/7010/73A0/81D0 and B150/7140/8EF0. Complete
//! native wrappers/kernels inspected statically; no DLL is executed. Native
//! signatures, surface flags and aliases stay here. Shared TriangleRaster owns
//! inverse homogeneous coefficients, coverage, sampling and byte conversion.
use super::*;
use crysirror_image::{
    byte_raster::ByteRaster,
    triangle_raster::{TrianglePaint, TriangleRaster},
};
pub(super) enum Plan {
    Noop,
    Draw { number: i32, input: ImageInput },
}
pub(super) fn operation(
    owner: ClientId,
    name: &[u8],
    args: &[Argument],
    bridge: &Bridge,
    cg: &NativeCg,
    native_tables: bool,
) -> Result<Option<Plan>> {
    if !matches!(name, b"DrawPolygon" | b"DrawColorPolygon") {
        return Ok(None);
    }
    ensure!(
        native_tables,
        "native polygons require the reviewed Graph/DrawGraph profile"
    );
    let int = |index: usize| -> Result<i32> {
        match args.get(index) {
            Some(Argument::Scalar { kind: 10, bits }) => Ok(*bits),
            _ => bail!("native polygon integer ABI at{index}"),
        }
    };
    let float = |index: usize| -> Result<f32> {
        match args.get(index) {
            Some(Argument::Scalar { kind: 11, bits }) => Ok(f32::from_bits(*bits as u32)),
            _ => bail!("native polygon float ABI at{index}"),
        }
    };
    ensure!(
        args.len() == if name == b"DrawPolygon" { 17 } else { 22 },
        "native polygon argument count"
    );
    let number = int(0)?;
    let mut positions = [[0.; 3]; 3];
    let paint = if name == b"DrawPolygon" {
        let _ = int(1)?;
        let mut uv = [[0.; 2]; 3];
        for vertex in 0..3 {
            let at = 2 + vertex * 5;
            for axis in 0..3 {
                positions[vertex][axis] = float(at + axis)?;
            }
            for axis in 0..2 {
                uv[vertex][axis] = float(at + 3 + axis)?;
            }
        }
        TrianglePaint::Texture {
            uv,
            source_alpha: false,
            feedback: false,
        }
    } else {
        let mut rgba = [[0; 4]; 3];
        for vertex in 0..3 {
            let at = 1 + vertex * 7;
            for axis in 0..3 {
                positions[vertex][axis] = float(at + axis)?;
            }
            for channel in 0..4 {
                rgba[vertex][channel] = int(at + 3 + channel)?;
            }
        }
        TrianglePaint::Color { rgba }
    };
    let mut spec = TriangleRaster { positions, paint };
    spec.validate()?;
    let sprites = bridge
        .sact
        .as_ref()
        .filter(|s| s.initialized)
        .and_then(|s| s.sprites.as_ref())
        .context("native polygon before SACT initialization")?;
    sprites.require_owner(owner)?;
    //7010/7140 return without drawing or issuing a window update for an absent
    //destination or a missing RGB/alpha plane. These are reviewed native no-ops.
    let Some(destination) = sprites.graphics(number) else {
        return Ok(Some(Plan::Noop));
    };
    if !destination.alpha || destination.surface.is_none() || destination.size.contains(&0) {
        return Ok(Some(Plan::Noop));
    }
    let base = cg.sprite_input(owner, number, bridge)?;
    let source = if let TrianglePaint::Texture {
        source_alpha,
        feedback,
        ..
    } = &mut spec.paint
    {
        let source_number = int(1)?;
        let Some(source) = sprites.graphics(source_number) else {
            return Ok(Some(Plan::Noop));
        };
        if source.surface.is_none() || source.size.contains(&0) {
            return Ok(Some(Plan::Noop));
        }
        *source_alpha = source.alpha;
        *feedback = source_number == number;
        cg.sprite_input(owner, source_number, bridge)?
    } else {
        base.clone()
    };
    let input = ImageInput::Process {
        input: Box::new(ImageInput::Raster {
            destination: Box::new(base),
            source: Box::new(source),
        }),
        effects: serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::ByteRaster {
            spec: ByteRaster::Triangle { spec },
        }])?
        .into(),
    };
    Ok(Some(Plan::Draw { number, input }))
}
