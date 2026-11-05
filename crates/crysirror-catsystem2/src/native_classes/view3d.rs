// SPDX-License-Identifier: MPL-2.0
//! Native view-task policy for the two inspected desktop profiles.
//!
//! Neither Grisaia's 2011 factory nor Nekopara's 2017 factory constructs class
//! 43: both dispatch class 14 and 43 directly to their null-return epilogue.
//! This is a factory choice, independent of the presence of `3DTable.txt`.
//! A different profile with a real view task still requires its native resource
//! and message semantics; it must not be represented by an opaque fake task.

use super::Ctx;
use anyhow::{ensure, Context, Result};
use crysirror_resource::pe::PeData;

/// Verify the reviewed factory's code/table entries as immutable PE data. This
/// does not execute the module, and does not classify a game by its filename.
pub(crate) fn has_null_constructor(pe: &PeData<'_>) -> bool {
    fn matches(pe: &PeData<'_>, address: u64, expected: &[u8]) -> bool {
        pe.va_slice(address, expected.len()).is_ok_and(|bytes| bytes == expected)
    }
    // 5DD580: cmp ebp,63; ja default; jmp [ebp*4+5DE274].
    let neko = matches(pe, 0x5dd5ce, &[0x83, 0xfd, 0x3f, 0x0f, 0x87, 0x70, 0x0c, 0, 0, 0xff, 0x24, 0xad, 0x74, 0xe2, 0x5d, 0])
        && matches(pe, 0x5de274 + 14 * 4, &0x5de259u32.to_le_bytes())
        && matches(pe, 0x5de274 + 43 * 4, &0x5de259u32.to_le_bytes())
        && matches(pe, 0x5de259, &[0x33, 0xc0, 0x8b, 0x4c, 0x24, 0x18]);
    // 419140: cmp eax,53; ja default; jmp [eax*4+419AA4].
    let grisaia = matches(pe, 0x419191, &[0x83, 0xf8, 0x35, 0x0f, 0x87, 0xee, 0x08, 0, 0, 0xff, 0x24, 0x85, 0xa4, 0x9a, 0x41, 0])
        && matches(pe, 0x419aa4 + 14 * 4, &0x419a99u32.to_le_bytes())
        && matches(pe, 0x419aa4 + 43 * 4, &0x419a99u32.to_le_bytes())
        && matches(pe, 0x419a99, &[0x5f, 0x5e, 0x33, 0xc0, 0x5b, 0x8b, 0xe5, 0x5d, 0xc2, 0x0c, 0]);
    neko || grisaia
}

/// 7269B0 reads the serialized native tree used by 689. Each node contains
/// sibling/child relative offsets, payload length and payload. No pointers from
/// the file are materialized. Bound its traversal even when there is no task
/// receiver, since the original parses the record before sending message 440.
fn validate_tree(bytes: &[u8]) -> Result<()> {
    ensure!(bytes.len() <= 16 * 1024 * 1024, "CatSystem2 view save tree budget");
    let mut pending = vec![0usize];
    let mut nodes = 0usize;
    while let Some(at) = pending.pop() {
        nodes += 1;
        ensure!(nodes <= 65536, "CatSystem2 view save tree node budget");
        let header_end = at.checked_add(12).context("CatSystem2 view save tree offset overflow")?;
        let header = bytes.get(at..header_end).context("CatSystem2 truncated view save tree node")?;
        let word = |offset| u32::from_le_bytes(header[offset..offset + 4].try_into().unwrap()) as usize;
        let end = header_end.checked_add(word(8)).context("CatSystem2 view save tree payload overflow")?;
        ensure!(end <= bytes.len(), "CatSystem2 truncated view save tree payload");
        for offset in [word(0), word(4)] {
            if offset != 0 {
                let next = at.checked_add(offset).context("CatSystem2 view save tree link overflow")?;
                ensure!(next < bytes.len(), "CatSystem2 view save tree link outside record");
                pending.push(next);
            }
        }
    }
    Ok(())
}

/// 648E10/649A00/648E80: preserve absent/misdirected receiver results and CSS
/// first-record precedence. 689 returns one after parsing an existing record,
/// irrespective of whether message 440 found a receiver; 576/688 require the
/// receiver to return state and therefore return zero for these null profiles.
pub(crate) fn save(cx: &mut Ctx<'_>) -> Result<()> {
    let a = &cx.call.arguments;
    ensure!(a.len() == 3, "CatSystem2 view save arguments");
    cx.machine.preflight_native(true, &[])?;
    let result = if a[0] == 0 {
        0
    } else if cx.call.id == 688 {
        ensure!(cx.tasks.opaque_class(a[2]) != Some(43), "CatSystem2 class 43 view-state export requires an implemented native view profile");
        0
    } else {
        let record = cx.saves.get(&a[0]).context("CatSystem2 view save container")?.first(a[1]);
        if let Some(record) = record {
            if cx.call.id == 689 {
                validate_tree(record)?;
            }
            ensure!(cx.tasks.opaque_class(a[2]) != Some(43), "CatSystem2 class 43 view-state restore requires an implemented native view profile");
            u32::from(cx.call.id == 689)
        } else {
            0
        }
    };
    cx.machine.acknowledge(Some(result))?;
    Ok(())
}
