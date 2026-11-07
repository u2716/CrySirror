// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded filesystem work within a supplied storage capability. Call on a
//! worker: enumeration, copying and deletion may perform storage I/O. A plan
//! contains canonical logical paths, never ambient paths or OS file handles.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, StorageMetadata},
};

#[derive(Clone, Copy, Debug)]
pub struct TreeLimits {
    pub entries: usize,
    pub bytes: u64,
    pub depth: usize,
}
impl Default for TreeLimits {
    fn default() -> Self {
        Self {
            entries: 65536,
            bytes: 256 * 1024 * 1024,
            depth: 64,
        }
    }
}
#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub path: VfsPath,
    pub metadata: StorageMetadata,
}
#[derive(Clone, Copy, Debug)]
pub struct TreeCopyPolicy {
    pub preserve_metadata: bool,
    pub replace_read_only: bool,
    pub merge_directories: bool,
    pub continue_on_io_error: bool,
}
impl Default for TreeCopyPolicy {
    fn default() -> Self {
        Self {
            preserve_metadata: false,
            replace_read_only: false,
            merge_directories: true,
            continue_on_io_error: false,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeWorkReport {
    pub entries: usize,
    pub bytes: u64,
    pub complete: bool,
}
fn ordinary_io(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<crysirror_core::Error>()
        .is_some_and(|e| {
            matches!(
                e,
                crysirror_core::Error::NotFound
                    | crysirror_core::Error::ReadOnly
                    | crysirror_core::Error::Io(_)
            )
        })
}
fn join(mount: Mount, directory: &str, name: &str) -> Result<VfsPath> {
    ensure!(
        !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != "..",
        "capability tree direct child"
    );
    Ok(VfsPath::new(
        mount,
        &if directory.is_empty() {
            name.to_owned()
        } else {
            format!("{directory}/{name}")
        },
    )?)
}
fn limits(limits: TreeLimits) -> Result<()> {
    ensure!(
        limits.entries > 0
            && limits.entries <= 1 << 21
            && limits.depth > 0
            && limits.depth <= 256
            && limits.bytes > 0,
        "capability tree limits"
    );
    Ok(())
}
/// Provider-order snapshot of immediate DOS-wildcard matches. Metadata must
/// exist for every returned entry; a disappearing entry is simply omitted.
pub fn find(
    storage: &dyn ClientStorage,
    mount: Mount,
    directory: &str,
    pattern: &str,
    limit: TreeLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<TreeEntry>> {
    limits(limit)?;
    ensure!(
        pattern.len() <= 4096 && !pattern.contains(['/', '\\', '\0']),
        "capability find pattern"
    );
    if !directory.is_empty() {
        ensure!(
            VfsPath::new(mount, directory)?.relative() == directory,
            "capability find canonical directory"
        );
    }
    check()?;
    let names = storage.list(mount, directory)?;
    ensure!(names.len() <= limit.entries, "capability find entry budget");
    let mut result = Vec::new();
    for name in names {
        check()?;
        if name == "." || name == ".." {
            continue;
        }
        let path = join(mount, directory, &name)?;
        if !crate::dos_glob::matches(pattern, &name) {
            continue;
        }
        if let Some(metadata) = storage.metadata(mount, path.relative())? {
            result.push(TreeEntry { path, metadata });
        }
    }
    Ok(result)
}
/// Parent-before-child snapshot. All metadata, path depth and aggregate byte
/// limits are admitted before a caller starts mutating a destination tree.
pub fn scan(
    storage: &dyn ClientStorage,
    root: &VfsPath,
    limit: TreeLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<TreeEntry>> {
    limits(limit)?;
    let mut pending = vec![(root.clone(), 0usize)];
    let mut result = Vec::new();
    let mut bytes = 0u64;
    let mut observed = 0usize;
    while let Some((path, depth)) = pending.pop() {
        check()?;
        ensure!(depth <= limit.depth, "capability tree depth budget");
        let metadata = storage
            .metadata(path.mount(), path.relative())?
            .ok_or(crysirror_core::Error::NotFound)?;
        if metadata.is_directory {
            let names = storage.list(path.mount(), path.relative())?;
            observed = observed
                .checked_add(names.len())
                .context("capability tree entry overflow")?;
            ensure!(observed <= limit.entries, "capability tree entry budget");
            for name in names.into_iter().rev() {
                if name == "." || name == ".." {
                    continue;
                }
                pending.push((join(path.mount(), path.relative(), &name)?, depth + 1));
            }
        } else {
            bytes = bytes
                .checked_add(
                    metadata
                        .size
                        .context("capability tree file size unavailable")?,
                )
                .context("capability tree byte overflow")?;
            ensure!(bytes <= limit.bytes, "capability tree byte budget");
        }
        ensure!(result.len() < limit.entries, "capability tree entry budget");
        result.push(TreeEntry { path, metadata });
    }
    Ok(result)
}
/// Copy bytes of a file or complete directory tree. Files publish atomically;
/// the complete tree is not a transaction. Source and destination trees
/// must be disjoint. Provider access modes remain authoritative for every call.
pub fn copy(
    storage: &dyn ClientStorage,
    source: &VfsPath,
    destination: &VfsPath,
    limit: TreeLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let report = copy_with_policy(
        storage,
        source,
        destination,
        limit,
        TreeCopyPolicy::default(),
        check,
    )?;
    ensure!(report.complete, "capability tree copy incomplete");
    Ok(())
}
/// Metadata-preserving copies use the host's real copy capability, without a
/// byte-upload fallback. That backend decides timestamp/permission support and
/// does not promise atomic file replacement. Ordinary I/O failures can be
/// reported while continuing siblings; limits/capability failures still abort.
pub fn copy_with_policy(
    storage: &dyn ClientStorage,
    source: &VfsPath,
    destination: &VfsPath,
    limit: TreeLimits,
    policy: TreeCopyPolicy,
    check: &dyn Fn() -> Result<()>,
) -> Result<TreeWorkReport> {
    if source.mount() == destination.mount() {
        let a = source.relative().to_ascii_lowercase();
        let b = destination.relative().to_ascii_lowercase();
        ensure!(
            a != b && !b.starts_with(&format!("{a}/")) && !a.starts_with(&format!("{b}/")),
            "capability copy overlapping trees"
        );
    }
    let plan = scan(storage, source, limit, check)?;
    let mut report = TreeWorkReport {
        complete: true,
        ..TreeWorkReport::default()
    };
    for entry in plan {
        check()?;
        report.entries += 1;
        if !entry.metadata.is_directory {
            report.bytes = report
                .bytes
                .checked_add(entry.metadata.size.unwrap_or(0))
                .context("capability copy byte overflow")?;
            ensure!(report.bytes <= limit.bytes, "capability copy byte budget");
        }
        let result = (|| -> Result<()> {
            let suffix = entry
                .path
                .relative()
                .strip_prefix(source.relative())
                .context("capability copy source prefix")?;
            let requested = VfsPath::new(
                destination.mount(),
                &format!("{}{suffix}", destination.relative()),
            )?;
            let target =
                crate::capability_lookup::resolve_ascii_case_insensitive(storage, &requested)?
                    .unwrap_or(requested);
            if entry.metadata.is_directory {
                match storage.metadata(target.mount(), target.relative())? {
                    Some(info) => {
                        if !info.is_directory || !policy.merge_directories {
                            return Err(crysirror_core::Error::Io(
                                "copy destination directory already exists".into(),
                            )
                            .into());
                        }
                    }
                    None => storage.create_directory(&target)?,
                }
            } else {
                if policy.preserve_metadata {
                    storage.copy_file(&entry.path, &target, policy.replace_read_only)?;
                } else {
                    let reader = storage.open_read(&entry.path)?;
                    ensure!(
                        reader.len()? <= entry.metadata.size.unwrap_or(0),
                        "capability copy source grew after admission"
                    );
                    let data = crate::read_all(reader.as_ref(), entry.metadata.size.unwrap_or(0))?;
                    check()?;
                    storage.write_atomic(&target, &data)?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            if policy.continue_on_io_error && ordinary_io(&error) {
                report.complete = false;
            } else {
                return Err(error);
            }
        }
    }
    Ok(report)
}
/// Remove children before parents. Partial I/O failure is visible and does not
/// claim a whole-tree transaction or recursively follow an ambient OS path.
pub fn remove(
    storage: &dyn ClientStorage,
    root: &VfsPath,
    limit: TreeLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let report = remove_with_policy(storage, root, limit, false, false, check)?;
    ensure!(report.complete, "capability tree removal incomplete");
    Ok(())
}
/// Optionally honor read-only file attributes and continue after ordinary I/O
/// failures. Children always precede parents; there is no whole-tree rollback.
pub fn remove_with_policy(
    storage: &dyn ClientStorage,
    root: &VfsPath,
    limit: TreeLimits,
    refuse_read_only_files: bool,
    continue_on_io_error: bool,
    check: &dyn Fn() -> Result<()>,
) -> Result<TreeWorkReport> {
    let plan = scan(storage, root, limit, check)?;
    let mut report = TreeWorkReport {
        complete: true,
        ..TreeWorkReport::default()
    };
    for entry in plan.into_iter().rev() {
        check()?;
        report.entries += 1;
        let result = (|| -> Result<()> {
            if entry.metadata.is_directory {
                storage.remove_directory(&entry.path)?;
            } else {
                if refuse_read_only_files
                    && storage.read_only(entry.path.mount(), entry.path.relative())?
                {
                    return Err(crysirror_core::Error::ReadOnly.into());
                }
                storage.remove(&entry.path)?;
                report.bytes = report
                    .bytes
                    .checked_add(entry.metadata.size.unwrap_or(0))
                    .context("capability removal byte overflow")?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            if continue_on_io_error && ordinary_io(&error) {
                report.complete = false;
            } else {
                return Err(error);
            }
        }
    }
    Ok(report)
}
