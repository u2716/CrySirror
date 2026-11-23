//! Ordinal Circus image frame packs, independent of a script VM. The legacy
//! count/offset/size index and CRXB directories preserve their original order.
use crate::{Archive,ArchiveEntry,ArchiveFormat,DecodeContext,Probe,read_exact_at,subrange};
use anyhow::{Context,Result,ensure};
use crysirror_core::service::AssetReader;
use std::{collections::BTreeMap,sync::Arc};
pub struct CircusFramePack;
fn word(bytes:&[u8],offset:usize)->Result<u32>{Ok(u32::from_le_bytes(bytes.get(offset..offset+4).context("frame pack dword")?.try_into()?))}
impl ArchiveFormat for CircusFramePack {
    fn id(&self)->&'static str{"circus/frame_pack"}
    fn probe(&self,source:&dyn AssetReader)->Result<Probe>{
        if source.len()?<12{return Ok(Probe{confidence:0})}
        let mut bytes=[0;12];read_exact_at(source,0,&mut bytes)?;
        let count=word(&bytes,0)?;
        Ok(Probe{confidence:if &bytes[..4]==b"CRXB"{95}else if count>0&&count<=100000&&u64::from(word(&bytes,4)?)>=4+8*u64::from(count){40}else{0}})
    }
    fn open(&self,source:Arc<dyn AssetReader>,context:&DecodeContext)->Result<Box<dyn Archive>>{
        let length=source.len()?;ensure!(length>=12,"frame pack header");
        let mut header=[0;12];read_exact_at(source.as_ref(),0,&mut header)?;
        let named=&header[..4]==b"CRXB";
        let count=word(&header,if named{8}else{0})? as usize;
        ensure!(count>0&&count<=context.limits.max_entries,"frame pack entry budget");
        let (start,stride)=if named{(16usize,32usize)}else{(4,8)};
        let end=start.checked_add(count.checked_mul(stride).context("frame pack index overflow")?).context("frame pack index overflow")?;
        ensure!(end as u64<=length&&end as u64<=context.limits.max_index_size,"frame pack index extent");
        let mut table=vec![0;count*stride];read_exact_at(source.as_ref(),start as u64,&mut table)?;
        let mut ranges=Vec::with_capacity(count);let mut offsets=BTreeMap::new();
        for (id,row) in table.chunks_exact(stride).enumerate(){
            let offset=u64::from(word(row,0)?);
            ensure!(offset>=end as u64&&offset<length,"frame pack image offset");
            let size=if named{0}else{u64::from(word(row,4)?)};
            if !named{ensure!(size>0&&offset.checked_add(size).is_some_and(|end|end<=length),"frame pack image extent");}
            // Repeated ranges are valid in ordinal animation packs; frames
            // may share the same immutable source image.
            offsets.insert(offset,());ranges.push((offset,size));
            ensure!(id<u32::MAX as usize,"frame pack ordinal overflow");
        }
        let sorted=offsets.keys().copied().collect::<Vec<_>>();
        let mut entries=Vec::with_capacity(count);
        for (id,(offset,size)) in ranges.iter_mut().enumerate(){
            if named {let index=sorted.binary_search(offset).unwrap();*size=sorted.get(index+1).copied().unwrap_or(length)-*offset;}
            ensure!(*size<=context.limits.max_entry_size,"frame pack image budget");
            entries.push(ArchiveEntry{id:id as u32,path:format!("{id:06}.crx"),size:*size,stored_size:*size,metadata:serde_json::json!({"offset":offset,"format":self.id()})});
        }
        Ok(Box::new(Frames{source,ranges,entries,named}))
    }
}
struct Frames {source:Arc<dyn AssetReader>,ranges:Vec<(u64,u64)>,entries:Vec<ArchiveEntry>,named:bool}
impl Archive for Frames {
    fn entries(&self)->&[ArchiveEntry]{&self.entries}
    fn resolve(&self,path:&str)->Option<u32>{self.entries.iter().find(|entry|entry.path.eq_ignore_ascii_case(path)).map(|entry|entry.id)}
    fn open(&self,id:u32,context:&DecodeContext)->Result<Arc<dyn AssetReader>>{
        let (mut offset,mut size)=*self.ranges.get(id as usize).context("frame pack ordinal")?;
        ensure!(size<=context.limits.max_entry_size,"frame pack image budget");
        if self.named {
            let mut signature=[0;4];read_exact_at(self.source.as_ref(),offset,&mut signature)?;
            if &signature==b"CRXD"{ensure!(size>=36,"frame pack CRXD header");offset+=32;size-=32;read_exact_at(self.source.as_ref(),offset,&mut signature)?;}
            if &signature==b"CRXJ" {
                ensure!(size>=12,"frame pack CRXJ header");let mut row=[0;12];read_exact_at(self.source.as_ref(),offset,&mut row)?;
                let reference=u64::from(word(&row,8)?);
                let (_,extent)=self.ranges.iter().find(|(at,_)|*at==reference).context("frame pack CRXJ reference")?;
                ensure!(*extent>=36,"frame pack CRXJ image extent");offset=reference+32;size=*extent-32;
            }
        }
        subrange(self.source.clone(),offset,size)
    }
}
#[cfg(test)] mod tests {
    use super::*;use crysirror_core::{service::MemoryAssetReader,state::ClientId};
    #[test] fn ordinals_preserve_shared_images_and_reject_external_ranges(){
        let context=DecodeContext::new(ClientId(1));let mut bytes=vec![0;24];
        bytes[..4].copy_from_slice(&2u32.to_le_bytes());
        for at in [4,12]{bytes[at..at+4].copy_from_slice(&20u32.to_le_bytes());bytes[at+4..at+8].copy_from_slice(&4u32.to_le_bytes());}
        bytes[20..].copy_from_slice(b"CRXG");
        let pack=CircusFramePack.open(Arc::new(MemoryAssetReader(bytes.clone().into())),&context).unwrap();
        assert_eq!(pack.entries().len(),2);
        assert_eq!(crate::read_all(pack.open(1,&context).unwrap().as_ref(),4).unwrap(),b"CRXG");
        bytes[4..8].copy_from_slice(&19u32.to_le_bytes());
        assert!(CircusFramePack.open(Arc::new(MemoryAssetReader(bytes.into())),&context).is_err());
    }
    #[test] fn named_frame_indirection_is_bounded_and_keeps_delta_payload_policy(){
        let context=DecodeContext::new(ClientId(1));let mut bytes=vec![0;160];
        bytes[..4].copy_from_slice(b"CRXB");bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&80u32.to_le_bytes());bytes[48..52].copy_from_slice(&124u32.to_le_bytes());
        bytes[80..84].copy_from_slice(b"CRXD");bytes[112..116].copy_from_slice(b"CRXJ");bytes[120..124].copy_from_slice(&124u32.to_le_bytes());
        bytes[124..128].copy_from_slice(b"CRXD");bytes[156..160].copy_from_slice(b"CRXG");
        let pack=CircusFramePack.open(Arc::new(MemoryAssetReader(bytes.into())),&context).unwrap();
        assert_eq!(crate::read_all(pack.open(0,&context).unwrap().as_ref(),4).unwrap(),b"CRXG");
    }
}
