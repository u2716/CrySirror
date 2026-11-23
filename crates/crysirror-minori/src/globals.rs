// SPDX-License-Identifier: MPL-2.0
//! ef first tale global-variable record. Slot saves have a distinct format.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_resource::{blowfish_le_decrypt, blowfish_le_encrypt, read_all};
use encoding_rs::SHIFT_JIS;
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};

const LIMIT: usize = 2 << 20;
const KEY: &[u8] = b"minori";
type Variables = HashMap<String, i32>;

pub(super) fn read(storage: &dyn ClientStorage) -> Result<Variables> {
    let path = VfsPath::new(Mount::Save, "system.dat")?;
    let reader = match storage.open_read(&path) {
        Ok(reader) => reader,
        Err(Error::NotFound) => return Ok(Variables::new()),
        Err(e) => return Err(e.into()),
    };
    decode(&read_all(reader.as_ref(), LIMIT as u64 + 65536)?)
}

fn decode(record: &[u8]) -> Result<Variables> {
    ensure!(
        record.len() >= 8 && record.len().is_multiple_of(8),
        "invalid minori system.dat block count"
    );
    let packed = blowfish_le_decrypt(record, KEY, record.len())?;
    let length = u32::from_le_bytes(packed[..4].try_into()?) as usize;
    ensure!((1..=LIMIT).contains(&length), "minori system.dat length");
    let mut z = flate2::Decompress::new(true);
    let mut plain = vec![0; length + 1];
    ensure!(
        z.decompress(&packed[4..], &mut plain, flate2::FlushDecompress::Finish)?
            == flate2::Status::StreamEnd
            && z.total_out() == length as u64,
        "invalid minori system.dat compressed record"
    );
    ensure!(
        plain[..length].iter().position(|v| *v == 0) == Some(length - 1),
        "invalid minori system.dat terminator"
    );
    let mut values = Variables::new();
    for line in plain[..length - 1]
        .split(|&v| v == b'\n')
        .filter(|l| !l.is_empty())
    {
        let tab = line
            .iter()
            .position(|&v| v == b'\t')
            .context("minori global separator")?;
        let key = SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&line[..tab])
            .context("minori global name encoding")?;
        ensure!(
            !key.is_empty() && !key.chars().any(char::is_control),
            "minori global name"
        );
        let value = std::str::from_utf8(&line[tab + 1..])?.parse::<i32>()?;
        values.insert(key.into_owned(), value);
    }
    Ok(values)
}
fn encode(values: &Variables) -> Result<Vec<u8>> {
    let mut sorted = BTreeMap::new();
    for (name, value) in values {
        ensure!(
            !name.is_empty() && !name.chars().any(char::is_control),
            "invalid minori global name"
        );
        let (name, _, errors) = SHIFT_JIS.encode(name);
        ensure!(!errors, "unrepresentable minori global");
        sorted.insert(name, *value);
    }
    let mut plain = Vec::new();
    for (name, value) in sorted {
        ensure!(
            plain.len() + name.len() + 16 <= LIMIT,
            "minori globals exceed save limit"
        );
        plain.extend_from_slice(&name);
        writeln!(plain, "\t{value}")?;
    }
    plain.push(0);
    let mut out = (plain.len() as u32).to_le_bytes().to_vec();
    let mut encoder = flate2::write::ZlibEncoder::new(&mut out, flate2::Compression::new(6));
    encoder.write_all(&plain)?;
    encoder.finish()?;
    out.resize(out.len().next_multiple_of(8), 0);
    blowfish_le_encrypt(&out, KEY, out.len())
}
pub(super) fn write(storage: &dyn ClientStorage, values: &Variables) -> Result<()> {
    let data = encode(values)?;
    storage.write_atomic(&VfsPath::new(Mount::Save, "system.dat")?, &data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_record_round_trip() {
        let values = Variables::from([("ルート".into(), -31), ("END_A".into(), 1)]);
        let data = encode(&values).unwrap();
        assert_eq!(decode(&data).unwrap(), values);
        assert!(decode(&data[..data.len() - 1]).is_err());
    }
}
