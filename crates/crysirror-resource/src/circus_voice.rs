//! Circus's older64-byte voice directory and aligned split data volumes.
//! DC2PC4265A0/426690 read count +64-byte(name60,packed_offset4) records.
//! Low8 offset bits select VOICE.DAT/VOICE2.DAT/...; the upper24 bits are
//! a256-aligned byte offset within that volume (427BA0 clears its low byte).
//! Only the index is resident. Entries are independent thread-safe range
//! readers and keep their source alive after the directory itself is dropped.
use crate::{Archive, ArchiveEntry, ArchiveFormat, DecodeContext, Probe, read_exact_at, subrange};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::AssetReader,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub struct CircusVoice;
/// Early D.C. directory: 48-byte name and a complete 32-bit byte offset.
/// DC.EXE4299A0/429A90/42CD60 read 52-byte records from a single file;
/// its low offset byte is data, not DC2's split-volume selector.
pub struct CircusVoice52;
const RECORD: usize = 64;
const MAX_ENTRIES: usize = 100_000;
fn count(source: &dyn AssetReader) -> Result<usize> {
    let mut bytes = [0; 4];
    read_exact_at(source, 0, &mut bytes)?;
    let count = u32::from_le_bytes(bytes) as usize;
    ensure!(
        count > 0 && count <= MAX_ENTRIES,
        "Circus voice directory count"
    );
    Ok(count)
}
impl ArchiveFormat for CircusVoice {
    fn id(&self) -> &'static str {
        "circus/voice64"
    }
    fn probe(&self, source: &dyn AssetReader) -> Result<Probe> {
        let len = source.len()?;
        if len < 4 + RECORD as u64 {
            return Ok(Probe { confidence: 0 });
        }
        let Ok(count) = count(source) else {
            return Ok(Probe { confidence: 0 });
        };
        let end = 4 + count as u64 * RECORD as u64;
        if end > len {
            return Ok(Probe { confidence: 0 });
        }
        let mut first = [0; RECORD];
        read_exact_at(source, 4, &mut first)?;
        let Some(nul) = first[..60].iter().position(|value| *value == 0) else {
            return Ok(Probe { confidence: 0 });
        };
        if nul == 0 {
            return Ok(Probe { confidence: 0 });
        }
        let (name, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&first[..nul]);
        let loc = u32::from_le_bytes(first[60..].try_into()?);
        let offset = u64::from(loc & !255);
        let valid = !error
            && name.to_ascii_lowercase().ends_with(".pcm")
            && loc != 0
            && (loc as u8 != 0 || offset >= end && offset < len);
        Ok(Probe {
            confidence: if valid { 85 } else { 0 },
        })
    }
    fn open(
        &self,
        source: Arc<dyn AssetReader>,
        context: &DecodeContext,
    ) -> Result<Box<dyn Archive>> {
        let count = count(source.as_ref())?;
        let index_end = 4 + count as u64 * RECORD as u64;
        ensure!(
            count <= context.limits.max_entries
                && index_end <= context.limits.max_index_size
                && index_end <= source.len()?,
            "Circus voice index bounds/budget"
        );
        let mut bytes = vec![0; count * RECORD];
        read_exact_at(source.as_ref(), 4, &mut bytes)?;
        let mut directory = Vec::with_capacity(count);
        let mut lookup = BTreeMap::new();
        let mut positions = BTreeMap::<u8, BTreeSet<u64>>::new();
        for (id, record) in bytes.chunks_exact(RECORD).enumerate() {
            let nul = record[..60]
                .iter()
                .position(|byte| *byte == 0)
                .context("Circus voice directory filename terminator")?;
            ensure!(nul > 0, "Circus voice empty directory name");
            let (name, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&record[..nul]);
            ensure!(!error, "Circus voice directory CP932");
            let name = name.replace('\\', "/");
            ensure!(
                VfsPath::new(Mount::Asset, &name)?.relative() == name,
                "Circus voice noncanonical member path"
            );
            let packed = u32::from_le_bytes(record[60..].try_into()?);
            ensure!(packed != 0, "Circus voice null native offset");
            let volume = packed as u8;
            let offset = u64::from(packed & !255);
            ensure!(
                volume != 0 || offset >= index_end,
                "Circus voice member overlaps its directory"
            );
            ensure!(
                lookup
                    .insert(name.to_ascii_lowercase(), id as u32)
                    .is_none(),
                "Circus voice duplicate member path"
            );
            positions.entry(volume).or_default().insert(offset);
            directory.push((name, volume, offset));
        }
        let mut sources = BTreeMap::new();
        sources.insert(0, source);
        for &volume in positions.keys().filter(|volume| **volume != 0) {
            let name = context
                .source_name
                .as_ref()
                .context("Circus split voice source_name required")?;
            let base = name
                .rsplit('/')
                .next()
                .context("Circus voice volume basename")?;
            let (stem, extension) = base
                .rsplit_once('.')
                .context("Circus voice volume extension")?;
            ensure!(
                !stem.is_empty() && !extension.is_empty(),
                "Circus voice volume filename"
            );
            let sibling = format!("{stem}{}.{extension}", u16::from(volume) + 1);
            sources.insert(volume, context.open_sibling(&sibling)?);
        }
        let mut extents = BTreeMap::new();
        for (&volume, offsets) in &positions {
            let len = sources
                .get(&volume)
                .context("Circus voice required volume missing")?
                .len()?;
            let offsets = offsets.iter().copied().collect::<Vec<_>>();
            for (index, &offset) in offsets.iter().enumerate() {
                let end = offsets.get(index + 1).copied().unwrap_or(len);
                ensure!(
                    offset < end && end <= len,
                    "Circus voice member range outside volume"
                );
                let size = end - offset;
                ensure!(
                    size >= 12 && size <= context.limits.max_entry_size,
                    "Circus voice member size/budget"
                );
                extents.insert((volume, offset), size);
            }
        }
        let mut entries = Vec::with_capacity(count);
        let mut ranges = Vec::with_capacity(count);
        for (id, (path, volume, offset)) in directory.into_iter().enumerate() {
            let size = extents[&(volume, offset)];
            ranges.push((volume, offset, size));
            entries.push(ArchiveEntry{id:id as u32,path,size,stored_size:size,metadata:serde_json::json!({"format":self.id(),"volume":volume,"offset":offset,"alignment":256})});
        }
        Ok(Box::new(Voice {
            sources,
            entries,
            ranges,
            lookup,
        }))
    }
}
impl ArchiveFormat for CircusVoice52 {
    fn id(&self) -> &'static str { "circus/voice52" }
    fn probe(&self, source: &dyn AssetReader) -> Result<Probe> {
        let len=source.len()?;
        if len<56 {return Ok(Probe::NO);}
        let Ok(n)=count(source) else{return Ok(Probe::NO)};
        let end=4+n as u64*52;
        if end>len {return Ok(Probe::NO);}
        let mut first=[0;52];read_exact_at(source,4,&mut first)?;
        let Some(zero)=first[..48].iter().position(|b|*b==0) else{return Ok(Probe::NO)};
        let (name,error)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&first[..zero]);
        let at=u64::from(u32::from_le_bytes(first[48..].try_into()?));
        Ok(Probe{confidence:if zero>0&&!error&&name.to_ascii_lowercase().ends_with(".pcm")&&at>=end&&at<len{85}else{0}})
    }
    fn open(&self, source:Arc<dyn AssetReader>, context:&DecodeContext)->Result<Box<dyn Archive>> {
        let n=count(source.as_ref())?;
        let end=4+n as u64*52;
        let len=source.len()?;
        ensure!(n<=context.limits.max_entries&&end<=context.limits.max_index_size&&end<=len,"Circus voice52 index bounds/budget");
        let mut bytes=vec![0;n*52];read_exact_at(source.as_ref(),4,&mut bytes)?;
        let mut directory=Vec::with_capacity(n);
        let mut lookup=BTreeMap::new();let mut offsets=BTreeSet::new();
        for (id,row) in bytes.chunks_exact(52).enumerate() {
            let zero=row[..48].iter().position(|b|*b==0).context("Circus voice52 filename terminator")?;
            ensure!(zero>0,"Circus voice52 empty filename");
            let (name,error)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&row[..zero]);
            ensure!(!error,"Circus voice52 filename CP932");
            let name=name.replace('\\',"/");
            ensure!(VfsPath::new(Mount::Asset,&name)?.relative()==name,"Circus voice52 member path");
            let offset=u64::from(u32::from_le_bytes(row[48..].try_into()?));
            ensure!(offset>=end&&offset<len,"Circus voice52 member overlaps directory/outside file");
            ensure!(lookup.insert(name.to_ascii_lowercase(),id as u32).is_none(),"Circus voice52 duplicate member");
            offsets.insert(offset);directory.push((name,offset));
        }
        let ordered=offsets.into_iter().collect::<Vec<_>>();
        let mut ranges=Vec::with_capacity(n);let mut entries=Vec::with_capacity(n);
        for (id,(path,offset)) in directory.into_iter().enumerate() {
            let index=ordered.binary_search(&offset).unwrap();
            let size=ordered.get(index+1).copied().unwrap_or(len)-offset;
            ensure!(size>=12&&size<=context.limits.max_entry_size,"Circus voice52 member size/budget");
            ranges.push((0,offset,size));
            entries.push(ArchiveEntry{id:id as u32,path,size,stored_size:size,metadata:serde_json::json!({"format":self.id(),"offset":offset})});
        }
        Ok(Box::new(Voice{sources:BTreeMap::from([(0,source)]),entries,ranges,lookup}))
    }
}
struct Voice {
    sources: BTreeMap<u8, Arc<dyn AssetReader>>,
    entries: Vec<ArchiveEntry>,
    ranges: Vec<(u8, u64, u64)>,
    lookup: BTreeMap<String, u32>,
}
impl Archive for Voice {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.lookup
            .get(&path.replace('\\', "/").to_ascii_lowercase())
            .copied()
    }
    fn open(&self, id: u32, context: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        let &(volume, offset, size) = self
            .ranges
            .get(id as usize)
            .context("Circus voice entry id")?;
        ensure!(
            size <= context.limits.max_entry_size,
            "Circus voice entry budget"
        );
        subrange(
            self.sources
                .get(&volume)
                .context("Circus voice volume ownership")?
                .clone(),
            offset,
            size,
        )
    }
}
