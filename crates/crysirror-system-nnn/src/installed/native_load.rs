// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native configured quick-load action. The owner retains its current PRINT/
//! SELECT continuation while scoped workers read the slot; Runtime alone
//! prepares and atomically installs the replacement VM/resources/audio.
use super::*;
use crate::{
    config::Names,
    native_bootstrap::NativeConfiguration,
    native_save::{LegacySaveProfile, NativeSaveFile},
};
use crysirror_core::native_restore::{OriginalRestoreStatus, OriginalRestoreTicket};

#[derive(Clone)]
pub(super) struct Policy {
    key: u64,
    slot: i32,
    delayed: bool,
    notice: bool,
    get_screen: bool,
    sound: i32,
    error_sound: i32,
}
pub(super) fn table(
    config: &NativeConfiguration,
    storage: &dyn ClientStorage,
    name: &str,
    cancel: &Cancellation,
) -> Result<Names> {
    for (folder, extension, cipher) in [("init2", "fxf", true), ("init", "xtx", false)] {
        cancel.check()?;
        let path = config
            .resources
            .path(&format!("{folder}/{name}.{extension}"))?;
        if let Some(bytes) = read(storage, &path, 1 << 20, cancel)? {
            return Names::parse(&bytes, cipher);
        }
    }
    Names::parse(&[], false)
}
pub(super) fn key_mask(code: i32) -> Result<u64> {
    use crysirror_core::service::buttons as B;
    Ok(match code {
        0 => 0,
        8 => B::BACKSPACE,
        9 => B::TAB,
        13 => B::ENTER,
        16 => B::SHIFT,
        17 => B::CTRL,
        26 => B::PAGE_DOWN,
        27 => B::PAGE_UP,
        28 => B::LEFT,
        29 => B::RIGHT,
        30 => B::UP,
        31 => B::DOWN,
        32 => B::SPACE,
        33 => B::PAGE_UP,
        34 => B::PAGE_DOWN,
        35 => B::END,
        36 => B::HOME,
        37 => B::LEFT,
        38 => B::UP,
        39 => B::RIGHT,
        40 => B::DOWN,
        45 => B::INSERT,
        48..=57 => B::KEY_0 << (code - 48),
        65 => B::KEY_A,
        67 => B::KEY_C,
        68 => B::KEY_D,
        69 => B::KEY_E,
        70 => B::KEY_F,
        76 => B::KEY_L,
        81 => B::KEY_Q,
        82 => B::KEY_R,
        83 => B::KEY_S,
        87 => B::KEY_W,
        90 => B::KEY_Z,
        112..=123 => B::F1 << (code - 112),
        _ => {
            anyhow::bail!("NNN configured quick-load key {code} has no shared host key observation")
        }
    })
}
impl Policy {
    pub(super) fn prepare(
        config: &NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let setup = &config.setup;
        let enabled = setup
            .integer(b"quickLoadEnable")?
            .unwrap_or(setup.integer(b"quickSaveEnable")?.unwrap_or(0));
        if enabled == 0 {
            return Ok(None);
        }
        let key = if let Some(name) = setup.parameter(b"quickLoadKey") {
            let number = crate::config::integer(name)?;
            let code = if number == 0 && name.first().is_some_and(|byte| *byte != b'0') {
                i32::from(name[0])
            } else {
                number
            };
            key_mask(code)?
        } else {
            0
        };
        let data = table(config, storage, "dataFile", cancel)?;
        let slot = data
            .integer(b"quickSlotNumber")?
            .unwrap_or(1)
            .checked_sub(1)
            .context("NNN quick-slot index overflow")?;
        ensure!(
            (0..=999999).contains(&slot),
            "NNN quick-load slot outside bounded native file namespace"
        );
        let sound = setup
            .integer(b"quickLoadSound")?
            .unwrap_or(setup.integer(b"quickSaveSound")?.unwrap_or(-1));
        Ok(Some(Self {
            key,
            slot,
            delayed: setup.integer(b"quickLoadDelay")?.unwrap_or(0) != 0,
            notice: setup.integer(b"noticeFlag")?.unwrap_or(0) != 0
                && setup.integer(b"quickLoadPrint")?.unwrap_or(enabled) != 0,
            get_screen: setup.integer(b"quickLoadGetScreen")?.unwrap_or(0) != 0,
            sound,
            error_sound: setup.integer(b"quickLoadErrorSound")?.unwrap_or(sound),
        }))
    }
}
enum Slot {
    Missing,
    Invalid(String),
    Ready(Arc<[u8]>),
}
struct Pending {
    triggered: u64,
    read: Option<Task<Slot>>,
    ready: Option<Slot>,
    request: Option<OriginalRestoreTicket>,
}
pub(super) struct Controller {
    pub policy: Policy,
    pending: Option<Pending>,
}
impl Controller {
    pub(super) fn new(policy: Policy) -> Self {
        Self {
            policy,
            pending: None,
        }
    }
    pub(super) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn request(&self) -> Option<OriginalRestoreTicket> {
        self.pending.as_ref().and_then(|pending| pending.request)
    }
    pub(super) fn after_restore(policy: Policy, request: Option<OriginalRestoreTicket>) -> Self {
        Self {
            policy,
            pending: request.map(|request| Pending {
                triggered: 0,
                read: None,
                ready: None,
                request: Some(request),
            }),
        }
    }
}
impl InstalledAdapter {
    pub(super) fn poll_slot_load(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<bool> {
        let Some(load) = self.native_load.as_mut() else {
            return Ok(false);
        };
        if load.pending.is_none() {
            if input.buttons_pressed & load.policy.key == 0 {
                return Ok(false);
            }
            // Actual537310 dispatches ordinary L/S/C menus before quick keys.
            use crysirror_core::service::buttons as B;
            if input.buttons_pressed & (B::KEY_L | B::KEY_S | B::KEY_C) != 0 {
                return Ok(false);
            }
            let Some(vm) = self.vm.as_ref() else {
                return Ok(false);
            };
            let Some(_mode) = vm.original_input_mode() else {
                return Ok(false);
            };
            ensure!(
                !load.policy.notice,
                "NNN quick-load notification requires native Notice actor"
            );
            ensure!(
                !load.policy.get_screen,
                "NNN quick-load exit-screen capture requires native General capture ownership"
            );
            load.pending = Some(Pending {
                triggered: host.now_ms(),
                read: None,
                ready: None,
                request: None,
            });
        }
        let pending = load.pending.as_mut().unwrap();
        if let Some(ticket) = pending.request {
            match host.original_save_restore_request_status(self.client, ticket)? {
                OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready => return Ok(true),
                OriginalRestoreStatus::Committed => {
                    // The replacement inherits this receipt, never the old
                    // worker/VM. Retire it after the atomic owner installation.
                    host.release_original_save_restore_request(self.client, ticket)?;
                    load.pending = None;
                    return Ok(true);
                }
                OriginalRestoreStatus::Failed(error) => {
                    host.release_original_save_restore_request(self.client, ticket)?;
                    anyhow::bail!(
                        "NNN native quick-load preparation failed; original continuation preserved: {error}"
                    )
                }
                OriginalRestoreStatus::Cancelled => {
                    host.release_original_save_restore_request(self.client, ticket)?;
                    load.pending = None;
                    return Ok(true);
                }
            }
        }
        if load.policy.delayed && host.now_ms() == pending.triggered {
            return Ok(true);
        }
        if pending.read.is_none() && pending.ready.is_none() {
            let storage = host.storage(self.client)?;
            let config = self
                .vm
                .as_ref()
                .context("NNN native load VM absent")?
                .configuration()
                .cloned()
                .context("NNN native load original configuration absent")?;
            let slot = load.policy.slot;
            pending.read = Some(
                self.pool
                    .as_ref()
                    .context("NNN slot worker pool absent")?
                    .spawn(move |cancel| {
                        cancel.check()?;
                        let path = VfsPath::new(Mount::Save, &format!("data{slot}.sav"))?;
                        let source = match storage.open_read(&path) {
                            Ok(source) => source,
                            Err(Error::NotFound) => return Ok(Slot::Missing),
                            Err(error) => return Err(error.into()),
                        };
                        let bytes = crysirror_resource::read_all(source.as_ref(), 64 << 20)?;
                        let profile = LegacySaveProfile::for_configuration(&config)?;
                        cancel.check()?;
                        Ok(match NativeSaveFile::parse(&bytes, &profile) {
                            Ok(_) => Slot::Ready(bytes.into()),
                            Err(error) => Slot::Invalid(format!("{error:#}")),
                        })
                    })?,
            );
            return Ok(true);
        }
        if let Some(read) = pending.read.as_mut() {
            let Some(result) = read.try_take()? else {
                return Ok(true);
            };
            pending.ready = Some(result);
            pending.read = None;
        }
        let sound = match pending.ready.as_ref().unwrap() {
            Slot::Invalid(_) => load.policy.error_sound,
            _ => load.policy.sound,
        };
        if sound > 0
            && !self
                .vm
                .as_mut()
                .unwrap()
                .poll_system_sound(host, sound - 1)?
        {
            return Ok(true);
        }
        match pending.ready.take().unwrap() {
            Slot::Missing => {
                load.pending = None;
                Ok(true)
            }
            Slot::Invalid(error) => {
                host.append_log(
                    self.client,
                    &format!(
                        "NNN native quick-load slot{} rejected: {error}",
                        load.policy.slot
                    ),
                )?;
                load.pending = None;
                Ok(true)
            }
            Slot::Ready(bytes) => {
                // Queue only: no VM receipt acknowledgement, direct Runtime
                // call, eager image/audio mutation or replacement ownership.
                pending.request = Some(host.request_original_save_restore(self.client, bytes)?);
                Ok(true)
            }
        }
    }
}
