//! D.C.II P.C.425570's version6DAT/names.dat directory, independent of the
//! later version7 GRP/NAMES.DAT. Native records retain their complete bytes;
//! names are semantic paths and never executable/GPU pointers.
use anyhow::{Context, Result, ensure};
const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug)]
pub struct Record {
    bytes: Vec<u8>,
}
impl Record {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn name(&self) -> Result<String> {
        cstring(
            self.bytes
                .get(..128)
                .context("DC2 native resource-name field")?,
        )
    }
}
#[derive(Clone, Debug)]
pub struct Names {
    pub bootstrap: Vec<String>,
    pub music: Vec<Record>,
    pub sfx: Vec<Record>,
    pub images: Vec<Record>,
    pub scenes: Vec<Record>,
    ///425570 performs no EOF test after the last table. Retain any bounded
    ///unused tail without treating it as another recognized format/profile.
    pub trailing: Vec<u8>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, length: usize) -> Result<&[u8]> {
        let end = self
            .at
            .checked_add(length)
            .context("DC2 name-directory cursor overflow")?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .context("truncated DC2 name directory")?;
        self.at = end;
        Ok(bytes)
    }
    fn count(&mut self) -> Result<usize> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?) as usize)
    }
    fn table(&mut self, stride: usize, exclusive_limit: usize) -> Result<Vec<Record>> {
        let count = self.count()?;
        ensure!(count < exclusive_limit, "DC2 native name table count");
        let bytes = self.take(count.checked_mul(stride).context("DC2 name table extent")?)?;
        bytes
            .chunks_exact(stride)
            .map(|bytes| {
                cstring(&bytes[..128])?;
                Ok(Record {
                    bytes: bytes.to_vec(),
                })
            })
            .collect()
    }
}
impl Names {
    ///Only semantic names are projected to the shared resource lookup. Full
    ///version6 rows stay retained separately for native flags/volume metadata.
    pub(crate) fn semantic_names(&self) -> Result<crate::names::Names> {
        let names =
            |records: &[Record]| records.iter().map(Record::name).collect::<Result<Vec<_>>>();
        Ok(crate::names::Names {
            bootstrap: self.bootstrap.clone(),
            music: names(&self.music)?,
            sfx: names(&self.sfx)?,
            images: names(&self.images)?,
            scenes: names(&self.scenes)?,
        })
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_BYTES, "DC2 name directory byte limit");
        let mut reader = Reader { bytes, at: 0 };
        ensure!(reader.count()? == 6, "DC2 native name directory version");
        let bootstrap = reader
            .take(0x300)?
            .chunks_exact(32)
            .map(cstring)
            .collect::<Result<Vec<_>>>()?;
        let music = reader.table(192, 100)?;
        let sfx = reader.table(132, 300)?;
        let images = reader.table(128, 1500)?;
        let scenes = reader.table(188, 1500)?;
        let trailing = reader.bytes[reader.at..].to_vec();
        Ok(Self {
            bootstrap,
            music,
            sfx,
            images,
            scenes,
            trailing,
        })
    }
}
fn cstring(bytes: &[u8]) -> Result<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .context("unterminated DC2 native resource name")?;
    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes[..end]);
    ensure!(!error, "DC2 resource-name CP932 encoding");
    Ok(text.into_owned())
}
