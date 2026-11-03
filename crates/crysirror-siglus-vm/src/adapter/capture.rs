// SPDX-License-Identifier: MPL-2.0
//! Composed-frame capture and system message boxes over engine services: the
//! presenter's frame capture supplies the immutable image the original
//! save and thumbnail code consumes.
use super::*;
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    service::EngineQuery,
};
use serde_json::{Value, json};

const MESSAGE_DIALOG: u32 = 0x7fff_0200;

#[derive(Default)]
pub(super) struct FrameCapture {
    job: Option<ImageJob>,
}

impl SiglusVm {
    /// True once `io.frame_capture` holds the current composed frame.
    pub(super) fn poll_frame_capture(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let client = ClientId(self.state.client);
        if self.capture.job.is_none() {
            self.publish(host)?;
            self.capture.job = Some(host.begin_frame_capture(client).map_err(err)?);
        }
        match self.capture.job.as_mut().unwrap().poll().map_err(err)? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => anyhow::bail!("native frame capture: {error}"),
            ImageProcessState::Ready(image) => {
                self.capture.job = None;
                let [w, h] = image.size();
                self.state.io.frame_capture = Some(Arc::new(crate::assets::RgbaImage {
                    width: w,
                    height: h,
                    center_x: 0,
                    center_y: 0,
                    rgba: image.rgba().to_vec(),
                }));
                Ok(true)
            }
        }
    }
    /// Present and complete `SYSTEM.MESSAGEBOX` through the engine dialog service.
    pub(super) fn poll_system_messagebox(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let client = ClientId(self.state.client);
        let ctx = &mut self.state.vm.as_mut().unwrap().ctx;
        let Some(modal) = ctx.globals.system.messagebox_modal.as_mut() else {
            return Ok(());
        };
        if !modal.native_pending {
            modal.native_pending = true;
            let buttons: Vec<Value> = modal.buttons.iter().enumerate().map(|(index, button)| json!({"id":index,"label":button.label})).collect();
            let batch = json!({"ui":[{"op":"open_dialog","dialog":{"id":MESSAGE_DIALOG,"title":"","message":modal.text,"buttons":buttons,"default":modal.cursor,"font":"default"}}]});
            host.submit_engine(client, 1, &serde_json::to_vec(&batch).map_err(err)?).map_err(err)?;
            return Ok(());
        }
        let events: Vec<Value> = serde_json::from_slice(&host.query_engine(client, EngineQuery::UiEvents).map_err(err)?).map_err(err)?;
        let Some(event) = events.iter().find(|e| e["kind"] == "dialog_result" && e["widget"]["id"].as_u64() == Some(u64::from(MESSAGE_DIALOG))) else {
            return Ok(());
        };
        let index = event["value"].as_f64().unwrap_or(0.) as usize;
        modal.cursor = index.min(modal.buttons.len().saturating_sub(1));
        let value = modal.selected_value();
        let with_value = modal.complete_wait_with_value;
        ctx.globals.system.messagebox_modal = None;
        ctx.globals.system.messagebox_modal_result = Some(value);
        if with_value {
            ctx.wait.finish_system_modal(crate::runtime::Value::Int(value));
        } else {
            ctx.wait.finish_system_modal_void();
        }
        Ok(())
    }
}
