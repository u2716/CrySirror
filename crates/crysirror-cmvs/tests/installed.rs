// SPDX-License-Identifier: MPL-2.0
#![cfg(feature = "runtime")]

use crysirror_cmvs::{
    vm::{decode_at, Cpz6Resources, Expression, InstructionKind, Term, VmProfile},
    Script,
};
use crysirror_core::{
    path::{Mount, VfsPath},
    state::ClientId,
};
use crysirror_resource::{DecodeContext, FormatRegistry};
use crysirror_runtime::vfs::DiskVfs;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Writable(PathBuf);
impl Drop for Writable {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires the external CMVS 20160201 x64 corpus"]
fn original_script_archive_opens_through_scoped_resources() -> anyhow::Result<()> {
    let game = PathBuf::from(std::env::var_os("CRYSIRROR_CMVS_GAME").expect("game root"));
    let tag = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary =
        std::env::temp_dir().join(format!("crysirror-cmvs-{}-{tag}", std::process::id()));
    std::fs::create_dir(&temporary)?;
    let writable = Writable(temporary);
    let client = ClientId(1);
    let mut vfs = DiskVfs::new(&game, &writable.0)?;
    vfs.register(client, "cmvs", "fixture")?;
    let storage = vfs.storage(client)?;
    let source = storage.open_asset(&VfsPath::new(Mount::Asset, "cmvs64.exe")?)?;
    let cpz6 = Cpz6Resources::prepare(
        VmProfile::Pc20160201X64,
        DecodeContext::new(client),
        source.as_ref(),
    )?;
    let mut registry = FormatRegistry::default();
    registry.register(crysirror_formats_arc::legacy_archive("purple-software/cpz6").unwrap())?;
    let archive = cpz6.open(
        storage.as_ref(),
        &registry,
        &VfsPath::new(Mount::Asset, "data/pack/script.cpz")?,
    )?;
    let main_path = archive
        .entries()
        .iter()
        .find(|entry| {
            entry
                .path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("main.ps3"))
        })
        .expect("main.ps3")
        .path
        .clone();
    let main = archive.open_entry(&main_path)?;
    let script = Script::read(main.as_ref())?;
    assert_eq!(archive.entries().len(), 87);
    assert_eq!(script.entry_pc(), 0x38d1e);
    assert_eq!(script.code().len(), 235208);
    Ok(())
}

#[test]
#[ignore = "requires the external CMVS 20160201 x64 corpus"]
fn audit_installed_script_opcode_inventory() -> anyhow::Result<()> {
    fn expression_tags(expression: &Expression, tags: &mut BTreeMap<u16, BTreeSet<u16>>) {
        for term in &expression.terms {
            let tag = match term {
                Term::Scalar { tag, .. } => *tag,
                Term::Operator { opcode, .. } => *opcode,
                Term::Nested {
                    tag, expression, ..
                } => {
                    expression_tags(expression, tags);
                    *tag
                }
                Term::Indexed { tag, indices, .. } => {
                    for index in indices {
                        expression_tags(index, tags);
                    }
                    *tag
                }
            };
            tags.entry(expression.kind).or_default().insert(tag);
        }
    }

    let game = PathBuf::from(std::env::var_os("CRYSIRROR_CMVS_GAME").expect("game root"));
    let tag = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = std::env::temp_dir().join(format!(
        "crysirror-cmvs-inventory-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir(&temporary)?;
    let writable = Writable(temporary);
    let client = ClientId(1);
    let mut vfs = DiskVfs::new(&game, &writable.0)?;
    vfs.register(client, "cmvs", "inventory")?;
    let storage = vfs.storage(client)?;
    let exe = storage.open_asset(&VfsPath::new(Mount::Asset, "cmvs64.exe")?)?;
    let cpz6 = Cpz6Resources::prepare(
        VmProfile::Pc20160201X64,
        DecodeContext::new(client),
        exe.as_ref(),
    )?;
    let mut registry = FormatRegistry::default();
    registry.register(crysirror_formats_arc::legacy_archive("purple-software/cpz6").unwrap())?;
    let archive = cpz6.open(
        storage.as_ref(),
        &registry,
        &VfsPath::new(Mount::Asset, "data/pack/script.cpz")?,
    )?;

    let mut syscalls = BTreeSet::new();
    let mut instructions = BTreeSet::new();
    let mut tags = BTreeMap::new();
    let startup = storage.open_asset(&VfsPath::new(Mount::Asset, "data/pack/start.ps3")?)?;
    let mut count = 0;
    for input in std::iter::once(Ok(startup)).chain(
        archive
            .entries()
            .iter()
            .map(|entry| archive.open_entry(&entry.path)),
    ) {
        let script = Script::read(input?.as_ref())?;
        let mut pc = 0;
        while pc < script.code().len() {
            let instruction = decode_at(script.code(), pc)?;
            match instruction.kind {
                InstructionKind::System { .. } => {
                    syscalls.insert(instruction.opcode);
                }
                InstructionKind::Expression(expression) => {
                    instructions.insert(instruction.opcode);
                    expression_tags(&expression, &mut tags);
                }
                _ => {
                    instructions.insert(instruction.opcode);
                }
            }
            pc = instruction.end;
            count += 1;
        }
    }
    assert_eq!(count, 2_957_772);
    assert_eq!(syscalls.len(), 431);
    eprintln!("Top-level opcodes {instructions:04x?}");
    eprintln!("Expression operand/operator tags {tags:04x?}");
    eprintln!("Syscalls {syscalls:04x?}");
    Ok(())
}

#[test]
#[ignore = "requires the external CMVS 20070119 i386 installation"]
fn ps2_script_pack_opens_through_scoped_resources() -> anyhow::Result<()> {
    let game = PathBuf::from(std::env::var_os("CRYSIRROR_CMVS_GAME").expect("game root"));
    let tag = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let writable = Writable(std::env::temp_dir().join(format!("cmvs-ps2-{tag}")));
    std::fs::create_dir(&writable.0)?;
    let client = ClientId(1);
    let mut vfs = DiskVfs::new(&game, &writable.0)?;
    vfs.register(client, "cmvs", "fixture")?;
    let storage = vfs.storage(client)?;
    let source = storage.open_asset(&VfsPath::new(Mount::Asset, "cmvs.exe")?)?;
    let resources = Cpz6Resources::prepare(
        VmProfile::Pc20070119X86,
        DecodeContext::new(client),
        source.as_ref(),
    )?;
    let mut registry = FormatRegistry::default();
    registry.register(crysirror_formats_arc::legacy_archive("purple-software/cpz3").unwrap())?;
    let archive = resources.open(
        storage.as_ref(),
        &registry,
        &VfsPath::new(Mount::Asset, "data/pack/script.cpz")?,
    )?;
    assert_eq!(archive.entries().len(), 140);
    for member in archive.entries() {
        let script = Script::read(archive.open_entry(&member.path)?.as_ref())?;
        assert!(!script.code().is_empty(), "{}", member.path);
        if member.path == "main.ps2" {
            assert_eq!(script.code().len(), 129448);
        }
    }
    Ok(())
}
