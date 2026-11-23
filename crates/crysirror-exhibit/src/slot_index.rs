// SPDX-License-Identifier: MPL-2.0
//! RLD v1 RetouchSaveDataControl index used by the title's Continue query.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AssetReader, ClientStorage},
};
use crysirror_resource::read_exact_at;

pub(super) fn default_counts() -> [usize; 3] {
    [225, 25, 3]
}

fn occupied(reader: &dyn AssetReader, slots: usize) -> Result<bool> {
    let size = reader.len()?;
    ensure!(size / 8 >= slots as u64, "short ExHibit slot index");
    let mut at = 0u64;
    let mut found = false;
    for _ in 0..slots {
        let mut header = [0; 8];
        read_exact_at(reader, at, &mut header)?;
        found |= header[..4] != [0; 4];
        let memo_len = u32::from_le_bytes(header[4..].try_into()?);
        at = at
            .checked_add(8 + u64::from(memo_len))
            .context("slot offset overflow")?;
        ensure!(at <= size, "truncated ExHibit slot memo");
    }
    Ok(found)
}

pub(super) fn any(storage: &dyn ClientStorage, counts: [usize; 3]) -> Result<bool> {
    for (name, count) in [
        ("user/adb", counts[1]),
        ("user/udb", counts[0]),
        ("user/qdb", counts[2]),
    ] {
        if count == 0 {
            continue;
        }
        for mount in [Mount::Save, Mount::Asset] {
            if storage.metadata(mount, name)?.is_some() {
                let reader = storage.open_read(&VfsPath::new(mount, name)?)?;
                if occupied(reader.as_ref(), count).with_context(|| format!("ExHibit {name}"))? {
                    return Ok(true);
                }
                break;
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::MemoryAssetReader;

    #[test]
    fn timestamp_determines_occupancy_and_count_bounds_the_scan() -> Result<()> {
        let mut data = vec![0, 0, 0, 0, 3, 0, 0, 0, 0x82, 0xa0, 0];
        data.extend([1, 0, 0, 0, 0, 0, 0, 0]);
        let r = MemoryAssetReader(data.into());
        assert!(!occupied(&r, 1)?);
        assert!(occupied(&r, 2)?);
        assert!(occupied(&r, 3).is_err());
        let broken = MemoryAssetReader(vec![1, 0, 0, 0, 1, 0, 0, 0].into());
        assert!(occupied(&broken, 1).is_err());
        Ok(())
    }
}
