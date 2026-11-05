// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native5D3C20/5D4B50/5D59E0 resource-name policy. Storage capabilities,
//! case-insensitive lookup and archive opening are reusable engine services.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_resource::{capability_lookup::resolve_ascii_case_insensitive, xml::XmlDocument};

#[derive(Clone, Debug)]
struct Row {
    name: String,
    archive: String,
    extension: String,
    directory: String,
}
#[derive(Clone, Debug)]
pub(crate) struct Catalog {
    rows: Vec<Row>,
}
fn path(value: &str) -> Result<VfsPath> {
    let normalized = value.replace('\\', "/");
    let p = VfsPath::new(Mount::Asset, &normalized)?;
    ensure!(
        p.relative() == normalized,
        "CatSystem2 noncanonical native resource path"
    );
    Ok(p)
}
impl Catalog {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let xml = XmlDocument::parse_with_fallback(bytes,encoding_rs::SHIFT_JIS)?;
        let table = xml
            .unique_child_path("INT")?
            .context("CatSystem2 native INT table missing")?;
        let mut rows = Vec::new();
        for node in xml.children(table)? {
            let field = |key: &str| -> Result<String> {
                let mut nodes = xml.children(node)?.filter(|n| n.name == key);
                let value = nodes
                    .next()
                    .map(|n| n.text.replace('\\', "/"))
                    .unwrap_or_default();
                ensure!(
                    nodes.next().is_none(),
                    "CatSystem2 duplicate native INT field"
                );
                Ok(value)
            };
            let name = field("name")?;
            if name.is_empty() {
                continue;
            }
            ensure!(rows.len() < 256, "CatSystem2 INT table allocation budget");
            // Grisaia declares kx3 twice with the same values; the first row stands.
            if rows.iter().any(|r: &Row| r.name.eq_ignore_ascii_case(&name)) {
                continue;
            }
            let archive = field("int")?;
            let extension = field("ext")?;
            let directory = field("dir")?;
            ensure!(
                name.is_ascii()
                    && extension.is_ascii()
                    && !extension.contains(['.', '/', ':'])
                    && extension.len() < 32,
                "CatSystem2 native resource type/extension profile"
            );
            if !archive.is_empty() {
                path(&archive)?;
            }
            if !directory.is_empty() {
                path(&directory)?;
            }
            rows.push(Row {
                name,
                archive,
                extension,
                directory,
            });
        }
        Ok(Self { rows })
    }
    pub fn load(storage: &dyn ClientStorage) -> Result<Self> {
        let p = VfsPath::new(Mount::Asset, "config/startup.xml")?;
        let actual = resolve(storage, &p)?.context("CatSystem2 native resource startup missing")?;
        let reader = storage.open_asset(&actual)?;
        Self::parse(&crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?)
    }
    pub fn candidates(
        &self,
        storage: &dyn ClientStorage,
        kind: &str,
        name: &str,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<VfsPath>> {
        check()?;
        ensure!(
            !name.is_empty() && name.len() < 1040 && name.encode_utf16().count()<260 && !name.contains('\0'),
            "CatSystem2 native resource name extent/encoding"
        );
        let name = name.replace('\\', "/");
        path(&name)?;
        let logical = if name.contains('/') {
            name
        } else {
            let row = self
                .rows
                .iter()
                .find(|r| r.name.eq_ignore_ascii_case(kind))
                .context("CatSystem2 native resource type absent")?;
            let filename = if name.contains('.') || row.extension.is_empty() {
                name
            } else {
                format!("{name}.{}", row.extension)
            };
            if row.archive.is_empty() {
                filename
            } else {
                format!("{}/{filename}", row.archive)
            }
        };
        path(&logical)?;
        let Some((archive, member)) = logical.split_once('/') else {
            return Ok(vec![path(&logical)?]);
        };
        if archive
            .rsplit('.')
            .next()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lst"))
        {
            let list_path = resolve(storage, &path(archive)?)?
                .context("CatSystem2 native LST catalog missing")?;
            let bytes = crysirror_resource::read_all(
                storage.open_asset(&list_path)?.as_ref(),
                1024 * 1024,
            )?;
            let mut candidates = Vec::new();
            for row in list_rows(&bytes)? {
                check()?;
                //5D4B50 inserts list rows at the linked-list tail. Mode3 in
                //5D59E0 visits each row: loose directory, then archive slot.
                if !row.0.is_empty() {
                    candidates.push(path(&format!("{}/{member}", row.0))?)
                }
                candidates.push(path(&format!("{}/{member}", row.1))?);
            }
            return Ok(candidates);
        }
        let mut candidates = Vec::new();
        if let Some(row) = self
            .rows
            .iter()
            .find(|r| r.archive.eq_ignore_ascii_case(archive))
        {
            if !row.directory.is_empty() {
                candidates.push(path(&format!("{}/{member}", row.directory))?)
            }
        }
        candidates.push(path(&logical)?);
        Ok(candidates)
    }
}
pub(crate) fn resolve(storage: &dyn ClientStorage, p: &VfsPath) -> Result<Option<VfsPath>> {
    match resolve_ascii_case_insensitive(storage, p) {
        Ok(value) => Ok(value),
        Err(error)
            if matches!(
                error.downcast_ref::<crysirror_core::Error>(),
                Some(crysirror_core::Error::NotFound)
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
///6A73A0 recognizes LF/CRLF, consumes the final nonterminated line and stops
///at NUL. It does not strip whitespace/comments or treat standalone CR as LF.
///The original32byte scratch row is bounded here rather than overflowing it.
fn list_rows(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    let mut rows = Vec::new();
    for raw in bytes.split_inclusive(|b| *b == b'\n') {
        let row = if let Some(line) = raw.strip_suffix(b"\n") {
            line.strip_suffix(b"\r").unwrap_or(line)
        } else {
            raw
        };
        ensure!(
            !row.is_empty() && row.len() < 32 && row.is_ascii() && !row.contains(&b'\r'),
            "CatSystem2 LST native row extent/encoding"
        );
        let row = std::str::from_utf8(row)?;
        let (directory, archive) = row
            .split_once('/')
            .context("CatSystem2 LST directory/archive separator missing")?;
        ensure!(
            rows.len() < 256 && !archive.is_empty() && !archive.contains('/'),
            "CatSystem2 LST archive/row allocation profile"
        );
        let directory = directory.replace('\\', "/");
        if !directory.is_empty() {
            path(&directory)?;
        }
        path(archive)?;
        ensure!(
            !archive
                .rsplit('.')
                .next()
                .is_some_and(|v| v.eq_ignore_ascii_case("lst")),
            "CatSystem2 recursive native LST catalog unsupported"
        );
        rows.push((directory, archive.to_owned()));
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::{Error, Result as CoreResult};
    use std::collections::BTreeMap;
    struct Storage {
        files: BTreeMap<String, Vec<u8>>,
    }
    impl ClientStorage for Storage {
        fn read(&self, p: &VfsPath) -> CoreResult<Vec<u8>> {
            self.files.get(p.relative()).cloned().ok_or(Error::NotFound)
        }
        fn list(&self, _: Mount, dir: &str) -> CoreResult<Vec<String>> {
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            let names = self
                .files
                .keys()
                .filter_map(|path| path.strip_prefix(&prefix))
                .map(|tail| tail.split('/').next().unwrap().to_owned())
                .collect::<std::collections::BTreeSet<_>>();
            Ok(names.into_iter().collect())
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> CoreResult<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> CoreResult<()> {
            Err(Error::ReadOnly)
        }
    }
    fn catalog() -> Catalog {
        Catalog::parse(b"<root><INT><int00><name>image</name><int>image.lst</int><ext>hg3</ext><dir>image</dir></int00><int07><name>mot</name><int>mot.int</int><ext>kcs</ext><dir>motions</dir></int07></INT></root>").unwrap()
    }
    #[test]
    fn original_list_tail_order_and_loose_before_archive_use_shared_case_lookup() {
        let storage = Storage {
            files: BTreeMap::from([
                (
                    "Image.LST".into(),
                    b"image\\bg/bg.int\r\nimage\\ev/ev.int\n".to_vec(),
                ),
                ("Image/BG/Logo.HG3".into(), vec![]),
            ]),
        };
        let candidates = catalog()
            .candidates(&storage, "image", "logo", &|| Ok(()))
            .unwrap();
        assert_eq!(
            candidates.iter().map(|p| p.relative()).collect::<Vec<_>>(),
            [
                "image/bg/logo.hg3",
                "bg.int/logo.hg3",
                "image/ev/logo.hg3",
                "ev.int/logo.hg3"
            ]
        );
        assert_eq!(
            resolve(&storage, &candidates[0])
                .unwrap()
                .unwrap()
                .relative(),
            "Image/BG/Logo.HG3"
        );
        let mot = catalog()
            .candidates(&storage, "mot", "pBase", &|| Ok(()))
            .unwrap();
        assert_eq!(
            mot.iter().map(|p| p.relative()).collect::<Vec<_>>(),
            ["motions/pBase.kcs", "mot.int/pBase.kcs"]
        );
        let direct = catalog()
            .candidates(&storage, "image", "other/NoExt", &|| Ok(()))
            .unwrap();
        assert_eq!(direct[0].relative(), "other/NoExt");
    }
    #[test]
    fn original_line_rules_do_not_drop_final_rows_or_invent_comment_processing() {
        assert_eq!(
            list_rows(b"loose/a.int\r\n/second.int\nlast/third.int\0ignored/fourth.int").unwrap(),
            [
                ("loose".into(), "a.int".into()),
                ("".into(), "second.int".into()),
                ("last".into(), "third.int".into())
            ]
        );
        for bytes in [
            b"no-separator".as_slice(),
            b"\n",
            b"../a.int",
            b"dir/sub/a.int",
            b"dir/recursive.lst",
            b"dir/invalid\rname.int",
        ] {
            assert!(list_rows(bytes).is_err(), "{bytes:?}");
        }
        assert!(list_rows(&[b'x'; 32]).is_err());
        assert!(catalog()
            .candidates(
                &Storage {
                    files: Default::default()
                },
                "image",
                "../escape",
                &|| Ok(())
            )
            .is_err());
        assert!(catalog()
            .candidates(
                &Storage {
                    files: Default::default()
                },
                "unknown",
                "file",
                &|| Ok(())
            )
            .is_err());
    }
}
