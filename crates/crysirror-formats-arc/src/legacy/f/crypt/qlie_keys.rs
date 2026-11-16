//! FilePack3.0 companion policy. Native contracts follow arc_unpacker
//! 456834ecf2e5686813802c37efd829310485c57d (C)2016 rr-, GPL-3.0-or-later:
//! the complete fkey bytes seed key1; RT_RCDATA/TFORM1's IconKeyImage
//! Picture.Data supplies key2. PE is parsed as data, never loaded/executed.
use super::*;
use anyhow::bail;
use crysirror_resource::{
    KeyMaterial,
    pe::{PeData, PeResourceName},
};

pub(super) fn companions(cx: &DecodeContext) -> Result<(Option<KeyMaterial>, Option<KeyMaterial>)> {
    let p = cx.parameters.get("qlie/pack");
    let key1 = if let Some(id) = p.and_then(|p| p["key1_id"].as_str()) {
        Some(cx.keys.key(cx.client, id)?)
    } else if let Some(path) = p.and_then(|p| p["fkey"].as_str()) {
        let source = cx.open_dependency(path)?;
        let bytes = crysirror_resource::read_all(source.as_ref(), 1024 * 1024)?;
        ensure!(
            bytes.len() >= 256,
            "QLIE fkey is shorter than its 256-byte state"
        );
        Some(KeyMaterial::new(bytes)?)
    } else {
        None
    };
    let key2 = if let Some(id) = p.and_then(|p| p["key2_id"].as_str()) {
        Some(cx.keys.key(cx.client, id)?)
    } else if let Some(path) = p.and_then(|p| p["executable"].as_str()) {
        let source = cx.open_dependency(path)?;
        let bytes = crysirror_resource::read_all(
            source.as_ref(),
            crysirror_resource::pe::MAX_PE_BYTES as u64,
        )?;
        let pe = PeData::parse(&bytes)?;
        let mut found: Option<Vec<u8>> = None;
        for resource in pe.raw_resources(&PeResourceName::Id(10), &|| cx.check_cancelled())? {
            if !matches!(resource.name, PeResourceName::Name(ref name) if name.eq_ignore_ascii_case("TFORM1"))
            {
                continue;
            }
            if let Some(key) = icon_key(&resource.bytes)? {
                ensure!(
                    found.as_ref().is_none_or(|previous| *previous == key),
                    "QLIE conflicting TFORM1 language keys"
                );
                found = Some(key);
            }
        }
        found.map(KeyMaterial::new).transpose()?
    } else {
        None
    };
    Ok((key1, key2))
}

/// Match a Delphi component declaration inside the named form resource,
/// then parse its real property values. Bare TIcon bytes in machine code or
/// unrelated resource blobs are never interpreted as key material.
fn icon_key(bytes: &[u8]) -> Result<Option<Vec<u8>>> {
    ensure!(
        bytes.starts_with(b"TPF0"),
        "QLIE TFORM1 is not a binary Delphi form"
    );
    const COMPONENT: &[u8] = b"\x06TImage\x0cIconKeyImage";
    let mut found = None;
    for (offset, _) in bytes
        .windows(COMPONENT.len())
        .enumerate()
        .filter(|(_, data)| *data == COMPONENT)
    {
        let mut c = Cursor::new(&bytes[offset + COMPONENT.len()..]);
        for _ in 0..256 {
            let length = c.u8()? as usize;
            if length == 0 {
                break;
            }
            let property = c.take(length)?;
            let value = value(&mut c, 0)?;
            if property == b"Picture.Data" {
                let value = value.context("QLIE IconKeyImage Picture.Data must be binary")?;
                ensure!(
                    value.len() >= 262 && value.starts_with(b"\x05TIcon"),
                    "QLIE IconKeyImage TIcon data is truncated or malformed"
                );
                let key = value[6..262].to_vec();
                ensure!(
                    found.as_ref().is_none_or(|previous| *previous == key),
                    "QLIE ambiguous IconKeyImage components"
                );
                found = Some(key);
            }
        }
    }
    Ok(found)
}
fn value<'a>(c: &mut Cursor<'a>, depth: usize) -> Result<Option<&'a [u8]>> {
    ensure!(depth <= 32, "QLIE Delphi property nesting budget");
    let kind = c.u8()?;
    match kind {
        0 => bail!("QLIE unexpected Delphi null property"),
        1 => {
            for _ in 0..4096 {
                let at = c.at;
                let kind = c.u8()?;
                if kind == 0 {
                    return Ok(None);
                }
                c.at = at;
                value(c, depth + 1)?;
            }
            bail!("QLIE Delphi list budget")
        }
        2 => {
            c.take(1)?;
        }
        3 => {
            c.take(2)?;
        }
        4 | 15 => {
            c.take(4)?;
        }
        5 => {
            c.take(10)?;
        }
        6 | 7 => {
            let n = c.u8()? as usize;
            c.take(n)?;
        }
        8 | 9 | 13 => {}
        10 => {
            let n = c.le32()? as usize;
            ensure!(n <= 4 * 1024 * 1024, "QLIE Delphi binary property budget");
            return Ok(Some(c.take(n)?));
        }
        11 => {
            for _ in 0..4096 {
                let n = c.u8()? as usize;
                if n == 0 {
                    return Ok(None);
                }
                c.take(n)?;
            }
            bail!("QLIE Delphi set budget")
        }
        12 | 20 => {
            let n = c.le32()? as usize;
            c.take(n)?;
        }
        16 | 17 | 19 => {
            c.take(8)?;
        }
        18 => {
            let n = c.le32()? as usize;
            c.take(n.checked_mul(2).context("QLIE Delphi string overflow")?)?;
        }
        _ => bail!("QLIE unsupported Delphi property type {kind}"),
    }
    Ok(None)
}
