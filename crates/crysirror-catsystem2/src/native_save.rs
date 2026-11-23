// SPDX-License-Identifier: MPL-2.0
//! Original CSS save-container layout and native handle policy. Capability I/O
//! and compression belong to shared Storage and Resource transforms.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, MemoryAssetReader},
    state::ClientId,
    Error,
};
use crysirror_resource::{DecodeContext, TransformSpec};
use std::sync::Arc;
const LIMIT: usize = 16 * 1024 * 1024;
#[derive(Clone, Debug)]
pub(crate) struct Container {
    pub tag: u32,
    pub bytes: Vec<u8>,
    pub records: Vec<(u32, std::ops::Range<usize>)>,
}
impl Container {
    /// 78B600: an empty container with its tag.
    pub fn new(tag: u32) -> Self {
        Self { tag, bytes: Vec::new(), records: Vec::new() }
    }
    /// 78B790: the first record with this key.
    pub fn first(&self, key: u32) -> Option<&[u8]> {
        self.records.iter().find(|(id, _)| *id == key).map(|(_, range)| &self.bytes[range.clone()])
    }
    /// 78B750: append; an existing key stays and keeps precedence.
    pub fn append(&mut self, key: u32, data: &[u8]) -> Result<()> {
        ensure!(self.records.len() < 65536, "CatSystem2 CSS record budget");
        let end = self.bytes.len() + 8 + data.len();
        ensure!(end <= LIMIT && u32::try_from(data.len()).is_ok(), "CatSystem2 CSS extent budget");
        self.bytes.extend_from_slice(&key.to_le_bytes());
        self.bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        let begin = self.bytes.len();
        self.bytes.extend_from_slice(data);
        self.records.push((key, begin..end));
        Ok(())
    }
    /// 78B6C0: remove the first record with this key.
    pub fn remove(&mut self, key: u32) {
        let Some(at) = self.records.iter().position(|(id, _)| *id == key) else { return };
        let range = self.records.remove(at).1;
        let (from, count) = (range.start - 8, range.len() + 8);
        self.bytes.drain(from..from + count);
        for (_, other) in &mut self.records {
            if other.start > from {
                *other = other.start - count..other.end - count;
            }
        }
    }
    /// 78B680: forget the contents.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.records.clear();
    }
    /// 78B7E0: the nested form, a 16-byte header of pointer, capacity, size and tag.
    pub fn nested(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes.len() + 16);
        for word in [0, self.bytes.len() as u32, self.bytes.len() as u32, self.tag] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&self.bytes);
        out
    }
    /// 78B820: read the nested form.
    pub fn from_nested(data: &[u8]) -> Result<Self> {
        ensure!(data.len() >= 16, "CatSystem2 nested CSS truncated");
        let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        let size = word(8) as usize;
        let body = data.get(16..16 + size).context("CatSystem2 nested CSS extent")?.to_vec();
        Ok(Self { tag: word(12), records: records(&body, &crysirror_tasks::Cancellation::default())?, bytes: body })
    }
    /// 78B890's file image: `CSS\0`, packed size, size, tag, then zlib data.
    pub fn file_image(&self) -> Result<Vec<u8>> {
        use std::io::Write;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&self.bytes)?;
        let packed = encoder.finish()?;
        let mut out = Vec::with_capacity(packed.len() + 16);
        out.extend_from_slice(b"CSS\0");
        for word in [packed.len() as u32, self.bytes.len() as u32, self.tag] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&packed);
        Ok(out)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"tag":self.tag,"bytes":self.bytes.len(),"records":self.records.len()})
    }
}
///78BB10: missing file or wrong signature returns null; stricter bounded
/// validation rejects unsupported damaged extents rather than fake success.
pub(crate) fn load(
    storage: &dyn ClientStorage,
    client: ClientId,
    name: &str,
    cancel: crysirror_tasks::Cancellation,
) -> Result<Option<Container>> {
    cancel.check()?;
    let path = VfsPath::new(Mount::Save, name)?;
    let reader = match storage.open_read(&path) {
        Ok(r) => r,
        Err(Error::NotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let bytes = crysirror_resource::read_all(reader.as_ref(), LIMIT as u64)?;
    parse(bytes, client, cancel)
}
fn parse(
    bytes: Vec<u8>,
    client: ClientId,
    cancel: crysirror_tasks::Cancellation,
) -> Result<Option<Container>> {
    cancel.check()?;
    if bytes.get(..4) != Some(b"CSS\0") {
        return Ok(None);
    }
    ensure!(bytes.len() >= 16, "CatSystem2 CSS truncated header");
    let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let packed = word(4) as usize;
    let size = word(8) as usize;
    let tag = word(12);
    ensure!(
        size <= LIMIT && packed <= LIMIT && packed.checked_add(16) == Some(bytes.len()),
        "CatSystem2 CSS extent budget/layout"
    );
    let context = DecodeContext::new(client).with_cancellation(cancel.clone());
    let mut spec = TransformSpec::new("zlib");
    spec.expected_size = Some(size as u64);
    let stream = context.transforms.apply(
        &spec,
        Arc::new(MemoryAssetReader(Arc::from(bytes[16..].to_vec()))),
        &serde_json::Value::Null,
        &context,
    )?;
    let bytes = crysirror_resource::read_all(stream.as_ref(), LIMIT as u64)?;
    let records = records(&bytes, &cancel)?;
    cancel.check()?;
    Ok(Some(Container { tag, bytes, records }))
}
fn records(bytes: &[u8], cancel: &crysirror_tasks::Cancellation) -> Result<Vec<(u32, std::ops::Range<usize>)>> {
    let mut records = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        cancel.check()?;
        ensure!(records.len() < 65536, "CatSystem2 CSS record budget");
        let header = bytes
            .get(offset..offset + 8)
            .context("CatSystem2 CSS truncated record")?;
        let id = u32::from_le_bytes(header[..4].try_into()?);
        let len = u32::from_le_bytes(header[4..].try_into()?) as usize;
        let begin = offset + 8;
        let end = begin
            .checked_add(len)
            .context("CatSystem2 CSS record overflow")?;
        ensure!(end <= bytes.len(), "CatSystem2 CSS record extent");
        records.push((id, begin..end));
        offset = end;
    }
    Ok(records)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn css(body: &[u8]) -> Vec<u8> {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(body).unwrap();
        let z = z.finish().unwrap();
        let mut b = b"CSS\0".to_vec();
        for x in [z.len() as u32, body.len() as u32, 42] {
            b.extend(x.to_le_bytes());
        }
        b.extend(z);
        b
    }
    #[test]
    fn records_append_remove_nest_and_round_trip_the_file_image() {
        let mut c = Container::new(9);
        c.append(1, b"one").unwrap();
        c.append(2, b"two!").unwrap();
        c.append(1, b"late").unwrap();
        assert_eq!(c.first(1), Some(&b"one"[..]));
        c.remove(1);
        assert_eq!(c.first(1), Some(&b"late"[..]));
        assert_eq!(c.first(2), Some(&b"two!"[..]));
        let nested = Container::from_nested(&c.nested()).unwrap();
        assert_eq!((nested.tag, nested.first(2)), (9, Some(&b"two!"[..])));
        let parsed = parse(c.file_image().unwrap(), ClientId(1), crysirror_tasks::Cancellation::default())
            .unwrap().unwrap();
        assert_eq!((parsed.tag, parsed.first(1)), (9, Some(&b"late"[..])));
        c.clear();
        assert_eq!(c.first(2), None);
    }
    #[test]
    fn actual_header_tag_and_duplicate_record_order_are_preserved() {
        let c = ClientId(1);
        let token = crysirror_tasks::Cancellation::default();
        let mut body = Vec::new();
        body.extend(7u32.to_le_bytes());
        body.extend(3u32.to_le_bytes());
        body.extend(b"abc");
        body.extend(7u32.to_le_bytes());
        body.extend(0u32.to_le_bytes());
        let parsed = parse(css(&body), c, token.clone()).unwrap().unwrap();
        assert_eq!(parsed.tag, 42);
        assert_eq!(parsed.records, vec![(7, 8..11), (7, 19..19)]);
        assert_eq!(parsed.bytes, body);
        assert!(parse(b"not CSS".to_vec(), c, token.clone())
            .unwrap()
            .is_none());
        assert!(parse(css(&[1, 2]), c, token).is_err());
    }
}
