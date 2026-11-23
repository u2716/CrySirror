//! Bounded differential CRXD decoding and CRXJ dependency resolution.
//! Reference: GARbro ImageCRXD.cs at b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0,
//! Copyright (C) 2016 morkt. MIT notice retained in crxd.LICENSE.
use crate::{crx::{self,CrxImage},MAX_DECODED_BYTES};
use anyhow::{Context,Result,bail,ensure};
use std::collections::BTreeSet;

/// A declarative resource dependency; callers resolve it within their owned
/// archive or filesystem capability. These are never executable addresses.
#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum Dependency { Name(String), ArchiveOffset(u32) }

/// Decode an ordinary image or differential image with caller-scoped resources.
/// Offsets are exact directory-entry offsets, never arbitrary byte slices.
pub fn decode(bytes:&[u8],resolve:&mut dyn FnMut(&Dependency)->Result<Vec<u8>>)->Result<CrxImage>{
    decode_with_optional_offsets(bytes,&mut |dependency|resolve(dependency).map(Some))
}
/// None for an offset means the caller has no archive-offset capability. An
/// invalid offset in an available archive is an error and never falls back to
/// another named resource. Named dependencies must always return Some bytes.
pub fn decode_with_optional_offsets(bytes:&[u8],resolve:&mut dyn FnMut(&Dependency)->Result<Option<Vec<u8>>>)->Result<CrxImage>{
    ensure!(bytes.len()<=MAX_DECODED_BYTES,"CRXD encoded image budget");
    if bytes.starts_with(b"CRXG"){return crx::decode(bytes)}
    ensure!(bytes.starts_with(b"CRXD")&&bytes.len()>=36,"CRXD header/signature");
    let mut seen=BTreeSet::new();let mut budget=bytes.len();
    let delta=delta(bytes,resolve,&mut seen,&mut budget,0)?;
    let offset=u32::from_le_bytes(bytes[8..12].try_into()?);
    let name_bytes=&bytes[12..32];let end=name_bytes.iter().position(|v|*v==0).unwrap_or(name_bytes.len());
    let (name,errors)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&name_bytes[..end]);
    ensure!(!errors,"CRXD base name encoding");
    let base_bytes=match resolve(&Dependency::ArchiveOffset(offset))? {
        Some(data)=>data,
        None if !name.is_empty()=>resolve(&Dependency::Name(name.into_owned()))?.context("CRXD named base resource missing")?,
        None=>bail!("CRXD base has no available archive or named dependency"),
    };
    ensure!(base_bytes.len()<=MAX_DECODED_BYTES,"CRXD base resource budget");
    let base=crx::decode(&base_bytes)?;
    let colors=u16::from_le_bytes(base_bytes[16..18].try_into()?);
    ensure!(colors==delta.1&&colors<=1,"CRXD base/delta pixel formats differ or are indexed");
    apply_delta(base,&delta.0,colors==1)
}
fn delta(bytes:&[u8],resolve:&mut dyn FnMut(&Dependency)->Result<Option<Vec<u8>>>,seen:&mut BTreeSet<u32>,budget:&mut usize,depth:usize)->Result<(CrxImage,u16)>{
    ensure!(depth<32&&bytes.starts_with(b"CRXD")&&bytes.len()>=36,"CRXD linked delta depth/header");
    match &bytes[32..36]{
        b"CRXG"=>{
            let payload=&bytes[32..];let image=crx::decode(payload)?;
            Ok((image,u16::from_le_bytes(payload[16..18].try_into()?)))
        },
        b"CRXJ"=>{
            let pointer=u32::from_le_bytes(bytes.get(40..44).context("CRXD truncated CRXJ link")?.try_into()?);
            ensure!(seen.insert(pointer),"CRXD linked delta cycle");
            let data=resolve(&Dependency::ArchiveOffset(pointer))?.context("CRXD linked delta requires an archive-offset capability")?;
            *budget=budget.checked_add(data.len()).context("CRXD dependency budget overflow")?;
            ensure!(*budget<=MAX_DECODED_BYTES,"CRXD dependency byte budget");
            delta(&data,resolve,seen,budget,depth+1)
        },
        _=>bail!("CRXD unknown delta signature"),
    }
}
/// Native differential bytes wrap rather than saturate. Alpha is subtracted;
/// RGB is added. Origins are absolute image coordinates and may be negative.
pub fn apply_delta(mut base:CrxImage,delta:&CrxImage,alpha:bool)->Result<CrxImage>{
    base.surface.validate()?;delta.surface.validate()?;
    let left=i64::from(base.offset[0]).max(i64::from(delta.offset[0]));
    let top=i64::from(base.offset[1]).max(i64::from(delta.offset[1]));
    let right=(i64::from(base.offset[0])+i64::from(base.surface.width)).min(i64::from(delta.offset[0])+i64::from(delta.surface.width));
    let bottom=(i64::from(base.offset[1])+i64::from(base.surface.height)).min(i64::from(delta.offset[1])+i64::from(delta.surface.height));
    for y in top..bottom {for x in left..right {
        let dst=usize::try_from(((y-i64::from(base.offset[1]))*i64::from(base.surface.width)+x-i64::from(base.offset[0]))*4)?;
        let src=usize::try_from(((y-i64::from(delta.offset[1]))*i64::from(delta.surface.width)+x-i64::from(delta.offset[0]))*4)?;
        for channel in 0..3 {base.surface.rgba[dst+channel]=base.surface.rgba[dst+channel].wrapping_add(delta.surface.rgba[src+channel]);}
        if alpha {base.surface.rgba[dst+3]=base.surface.rgba[dst+3].wrapping_sub(delta.surface.rgba[src+3]);}
    }}
    Ok(base)
}
#[cfg(test)]
mod tests{
    use super::*;use crate::ImageSurface;
    fn image(offset:[i16;2],width:u32,pixels:&[u8])->CrxImage{CrxImage{offset,surface:ImageSurface{width,height:1,rgba:pixels.to_vec()}}}
    #[test]fn absolute_overlap_wraps_color_and_subtracts_alpha(){
        let base=image([-2,7],2,&[250,2,10,20,5,250,7,1]);let delta=image([-1,7],2,&[253,10,255,2,9,9,9,9]);
        let result=apply_delta(base,&delta,true).unwrap();assert_eq!(result.surface.rgba,[250,2,10,20,2,4,6,255]);assert_eq!(result.offset,[-2,7]);
    }
    #[test]fn disjoint_origins_preserve_base(){let base=image([0,0],1,&[1,2,3,4]);let result=apply_delta(base.clone(),&image([0,1],1,&[255;4]),true).unwrap();assert_eq!(result,base);}
    #[test]fn cyclic_link_is_bounded_before_base_resolution(){let mut b=vec![0;44];b[..4].copy_from_slice(b"CRXD");b[32..36].copy_from_slice(b"CRXJ");b[40..44].copy_from_slice(&7u32.to_le_bytes());let mut calls=0;assert!(decode(&b,&mut |_|{calls+=1;Ok(b.clone())}).is_err());assert_eq!(calls,1);}
}
