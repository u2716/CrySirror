//! Seekable CRXB/CRM directory; usable independently of any VM.
//! Format reference: GARbro ArcFormats/Circus/ArcCRM.cs at
//! b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0 (2016 morkt, MIT).
//! The complete permission notice is retained in circus_crm.LICENSE.
use crate::{Archive,ArchiveEntry,ArchiveFormat,DecodeContext,Probe,read_exact_at,subrange};
use anyhow::{Context,Result,ensure};
use crysirror_core::{path::{Mount,VfsPath},service::AssetReader};
use std::{collections::BTreeMap,sync::Arc};
pub struct CircusCrm;
impl ArchiveFormat for CircusCrm {
    fn id(&self)->&'static str {"circus/crm"}
    fn probe(&self,source:&dyn AssetReader)->Result<Probe>{
        if source.len()?<16{return Ok(Probe{confidence:0})}
        let mut signature=[0;4];read_exact_at(source,0,&mut signature)?;
        Ok(Probe{confidence:if &signature==b"CRXB"{100}else{0}})
    }
    fn open(&self,source:Arc<dyn AssetReader>,context:&DecodeContext)->Result<Box<dyn Archive>>{
        ensure!(self.probe(source.as_ref())?.confidence==100,"Circus CRM signature");
        let length=source.len()?;let mut header=[0;16];read_exact_at(source.as_ref(),0,&mut header)?;
        let count=u32::from_le_bytes(header[8..12].try_into()?) as usize;
        ensure!(count>0&&count<=context.limits.max_entries,"Circus CRM entry budget");
        let end=16+u64::try_from(count)?*32;
        ensure!(end<=length&&end<=context.limits.max_index_size,"Circus CRM directory extent");
        let mut directory=vec![0;count*32];read_exact_at(source.as_ref(),16,&mut directory)?;
        let mut offsets=BTreeMap::new();let mut names=BTreeMap::new();let mut entries=Vec::with_capacity(count);
        for (id,row) in directory.chunks_exact(32).enumerate(){
            let offset=u64::from(u32::from_le_bytes(row[..4].try_into()?));
            ensure!(offset>=end&&offset<length&&offsets.insert(offset,id as u32).is_none(),"Circus CRM entry offset/alias");
            let nul=row[8..].iter().position(|v|*v==0).unwrap_or(24);
            let (name,errors)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&row[8..8+nul]);
            ensure!(!errors&&!name.is_empty(),"Circus CRM entry filename");
            let path=name.replace('\\',"/");
            ensure!(VfsPath::new(Mount::Asset,&path)?.relative()==path,"Circus CRM noncanonical path");
            ensure!(names.insert(path.to_ascii_lowercase(),id as u32).is_none(),"Circus CRM duplicate entry name");
            entries.push(ArchiveEntry{id:id as u32,path,size:0,stored_size:0,metadata:serde_json::json!({"offset":offset,"format":self.id()})});
        }
        let sorted=offsets.iter().map(|(offset,id)|(*offset,*id)).collect::<Vec<_>>();let mut ranges=vec![(0,0);count];
        for (n,(offset,id)) in sorted.iter().copied().enumerate(){
            let next=sorted.get(n+1).map_or(length,|entry|entry.0);let size=next-offset;
            ensure!(size<=context.limits.max_entry_size,"Circus CRM entry extent budget");
            entries[id as usize].size=size;entries[id as usize].stored_size=size;ranges[id as usize]=(offset,size);
        }
        Ok(Box::new(Crm{source,entries,names,ranges}))
    }
}
struct Crm{source:Arc<dyn AssetReader>,entries:Vec<ArchiveEntry>,names:BTreeMap<String,u32>,ranges:Vec<(u64,u64)>}
impl Archive for Crm {
    fn entries(&self)->&[ArchiveEntry]{&self.entries}
    fn resolve(&self,path:&str)->Option<u32>{self.names.get(&path.replace('\\',"/").to_ascii_lowercase()).copied()}
    fn open(&self,id:u32,context:&DecodeContext)->Result<Arc<dyn AssetReader>>{
        let (offset,size)=*self.ranges.get(id as usize).context("Circus CRM entry ID")?;
        ensure!(size<=context.limits.max_entry_size,"Circus CRM entry budget");subrange(self.source.clone(),offset,size)
    }
}
#[cfg(test)]
mod tests{
    use super::*;use crysirror_core::{service::MemoryAssetReader,state::ClientId};
    fn data()->Vec<u8>{let mut b=vec![0;84];b[..4].copy_from_slice(b"CRXB");b[8..12].copy_from_slice(&2u32.to_le_bytes());b[16..20].copy_from_slice(&82u32.to_le_bytes());b[24..25].copy_from_slice(b"B");b[48..52].copy_from_slice(&80u32.to_le_bytes());b[56..57].copy_from_slice(b"A");b[80..].copy_from_slice(b"abcd");b}
    #[test]fn unsorted_directory_resolves_case_and_infers_sizes(){let context=DecodeContext::new(ClientId(1));let arc=CircusCrm.open(Arc::new(MemoryAssetReader(data().into())),&context).unwrap();assert_eq!(arc.resolve("a"),Some(1));assert_eq!(crate::read_all(arc.open(1,&context).unwrap().as_ref(),2).unwrap(),b"ab");assert_eq!(crate::read_all(arc.open(0,&context).unwrap().as_ref(),2).unwrap(),b"cd");}
    #[test]fn aliases_and_directory_overlap_are_rejected(){for offset in [16,80,84]{let mut b=data();b[16..20].copy_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());assert!(CircusCrm.open(Arc::new(MemoryAssetReader(b.into())),&DecodeContext::new(ClientId(1))).is_err());}}
}
