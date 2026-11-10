// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy overlap wait/presentation policy over shared engine jobs.
use crate::{
    machine::{Machine, NativeCall, Payload},
    native_presentation::{NativeFrameResult, NativeFrameWork},
};
use anyhow::{bail, ensure, Result};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    service::HostServices,
    state::ClientId,
};
pub struct OverlapResult {
    pub frame: NativeFrameResult,
    pub(crate) tables: crate::native_simple_wipe::Tables,
    pub display_path: Option<String>,
}
pub struct NativeOverlap {
    receipt: NativeCall,
    client: ClientId,
    display: u32,
    order: u16,
    size: [u32; 2],
    frame_ms: u64,
    frames: u32,
    wipe: i32,
    tables: crate::native_simple_wipe::Tables,
    started_ms: u64,
    work: NativeFrameWork,
    frame: Option<NativeFrameResult>,
    blend: Option<ImageJob>,
    rendering_count: u32,
    presented_count: Option<u32>,
    display_path: Option<String>,
    presented_frame: Option<std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>>,
    pub api_calls: u64,
}
impl NativeOverlap {
    pub fn begin(
        client: ClientId,
        display: u32,
        order: u16,
        size: [u32; 2],
        frame_ms: u64,
        call: &NativeCall,
        work: NativeFrameWork,
        display_path: Option<String>,
        now_ms: u64,
    ) -> Result<Self> {
        let Payload::Words { count, words } = &call.payload else {
            bail!("NNN overlap payload")
        };
        ensure!(
            *count >= 2 && words.len() >= 2,
            "NNN overlap parameter count"
        );
        ensure!(
            crate::native_simple_wipe::supported(words[0]),
            "NNN native wipe type {} requires shared exact profile",
            words[0]
        );
        let frames =
            u32::try_from(words[1]).map_err(|_| anyhow::anyhow!("NNN negative overlap length"))?;
        ensure!(
            frames > 0 && frames <= 864000,
            "NNN overlap frame length bounds"
        );
        Ok(Self {
            receipt: call.clone(),
            client,
            display,
            order,
            size,
            frame_ms,
            frames,
            wipe: words[0],
            tables: Default::default(),
            started_ms: now_ms,
            work,
            frame: None,
            blend: None,
            rendering_count: 0,
            presented_count: None,
            display_path,
            presented_frame: None,
            api_calls: 0,
        })
    }
    pub(crate) fn bind_tables(&mut self,tables:crate::native_simple_wipe::Tables) {self.tables=tables;}
    pub(crate) fn presented_frame(
        &self,
    ) -> Option<std::sync::Arc<dyn crysirror_core::image_processing::ImageResource>> {
        self.presented_frame.clone()
    }
    pub fn dependency_pending(&self) -> bool {
        self.frame.is_none() || self.blend.is_some()
    }
    pub(crate) fn owned_path(&self)->Option<&str>{self.display_path.as_deref()}
    pub(crate) fn reanchor_after_pause(&mut self,now:u64,paused:u64)->Result<()> {
        let elapsed=now.checked_sub(paused).ok_or_else(||anyhow::anyhow!("NNN overlap pause clock reversed"))?;
        self.started_ms=self.started_ms.checked_add(elapsed).ok_or_else(||anyhow::anyhow!("NNN overlap pause clock overflow"))?;Ok(())
    }
    pub(crate) fn poll_admitted_frame(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.frame.is_none() {
            let before = self.work.api_calls;
            let result = self.work.poll(host);
            self.api_calls += self.work.api_calls - before;
            self.frame = result?;
            if let Some(frame)=&self.frame {self.tables=frame.buffers.wipe_tables.clone();}
            if self.frame.is_none() {
                return Ok(false);
            }
        }
        if let Some(job) = self.blend.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    bail!("NNN shared integer overlap failed: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(image.client() == self.client, "NNN foreign overlap frame");
                    let published =
                        host.publish_image_resource(self.client, self.display, image.clone())?;
                    self.api_calls += 1;
                    let [w, h] = self.size;
                    // A sibling of the source hierarchy: captures cannot feed a
                    // previously composed display image back into the next frame.
                    if let Err(error)=host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.display,"rect":[0,0,w,h],"order":[self.order,0],"paint":{"kind":"image","asset":published.path.relative()}}}]}))?){
                        let _=host.release_image_resource(self.client,published.path.relative());return Err(error.into())
                    }
                    self.api_calls += 1;
                    if let Some(previous) = self
                        .display_path
                        .replace(published.path.relative().to_owned())
                    {
                        host.release_image_resource(self.client, &previous)?;
                    }
                    self.presented_frame = Some(image);
                    self.presented_count = Some(self.rendering_count);
                    self.blend = None;
                }
            }
        }
        Ok(true)
    }
    /// Drain only admitted target/blend dependencies at an explicit native
    /// outer-mode exit. No new blend or native acknowledgement is produced.
    pub(crate) fn cancel_at_frame(
        &mut self,
        machine: &Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<OverlapResult>> {
        machine.validate_receipt(call)?;
        ensure!(
            self.receipt == *call,
            "NNN cancelled overlap receipt changed"
        );
        if !self.poll_admitted_frame(host)? {
            return Ok(None);
        }
        let mut frame = self
            .frame
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("NNN cancelled overlap target absent"))?
            .clone();
        frame.target = self
            .presented_frame
            .clone()
            .unwrap_or_else(|| frame.old.clone());
        Ok(Some(OverlapResult {
            frame,
            tables: self.tables.clone(),
            display_path: self.display_path.take(),
        }))
    }
    pub fn poll(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<OverlapResult>> {
        self.poll_with_skip(machine,call,host,false)
    }
    pub(crate) fn poll_with_skip(&mut self,machine:&mut Machine,call:&NativeCall,host:&mut dyn HostServices,skip:bool)->Result<Option<OverlapResult>> {
        ensure!(self.receipt == *call, "NNN overlap receipt changed");
        machine.validate_receipt(call)?;
        if !self.poll_admitted_frame(host)? {
            return Ok(None);
        }
        if self.presented_count == Some(self.frames) {
            let frame = self.frame.as_ref().unwrap();
            let mut candidate = machine.native_variables_candidate(call)?;
            candidate.replace_random_state(frame.random.clone())?;
            machine.acknowledge_with_variables(call, candidate)?;
            return Ok(Some(OverlapResult {
                frame: frame.clone(),
                tables: self.tables.clone(),
                display_path: self.display_path.take(),
            }));
        }
        let count = if skip {self.frames} else {(host.now_ms().saturating_sub(self.started_ms) / self.frame_ms)
            .min(u64::from(self.frames)) as u32};
        if self.presented_count == Some(count) {
            return Ok(None);
        }
        let frame = self.frame.as_mut().unwrap();
        let (input,effects) = crate::native_simple_wipe::plan_with_random(self.wipe,frame.old.clone(),frame.target.clone(),count,self.frames,&mut frame.random,&mut self.tables)?;
        self.blend = Some(host.image_metadata(self.client)?.begin_process(input,serde_json::to_vec(&effects)?.into())?);
        self.api_calls += 1;
        self.rendering_count = count;
        Ok(None)
    }
}
