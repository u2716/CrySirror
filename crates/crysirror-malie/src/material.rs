// SPDX-License-Identifier: MPL-2.0
//! Original Ragnarok 4EE960 reads its caller-owned archive key at RVA3A60A4
//! and rotation operands at the four verified instruction RVAs below.
//! Read PE data only; no key bytes, EXE code or proprietary files are shipped.
use anyhow::{ensure, Result};
use crysirror_resource::pe::PeData;
pub fn archive_key(executable: &[u8]) -> Result<Vec<u8>> {
    let pe = PeData::parse(executable)?;
    let expected = [
        0xf6, 0x88, 0x83, 0x99, 0x90, 0xc4, 0xb8, 0x38, 0xc0, 0x80, 0x55, 0x78, 0xd5, 0x2c, 0x67,
        0x21, 0x3a, 0x13, 0xaf, 0xa9, 0x4f, 0x68, 0x49, 0x1c, 0xfe, 0x03, 0xee, 0x3d, 0xb8, 0xa2,
        0x58, 0x92,
    ];
    ensure!(
        pe.sha256() == expected,
        "Malie executable profile is unknown; archive material must be audited for this build"
    );
    let mut material = pe.rva_slice(0x3a60a4, 32)?.to_vec();
    for rva in [0xee9bc, 0xee9f1, 0xeea28, 0xeea60] {
        material.extend_from_slice(pe.rva_slice(rva, 4)?);
    }
    Ok(material)
}
