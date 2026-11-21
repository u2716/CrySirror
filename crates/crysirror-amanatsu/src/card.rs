// SPDX-License-Identifier: MPL-2.0
//! `Character.HumanData` card files (`UserData/chara`, `DefaultData/*/chara`):
//! a PNG thumbnail followed by `productNo`, the `【ALChara】` marker, the
//! version, an optional face image, a MessagePack `BlockHeader`
//! (`lstInfo`: name, version, pos, size) and the block bytes
//! (`Custom`, `Coordinate`, `Parameter`, `Status`, `Graphic`, `About`,
//! `GameParameter_AL`, `GameInfo_AL`, …).
use anyhow::{ensure, Context, Result};
use crysirror_unity::msgpack::{self, Value};

pub const MARKER: &str = "【ALChara】";

/// A card's blocks: (name, version, MessagePack bytes).
#[derive(Clone, Debug)]
pub struct Card {
    pub product: i32,
    pub version: String,
    pub blocks: Vec<(String, String, Vec<u8>)>,
}

struct R<'a> {
    b: &'a [u8],
    at: usize,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let s = self.b.get(self.at..self.at.checked_add(n).context("card length")?).context("card truncated")?;
        self.at += n;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into()?))
    }
    /// `BinaryWriter.Write(string)`: 7-bit encoded length, UTF-8.
    fn string(&mut self) -> Result<String> {
        let mut n = 0usize;
        for shift in (0..35).step_by(7) {
            let b = self.take(1)?[0];
            n |= usize::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                break;
            }
        }
        Ok(String::from_utf8(self.take(n)?.to_vec())?)
    }
}

/// End of the PNG (after the `IEND` chunk), or 0 when the data is not a PNG.
fn png_end(b: &[u8]) -> Result<usize> {
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(0);
    }
    let mut at = 8;
    loop {
        let len = u32::from_be_bytes(b.get(at..at + 4).context("PNG chunk")?.try_into()?) as usize;
        let ty = b.get(at + 4..at + 8).context("PNG chunk type")?;
        at = at.checked_add(12 + len).context("PNG chunk length")?;
        if ty == b"IEND" {
            return Ok(at);
        }
    }
}

impl Card {
    /// A `【ALChara】` card (PNG prefix allowed).
    pub fn parse(b: &[u8]) -> Result<Self> {
        let c = Self::container(b)?;
        ensure!(c.0 == MARKER, "not an AmanatsuLocation character card ({:?})", c.0);
        Ok(c.1)
    }

    /// Any ILLGAMES block container (`【ALChara】`, `【ALClothes】`, …): the
    /// marker and its blocks. Between the version and the MessagePack block
    /// header the formats differ (character: face image; clothes: name and
    /// flags), so the header is located by its `lstInfo` map.
    pub fn container(b: &[u8]) -> Result<(String, Self)> {
        let mut r = R { b, at: png_end(b)? };
        let product = r.i32()?;
        let marker = r.string()?;
        ensure!(marker.starts_with('【'), "not an ILLGAMES container ({marker:?})");
        let version = r.string()?;
        let mut header = None;
        let start = r.at;
        let face = R { b, at: start }.i32().unwrap_or(-1);
        // Character cards: face image size and bytes first.
        let mut candidates: Vec<usize> = Vec::new();
        if face >= 0 && (face as usize) < b.len() {
            candidates.push(start + 4 + face as usize);
        }
        candidates.extend(start..start + 64);
        for at in candidates {
            let mut t = R { b, at };
            let Ok(len) = t.i32() else { continue };
            if len <= 0 || !matches!(b.get(t.at), Some(0x80..=0x8f)) {
                continue;
            }
            let Ok(bytes) = t.take(len as usize) else { continue };
            if let Ok(h) = msgpack::decode(bytes) {
                if h.get("lstInfo").is_some() {
                    header = Some((h, t.at));
                    break;
                }
            }
        }
        let (header, at) = header.context("card block header")?;
        r.at = at;
        let total = r.i64()?;
        ensure!(total >= 0, "card data size");
        let data = r.take(total as usize)?;
        let mut blocks = Vec::new();
        for info in header.get("lstInfo").map(Value::as_array).unwrap_or(&[]) {
            let name = info.get("name").and_then(Value::as_str).unwrap_or("").to_owned();
            let ver = info.get("version").and_then(Value::as_str).unwrap_or("").to_owned();
            let pos = info.get("pos").and_then(Value::as_i64).unwrap_or(-1);
            let size = info.get("size").and_then(Value::as_i64).unwrap_or(-1);
            ensure!(pos >= 0 && size >= 0, "card block {name} range");
            let bytes = data.get(pos as usize..(pos + size) as usize).with_context(|| format!("card block {name}"))?;
            blocks.push((name, ver, bytes.to_vec()));
        }
        Ok((marker, Self { product, version, blocks }))
    }

    /// A block of consecutive `i32`-length-prefixed MessagePack values
    /// (`Custom`: face, body).
    pub fn parts(&self, name: &str) -> Result<Vec<Value>> {
        let b = self.block(name).with_context(|| format!("card block {name}"))?;
        let mut r = R { b, at: 0 };
        let mut out = Vec::new();
        while r.at < b.len() {
            let n = r.i32()?;
            ensure!(n >= 0, "card {name} part size");
            out.push(msgpack::decode(r.take(n as usize)?)?);
        }
        Ok(out)
    }

    pub fn block(&self, name: &str) -> Option<&[u8]> {
        self.blocks.iter().find(|b| b.0 == name).map(|b| b.2.as_slice())
    }
}
