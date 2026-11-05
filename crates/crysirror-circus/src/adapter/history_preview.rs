//!402BC0 owns one real temporary-replay image. Capture is a shared selected
//!Canvas/ImageJob, never a slot PNG, desktop read or VM pixel compositor.
use super::*;
use crate::machine::history_ops::{HistoryReply,HistoryRequest};
use crysirror_core::image_processing::{ImageJob,ImageProcessState};
const LAYER:u32=0x38cf_0000;
const IMAGE:u32=0x35cf_0000;
#[derive(Default)]
pub(super) struct NativeHistoryPreview {
    owner:Option<i32>,
    image:Option<UploadedImage>,
    job:Option<ImageJob>,
    published:bool,
    sampled:bool,
}
impl NativeHistoryPreview {
    pub(super) fn pending(&self)->bool {self.job.is_some()}
}
impl Bridge<'_> {
    pub(super) fn close_history_preview(&mut self)->Result<()> {
        self.hide_history_preview()?;
        let preview=&mut self.services.history_preview;
        preview.job=None;preview.owner=None;preview.sampled=false;
        if let Some(image)=preview.image.take(){self.host.release_image_resource(self.client,image.path.relative())?;}
        Ok(())
    }
    fn hide_history_preview(&mut self)->Result<()> {
        if self.services.history_preview.published {
            self.submit(vec![json!({"op":"remove","id":LAYER})],vec![])?;
            self.services.history_preview.published=false;
        }
        Ok(())
    }
    pub(super) fn history_preview_request(&mut self,request:&HistoryRequest)->Result<Option<HistoryReply>> {
        match request {
            HistoryRequest::CapturePreview{flat_index}=>{
                if self.services.history_preview.owner==Some(*flat_index)&&self.services.history_preview.image.is_some(){return Ok(Some(HistoryReply::Ready))}
                if self.services.history_preview.owner!=Some(*flat_index){
                    self.close_history_preview()?;
                    self.services.history_preview.owner=Some(*flat_index);
                }
                if !self.services.history_preview.sampled {
                    if !self.sprite_mask_resources_ready()?||!self.prepare_sprite_mask_frame()?
                        ||!self.prepare_packed_tick(self.native_effect_fast())?||!self.effects_ready()? {return Ok(None)}
                    //402BC0 explicitly executes433E50 before drawing. It is
                    //one native update even when the outer host timestamp is
                    //unchanged; prepare resource dependencies before advancing.
                    self.services.frame_at=None;
                    self.native_frame()?;
                    self.services.history_preview.sampled=true;
                }
                if self.services.history_preview.job.is_none(){
                    let Some(job)=self.begin_history_preview_image()? else{return Ok(None)};
                    self.services.history_preview.job=Some(job);
                }
                ensure!(self.services.history_preview.owner==Some(*flat_index),"Circus history preview capture owner");
                let image=match self.services.history_preview.job.as_mut().unwrap().poll()? {
                    ImageProcessState::Pending=>return Ok(None),
                    ImageProcessState::Failed(error)=>bail!("Circus history preview capture failed: {error}"),
                    ImageProcessState::Ready(image)=>image,
                };
                ensure!(image.client()==self.client&&image.size()==[1280,720],"Circus history preview image identity");
                let uploaded=self.host.publish_image_resource(self.client,IMAGE,image)?;
                self.services.history_preview.image=Some(uploaded);
                self.services.history_preview.job=None;
                Ok(Some(HistoryReply::Ready))
            }
            HistoryRequest::ShowPreview{flat_index,row,large}=>{
                ensure!(self.services.history_preview.owner==Some(*flat_index),"Circus history preview display owner");
                let image=self.services.history_preview.image.as_ref().context("Circus history preview image pending")?;
                let asset=image.path.relative().to_owned();
                let (position,size)=if *large{([516,88],[256,144])}else{([184,150*i32::from(*row)+122],[128,72])};
                self.submit(vec![json!({"op":"put","layer":{"id":LAYER,"parent":0x38c0_0000u32,"rect":[position[0],position[1],size[0],size[1]],"paint":{"kind":"image","asset":asset},"order":[500,0]}})],vec![])?;
                self.services.history_preview.published=true;
                Ok(Some(HistoryReply::Ready))
            }
            HistoryRequest::HidePreview=>{self.hide_history_preview()?;Ok(Some(HistoryReply::Ready))}
            _=>bail!("Circus history preview request kind"),
        }
    }
}
