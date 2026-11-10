// Copyright (c) 2026 CrySirror contributors. MIT.
//! YUKIIRO403950 native media receipt. Portable installed data is an explicit
//! launch policy, not a fabricated CD drive or an unconditional native success.
use super::*;
use crysirror_core::service::EngineQuery;
const DIALOG: u32 = 0xffe1;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaPolicy {
    /// Original optical-drive lookup requires a native-media backend.
    #[default]
    NativeOptical,
    /// Use this client's verified installed EXE, ENV and root MJO instead of
    /// requiring those same files to be repeated on physical optical media.
    InstalledResources,
}
pub(super) struct MediaCheck {
    context: u64,
    pc: u32,
    folder: Vec<u8>,
    task: Option<Task<Vec<String>>>,
    missing: Option<Vec<String>>,
    opened: bool,
}
impl MajiroVm {
    pub(super) fn media_check(
        &mut self,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        let Event::Native {
            pc,
            arguments,
            returns,
            ..
        } = event
        else {
            bail!("Majiro media receipt type");
        };
        ensure!(arguments.len() <= 1, "Majiro optical media check ABI");
        let folder = arguments
            .first()
            .map(Value::bytes)
            .transpose()?
            .unwrap_or_default();
        let folder = folder.split(|b| *b == 0).next().unwrap_or_default();
        ensure!(folder.len() <= 240, "Majiro native optical folder capacity");
        ensure!(
            encoding_rs::SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(folder)
                .is_some(),
            "Majiro optical folder CP932"
        );
        ensure!(
            self.config.media_policy == MediaPolicy::InstalledResources,
            "Majiro native optical media backend is unavailable; installed-resource policy must be explicitly selected by the host"
        );
        if self.media_check.is_none() {
            let executable = self
                .config
                .executable
                .clone()
                .context("Majiro installed media requires original executable metadata")?;
            let environment = self
                .config
                .environment
                .clone()
                .context("Majiro installed media requires original ENV resource")?;
            let paths = [
                (executable, 0u8),
                (environment, 1),
                (self.config.script.clone(), 2),
            ];
            let storage = host.storage(self.client).map_err(core_error)?;
            let task = self.pool.spawn(move |cancel| {
                let mut missing = Vec::new();
                for (name, kind) in paths {
                    cancel.check()?;
                    let path = VfsPath::new(Mount::Asset, &name)?;
                    // Failed opens, metadata and short/raced reads all enter
                    // the retained Retry/Cancel path, including remote mounts.
                    let inspected = (|| -> Result<()> {
                        let reader = storage.open_asset(&path)?;
                        let len = reader.len()?;
                        let minimum = match kind {
                            0 => 64,
                            2 => 32,
                            _ => 1,
                        };
                        ensure!(len >= minimum, "truncated resource");
                        let mut header = [0u8; 16];
                        let length = len.min(16) as usize;
                        let mut at = 0;
                        while at < length {
                            cancel.check()?;
                            let count = reader.read_at(at as u64, &mut header[at..length])?;
                            ensure!(
                                count > 0 && count <= length - at,
                                "invalid or short resource read"
                            );
                            at += count;
                        }
                        ensure!(
                            (kind != 0 || header.starts_with(b"MZ"))
                                && (kind != 2
                                    || header == *b"MajiroObjX1.000\0"
                                    || header == *b"MajiroObjV1.000\0"),
                            "invalid original resource header"
                        );
                        Ok(())
                    })();
                    if let Err(error) = inspected {
                        missing.push(format!("{name}: {error:#}"));
                    }
                }
                cancel.check()?;
                Ok(missing)
            })?;
            self.media_check = Some(MediaCheck {
                context: self.service_context,
                pc: *pc,
                folder: folder.to_vec(),
                task: Some(task),
                missing: None,
                opened: false,
            });
            return Ok(None);
        }
        let pending = self.media_check.as_mut().unwrap();
        ensure!(
            pending.context == self.service_context
                && pending.pc == *pc
                && pending.folder == folder,
            "Majiro changed media receipt before completion"
        );
        if let Some(task) = pending.task.as_mut() {
            let Some(missing) = task.try_take()? else {
                return Ok(None);
            };
            pending.task = None;
            if missing.is_empty() {
                self.media_check = None;
                return Ok(Some(if *returns {
                    vec![Value::Integer(0)]
                } else {
                    vec![]
                }));
            }
            pending.missing = Some(missing);
        }
        if !pending.opened {
            let message = format!(
                "Required installed game resources are unavailable. Restore the original files and choose Retry.\n\n{}",
                pending
                    .missing
                    .as_ref()
                    .context("Majiro missing-media detail")?
                    .join("\n")
            );
            ensure!(message.len() <= 16384, "Majiro media dialog size");
            let font = if let Some(font) = &self.font {
                font.alias.clone()
            } else {
                let fallback = self
                    .config
                    .font_fallback
                    .as_ref()
                    .context("Majiro media dialog font is not configured")?;
                host.font_library(self.client)?
                    .resolve_alias(fallback)?
                    .context("Majiro media dialog font is unavailable")?
            };
            host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"ui":[{"op":"open_dialog","dialog":{"id":DIALOG,"title":"Game resources","message":message,"buttons":[{"id":1,"label":"Retry"},{"id":0,"label":"Cancel"}],"default":1,"cancel":0,"font":font,"text_size":22}}]}))?)?;
            self.media_check.as_mut().unwrap().opened = true;
            return Ok(None);
        }
        let bytes = host.query_engine(self.client, EngineQuery::DialogEvents)?;
        ensure!(bytes.len() <= 65536, "Majiro media events size");
        let events: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
        ensure!(events.len() <= 8, "Majiro media event count");
        if events.is_empty() {
            return Ok(None);
        }
        ensure!(
            events.len() == 1
                && events[0]["widget"]["owner"].as_u64() == Some(u64::from(self.client.0))
                && events[0]["widget"]["id"].as_u64() == Some(u64::from(DIALOG))
                && events[0]["kind"] == "dialog_result",
            "Majiro foreign media dialog receipt"
        );
        let choice = events[0]["value"]
            .as_f64()
            .context("Majiro media dialog value")?;
        ensure!(choice == 0.0 || choice == 1.0, "Majiro media dialog choice");
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"ui":[{"op":"close_dialog","id":DIALOG}]}))?,
        )?;
        self.media_check = None;
        if choice == 1.0 {
            return self.media_check(event, host);
        }
        // Original Cancel performs cleanup and exits; no subsequent script
        // instruction may execute after acknowledging this retained call.
        self.exit_requested = true;
        Ok(Some(if *returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
}
