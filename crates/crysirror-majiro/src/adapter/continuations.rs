// Copyright (c) 2026 CrySirror contributors. MIT.
//! Resource receipts belong to a unique native call frame, not merely its PC.
//! An interrupt or another script thread cannot consume an older frame's work.
use super::*;
#[derive(Default)]
pub(super) struct SuspendedServices {
    input_file: Option<input_files::PendingInputFile>,
    library: Option<libraries::PendingLibrary>,
    picture: Option<pictures::PendingPicture>,
    settings_write: Option<PendingSettingsWrite>,
    wait: Option<native_services::NativeWait>,
}
impl SuspendedServices {
    pub(super) fn has_settings_write(&self) -> bool {
        self.settings_write.is_some()
    }
    fn is_empty(&self) -> bool {
        self.input_file.is_none()
            && self.library.is_none()
            && self.picture.is_none()
            && self.settings_write.is_none()
            && self.wait.is_none()
    }
}
impl MajiroVm {
    fn take_services(&mut self) -> SuspendedServices {
        SuspendedServices {
            input_file: self.input_file_load.take(),
            library: self.library.take(),
            picture: self.picture.take(),
            settings_write: self.settings_write.take(),
            wait: self.wait.take(),
        }
    }
    pub(super) fn select_service_context(&mut self, id: u64) -> Result<()> {
        ensure!(id != 0, "Majiro native service frame is missing");
        if id == self.service_context {
            return Ok(());
        }
        let previous = self.take_services();
        if !previous.is_empty() {
            ensure!(
                self.suspended_services.len() < 4096,
                "Majiro suspended service budget"
            );
            self.suspended_services
                .insert(self.service_context, previous);
        }
        let next = self.suspended_services.remove(&id).unwrap_or_default();
        self.library = next.library;
        self.input_file_load = next.input_file;
        self.picture = next.picture;
        self.settings_write = next.settings_write;
        self.wait = next.wait;
        self.service_context = id;
        Ok(())
    }
    pub(super) fn collect_service_contexts(&mut self, machine: &Machine) {
        let live = machine.live_contexts();
        self.suspended_services.retain(|id, _| live.contains(id));
        if !live.contains(&self.service_context) {
            //Dropping a Task cancels unpublished work; no partial result is
            //published when a thread or interrupted call frame disappears.
            drop(self.take_services());
            self.service_context = 0;
        }
    }
}
