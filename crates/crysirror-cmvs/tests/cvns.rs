// SPDX-License-Identifier: MPL-2.0
use crysirror_cmvs::{
    Script,
    vm::{CmvsVm, VmProfile, VmYield},
};
use crysirror_core::service::MemoryAssetReader;

#[test]
fn cvns_admits_packed_and_unpacked_pss0_and_keeps_the_abi_boundary() -> anyhow::Result<()> {
    // One label, a frame yield instruction, and a raw text byte.
    let payload = [0, 0, 0, 0, 0, 0x20, 0];
    for compressed in [false, true] {
        let mut bytes = b"PSS0".to_vec();
        for word in [
            48u32,
            0,
            0,
            1,
            2,
            0,
            1,
            0,
            if compressed { 8 } else { 0 },
            7,
            0,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        if compressed {
            bytes.push(0x7f);
        }
        bytes.extend_from_slice(&payload);
        let script = Script::decode(bytes.clone())?;
        assert_eq!(script.code(), [0, 0x20]);
        assert_eq!(script.labels().collect::<Vec<_>>(), [0]);
        let reader = MemoryAssetReader(bytes.into());
        let mut vm = CmvsVm::prepare(VmProfile::Cvns20040515X86, &reader)?;
        assert!(matches!(vm.run(1)?, VmYield::Frame));
        assert!(CmvsVm::prepare(VmProfile::Pc20160201X64, &reader).is_err());
    }
    Ok(())
}

#[cfg(feature = "runtime")]
#[test]
#[ignore = "requires the original CVNS game directory"]
fn cvns_original_archive_members_are_read_through_the_profile() -> anyhow::Result<()> {
    use crysirror_cmvs::vm::{Cpz6Resources, decode_at};
    use crysirror_core::{
        path::{Mount, VfsPath},
        state::ClientId,
    };
    use crysirror_resource::{DecodeContext, FormatRegistry};
    use crysirror_runtime::vfs::DiskVfs;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct Writable(PathBuf);
    impl Drop for Writable {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    let game = PathBuf::from(std::env::var_os("CRYSIRROR_CVNS_GAME").expect("game root"));
    let tag = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let writable =
        Writable(std::env::temp_dir().join(format!("crysirror-cvns-{}-{tag}", std::process::id())));
    std::fs::create_dir(&writable.0)?;
    let client = ClientId(1);
    let mut vfs = DiskVfs::new(&game, &writable.0)?;
    vfs.register(client, "cmvs", "fixture")?;
    let storage = vfs.storage(client)?;
    let exe = storage.open_asset(&VfsPath::new(Mount::Asset, "cvns.exe")?)?;
    let resources = Cpz6Resources::prepare(
        VmProfile::Cvns20040515X86,
        DecodeContext::new(client),
        exe.as_ref(),
    )?;
    let mut formats = FormatRegistry::default();
    formats.register(crysirror_formats_arc::legacy_archive("purple-software/cpz1").unwrap())?;
    let archive = resources.open(
        storage.as_ref(),
        &formats,
        &VfsPath::new(Mount::Asset, "data/pack/script.cpz")?,
    )?;
    let main = Script::read(archive.open_entry("main.pss")?.as_ref())?;
    assert_eq!(archive.entries().len(), 89);
    assert_eq!(main.code().len(), 0x9adc6);
    for entry in archive.entries() {
        use anyhow::Context;
        Script::read(archive.open_entry(&entry.path)?.as_ref())
            .with_context(|| format!("PSS0 member {}", entry.path))?;
    }
    let startup = storage.open_asset(&VfsPath::new(Mount::Asset, "data/pack/start.pss")?)?;
    let startup = Script::read(startup.as_ref())?;
    assert_eq!(startup.code().len(), 0xc40);
    for (script, count) in [(startup, 270), (main, 72_315)] {
        let mut pc = 0;
        for _ in 0..count {
            pc = decode_at(script.code(), pc)?.end;
        }
        assert_eq!(pc, script.code().len());
    }
    Ok(())
}
