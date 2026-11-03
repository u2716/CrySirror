// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS natives over save containers and raw native blocks (78B600 family, dispatch
//! table A2DD38). File reads and writes stay in the adapter's worker tasks.
use crate::{kcs::Machine, native_save::Container};
use anyhow::Result;
use std::collections::BTreeMap;

type Outcome = (Option<u32>, Vec<(u32, Vec<u8>)>);

/// 6493F0's record copy: the bytes a later 387 stores under another key.
#[derive(Default)]
pub(crate) struct Stores {
    blobs: BTreeMap<u32, Vec<u8>>,
    next_blob: u32,
}
pub(crate) const IDS: &[u32] = &[238, 242, 244, 246, 248, 252, 254, 387, 388, 389, 390, 469, 833, 834, 835, 836];

pub(crate) fn apply(
    id: u32,
    args: &[u32],
    machine: &mut Machine,
    saves: &mut BTreeMap<u32, Container>,
    next_save: &mut u32,
    stores: &mut Stores,
) -> Result<Outcome> {
    let arg = |at: usize| args.get(at).copied().unwrap_or(0);
    let mut add = |saves: &mut BTreeMap<u32, Container>, container: Container| {
        let handle = *next_save;
        *next_save = handle.wrapping_add(1).max(1);
        saves.insert(handle, container);
        handle
    };
    Ok(match id {
        238 => (Some(add(saves, Container::new(arg(0)))), vec![]),
        246 => {
            saves.remove(&arg(0));
            (None, vec![])
        }
        248 => (Some(saves.get(&arg(0)).map_or(0, |c| c.tag)), vec![]),
        244 | 835 => match saves.get_mut(&arg(0)) {
            Some(container) if arg(3) != 0 => {
                let data = machine.read_bytes(arg(3), arg(2) as usize)?;
                container.append(arg(1), &data)?;
                (Some(1), vec![])
            }
            _ => (Some(0), vec![]),
        },
        242 => {
            let nested = saves.get(&arg(2)).map(Container::nested);
            match (nested, saves.get_mut(&arg(0))) {
                (Some(nested), Some(container)) => {
                    container.append(arg(1), &nested)?;
                    (Some(1), vec![])
                }
                _ => (Some(0), vec![]),
            }
        }
        252 => {
            let nested = saves.get(&arg(0)).and_then(|c| c.first(arg(1))).map(Container::from_nested).transpose()?;
            (Some(nested.map_or(0, |c| add(saves, c))), vec![])
        }
        254 => match saves.get(&arg(0)).and_then(|c| c.first(arg(1))) {
            Some(data) if arg(2) != 0 => {
                let count = data.len().min(arg(3) as usize);
                (Some(count as u32), vec![(arg(2), data[..count].to_vec())])
            }
            _ => (Some(0), vec![]),
        },
        388 => match saves.get(&arg(0)).and_then(|c| c.first(arg(1))) {
            Some(data) => {
                stores.next_blob = stores.next_blob.wrapping_add(1).max(1);
                stores.blobs.insert(stores.next_blob, data.to_vec());
                (Some(stores.next_blob), vec![])
            }
            None => (Some(0), vec![]),
        },
        387 => match (stores.blobs.get(&arg(2)), saves.get_mut(&arg(0))) {
            (Some(data), Some(container)) => {
                container.append(arg(1), data)?;
                (Some(1), vec![])
            }
            _ => (Some(0), vec![]),
        },
        389 => match saves.get_mut(&arg(0)) {
            Some(container) => {
                container.clear();
                (Some(1), vec![])
            }
            None => (Some(0), vec![]),
        },
        390 => {
            let copy = saves.get(&arg(0)).cloned();
            (Some(copy.map_or(0, |c| add(saves, c))), vec![])
        }
        469 => {
            if let Some(container) = saves.get_mut(&arg(0)) {
                container.remove(arg(1));
            }
            (None, vec![])
        }
        833 => (Some(machine.allocate_memory((arg(0) as usize).max(1))?), vec![]),
        834 => {
            machine.free_memory(arg(0));
            (None, vec![])
        }
        836 => match saves.get(&arg(0)).and_then(|c| c.first(arg(1))).map(<[u8]>::to_vec) {
            Some(data) => {
                let block = machine.allocate_memory(data.len().max(1))?;
                let size = (data.len() as u32).to_le_bytes().to_vec();
                (Some(block), vec![(block, data), (arg(2), size)])
            }
            None => (Some(0), vec![]),
        },
        _ => anyhow::bail!("CatSystem2 store native {id} is not in this table"),
    })
}
