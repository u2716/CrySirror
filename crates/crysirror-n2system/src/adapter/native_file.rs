// SPDX-License-Identifier: MPL-2.0
//! Native4015B0/401650 packet policy and443160 basename byte key. 401BA0
//! uses E(x)=~D^3(x) for NPA; only a unique algebraic inverse is admissible.
//! Tables remain caller-owned scoped assets. No proprietary table is embedded.
use super::*;
use crate::archive::ArchiveMaterial;

fn inverse(
    storage: &dyn ClientStorage,
    material: Option<&ArchiveMaterial>,
) -> Result<Option<[u8; 256]>> {
    let Some(material) = material else {
        return Ok(None);
    };
    let source = material
        .npf_permutation
        .as_ref()
        .unwrap_or(&material.permutation);
    let reader = storage.open_read(&VfsPath::new(Mount::Asset, source)?)?;
    ensure!(reader.len()? == 256, "N2System NPF permutation length");
    let bytes = crysirror_resource::read_all(reader.as_ref(), 256)?;
    let table: [u8; 256] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("N2System NPF permutation extent"))?;
    Ok(Some(if material.npf_permutation.is_some() {
        table
    } else {
        crysirror_resource::npf::unique_complement_cube_root(&table)?
    }))
}
fn shift(path: &str) -> Result<u8> {
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .context("N2System NPF basename")?;
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(name);
    ensure!(
        !errors && bytes.len() <= 255,
        "N2System NPF basename encoding/length"
    );
    Ok(bytes
        .iter()
        .fold(427295025u32, |key, byte| key.wrapping_sub(u32::from(*byte)))
        .wrapping_mul(bytes.len() as u32) as u8)
}
pub(super) fn decode(
    storage: &dyn ClientStorage,
    material: Option<&ArchiveMaterial>,
    path: &str,
    bytes: Vec<u8>,
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    if !bytes.starts_with(b"NPSF") {
        check()?;
        return Ok(bytes);
    }
    let masked = bytes.get(8..12).is_some_and(|count| count != [0, 0, 0, 0]);
    let table = if masked {
        inverse(storage, material)?
    } else {
        None
    };
    crysirror_resource::npf::decode(&bytes, shift(path)?, table.as_ref(), limit, check)
}
pub(super) fn encode(
    storage: &dyn ClientStorage,
    material: Option<&ArchiveMaterial>,
    path: &str,
    bytes: &[u8],
    limit: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    let table = inverse(storage, material)?;
    // A zero-mask packet is an original reader-supported profile when this
    // session has no encoded-file material; do not synthesize a private key.
    crysirror_resource::npf::encode(bytes, shift(path)?, table.as_ref(), limit, check)
}
