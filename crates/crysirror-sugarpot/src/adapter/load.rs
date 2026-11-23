// SPDX-License-Identifier: MPL-2.0
//! 4056A0/433850 ->4561A0: capability read, confirmation, then the Runtime's
//! detached restore coordinator. The old VM's PC/wait survives every retry.
use super::*;
use crysirror_core::native_restore::{OriginalRestoreStatus, OriginalRestoreTicket};
const DIALOG: u32 = 65400;
pub(super) struct NativeLoad {
    read: Option<Task<Option<Vec<u8>>>>,
    bytes: Option<Arc<[u8]>>,
    confirm: bool,
    dialog: bool,
    ticket: Option<OriginalRestoreTicket>,
    resume: Option<Box<Wait>>,
    command: bool,
}
impl SugarPotVm {
    pub(super) fn start_native_load(
        &mut self,
        slot: i32,
        confirm: bool,
        command: bool,
    ) -> Result<()> {
        ensure!(
            self.native_load.is_none(),
            "SugarPot overlapping native load"
        );
        let path = crate::native_storage::slot_path(slot)?;
        let storage = self
            .storage
            .as_ref()
            .context("SugarPot load without storage")?
            .clone();
        let read = self
            .pool
            .as_ref()
            .unwrap()
            .spawn(move |cancel| crate::native_storage::read_file(storage, &path, &cancel))?;
        let resume = self.wait.take().map(Box::new);
        self.native_load = Some(NativeLoad {
            read: Some(read),
            bytes: None,
            confirm,
            dialog: false,
            ticket: None,
            resume,
            command,
        });
        self.wait = Some(Wait::OriginalLoad);
        Ok(())
    }
    pub(super) fn poll_native_load(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<Option<Box<Wait>>> {
        let mut load = self
            .native_load
            .take()
            .context("SugarPot native load continuation missing")?;
        let done = (|| -> Result<bool> {
            if let Some(read) = &mut load.read {
                let Some(bytes) = read.try_take()? else {
                    return Ok(false);
                };
                load.read = None;
                //4561A0 returns0 immediately for a missing file. This is the
                //native determined empty-slot branch, not a restore ACK.
                let Some(bytes) = bytes else { return Ok(true) };
                load.bytes = Some(bytes.into());
            }
            if load.confirm {
                if !load.dialog {
                    self.submit(h,vec![],vec![json!({"op":"open_dialog","dialog":{"id":DIALOG,"title":"確認","message":"ロードします。よろしいですか？","buttons":[{"id":1,"label":"はい"},{"id":2,"label":"いいえ"}],"default":1,"cancel":2,"font":self.config.font}})])?;
                    load.dialog = true;
                    input.buttons_pressed = 0;
                    return Ok(false);
                }
                let events: Vec<Json> = serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::DialogEvents)?,
                )?;
                let Some(event) = events.iter().find(|e| {
                    e["kind"] == "dialog_result"
                        && e["widget"]["id"] == DIALOG
                        && e["widget"]["owner"] == self.client.0
                }) else {
                    return Ok(false);
                };
                input.buttons_pressed = 0;
                if event["value"].as_f64() != Some(1.) {
                    return Ok(true);
                };
                load.confirm = false;
            }
            if let Some(ticket) = load.ticket {
                match h.original_save_restore_request_status(self.client, ticket)? {
                    OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready => {
                        return Ok(false);
                    }
                    OriginalRestoreStatus::Failed(error) => {
                        h.release_original_save_restore_request(self.client, ticket)?;
                        bail!("SugarPot original load preparation: {error}")
                    }
                    OriginalRestoreStatus::Cancelled => {
                        h.release_original_save_restore_request(self.client, ticket)?;
                        return Ok(true);
                    }
                    OriginalRestoreStatus::Committed => {
                        bail!("SugarPot old VM resumed after committed original restore")
                    }
                }
            }
            load.ticket = Some(h.request_original_save_restore(
                self.client,
                load.bytes.take().context("SugarPot load lost input")?,
            )?);
            Ok(false)
        })();
        match done {
            Ok(true) => {
                self.load_command_done = load.command;
                Ok(load.resume)
            }
            Ok(false) => {
                self.native_load = Some(load);
                Ok(Some(Box::new(Wait::OriginalLoad)))
            }
            Err(error) => {
                self.native_load = Some(load);
                Err(error)
            }
        }
    }
}
