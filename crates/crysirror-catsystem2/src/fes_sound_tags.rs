// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native SOUND tag tables. Storage and worker ownership belong to CrySirror;
//! only the original24-byte cue records and60Hz wait policy live here.
use anyhow::{ensure, Result};
use crysirror_core::{path::{Mount,VfsPath},service::ClientStorage};
use crysirror_tasks::{Task,TaskPool};
use std::{collections::BTreeMap,sync::Arc};
type Table=BTreeMap<Vec<u8>,u32>;
#[derive(Default)]
pub(crate) struct Tags {
    tables:BTreeMap<u32,Table>,
    completed:std::collections::BTreeSet<(u32,u32,u32)>,
    pending:Option<((u32,u32,u32),Task<Table>)>,
}
impl Tags {
    pub fn init(&mut self,owner:u32,pc:u32,id:u32,name:&[u8],storage:Arc<dyn ClientStorage>,pool:&TaskPool)->Result<bool>{
        let key=(owner,pc,id);
        if self.completed.contains(&key){return Ok(true);}
        if self.pending.is_none(){
            let stem=String::from_utf8_lossy(crate::native_scene::stem(name)).into_owned();
            let permit=match pool.reserve(){Ok(p)=>p,Err(e)if e.is::<crysirror_tasks::CapacityExceeded>()=>return Ok(false),Err(e)=>return Err(e)};
            self.pending=Some((key,permit.spawn(move|cancel|{
                cancel.check()?;
                let path=VfsPath::new(Mount::Asset,&format!("pcm_tag.int/{stem}.tag"))?;
                let Some(path)=crate::native_resource_paths::resolve(storage.as_ref(),&path)? else{return Ok(Table::new())};
                let bytes=crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(),1024*1024)?;
                ensure!(bytes.len()%24==0,"CatSystem2 SOUND tag table extent");
                let mut table=Table::new();
                for row in bytes.chunks_exact(24){cancel.check()?;let end=row[..20].iter().position(|b|*b==0).unwrap_or(20);table.insert(row[..end].to_vec(),u32::from_le_bytes(row[20..24].try_into().unwrap()));}
                Ok(table)
            })?));
            return Ok(false);
        }
        ensure!(self.pending.as_ref().unwrap().0==key,"CatSystem2 SOUND tag pending owner changed");
        let Some(table)=self.pending.as_mut().unwrap().1.try_take()?else{return Ok(false)};
        self.pending=None;self.tables.insert(id,table);self.completed.insert(key);Ok(true)
    }
    pub fn finish(&mut self,owner:u32,pc:u32){self.completed.retain(|key|key.0!=owner||key.1!=pc);}
    pub fn remaining(&self,id:u32,name:&[u8],first_play_ms:Option<u32>,now:u32)->u32{
        let cue=self.tables.get(&id).and_then(|t|t.get(name)).copied().unwrap_or(0);
        let elapsed=now.wrapping_sub(first_play_ms.unwrap_or(0));
        if elapsed<cue{cue.wrapping_sub(elapsed).wrapping_mul(60)/1000}else{0}
    }
}
