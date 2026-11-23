//! Malie archive formats through scoped Resources, not an engine runtime.
//! Reference layouts: GARbro b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0,
//! ArcFormats/Malie/{ArcLIB,ArcLIBU,LibScheme,MalieEncryption}.cs.
//! These are third-party format references, not original-program parity.
//! Current format/profile contract: `malie.md` beside this module.
// Copyright (C) 2015-2017 by morkt
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.
//

use super::*;

/// Additional format beyond the arc_unpacker decoder inventory. It uses the
/// same public registry/mount route as the existing LIBP and LIBU plugins.
pub(crate) static ADDITIONAL_FORMATS: &[Legacy] = &[Legacy {
    id: "malie/lib",
    probe: Some(|s| magic(s, 0, b"LIB\0")),
    open: lib,
}];

pub(crate) fn wrap(archive: Box<dyn Archive>, cx: &DecodeContext) -> Box<dyn Archive> {
    Box::new(EncryptedArchive {
        archive,
        client: cx.client,
    })
}
struct EncryptedArchive {
    archive: Box<dyn Archive>,
    client: crysirror_core::state::ClientId,
}
impl Archive for EncryptedArchive {
    fn entries(&self) -> &[ArchiveEntry] {
        self.archive.entries()
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.archive.resolve(path)
    }
    fn entry_metadata(&self, id: u32) -> serde_json::Value {
        let mut facts = self.archive.entry_metadata(id);
        if let Some(entry) = self.entries().get(id as usize) {
            if has_ext(&entry.path, "txtz") || has_ext(&entry.path, "psbz") {
                facts["malie_payload_wrapper"] =
                    serde_json::json!(if has_ext(&entry.path, "txtz") {
                        "zlib"
                    } else {
                        "psbz"
                    });
            }
        }
        facts
    }
    fn open(&self, id: u32, cx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        ensure!(
            cx.client == self.client,
            "Malie encrypted archive client mismatch"
        );
        cx.check_cancelled()?;
        let entry = self.entries().get(id as usize).context("Malie entry id")?;
        let mut reader = self.archive.open(id, cx)?;
        let mut spec = TransformSpec::new("zlib");
        if has_ext(&entry.path, "psbz") {
            // DatOpener.UnpackPsbz: a .psbz without the PSBZ header remains
            // raw. A recognized wrapper must decompress to its declared size.
            let prefix = bytes(reader.as_ref(), 0, length(reader.as_ref())?.min(8), 8)?;
            if !prefix.starts_with(b"PSBZ") {
                return Ok(reader);
            }
            let size = u64::from(le32(&prefix, 4)?);
            ensure!(size <= cx.limits.max_entry_size, "PSBZ decoded byte limit");
            let len = length(reader.as_ref())?;
            reader = crysirror_resource::subrange(reader, 8, len - 8)?;
            spec.expected_size = Some(size);
        } else if !has_ext(&entry.path, "txtz") {
            return Ok(reader);
        }
        cx.apply(&spec, reader, &entry.metadata)
    }
}

pub(crate) fn prepare(
    id: &str,
    source: Arc<dyn AssetReader>,
    cx: &DecodeContext,
) -> Result<Arc<dyn AssetReader>> {
    let Some(options) = cx.parameters.get(id) else {
        return Ok(source);
    };
    let options = options
        .as_object()
        .context("Malie format parameters must be an object")?;
    ensure!(
        options
            .keys()
            .all(|k| matches!(k.as_str(), "encryption" | "key_id")
                || (id == "malie/libp" && k == "data_alignment")),
        "unknown Malie format parameter"
    );
    let Some(encryption) = options.get("encryption") else {
        ensure!(
            !options.contains_key("key_id"),
            "Malie key_id requires encryption"
        );
        return Ok(source);
    };
    let algorithm = match encryption.as_str() {
        Some("camellia") => "malie-camellia",
        Some("cfi") => "malie-cfi",
        _ => bail!("unsupported Malie encryption profile"),
    };
    let key_id = options
        .get("key_id")
        .and_then(|v| v.as_str())
        .context("Malie encryption requires a host key_id")?;
    let mut spec = TransformSpec::new(algorithm);
    spec.key_id = Some(key_id.into());
    cx.apply(&spec, source, &serde_json::Value::Null)
}

fn join_name(root: &str, name: &str) -> Result<String> {
    let name = name.replace('\\', "/");
    ensure!(
        !name.contains([':', '\0'])
            && !name.starts_with('/')
            && (name.is_empty()
                || name
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != "..")),
        "invalid Malie archive path"
    );
    let path = if root.is_empty() {
        name
    } else if name.is_empty() {
        root.into()
    } else {
        format!("{root}/{name}")
    };
    ensure!(path.len() <= 4096, "Malie archive path length limit");
    Ok(path)
}

pub(super) fn lib(s: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    tree(s, cx, false)
}
pub(super) fn libu(s: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    tree(s, cx, true)
}
fn tree(s: &dyn AssetReader, cx: &DecodeContext, unicode: bool) -> Result<Vec<Row>> {
    let file_size = length(s)?;
    let magic_bytes: &[u8] = if unicode { b"LIBU" } else { b"LIB\0" };
    let id = if unicode { "malie/libu" } else { "malie/lib" };
    enum Work {
        Directory(u64, u64, String, usize),
        File(Row),
    }
    let mut stack = vec![Work::Directory(0, file_size, String::new(), 0)];
    let mut visited = std::collections::BTreeSet::new();
    let mut rows = Vec::new();
    let mut scanned = 0usize;
    let mut index_bytes = 0u64;
    while let Some(work) = stack.pop() {
        let (base, extent, root, depth) = match work {
            Work::Directory(base, extent, root, depth) => (base, extent, root, depth),
            Work::File(row) => {
                rows.push(row);
                continue;
            }
        };
        cx.check_cancelled()?;
        ensure!(
            depth <= 256 && visited.insert(base),
            "Malie cyclic/deep directory"
        );
        let encrypted = unicode && !magic(s, base, magic_bytes)?;
        let h = index_profile(
            cx,
            id,
            "__header__",
            base,
            bytes(s, base, 16, 16)?,
            encrypted,
        )?;
        ensure!(
            &h[..4] == magic_bytes,
            "Malie decoded directory header mismatch"
        );
        let n = if unicode {
            le32(&h, 8)? as usize
        } else {
            i16::from_le_bytes(h[8..10].try_into().unwrap()).max(0) as usize
        };
        ensure!(n > 0, "empty Malie directory");
        scanned = scanned
            .checked_add(n)
            .context("Malie index count overflow")?;
        limits(cx, scanned)?;
        let record_size = if unicode { 80u64 } else { 48u64 };
        let table_size = (n as u64)
            .checked_mul(record_size)
            .context("Malie table size overflow")?;
        index_bytes = index_bytes
            .checked_add(16)
            .and_then(|v| v.checked_add(table_size))
            .context("Malie index size overflow")?;
        ensure!(
            index_bytes <= cx.limits.max_index_size,
            "Malie cumulative index byte limit"
        );
        ensure!(
            16u64.checked_add(table_size).is_some_and(|v| v <= extent),
            "Malie directory table exceeds container"
        );
        let table_at = base
            .checked_add(16)
            .context("Malie directory offset overflow")?;
        let table = index_profile(
            cx,
            id,
            "__index__",
            table_at,
            index(s, cx, table_at, table_size)?,
            encrypted,
        )?;
        let data_at = table_at + table_size;
        let mut children = Vec::new();
        for p in table.chunks_exact(record_size as usize) {
            let (name, size, relative) = if unicode {
                let end = p[..68]
                    .chunks_exact(2)
                    .position(|v| v == [0, 0])
                    .unwrap_or(34)
                    * 2;
                (
                    utf16(&p[..end])?,
                    u64::from(le32(p, 68)?),
                    u64::from_le_bytes(p[72..80].try_into().unwrap()),
                )
            } else {
                (
                    sjis(zstr(&p[..36]))?,
                    u64::from(le32(p, 36)?),
                    u64::from(le32(p, 40)?),
                )
            };
            ensure!(
                relative <= i64::MAX as u64,
                "negative Malie relative offset"
            );
            let offset = base
                .checked_add(relative)
                .context("Malie entry offset overflow")?;
            if !unicode {
                range(file_size, offset, size)?;
                ensure!(
                    offset >= data_at && relative.checked_add(size).is_some_and(|v| v <= extent),
                    "LIB entry outside containing directory"
                );
            }
            let path = join_name(&root, &name)?;
            ensure!(!path.is_empty(), "empty Malie file/directory path");
            // Native LIB/LIBU distinguish embedded directories by their header,
            // not by a directory flag. Extensionless ordinary files stay files.
            let maybe_dir =
                !name.contains('.') && offset.checked_add(16).is_some_and(|v| v <= file_size);
            if maybe_dir {
                let child = index_profile(
                    cx,
                    id,
                    "__header__",
                    offset,
                    bytes(s, offset, 16, 16)?,
                    encrypted,
                )?;
                // The reference ReadDir returns false for a zero/nonpositive
                // count, so an extensionless empty container remains a file.
                let has_entries = if unicode {
                    i32::from_le_bytes(child[8..12].try_into().unwrap()) > 0
                } else {
                    i16::from_le_bytes(child[8..10].try_into().unwrap()) > 0
                };
                if &child[..4] == magic_bytes && has_entries {
                    children.push(Work::Directory(
                        offset,
                        if unicode { file_size - offset } else { size },
                        path,
                        depth + 1,
                    ));
                    continue;
                }
            }
            range(file_size, offset, size)?;
            children.push(Work::File(Row::packed(
                path,
                offset,
                size,
                size,
                if encrypted { Plan::Profile } else { Plan::Raw },
            )));
        }
        stack.extend(children.into_iter().rev());
    }
    checked_rows(s, cx, rows)
}

pub(super) fn libp(s: &dyn AssetReader, cx: &DecodeContext) -> Result<Vec<Row>> {
    let encrypted = !magic(s, 0, b"LIBP")?;
    let h = index_profile(
        cx,
        "malie/libp",
        "__header__",
        0,
        bytes(s, 0, 16, 16)?,
        encrypted,
    )?;
    ensure!(&h[..4] == b"LIBP", "LIBP decoded header mismatch");
    let n = le32(&h, 4)? as usize;
    let m = le32(&h, 8)? as usize;
    limits(cx, n)?;
    limits(cx, m)?;
    let t = index_profile(
        cx,
        "malie/libp",
        "__index__",
        16,
        index(
            s,
            cx,
            16,
            (n as u64)
                .checked_mul(32)
                .and_then(|v| v.checked_add((m as u64).checked_mul(4)?))
                .context("LIBP index size overflow")?,
        )?,
        encrypted,
    )?;
    let align = cx
        .parameters
        .get("malie/libp")
        .and_then(|p| p.get("data_alignment"))
        .map(|v| {
            v.as_u64()
                .context("LIBP data_alignment must be an unsigned integer")
        })
        .transpose()?
        .unwrap_or(4096);
    ensure!(
        align > 0 && align.is_power_of_two() && align <= 1 << 20,
        "invalid LIBP data alignment"
    );
    let base = (16 + t.len() as u64 + align - 1) & !(align - 1);
    ensure!(n > 0, "empty LIBP index");
    let mut rows = Vec::new();
    let mut stack = vec![(0usize, String::new(), 0usize)];
    let mut seen = std::collections::BTreeSet::new();
    while let Some((i, root, depth)) = stack.pop() {
        cx.check_cancelled()?;
        ensure!(depth <= 256 && seen.insert(i), "LIBP cyclic/deep directory");
        ensure!(i < n, "LIBP directory index");
        let p = &t[i * 32..i * 32 + 32];
        let name = sjis(zstr(&p[..20]))?;
        let path = join_name(&root, &name)?;
        let flags = le32(p, 20)?;
        let off = le32(p, 24)? as usize;
        let size = le32(p, 28)? as u64;
        if flags & 0x30000 != 0 {
            ensure!(off < m, "LIBP offset index");
            let offset = base + u64::from(le32(&t, n * 32 + off * 4)?) * 1024;
            ensure!(!path.is_empty(), "empty LIBP file path");
            rows.push(Row::packed(
                path,
                offset,
                size,
                size,
                if encrypted { Plan::Profile } else { Plan::Raw },
            ));
        } else if off > i {
            ensure!(
                off.checked_add(size as usize).is_some_and(|v| v <= n),
                "LIBP directory range"
            );
            ensure!(
                stack
                    .len()
                    .checked_add(size as usize)
                    .is_some_and(|v| v <= cx.limits.max_entries),
                "LIBP pending directory limit"
            );
            for child in (off..off + size as usize).rev() {
                stack.push((child, path.clone(), depth + 1));
            }
        }
    }
    checked_rows(s, cx, rows)
}
