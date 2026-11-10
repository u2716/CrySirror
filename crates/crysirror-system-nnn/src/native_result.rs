// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Legacy542180/5355c0 ResultData policy. The record is exactly1024 little-
//! endian signed words, distinct from SystemFile and from a game slot. Scoped
//! storage owns reads/writes; no executable implementation is hosted here.
use crate::{config::Names, expression::Variables};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{ClientStorage, StorageWrite, StorageWriteState},
};
use crysirror_tasks::Cancellation;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct NativeResultData {
    words: Vec<i32>,
    get_registered: bool,
    result_variable: Option<u16>,
    auto_notice: i32,
    notice_enabled: bool,
    revision: u64,
    saved_revision: Option<u64>,
    notices: std::collections::VecDeque<i32>,
}
pub(crate) struct ResultWrite {
    ticket: Box<dyn StorageWrite>,
    revision: u64,
    complete: bool,
}
impl NativeResultData {
    /// Bootstrap worker only. Native fread keeps the zero-initialized tail of
    /// short files and ignores extra file bytes. Missing .sav falls back to
    /// .org; absence of both retains the fresh dirty zero record.
    pub(crate) fn prepare(
        setup: &Names,
        names: &Names,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        if setup.integer(b"resultFlag")?.unwrap_or(0) == 0 {
            return Ok(None);
        }
        let mut bytes = [0u8; 4096];
        let mut loaded = false;
        for name in ["result.sav", "result.org"] {
            cancel.check()?;
            let source = match storage.open_read(&VfsPath::new(Mount::Save, name)?) {
                Ok(source) => source,
                Err(Error::NotFound) => continue,
                Err(error) => return Err(error.into()),
            };
            let count = usize::try_from(source.len()?.min(bytes.len() as u64))?;
            crysirror_resource::read_exact_at(source.as_ref(), 0, &mut bytes[..count])?;
            loaded = true;
            break;
        }
        cancel.check()?;
        let variable = setup.parameter(b"getResultVarName");
        Ok(Some(Self {
            words: bytes
                .chunks_exact(4)
                .map(|word| i32::from_le_bytes(word.try_into().unwrap()))
                .collect(),
            get_registered: variable.is_some(),
            result_variable: variable.and_then(|name| names.index(name)),
            auto_notice: setup.integer(b"autoResultNotice")?.unwrap_or(0),
            notice_enabled: setup.integer(b"noticeFlag")?.unwrap_or(0) != 0,
            revision: 0,
            saved_revision: loaded.then_some(0),
            notices: Default::default(),
        }))
    }
    pub(crate) fn apply(
        &mut self,
        name: &[u8],
        parameters: &[i32],
        variables: &mut Variables,
    ) -> Result<bool> {
        if name.eq_ignore_ascii_case(b"setresult") {
            if let Some(&number) = parameters.first() {
                let index =
                    usize::try_from(number).context("NNN setresult negative native index")?;
                // Original SetData's `(index>=0)||(index<max)` is an invalid
                // bounds check and permits memory corruption. Keep valid
                // native indices; reject that undefined-memory domain.
                ensure!(
                    index < self.words.len(),
                    "NNN setresult outside native1024-word record"
                );
                if self.words[index] == 0 {
                    let notice = if self.auto_notice > 0 && self.notice_enabled {
                        ensure!(
                            self.notices.len() < 4096,
                            "NNN pending result-notice budget"
                        );
                        Some(
                            number
                                .checked_add(self.auto_notice)
                                .and_then(|n| n.checked_sub(2))
                                .context("NNN result-notice index overflow")?,
                        )
                    } else {
                        None
                    };
                    let revision = self
                        .revision
                        .checked_add(1)
                        .context("NNN result revision exhausted")?;
                    self.words[index] = parameters.get(1).copied().unwrap_or(1);
                    self.revision = revision;
                    if let Some(notice) = notice {
                        self.notices.push_back(notice);
                    }
                }
            }
            return Ok(true);
        }
        if name.eq_ignore_ascii_case(b"getresult") && self.get_registered {
            if let (Some(&number), Some(variable)) = (parameters.first(), self.result_variable) {
                let stored = usize::try_from(number)
                    .ok()
                    .and_then(|index| self.words.get(index))
                    .copied()
                    .unwrap_or(0);
                let answer = i32::from(stored >= parameters.get(1).copied().unwrap_or(1));
                let mut candidate = variables.clone();
                candidate.write_native(variable, answer)?;
                ensure!(
                    candidate.native_writes().is_empty(),
                    "NNN getresult bound resource variable needs an atomic native owner plan"
                );
                *variables = candidate;
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub(crate) fn take_notices(&mut self) -> std::collections::VecDeque<i32> {
        std::mem::take(&mut self.notices)
    }
    pub(crate) fn begin_save(&self, storage: &dyn ClientStorage) -> Result<ResultWrite> {
        let bytes: Arc<[u8]> = self
            .words
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>()
            .into();
        Ok(ResultWrite {
            ticket: storage.begin_write_atomic(&VfsPath::new(Mount::Save, "result.sav")?, bytes)?,
            revision: self.revision,
            complete: false,
        })
    }
}
impl ResultWrite {
    pub(crate) fn poll(&mut self, data: &mut NativeResultData) -> Result<bool> {
        if self.complete {
            return Ok(true);
        }
        match self.ticket.poll()? {
            StorageWriteState::Pending => Ok(false),
            StorageWriteState::Failed(error) => {
                anyhow::bail!("NNN native result atomic write: {error}")
            }
            StorageWriteState::Ready => {
                data.saved_revision = Some(self.revision);
                self.complete = true;
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_result_transition_queues_original_notice_offset_without_repeating_an_unlock() {
        let mut data = NativeResultData {
            words: vec![0; 1024],
            get_registered: true,
            result_variable: Some(2),
            auto_notice: 1,
            notice_enabled: true,
            revision: 0,
            saved_revision: Some(0),
            notices: Default::default(),
        };
        let mut variables = Variables::fresh(crate::expression::VariableProfile::Legacy1000);
        assert!(data.apply(b"setresult", &[4, 2], &mut variables).unwrap());
        assert!(data.apply(b"setresult", &[4, 99], &mut variables).unwrap());
        assert_eq!(data.words[4], 2);
        assert_eq!(data.take_notices().into_iter().collect::<Vec<_>>(), vec![3]);
        assert!(data.apply(b"getresult", &[4, 2], &mut variables).unwrap());
        assert_eq!(variables.read_stored(2), 1);
        assert!(data.apply(b"getresult", &[4, 3], &mut variables).unwrap());
        assert_eq!(variables.read_stored(2), 0);
        let before = data.words.clone();
        assert!(
            data.apply(b"setresult", &[1024, 1], &mut variables)
                .is_err()
        );
        assert_eq!(data.words, before);
        assert!(data.take_notices().is_empty());
    }
}
