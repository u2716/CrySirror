// GPL-2.0-or-later. Native query policy from pinned xsystem4 SACT2.c,
// DrawGraph.c and sprite.c (C) Nunuhara Cabbage; Rust (C) 2026 contributors.
// Shared ImageInput/ImageResource own checked storage access, never this VM.
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
pub(crate) fn dispatch(
    owner: ClientId,
    machine: &mut Machine,
    call: &ExternalCall,
    bridge: &mut Bridge,
    cg: &NativeCg,
    host: &mut dyn HostServices,
) -> Result<Option<Dispatch>> {
    let ExternalTarget::Hll { library, function } = call.target else {
        return Ok(None);
    };
    let program = machine.program.clone();
    let library = &program.libraries[library];
    let function = &library.functions[function];
    let name = function.name.as_slice();
    let graph = library.name == b"DrawGraph";
    if !(graph && matches!(name, b"GetPixelColor" | b"GetAlphaColor")
        || library.name == b"SACT2"
            && matches!(
                name,
                b"SP_GetPixelValue" | b"SP_GetAMapValue" | b"SP_IsPtIn" | b"SP_IsPtInRect"
            ))
    {
        return Ok(None);
    }
    ensure!(
        matches!(function.return_kind, 10 | 47),
        "System4 pixel query return ABI"
    );
    let args = machine.hll_arguments(call)?;
    if matches!(name, b"SP_IsPtIn" | b"SP_IsPtInRect") {
        let [
            Argument::Scalar {
                kind: 10,
                bits: number,
            },
            Argument::Scalar { kind: 10, bits: x },
            Argument::Scalar { kind: 10, bits: y },
        ] = args.as_slice()
        else {
            anyhow::bail!("native sprite hit query ABI")
        };
        let sprites = bridge
            .sact
            .as_ref()
            .filter(|s| s.initialized)
            .and_then(|s| s.sprites.as_ref())
            .context("native sprite hit before SACT")?;
        sprites.require_owner(owner)?;
        let inside = if let Some(sprite) = sprites.get(*number) {
            let rect = [
                i64::from(sprite.position[0]),
                i64::from(sprite.position[1]),
                i64::from(sprite.size[0]),
                i64::from(sprite.size[1]),
            ];
            if !crysirror_core::region_map::contains_rect(rect, [i64::from(*x), i64::from(*y)]) {
                false
            } else if name == b"SP_IsPtInRect" || !sprite.alpha {
                true
            } else {
                let point = [
                    (i64::from(*x) - rect[0]) as u32,
                    (i64::from(*y) - rect[1]) as u32,
                ];
                cg.scene_sprite_input(owner, *number, bridge)?
                    .resident_pixel(owner, point)?
                    .context("native hit source missing resident pixel")?[3]
                    != 0
            }
        } else {
            false
        };
        machine.complete_hll(call, HllReturn::Scalar(i32::from(inside)))?;
        return Ok(Some(Dispatch::Completed));
    }
    let alpha = matches!(name, b"GetAlphaColor" | b"SP_GetAMapValue");
    let count = if !alpha {
        6
    } else if graph {
        4
    } else {
        3
    };
    ensure!(args.len() == count, "System4 pixel query argument count");
    let mut values = [0; 3];
    for (i, arg) in args[..3].iter().enumerate() {
        let Argument::Scalar { kind: 10, bits } = arg else {
            anyhow::bail!("System4 pixel query integer ABI")
        };
        values[i] = *bits;
    }
    let mut refs = Vec::new();
    for arg in &args[3..] {
        let Argument::Reference {
            kind: 18,
            page,
            index,
        } = arg
        else {
            anyhow::bail!("System4 pixel output reference ABI")
        };
        machine.heap.get(*page, *index)?; // Preflight every output before mutation.
        refs.push((*page, *index));
    }
    let [number, x, y] = values;
    let sprites = bridge
        .sact
        .as_mut()
        .filter(|s| s.initialized)
        .and_then(|s| s.sprites.as_mut())
        .context("System4 pixel query before actual SACT init")?;
    sprites.require_owner(owner)?;
    // DrawGraph's GetAlphaColor uses sact_get_sprite (lazy allocation), while
    // the remaining three wrappers use sact_try_get_sprite. The empty logical
    // declaration has no Layer/texture until an actual surface is created.
    if graph && alpha && sprites.graphics(number).is_none() {
        sprites.update(owner, number, host, |_| Ok(()))?;
    }
    let sprite = if graph {
        sprites.graphics(number)
    } else {
        sprites.get(number)
    };
    let Some(sprite) = sprite else {
        machine.complete_hll(call, HllReturn::Scalar(if alpha { -1 } else { 0 }))?;
        return Ok(Some(Dispatch::Completed));
    };
    let inside = x >= 0 && y >= 0 && (x as u32) < sprite.size[0] && (y as u32) < sprite.size[1];
    if alpha && !inside {
        machine.complete_hll(call, HllReturn::Scalar(if graph { 0 } else { -1 }))?;
        return Ok(Some(Dispatch::Completed));
    }
    // Native RGB reads outside the surface call uninitialised GL readback;
    // that undefined profile is rejected, never replaced by an invented colour.
    ensure!(
        inside,
        "System4 out-of-bounds native RGB pixel profile unsupported"
    );
    let has_alpha = sprite.alpha;
    let pixel = if alpha && !has_alpha {
        None
    } else {
        Some(
            (if graph {
                cg.sprite_input(owner, number, bridge)?
            } else {
                cg.scene_sprite_input(owner, number, bridge)?
            })
            .resident_pixel(owner, [x as u32, y as u32])?
            .context("System4 resident pixel missing inside surface")?,
        )
    };
    let result = if alpha {
        let a = pixel.map_or(-1, |rgba| i32::from(rgba[3]));
        if graph {
            machine.heap.set(refs[0].0, refs[0].1, a)?;
            1
        } else {
            a
        }
    } else {
        let pixel = pixel.unwrap();
        for ((page, index), value) in refs.into_iter().zip(pixel[..3].iter()) {
            machine.heap.set(page, index, i32::from(*value))?;
        }
        1
    };
    machine.complete_hll(call, HllReturn::Scalar(result))?;
    Ok(Some(Dispatch::Completed))
}
