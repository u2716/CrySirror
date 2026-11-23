// SPDX-License-Identifier: MPL-2.0
//! sub_4312e0 writes the retained checkpoint together with the current main image.
use crate::adapter::*;
use crate::vm::{
    css0::SystemData,
    open_save_slot,
    save_slot::{SlotFormat, pack_native},
};

pub(in crate::adapter) struct Write {
    slot: u32,
    header: [u8; 600],
    records: Vec<u8>,
    main: Vec<u8>,
    thumbnail: Option<Arc<dyn AssetReader>>,
    system: SystemData,
    salts: [u8; 2],
    base: Option<(VfsPath, bool)>,
    path: VfsPath,
    backup: VfsPath,
    system_path: VfsPath,
}

impl Write {
    pub(in crate::adapter) fn capture(session: &mut Session, slot: u32) -> Result<Option<Self>> {
        let Some((header, records)) = &session.cvns_slot else {
            return Ok(None);
        };
        let mut header = **header;
        let enabled = session.vm.settings().thumbnail_size.iter().all(|&n| n != 0);
        header[548..550].copy_from_slice(&u16::from(enabled).to_le_bytes());
        Ok(Some(Self {
            slot,
            header,
            records: records.clone(),
            main: session.vm.cvns_main_image()?,
            thumbnail: enabled.then(|| session.thumbnail.saved_bitmap()).flatten(),
            system: session.vm.cvns_system_data()?,
            salts: session.cvns.save_salts(),
            base: session
                .vm
                .resources()
                .save_file_base()
                .map(|(p, d)| (p.clone(), d)),
            path: session.vm.save_slot_path(slot)?,
            backup: session.vm.save_slot_path(999)?,
            system_path: session.vm.save_slot_path(0)?,
        }))
    }

    pub(in crate::adapter) fn write(
        self,
        storage: &dyn ClientStorage,
        check: impl Fn() -> Result<()> + Copy,
    ) -> Result<bool> {
        check()?;
        let thumb = self
            .thumbnail
            .map(|image| read_all(image.as_ref(), 64 * 1024 * 1024))
            .transpose()?;
        let bytes = pack_native(
            SlotFormat::Csp1,
            self.header,
            &self.records,
            thumb.as_deref(),
            &self.main,
            check,
        )?;
        let system = self.system.encode(self.salts, check)?;
        let base = self.base.as_ref().map(|(path, dir)| (path, *dir));
        let existing = open_save_slot(storage, base, self.slot)?;
        let backed_up = self.slot < 950 && existing.is_some();
        let path = if let Some((previous, source)) = existing {
            if backed_up {
                let old = read_all(
                    source.as_ref(),
                    crate::script::MAX_PS2A_IMAGE as u64 * 2 + 128 * 1024 * 1024,
                )?;
                let backup =
                    open_save_slot(storage, base, 999)?.map_or(self.backup, |(path, _)| path);
                check()?;
                storage.write_atomic(&backup, &old)?;
            }
            previous
        } else {
            self.path
        };
        check()?;
        let system_path = open_save_slot(storage, base, 0)?.map_or(self.system_path, |(p, _)| p);
        storage.write_atomic(&system_path, &system)?;
        storage.write_atomic(&path, &bytes)?;
        Ok(backed_up)
    }
}
