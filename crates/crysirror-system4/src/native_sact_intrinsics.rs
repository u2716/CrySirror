// GPL-2.0-or-later. SACT policy: pinned xsystem4 SACT2.c (Nunuhara Cabbage),
// complete original SACT2 exports F9C0/F9F0/FCD0/FE30/10080 and helpers read
// statically. Rust (C)2026 CrySirror contributors; no native DLL is executed.
//! Native HLL types/message lookup over shared byte-string and input services.
use crate::{
    adapter::{Bridge, Dispatch},
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use std::sync::Arc;

pub(crate) fn dispatch(
    owner: ClientId,
    m: &mut Machine,
    call: &ExternalCall,
    _bridge: &Bridge,
    host: &mut dyn HostServices,
) -> Result<Option<Dispatch>> {
    let ExternalTarget::Hll { library, function } = call.target else {
        return Ok(None);
    };
    let lib = &m.program.libraries[library];
    let f = &lib.functions[function];
    let name = f.name.clone();
    if lib.name != b"SACT2"
        || !matches!(
            name.as_slice(),
            b"GAME_MSG_Get"
                | b"IntToZenkaku"
                | b"IntToHankaku"
                | b"StringPopFront"
                | b"Mouse_SetPos"
        )
    {
        return Ok(None);
    }
    let kind = f.return_kind;
    let args = m.hll_arguments(call)?;
    if name == b"Mouse_SetPos" {
        ensure!(kind == 10, "SACT Mouse_SetPos return ABI");
        let [
            Argument::Scalar { kind: 10, bits: x },
            Argument::Scalar { kind: 10, bits: y },
        ] = args.as_slice()
        else {
            bail!("SACT Mouse_SetPos argument ABI");
        };
        let observation = host.input_observation(owner)?;
        crate::input::Input::validate_scope(observation, owner)?;
        let focused = observation
            .pointer_inside
            .zip(observation.focused)
            .context("SACT Mouse_SetPos physical focus observation unavailable")?;
        host.warp_pointer(owner, *x, *y)?;
        m.complete_hll(call, HllReturn::Scalar(i32::from(focused.0 && focused.1)))?;
        return Ok(Some(Dispatch::Completed));
    }
    if name == b"GAME_MSG_Get" {
        ensure!(kind == 0, "SACT GAME_MSG_Get return ABI");
        let [
            Argument::Scalar {
                kind: 10,
                bits: index,
            },
            Argument::HeapReference { kind: 20, slot },
        ] = args.as_slice()
        else {
            bail!("SACT GAME_MSG_Get argument ABI");
        };
        //The original Message interface returns a C prefix, not bytes beyond
        //an embedded NUL in the AIN message record.
        let bytes = m
            .program
            .messages
            .get(usize::try_from(*index)?)
            .context("SACT GAME_MSG_Get message index")?;
        let length = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        let bytes = Arc::new(bytes[..length].to_vec());
        m.heap.set_string(*slot, bytes)?;
        m.complete_hll(call, HllReturn::Void)?;
        return Ok(Some(Dispatch::Completed));
    }
    #[cfg(not(feature = "native-text"))]
    bail!("SACT native byte-string capability is not linked");
    #[cfg(feature = "native-text")]
    {
        use crysirror_text_legacy::{
            ByteNumberSymbols, ByteStringContext, ByteStringLimits, ByteStringProfile,
        };
        let context = ByteStringContext::new(
            ByteStringProfile::SHIFT_JIS_CRT,
            ByteStringLimits::default(),
            &|| Ok(()),
        )?;
        if name == b"StringPopFront" {
            ensure!(kind == 10, "SACT StringPopFront return ABI");
            let [
                Argument::HeapReference {
                    kind: 20,
                    slot: dst,
                },
                Argument::HeapReference {
                    kind: 20,
                    slot: src,
                },
            ] = args.as_slice()
            else {
                bail!("SACT StringPopFront argument ABI");
            };
            let source = m.heap.string(*src)?;
            let prefix = source.split(|b| *b == 0).next().unwrap();
            let (front, tail, width) = context.split_front(prefix)?;
            let old = m.heap.string(*dst)?;
            //Preflight the only possible intermediate growth before changing
            //either reference. Aliased arguments retain native dst-then-src
            //assignment order; the source is frozen before the first write.
            m.heap.check_growth(
                0,
                front
                    .len()
                    .div_ceil(4)
                    .saturating_sub(old.len().div_ceil(4)),
            )?;
            m.heap.set_string(*dst, Arc::new(front))?;
            if width != 0 {
                m.heap.set_string(*src, Arc::new(tail))?;
            }
            m.complete_hll(call, HllReturn::Scalar(i32::try_from(width)?))?;
        } else {
            ensure!(kind == 0, "SACT integer formatting return ABI");
            let [
                Argument::HeapReference { kind: 20, slot },
                Argument::Scalar {
                    kind: 10,
                    bits: value,
                },
                Argument::Scalar {
                    kind: 10,
                    bits: figures,
                },
                Argument::Scalar {
                    kind: 10 | 47,
                    bits: zero,
                },
            ] = args.as_slice()
            else {
                bail!("SACT integer formatting argument ABI");
            };
            //F9F0/FCD0 prepend digits, then sign/padding. INT_MIN's original
            //signed negation can omit digits; use a defined magnitude instead.
            let bytes = context.decimal_format(
                *value,
                usize::try_from((*figures).max(0))?,
                *zero != 0,
                if name == b"IntToZenkaku" {
                    ByteNumberSymbols::ShiftJis
                } else {
                    ByteNumberSymbols::Ascii
                },
            )?;
            m.heap.set_string(*slot, Arc::new(bytes))?;
            m.complete_hll(call, HllReturn::Void)?;
        }
        Ok(Some(Dispatch::Completed))
    }
}
