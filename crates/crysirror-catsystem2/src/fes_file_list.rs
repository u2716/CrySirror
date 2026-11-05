// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FILELIST35 (765030/7650F0/765550): native ordered name→path state;
//! scoped enumeration and DOS wildcard matching belong to shared Storage/Resources.
use crate::{fes_object_bank::Bank, native_interpreter::Interpreter, native_values::StringVariables};
use anyhow::{ensure, Context, Result};
use crysirror_core::{path::{Mount, VfsPath}, service::ClientStorage};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::BTreeMap, sync::Arc};

type Files = BTreeMap<String, String>;
#[derive(Default)]
pub(crate) struct Lists {
    files: BTreeMap<u32, Files>,
    pending: Option<Pending>,
}
struct Pending { owner: u32, pc: u32, id: u32, target: Vec<u8>, arguments: Vec<Vec<u8>>, task: Task<Files> }

fn text(bytes: &[u8]) -> Result<String> {
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(bytes);
    ensure!(!errors, "CatSystem2 FILELIST Shift_JIS path");
    Ok(text.into_owned())
}
fn encode(text: &str) -> Result<Vec<u8>> {
    let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(!errors, "CatSystem2 FILELIST filename cannot be represented in Shift_JIS");
    Ok(bytes.into_owned())
}
fn directory(root: &str) -> Result<(Mount, String)> {
    let root = root.replace('\\', "/");
    let (mount, root) = match root.strip_prefix("%save%") {
        Some(tail) => (Mount::Save, tail.trim_start_matches('/')),
        None => (Mount::Asset, root.as_str()),
    };
    ensure!(!root.contains('%'), "CatSystem2 FILELIST root requires an unsupported host-directory substitution");
    let root = root.trim_end_matches('/');
    let root = root.strip_prefix("./").unwrap_or(root);
    let root = if root == "." { "" } else { root };
    if !root.is_empty() { VfsPath::new(mount, root)?; }
    Ok((mount, root.to_owned()))
}
fn enumerate(storage: &dyn ClientStorage, root: &str, patterns: &[String], cancel: &crysirror_tasks::Cancellation) -> Result<Files> {
    let (mount, root) = directory(root)?;
    let mut files = Files::new();
    for pattern in patterns {
        cancel.check()?;
        let pattern = pattern.replace('\\', "/");
        let (subdir, pattern) = pattern.rsplit_once('/').unwrap_or(("", pattern.as_str()));
        if pattern.is_empty() { continue; }
        let dir = if root.is_empty() { subdir.to_owned() } else if subdir.is_empty() { root.clone() } else { format!("{root}/{subdir}") };
        if !dir.is_empty() { VfsPath::new(mount, &dir)?; }
        let entries = match storage.list(mount, &dir) {
            Ok(entries) => entries,
            Err(crysirror_core::Error::NotFound) => continue,
            Err(error) => return Err(error.into()),
        };
        for name in entries {
            cancel.check()?;
            if !crysirror_resource::dos_glob::matches(pattern, &name) { continue; }
            ensure!(files.len() < 65536, "CatSystem2 FILELIST entry budget");
            let path = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            VfsPath::new(mount, &path)?;
            // 7650F0 lowercases the map key and retains the first duplicate;
            // the stored value keeps its enumerated filename spelling.
            let native_path = if mount == Mount::Save { format!("%save%/{path}") } else { path };
            files.entry(name.to_lowercase()).or_insert(native_path);
        }
    }
    Ok(files)
}

impl Lists {
    pub(crate) fn reap(&mut self, tasks: &crate::task_policy::NativeTaskPolicy) {
        self.files.retain(|id, _| tasks.fes(id.saturating_sub(1) / 4096).is_ok());
        if self.pending.as_ref().is_some_and(|pending| tasks.fes(pending.owner).is_err()) { self.pending = None; }
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply(&mut self, storage: Arc<dyn ClientStorage>, pool: &TaskPool, owner: u32, pc: u32, bank: &Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &mut StringVariables) -> Result<bool> {
        let method = arguments.first().context("CatSystem2 FILELIST method")?;
        if let Some(pending) = &mut self.pending {
            ensure!(pending.owner == owner && pending.pc == pc && pending.target == target && pending.arguments == arguments, "CatSystem2 FILELIST pending operation changed");
            let Some(found) = pending.task.try_take()? else { return Ok(false) };
            let id = pending.id;
            self.pending = None;
            let files = self.files.entry(id).or_default();
            for (name, path) in found { files.entry(name).or_insert(path); }
            return Ok(true);
        }
        let args = &arguments[1..];
        if method == b"make" {
            let permit = match pool.reserve() {
                Ok(permit) => permit,
                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(error) => return Err(error),
            };
            let reference = crate::fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
            // 779470 only accepts one selected member, not an all-array call.
            if reference.index == -1 { return Ok(true); }
            let Some(id) = bank.lookup(&reference) else { return Ok(true) };
            if bank.objects[&id].declared_type != 35 { return Ok(true); }
                let root = args.first().map_or(Ok(String::new()), |root| text(&strings.expand(root, 2560)?))?;
                let patterns = args.iter().skip(1).map(|pattern| text(&strings.expand(pattern, 2560)?)).collect::<Result<Vec<_>>>()?;
                self.pending = Some(Pending { owner, pc, id, target: target.to_vec(), arguments: arguments.to_vec(), task: permit.spawn(move |cancel| enumerate(storage.as_ref(), &root, &patterns, &cancel))? });
                return Ok(false);
        } else {
            ensure!(method == b"get", "CatSystem2 FILELIST method");
            let Some(kind) = args.first() else { return Ok(true) };
            for id in bank.targets(target, vm, strings)? {
                ensure!(bank.objects[&id].declared_type == 35, "CatSystem2 get on non-FILELIST class requires its native data implementation");
                let files = self.files.get(&id);
                if kind == b"count" {
                    if let Some(dest) = args.get(1) {
                        let dest = vm.evaluate(dest)?;
                        vm.set_flag(dest, files.map_or(0, |files| files.len() as u32));
                    }
                } else {
                    let success = args.get(1).map_or(Ok(u32::MAX), |arg| vm.evaluate(arg))?;
                    let dest = args.get(2).map_or(Ok(u32::MAX), |arg| vm.evaluate(arg))?;
                    let index = args.get(3).map_or(Ok(u32::MAX), |arg| vm.evaluate(arg))?;
                    let found = files.and_then(|files| files.iter().nth(index as usize));
                    if let Some((_, path)) = found {
                        let output = if kind == b"fullpath" { path.as_str() } else { path.rsplit('/').next().unwrap_or(path) };
                        strings.assign(dest, encode(output)?)?;
                    }
                    vm.set_flag(success, u32::from(found.is_some()));
                }
            }
        }
        Ok(true)
    }
}
