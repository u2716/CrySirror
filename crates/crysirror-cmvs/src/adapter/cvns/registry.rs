// SPDX-License-Identifier: MPL-2.0
//! 0x433db0–0x434050: client-scoped HKCU values under Software\<company>.
//! Both getters query the raw value data without inspecting the registry type.
use crate::vm::CmvsVm;
use anyhow::{Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::ClientStorage,
};

pub(super) fn name(bytes: &[u8]) -> Result<String> {
    let (name, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!invalid, "invalid CP932 registry name");
    Ok(name.into_owned())
}

// Encode each component so a native value name or subkey cannot acquire path authority.
// 'k' and 'v' keep child keys distinct from values with the same name.
fn component(prefix: char, name: &str) -> String {
    use std::fmt::Write;
    let mut encoded = prefix.to_string();
    for byte in name.to_lowercase().bytes() {
        write!(encoded, "{byte:02x}").expect("write to String");
    }
    encoded
}

pub(super) enum Action {
    ReadText(u32),
    ReadWord,
    Write(Vec<u8>),
}
pub(crate) struct Request {
    key: VfsPath,
    value: VfsPath,
    action: Action,
}
pub(crate) enum Reply {
    Text(u32, Vec<u8>),
    Result(u16, Option<u32>),
    UndefinedWord,
}

impl Request {
    pub(super) fn new(company: &str, key: &str, value: &str, action: Action) -> Result<Self> {
        let mut path = String::from("cvns-registry");
        for part in company
            .split('\\')
            .chain(key.split('\\'))
            .filter(|s| !s.is_empty())
        {
            path.push('/');
            path.push_str(&component('k', part));
        }
        Ok(Self {
            key: VfsPath::new(Mount::Save, &path)?,
            value: VfsPath::new(Mount::Save, &format!("{path}/{}", component('v', value)))?,
            action,
        })
    }

    pub fn run(self, storage: &dyn ClientStorage) -> Result<Reply> {
        let words = if matches!(self.action, Action::ReadWord) {
            2
        } else {
            3
        };
        if let Action::Write(bytes) = self.action {
            storage.write_atomic(&self.value, &bytes)?;
            return Ok(Reply::Result(words, Some(0)));
        }
        let reader = match storage.open_read(&self.value) {
            Ok(reader) => reader,
            Err(Error::NotFound) => {
                // 0x433fd0 returns whether RegOpenKeyEx succeeded, even if the value is absent.
                let exists = storage
                    .metadata(Mount::Save, self.key.relative())?
                    .is_some();
                return Ok(if !exists {
                    Reply::Result(words, Some(u32::MAX))
                } else if matches!(self.action, Action::ReadWord) {
                    Reply::UndefinedWord
                } else {
                    Reply::Result(words, Some(0))
                });
            }
            Err(e) => return Err(e.into()),
        };
        let bytes = crysirror_resource::read_all(reader.as_ref(), 4 * 1024 * 1024)?;
        Ok(match self.action {
            Action::ReadWord if bytes.len() < 4 => Reply::UndefinedWord,
            Action::ReadWord => {
                ensure!(
                    bytes.len() == 4,
                    "CVNS registry value exceeds native DWORD destination"
                );
                Reply::Result(
                    words,
                    Some(u32::from_le_bytes(bytes.try_into().expect("four bytes"))),
                )
            }
            Action::ReadText(dst) => Reply::Text(dst, bytes),
            Action::Write(_) => unreachable!(),
        })
    }
}

impl Reply {
    pub fn complete(self, vm: &mut CmvsVm) -> Result<()> {
        match self {
            Self::Text(dst, bytes) => vm.complete_registry_text(dst, &bytes)?,
            Self::Result(words, value) => vm.complete_owner_call(words, value)?,
            Self::UndefinedWord => vm.complete_cvns_undefined_word()?,
        }
        Ok(())
    }
}
