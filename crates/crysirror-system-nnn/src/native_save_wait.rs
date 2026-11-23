// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original53b630 message/selection stage, not replayed PRINT/SELECT opcodes.
//! Byte records and native mode/timeout policy stay here; workers and shared
//! GlyphRun/UI own layout, reveal, activation and immutable window resources.
use crate::{
    native_save::NativeSaveFile,
    native_text::{SavedTextPreparation, TextRestoreSeed},
    native_ui::{SavedSelectionPreparation, SelectionRestoreSeed},
};
use anyhow::{Result, ensure};
use crysirror_core::{image_processing::ImageResource, service::HostServices};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SavedWait {
    Message {
        rows: Vec<Vec<u8>>,
        mode: u32,
        serial: u32,
        window_number: i32,
        window_visible: bool,
        cannot_click: bool,
    },
    Selection {
        rows: Vec<Vec<u8>>,
        total: usize,
        supplemental: usize,
        time_limit: i32,
        auto_select: i32,
        special: i32,
    },
}
impl NativeSaveFile {
    pub(crate) fn status_word(&self, native: usize) -> Result<i32> {
        ensure!(native >= 8, "NNN original status word before record");
        let status = self
            .blocks
            .get(2)
            .ok_or_else(|| anyhow::anyhow!("NNN original status absent"))?;
        ensure!(
            status.code == 2 && status.payload.len() == 7056,
            "NNN original status ABI"
        );
        let at = (native - 8)
            .checked_mul(4)
            .ok_or_else(|| anyhow::anyhow!("NNN original status offset overflow"))?;
        let bytes = status
            .payload
            .get(
                at..at
                    .checked_add(4)
                    .ok_or_else(|| anyhow::anyhow!("NNN original status span overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("NNN original status word bounds"))?;
        Ok(i32::from_le_bytes(bytes.try_into()?))
    }
    pub(crate) fn saved_wait(&self) -> Result<Option<SavedWait>> {
        Ok(match self.status_word(20)? {
            10 => {
                let count = usize::try_from(self.status_word(29)?)?;
                let mode = self.status_word(30)? as u32;
                ensure!(
                    matches!(mode, 0x22220001 | 0x22220002),
                    "NNN saved PRINT/LPRINT native mode"
                );
                Some(SavedWait::Message {
                    rows: self.messages(count)?,
                    mode,
                    //Actual53ad20/53b630 reserve476 for configMask. This
                    //legacy layout does not persist the tagged message serial;
                    //the original live class retains it across slot restore.
                    serial: u32::MAX,
                    window_number: self.status_word(180)?,
                    window_visible: self.status_word(44)? != 0,
                    cannot_click: self.status_word(485)? != 0,
                })
            }
            9 => {
                let total = usize::try_from(self.status_word(33)?)?;
                let supplemental = usize::try_from(self.status_word(34)?)?;
                ensure!(
                    total <= 64 && supplemental < total,
                    "NNN saved selection total/supplemental count"
                );
                Some(SavedWait::Selection {
                    rows: self.messages(total)?,
                    total,
                    supplemental,
                    time_limit: self.status_word(480)?,
                    auto_select: self.status_word(481)?,
                    special: self.status_word(482)?,
                })
            }
            _ => None,
        })
    }
}
pub(crate) enum SavedWaitPreparation {
    Message(SavedTextPreparation),
    Selection(SavedSelectionPreparation),
}
pub(crate) struct PreparedWait {
    pub text: Option<crate::native_text::NativeTextBridge>,
    pub selection: Option<crate::native_ui::NativeSelectionBridge>,
    pub batch: serde_json::Value,
    pub release_images: Vec<String>,
}
impl SavedWait {
    pub(crate) fn begin(
        self,
        text: Option<&TextRestoreSeed>,
        selection: Option<&SelectionRestoreSeed>,
        background: Arc<dyn ImageResource>,
        host: &mut dyn HostServices,
    ) -> Result<SavedWaitPreparation> {
        self.begin_with_variables(
            text,
            selection,
            background,
            &crate::expression::Variables::fresh(crate::expression::VariableProfile::Legacy1000),
            host,
        )
    }
    pub(crate) fn begin_with_variables(
        self,
        text: Option<&TextRestoreSeed>,
        selection: Option<&SelectionRestoreSeed>,
        background: Arc<dyn ImageResource>,
        variables: &crate::expression::Variables,
        host: &mut dyn HostServices,
    ) -> Result<SavedWaitPreparation> {
        use anyhow::Context;
        Ok(match self {
            Self::Message {
                rows,
                mode,
                serial,
                window_number,
                window_visible,
                cannot_click,
            } => {
                let text = text.context("NNN saved message component not configured")?;
                ensure!(
                    window_number == text.window_variant(),
                    "NNN saved window variant preparation is incomplete"
                );
                SavedWaitPreparation::Message(text.begin(
                    rows,
                    mode,
                    if serial == u32::MAX {
                        text.retained_serial()
                    } else {
                        serial
                    },
                    // STATUS44 and the live WindowOn flag are independent.
                    // SavedTextPreparation projects window-off after retaining
                    // the raw original window flag for a later click restore.
                    window_visible && mode == 0x22220001,
                    cannot_click,
                    background,
                    host,
                )?)
            }
            Self::Selection {
                rows,
                total,
                supplemental,
                time_limit,
                auto_select,
                special,
            } => {
                // Ordinary/timed CASE waits use the same live UI consumer.
                // Active special layouts still require their native resources.
                let mut preparation = selection
                    .context("NNN saved selection component not configured")?
                    .begin_with_controls(
                        rows,
                        total,
                        supplemental,
                        (time_limit, auto_select, special),
                        variables,
                        host,
                    )?;
                preparation.set_controls(time_limit, auto_select, special)?;
                SavedWaitPreparation::Selection(preparation)
            }
        })
    }
}
impl SavedWaitPreparation {
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        match self {
            Self::Message(prep) => prep.poll(host),
            Self::Selection(prep) => prep.poll_resources(host),
        }
    }
    pub(crate) fn finish(self) -> Result<PreparedWait> {
        Ok(match self {
            Self::Message(prep) => {
                let done = prep.finish()?;
                PreparedWait {
                    text: Some(done.bridge),
                    selection: None,
                    batch: done.batch,
                    release_images: done.release_images,
                }
            }
            Self::Selection(prep) => {
                let done = prep.finish()?;
                PreparedWait {
                    text: None,
                    selection: Some(done.bridge),
                    batch: done.batch,
                    release_images: vec![],
                }
            }
        })
    }
}
