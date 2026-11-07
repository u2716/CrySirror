// SPDX-License-Identifier: MPL-2.0
//! Named/id PE resource payloads as bounded immutable data. No LoadLibrary,
//! relocation, executable mapping, native imports or platform handles.
use super::{PeData, PeResourceName};
use anyhow::{Context, Result, ensure};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeRawResource {
    pub name: PeResourceName,
    pub language: u16,
    pub bytes: Vec<u8>,
}
impl PeData<'_> {
    ///Read all variants of one explicit resource type. Count and cumulative
    ///payload limits apply before copying; cancellation is checked per row.
    ///Absent resource types return an empty list; malformed trees fail.
    pub fn raw_resources(
        &self,
        kind: &PeResourceName,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<PeRawResource>> {
        use pelite::resources::{FindError, Name};
        check()?;
        let image = pelite::PeFile::from_bytes(self.bytes).context("PE raw resource header")?;
        let resources = image.resources().context("PE raw resource directory")?;
        if let PeResourceName::Name(name) = kind {
            ensure!(
                !name.is_empty() && name.len() <= 1024 && !name.contains('\0'),
                "PE raw resource type name"
            );
        }
        let key = match kind {
            PeResourceName::Id(id) => Name::Id(*id),
            PeResourceName::Name(name) => Name::Str(name),
        };
        let directory = match resources.root()?.get_dir(key) {
            Ok(dir) => dir,
            Err(FindError::NotFound) => return Ok(vec![]),
            Err(e) => return Err(e).context("PE raw resource type directory"),
        };
        let mut output = vec![];
        let mut total = 0usize;
        for resource in directory.entries() {
            check()?;
            let name = match resource.name()? {
                Name::Id(id) => PeResourceName::Id(id),
                Name::Wide(s) => PeResourceName::Name(
                    String::from_utf16(s).context("PE raw resource Unicode key")?,
                ),
                Name::Str(s) => PeResourceName::Name(s.into()),
            };
            if let PeResourceName::Name(name) = &name {
                ensure!(name.len() <= 1024, "PE raw resource key budget");
            }
            let languages = resource
                .entry()?
                .dir()
                .context("PE raw resource language directory")?;
            for language in languages.entries() {
                check()?;
                ensure!(output.len() < 1024, "PE raw resource count budget");
                let id = match language.name()? {
                    Name::Id(id) => u16::try_from(id).context("PE raw resource language range")?,
                    _ => anyhow::bail!("named PE raw resource language"),
                };
                let data = language
                    .entry()?
                    .data()
                    .context("PE raw resource payload")?;
                total = total
                    .checked_add(data.size())
                    .context("PE raw resource cumulative size overflow")?;
                ensure!(total <= 32 * 1024 * 1024, "PE raw resource byte budget");
                let bytes = data.bytes()?;
                let mut copy = Vec::new();
                copy.try_reserve_exact(bytes.len())?;
                for chunk in bytes.chunks(64 * 1024) {
                    check()?;
                    copy.extend_from_slice(chunk);
                }
                output.push(PeRawResource {
                    name: name.clone(),
                    language: id,
                    bytes: copy,
                });
            }
        }
        check()?;
        Ok(output)
    }
}
