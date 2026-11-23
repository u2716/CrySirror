// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original537310 ->535f90(6)->504f90 quick save. The stable native receipt
//! stays untouched through shared capture/reduction, serialization and atomic
//! capability storage. There is no legacy screen renderer or raw filesystem.
use super::*;
use crysirror_core::service::{StorageWrite, StorageWriteState};

#[derive(Clone)]
pub(super) struct Policy {
    key: u64,
    slot: u32,
    sound: i32,
    notice: bool,
    notice_number: i32,
    retained_comments: bool,
}
impl Policy {
    pub(super) fn selected(slot: u32, sound: i32) -> Result<Self> {
        ensure!(slot < 10000, "NNN selected save slot bounds");
        Ok(Self {
            key: 0,
            slot,
            sound,
            notice: false,
            notice_number: 0,
            retained_comments: true,
        })
    }
    pub(super) fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let setup = &config.setup;
        let enabled = setup.integer(b"quickSaveEnable")?.unwrap_or(0);
        if enabled == 0 {
            return Ok(None);
        }
        let key = if let Some(name) = setup.parameter(b"quickSaveKey") {
            let number = crate::config::integer(name)?;
            let code = if number == 0 && name.first().is_some_and(|b| *b != b'0') {
                i32::from(name[0])
            } else {
                number
            };
            native_load::key_mask(code)?
        } else {
            0
        };
        let data = native_load::table(config, storage, "dataFile", cancel)?;
        let slot = data
            .integer(b"quickSlotNumber")?
            .unwrap_or(1)
            .checked_sub(1)
            .context("NNN quick save slot overflow")?;
        ensure!(
            (0..1_000_000).contains(&slot),
            "NNN quick save slot namespace budget"
        );
        Ok(Some(Self {
            key,
            slot: slot as u32,
            sound: setup.integer(b"quickSaveSound")?.unwrap_or(-1),
            notice: setup.integer(b"noticeFlag")?.unwrap_or(0) != 0
                && setup.integer(b"quickSavePrint")?.unwrap_or(enabled) != 0,
            notice_number: setup.integer(b"saveNoticeNumber")?.unwrap_or(2) - 1,
            retained_comments: setup.integer(b"quickSaveCommentFlag")?.unwrap_or(0) != 0,
        }))
    }
}
struct Pending {
    sound_done: bool,
    thumbnail: Option<crate::native_thumbnail::ThumbnailJob>,
    encode: Option<Task<Arc<[u8]>>>,
    bytes: Option<Arc<[u8]>>,
    write: Option<Box<dyn StorageWrite>>,
}
pub(super) struct Controller {
    pub policy: Policy,
    pending: Option<Pending>,
    ///504f90 immediately reloads the just-written header and picture. Retain
    /// the validated native bytes for that class catalogue, not an engine VM
    /// snapshot or a claim that this is a new live export after gameplay moves.
    pub last_file: Option<Arc<[u8]>>,
    /// Live CGameCallBack comments are independent of a slot's catalogue.
    comments: [Vec<u8>; 4],
}
impl Controller {
    pub(super) fn selected(
        policy: Policy,
        thumbnail: crate::native_thumbnail::ThumbnailJob,
        previous: Option<&Self>,
    ) -> Self {
        let mut controller = Self::new(policy);
        if let Some(previous) = previous {
            controller.comments = previous.comments.clone();
            controller.last_file = previous.last_file.clone();
        }
        controller.pending = Some(Pending {
            sound_done: false,
            thumbnail: Some(thumbnail),
            encode: None,
            bytes: None,
            write: None,
        });
        controller
    }
    pub(super) fn new(policy: Policy) -> Self {
        Self {
            policy,
            pending: None,
            last_file: None,
            comments: Default::default(),
        }
    }
    pub(super) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn restore_copy(&self) -> Self {
        Self {
            policy: self.policy.clone(),
            pending: None,
            last_file: self.last_file.clone(),
            comments: self.comments.clone(),
        }
    }
}
impl InstalledAdapter {
    pub(super) fn poll_slot_save(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        let Some(save) = self.native_save.as_mut() else {
            return Ok(false);
        };
        if save.pending.is_none() {
            if input.buttons_pressed & save.policy.key == 0 {
                return Ok(false);
            }
            // Actual537310 handles ordinary L/S/C mode keys first. A colliding
            // configured quick key must not turn that ordinary menu into an
            // immediate quick operation. Their own actors dispatch separately.
            use crysirror_core::service::buttons as B;
            if input.buttons_pressed & (B::KEY_L | B::KEY_S | B::KEY_C) != 0 {
                return Ok(false);
            }
            if !self
                .vm
                .as_ref()
                .is_some_and(|vm| vm.original_input_mode().is_some())
            {
                return Ok(false);
            }
            // Freeze the immutable Layer prefix at input admission. Audio
            // preparation may finish later; it cannot consume native reveal/
            // animation time or change which visible frame enters this slot.
            let thumbnail = self
                .vm
                .as_ref()
                .context("NNN quick save VM absent")?
                .begin_native_thumbnail(host)?;
            save.pending = Some(Pending {
                sound_done: false,
                thumbnail: Some(thumbnail),
                encode: None,
                bytes: None,
                write: None,
            });
        }
        let pending = save.pending.as_mut().unwrap();
        if !pending.sound_done {
            if save.policy.sound > 0
                && !self
                    .vm
                    .as_mut()
                    .context("NNN quick save VM absent")?
                    .poll_system_sound(host, save.policy.sound)?
            {
                return Ok(true);
            }
            pending.sound_done = true;
        }
        if pending.thumbnail.is_none() && pending.encode.is_none() && pending.bytes.is_none() {
            pending.thumbnail = Some(
                self.vm
                    .as_ref()
                    .context("NNN quick save VM absent")?
                    .begin_native_thumbnail(host)?,
            );
            return Ok(true);
        }
        if let Some(thumbnail) = pending.thumbnail.as_mut() {
            let Some((thumbnail, stamp)) = thumbnail.poll(host)? else {
                return Ok(true);
            };
            let vm = self.vm.as_mut().context("NNN quick save VM absent")?;
            let mut file = vm.produce_native_save(thumbnail, stamp)?;
            if save.policy.retained_comments {
                file.replace_comments(&save.comments)?;
            }
            let profile = crate::native_save::LegacySaveProfile::for_configuration(
                vm.configuration()
                    .context("NNN quick save configuration absent")?,
            )?;
            pending.encode = Some(
                self.pool
                    .as_ref()
                    .context("NNN quick save worker pool absent")?
                    .spawn(move |cancel| {
                        cancel.check()?;
                        let bytes = file.encode(&profile)?;
                        cancel.check()?;
                        Ok(Arc::<[u8]>::from(bytes))
                    })?,
            );
            pending.thumbnail = None;
            return Ok(true);
        }
        if let Some(encode) = pending.encode.as_mut() {
            let Some(bytes) = encode.try_take()? else {
                return Ok(true);
            };
            pending.bytes = Some(bytes);
            pending.encode = None;
        }
        if pending.write.is_none() {
            let path = crate::native_save::slot_path(self.vm.as_ref().unwrap().configuration().context("NNN save slot configuration absent")?,usize::try_from(save.policy.slot)?)?;
            let bytes = pending
                .bytes
                .clone()
                .context("NNN quick save encoded file absent")?;
            pending.write = Some(
                host.storage(self.client)?
                    .begin_write_atomic(&path, bytes)?,
            );
            return Ok(true);
        }
        match pending.write.as_mut().unwrap().poll()? {
            StorageWriteState::Pending => Ok(true),
            StorageWriteState::Ready => {
                // Legacy504f90 always calls LoadHeaderAndPic after Save and
                // has none of later quickSetLastFlag/lastSaveSlot mutations.
                save.last_file = pending.bytes.take();
                if save.policy.notice {
                    self.notice
                        .as_mut()
                        .context("NNN quick save notice is not prepared")?
                        .start(save.policy.notice_number);
                }
                save.pending = None;
                Ok(true)
            }
            StorageWriteState::Failed(reason) => {
                //4da970 returns-1 on failed fopen;504f90 still returnsTRUE.
                // Preserve the prior catalogue/continuation, but expose the
                // actual storage error instead of reporting a successful file.
                host.append_log(
                    self.client,
                    &format!("NNN quick-save slot{} failed: {reason}", save.policy.slot),
                )?;
                save.pending = None;
                Ok(true)
            }
        }
    }
}
