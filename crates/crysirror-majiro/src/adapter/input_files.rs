// Copyright (c) 2026 CrySirror contributors. MIT.
//! Original402380/4023F0/402420/4026F0 resource and EOF policy. Shared
//! ByteFieldCursor owns tokenization; storage tasks own reads and byte leases.
use super::*;
use crysirror_text_legacy::{
    ByteLeadProfile,
    fields::{ByteFieldCursor, ByteFieldOptions},
};
pub(super) struct InputFile {
    cursor: ByteFieldCursor,
    _lease: Arc<dyn crysirror_core::service::AssetReader>,
}
pub(super) struct PendingInputFile {
    event: Event,
    task: Task<InputFile>,
}
impl MajiroVm {
    pub(super) fn open_input_file(
        &mut self,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        let Event::Native {
            arguments, returns, ..
        } = event
        else {
            bail!("Majiro input-file event")
        };
        ensure!(arguments.len() == 1, "Majiro FileOpen ABI");
        ensure!(
            self.input_files.len() < 64 && self.next_input_file < i32::MAX as u32,
            "Majiro input-file handle budget"
        );
        if let Some(pending) = &self.input_file_load {
            ensure!(&pending.event == event, "Majiro stale input-file receipt");
        } else {
            let bytes = arguments[0].bytes()?;
            let bytes = &bytes[..bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len())];
            ensure!(bytes.len() <= 259, "Majiro FileOpen filename budget");
            let name = encoding_rs::SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(bytes)
                .context("Majiro FileOpen filename CP932")?
                .replace('\\', "/");
            let requested = VfsPath::new(Mount::Asset, &name)?;
            let path =
                if let Some(paths) = self.module_catalog.assets.get(&name.to_ascii_lowercase()) {
                    ensure!(
                        paths.len() == 1,
                        "Majiro duplicate file providers need native archive precedence"
                    );
                    VfsPath::new(Mount::Asset, &paths[0])?
                } else {
                    requested
                };
            let storage = host.storage(self.client).map_err(core_error)?;
            let context = self.picture_context.clone();
            let permit = match self.pool.reserve() {
                Ok(permit) => permit,
                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
                Err(error) => return Err(error),
            };
            let task = permit.spawn(move |cancel| {
                cancel.check()?;
                let reader = storage.open_asset(&path)?;
                let bytes = crysirror_resource::read_all(reader.as_ref(), 4_000_000)?;
                cancel.check()?;
                let bytes: Arc<[u8]> = bytes.into();
                let lease = context.memory_shared(bytes.clone())?;
                let options = ByteFieldOptions {
                    lead: ByteLeadProfile::TwoRanges {
                        ranges: [[0x81, 0x9f], [0xe0, 0xfc]],
                    },
                    separator: b',',
                    comments: vec![b";".to_vec(), b"//".to_vec()],
                    chunk_bytes: 511,
                    trim_max: 32,
                    strip_quote: Some(b'"'),
                    skip_blank_lines: true,
                    windows_text: true,
                };
                let cursor = ByteFieldCursor::new(bytes, options, 4_000_000)?;
                Ok(InputFile {
                    cursor,
                    _lease: lease,
                })
            })?;
            self.input_file_load = Some(PendingInputFile {
                event: event.clone(),
                task,
            });
            return Ok(None);
        }
        let Some(file) = self.input_file_load.as_mut().unwrap().task.try_take()? else {
            return Ok(None);
        };
        let id = self.next_input_file;
        self.next_input_file += 1;
        self.input_files.insert(id, file);
        self.input_file_load = None;
        Ok(Some(if *returns {
            vec![Value::Integer(id as i32)]
        } else {
            vec![]
        }))
    }
    pub(super) fn read_input_file(
        &mut self,
        hash: u32,
        args: &[Value],
        returns: bool,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(args.len() == 1, "Majiro input-file ABI");
        let id = u32::try_from(args[0].integer()?).context("Majiro input-file handle")?;
        let file = self
            .input_files
            .get_mut(&id)
            .context("Majiro missing/foreign input-file handle")?;
        let value = match hash {
            0xd6333b3c => {
                self.input_files.remove(&id);
                Value::Integer(0)
            }
            0x55232561 | 0x2539d212 => {
                let mut cursor = file.cursor.clone();
                let bytes = if hash == 0x55232561 {
                    cursor.next_field(&|| Ok(()))?
                } else {
                    cursor.next_raw_field(&|| Ok(()))?
                }
                .unwrap_or_default();
                //4026F0 returns empty at EOF;402420 applies atoi to its
                //zero/default field. Integer reads do not remove quotes.
                let value = if hash == 0x55232561 {
                    Value::native_string(&bytes)?
                } else {
                    Value::Integer(crate::byte_policy::integer(&bytes)?)
                };
                file.cursor = cursor;
                value
            }
            _ => bail!("Majiro unknown input-file operation"),
        };
        Ok(Some(if returns { vec![value] } else { vec![] }))
    }
}
