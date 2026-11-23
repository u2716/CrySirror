//! Authored ALD link maps and independent shared archive API; no game data.
use crysirror_core::{
    service::{AssetReader, MemoryAssetReader},
    state::ClientId,
};
use crysirror_formats_arc::ald_catalog::{AldCatalog, AldVolume};
use crysirror_resource::{Archive, DecodeContext, read_all};
use std::sync::Arc;
fn put_pointer(out: &mut [u8], index: usize, offset: u32) {
    out[index * 3..index * 3 + 3].copy_from_slice(&[
        (offset >> 8) as u8,
        (offset >> 16) as u8,
        (offset >> 24) as u8,
    ]);
}
fn volume(label: &str, links: &[(usize, u8, u16)]) -> Vec<u8> {
    let mut out = vec![0; 1024];
    for (i, offset) in [256, 512, 768, 1024].into_iter().enumerate() {
        put_pointer(&mut out, i, offset);
    }
    for &(id, number, ordinal) in links {
        out[256 + id * 3] = number + 1;
        out[257 + id * 3..259 + id * 3].copy_from_slice(&ordinal.to_le_bytes());
    }
    for (index, offset) in [512usize, 768].into_iter().enumerate() {
        let name = format!("{label}{}.dat", index + 1);
        let data = format!("{label}-payload-{}", index + 1);
        out[offset..offset + 4].copy_from_slice(&32u32.to_le_bytes());
        out[offset + 4..offset + 8].copy_from_slice(&(data.len() as u32).to_le_bytes());
        out[offset + 16..offset + 16 + name.len()].copy_from_slice(name.as_bytes());
        out[offset + 32..offset + 32 + data.len()].copy_from_slice(data.as_bytes());
    }
    out
}
fn input(number: u8, bytes: Vec<u8>) -> AldVolume {
    AldVolume {
        number,
        source: Arc::new(MemoryAssetReader(bytes.into())),
    }
}
#[test]
fn original_numbers_follow_cross_volume_links_and_keep_holes() {
    let links = [(0, 1, 2), (2, 0, 1), (3, 1, 1), (4, 0, 2), (5, 2, 1)];
    let cx = DecodeContext::new(ClientId(7));
    let catalog = AldCatalog::load(
        vec![input(1, volume("B", &links)), input(0, volume("A", &links))],
        &cx,
    )
    .unwrap();
    assert_eq!(
        catalog.entries().iter().map(|e| e.id).collect::<Vec<_>>(),
        [0, 2, 3, 4]
    );
    for (id, name, value) in [
        (0, "B2.dat", "B-payload-2"),
        (2, "A1.dat", "A-payload-1"),
        (3, "B1.dat", "B-payload-1"),
        (4, "A2.dat", "A-payload-2"),
    ] {
        assert_eq!(catalog.entry(id).unwrap().unwrap().path, name);
        let file = catalog.open(id, &cx).unwrap();
        assert_eq!(read_all(file.as_ref(), 100).unwrap(), value.as_bytes());
        assert_eq!(catalog.resolve(name), Some(id));
    }
    assert!(catalog.entry(1).unwrap().is_none());
    assert!(catalog.entry(99).unwrap().is_none());
    assert!(format!("{:#}", catalog.entry(5).unwrap_err()).contains("missing volume 2"));
    assert!(catalog.open(0, &DecodeContext::new(ClientId(8))).is_err());
    assert_eq!(
        cx.resident_bytes(),
        0,
        "only streamed source ranges, no retained decoded payload"
    );
}
#[test]
fn corrupt_links_intervals_names_and_budgets_do_not_fall_back_to_filename_order() {
    let cx = DecodeContext::new(ClientId(7));
    let base = volume("A", &[(0, 0, 1)]);
    let mut bad = base.clone();
    bad[257..259].copy_from_slice(&500u16.to_le_bytes());
    assert!(AldCatalog::load(vec![input(0, bad)], &cx).is_err());
    let mut bad = base.clone();
    put_pointer(&mut bad, 2, 0x10000);
    assert!(AldCatalog::load(vec![input(0, bad)], &cx).is_err());
    let mut bad = base.clone();
    bad[512..516].copy_from_slice(&15u32.to_le_bytes());
    assert!(AldCatalog::load(vec![input(0, bad)], &cx).is_err());
    let mut bad = base.clone();
    bad[528..544].fill(b'x');
    assert!(AldCatalog::load(vec![input(0, bad)], &cx).is_err());
    let mut tiny = cx.clone();
    tiny.limits.max_index_size = 511;
    assert!(AldCatalog::load(vec![input(0, base.clone())], &tiny).is_err());
    let mut tiny = cx.clone();
    tiny.limits.max_entries = 10;
    assert!(AldCatalog::load(vec![input(0, base.clone())], &tiny).is_err());
    let mut tiny = cx.clone();
    tiny.limits.max_entry_size = 2;
    assert!(AldCatalog::load(vec![input(0, base.clone())], &tiny).is_err());
    assert!(AldCatalog::load(vec![input(0, base.clone()), input(0, base)], &cx).is_err());
}
#[test]
fn short_reads_and_zero_next_pointer_keep_original_absence_semantics() {
    struct Partial(Vec<u8>);
    impl AssetReader for Partial {
        fn len(&self) -> crysirror_core::Result<u64> {
            Ok(self.0.len() as u64)
        }
        fn read_at(&self, offset: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
            let start = (offset.min(self.0.len() as u64)) as usize;
            let n = out.len().min(2).min(self.0.len() - start);
            out[..n].copy_from_slice(&self.0[start..start + n]);
            Ok(n)
        }
    }
    let cx = DecodeContext::new(ClientId(7));
    let mut file = volume("A", &[(0, 0, 1), (1, 0, 2)]);
    put_pointer(&mut file, 3, 0);
    let c = AldCatalog::load(
        vec![AldVolume {
            number: 0,
            source: Arc::new(Partial(file)),
        }],
        &cx,
    )
    .unwrap();
    assert_eq!(
        read_all(c.open(0, &cx).unwrap().as_ref(), 100).unwrap(),
        b"A-payload-1"
    );
    assert!(
        c.entry(1).unwrap().is_none(),
        "zero next pointer is absent, not a fabricated last file"
    );
}
#[test]
#[ignore = "reads owner ALD volumes at runtime; no archive bytes are committed"]
fn owner_rance7_numbered_cg_catalogue_uses_original_both_volumes() {
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
        sync::Mutex,
    };
    struct FileReader(Mutex<File>);
    impl AssetReader for FileReader {
        fn len(&self) -> crysirror_core::Result<u64> {
            self.0
                .lock()
                .unwrap()
                .metadata()
                .map(|m| m.len())
                .map_err(|e| crysirror_core::Error::Io(e.to_string()))
        }
        fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
            let mut f = self.0.lock().unwrap();
            f.seek(SeekFrom::Start(at))
                .and_then(|_| f.read(out))
                .map_err(|e| crysirror_core::Error::Io(e.to_string()))
        }
    }
    let root = std::path::PathBuf::from(std::env::var("CRYSIRROR_SYSTEM4_ROOT").unwrap());
    let mut volumes = Vec::new();
    for (number, suffix) in [(0, "GA.ald"), (1, "GB.ald")] {
        let path = std::fs::read_dir(&root)
            .unwrap()
            .map(|p| p.unwrap().path())
            .find(|p| p.to_string_lossy().ends_with(suffix))
            .unwrap();
        volumes.push(AldVolume {
            number,
            source: Arc::new(FileReader(Mutex::new(File::open(path).unwrap()))),
        });
    }
    let cx = DecodeContext::new(ClientId(7));
    let catalog = AldCatalog::load(volumes, &cx).unwrap();
    let mut counts = [0; 2];
    for entry in catalog.entries() {
        counts[entry.metadata["volume"].as_u64().unwrap() as usize] += 1;
        let source = catalog.open(entry.id, &cx).unwrap();
        assert_eq!(source.len().unwrap(), entry.size);
        let mut prefix = [0; 4];
        if entry.size >= 4 {
            crysirror_resource::read_exact_at(source.as_ref(), 0, &mut prefix).unwrap();
        }
    }
    println!(
        "original ALD numeric catalogue entries={} volumes={counts:?}",
        catalog.entries().len()
    );
    assert!(counts.iter().all(|n| *n > 0));
}
