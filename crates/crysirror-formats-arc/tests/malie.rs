//! Authored Malie bytes exercising the shared registry, scoped keys and mounts.
//! No game archives, executables, fonts or title keys are used.
use crysirror_core::{
    service::{AssetReader, MemoryAssetReader},
    state::ClientId,
};
use crysirror_resource::{
    ArchiveMount, DecodeContext, DependencyResolver, FormatRegistry, MemoryKeyProvider, read_all,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

fn context() -> DecodeContext {
    DecodeContext::new(ClientId(7))
}
fn memory(bytes: Vec<u8>) -> Arc<dyn AssetReader> {
    Arc::new(MemoryAssetReader(bytes.into()))
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn header(bytes: &mut [u8], at: usize, magic: &[u8; 4], count: u32) {
    bytes[at..at + 4].copy_from_slice(magic);
    put(bytes, at + 8, count);
}
fn unicode_entry(bytes: &mut [u8], at: usize, name: &str, offset: u64, size: u32) {
    for (i, c) in name.encode_utf16().enumerate() {
        bytes[at + i * 2..at + i * 2 + 2].copy_from_slice(&c.to_le_bytes());
    }
    put(bytes, at + 68, size);
    bytes[at + 72..at + 80].copy_from_slice(&offset.to_le_bytes());
}
fn open(
    id: &str,
    bytes: Vec<u8>,
    cx: &DecodeContext,
) -> anyhow::Result<Box<dyn crysirror_resource::Archive>> {
    crysirror_formats_arc::legacy_archive(id)
        .unwrap()
        .open(memory(bytes), cx)
}
fn libu(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut at = 16 + entries.len() * 80;
    let mut bytes = vec![0; at];
    header(&mut bytes, 0, b"LIBU", entries.len() as u32);
    for (i, (name, data)) in entries.iter().enumerate() {
        unicode_entry(&mut bytes, 16 + i * 80, name, at as u64, data.len() as u32);
        bytes.extend(data);
        at += data.len();
    }
    bytes
}
fn cipher_context(format: &str) -> DecodeContext {
    let mut cx = context();
    let keys = MemoryKeyProvider::default();
    let mut key = vec![0xa5; 32];
    key.extend([0; 16]);
    keys.insert(cx.client, "fixture", key).unwrap();
    cx.keys = Arc::new(keys);
    cx.parameters.insert(
        format.into(),
        serde_json::json!({"encryption":"cfi","key_id":"fixture"}),
    );
    cx
}
fn encrypt_cfi(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.resize(bytes.len().div_ceil(16) * 16, 0);
    // With this authored key all rotations/XOR words are zero. The native
    // first-byte whitening is its own inverse, independently of our decoder.
    for block in bytes.chunks_exact_mut(16) {
        let first = block[0];
        for b in &mut block[1..] {
            *b ^= first;
        }
    }
    bytes
}

fn zlib(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn encrypted_text_and_psb_wrappers_obey_decoded_budgets() {
    let mut psbz = b"PSBZ".to_vec();
    psbz.extend(3u32.to_le_bytes());
    psbz.extend(zlib(b"PSB"));
    let bytes = encrypt_cfi(libu(&[
        ("text.txtz", zlib(b"text")),
        ("scene.psbz", psbz),
        ("raw.psbz", b"plain".to_vec()),
        ("broken.psbz", b"PSBZ".to_vec()),
    ]));
    let cx = cipher_context("malie/libu");
    let archive = open("malie/libu", bytes, &cx).unwrap();
    let text = archive.open(0, &cx).unwrap();
    assert_eq!(read_all(text.as_ref(), 4).unwrap(), b"text");
    assert_eq!(cx.resident_bytes(), 4);
    drop(text);
    assert_eq!(cx.resident_bytes(), 0);
    assert_eq!(
        read_all(archive.open(1, &cx).unwrap().as_ref(), 3).unwrap(),
        b"PSB"
    );
    assert_eq!(
        read_all(archive.open(2, &cx).unwrap().as_ref(), 5).unwrap(),
        b"plain"
    );
    assert!(archive.open(3, &cx).is_err());
    let mut limited = cx.clone();
    limited.limits.max_entry_size = 2;
    assert!(archive.open(0, &limited).is_err());
    assert!(archive.open(1, &limited).is_err());
    assert_eq!(cx.resident_bytes(), 0);
}

#[test]
fn embedded_lib_and_libu_keep_depth_first_native_order() {
    let cx = context();
    let mut bytes = vec![0; 195];
    header(&mut bytes, 0, b"LIB\0", 2);
    bytes[16..19].copy_from_slice(b"dir");
    put(&mut bytes, 16 + 36, 67);
    put(&mut bytes, 16 + 40, 112);
    bytes[64..67].copy_from_slice(b"raw");
    put(&mut bytes, 64 + 36, 16);
    put(&mut bytes, 64 + 40, 179);
    header(&mut bytes, 112, b"LIB\0", 1);
    bytes[128..133].copy_from_slice(b"a.txt");
    put(&mut bytes, 128 + 36, 3);
    put(&mut bytes, 128 + 40, 64);
    bytes[176..179].copy_from_slice(b"ABC");
    header(&mut bytes, 179, b"LIB\0", 0);
    let archive = open("malie/lib", bytes, &cx).unwrap();
    assert_eq!(
        archive
            .entries()
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>(),
        ["dir/a.txt", "raw"]
    );
    assert_eq!(
        read_all(archive.open(0, &cx).unwrap().as_ref(), 10).unwrap(),
        b"ABC"
    );
    let raw = read_all(archive.open(1, &cx).unwrap().as_ref(), 16).unwrap();
    assert_eq!(&raw[..4], b"LIB\0");

    let mut bytes = vec![0; 195];
    header(&mut bytes, 0, b"LIBU", 1);
    // LIBU directory-record sizes are not bounds for their nested indexes.
    unicode_entry(&mut bytes, 16, "dir", 96, u32::MAX);
    header(&mut bytes, 96, b"LIBU", 1);
    unicode_entry(&mut bytes, 112, "日本語.txt", 96, 3);
    bytes[192..].copy_from_slice(b"ABC");
    let archive = open("malie/libu", bytes, &cx).unwrap();
    assert_eq!(archive.entries()[0].path, "dir/日本語.txt");
    assert_eq!(archive.entries()[0].metadata["offset"], 192);
    assert_eq!(
        read_all(archive.open(0, &cx).unwrap().as_ref(), 10).unwrap(),
        b"ABC"
    );
}

struct Sparse {
    header: Vec<u8>,
    payload: u64,
    read: AtomicU64,
}
impl AssetReader for Sparse {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.payload + 3)
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        if at >= self.payload + 3 {
            return Ok(0);
        }
        let n = (self.payload + 3 - at).min(out.len().min(3) as u64) as usize;
        self.read.fetch_add(n as u64, Ordering::Relaxed);
        for (i, b) in out[..n].iter_mut().enumerate() {
            let pos = at + i as u64;
            *b = if pos < self.header.len() as u64 {
                self.header[pos as usize]
            } else if pos >= self.payload {
                b"END"[(pos - self.payload) as usize]
            } else {
                0
            };
        }
        Ok(n)
    }
}
#[test]
fn libu_uses_full_64_bit_offsets_without_materializing_a_large_archive() {
    let mut bytes = vec![0; 96];
    header(&mut bytes, 0, b"LIBU", 1);
    let offset = 0x1_0000_0011;
    unicode_entry(&mut bytes, 16, "big.raw", offset, 3);
    let source = Arc::new(Sparse {
        header: bytes,
        payload: offset,
        read: AtomicU64::new(0),
    });
    let cx = context();
    let archive = crysirror_formats_arc::legacy_archive("malie/libu")
        .unwrap()
        .open(source.clone(), &cx)
        .unwrap();
    assert_eq!(archive.entries()[0].metadata["offset"], offset);
    assert_eq!(
        read_all(archive.open(0, &cx).unwrap().as_ref(), 3).unwrap(),
        b"END"
    );
    assert!(source.read.load(Ordering::Relaxed) < 1024);
    assert_eq!(cx.resident_bytes(), 0);
}

#[test]
fn libp_accepts_the_second_file_flag_and_profile_alignment() {
    let mut bytes = vec![0; 1155];
    bytes[..4].copy_from_slice(b"LIBP");
    put(&mut bytes, 4, 2);
    put(&mut bytes, 8, 1);
    put(&mut bytes, 16 + 24, 1);
    put(&mut bytes, 16 + 28, 1);
    bytes[48..53].copy_from_slice(b"a.txt");
    put(&mut bytes, 48 + 20, 0x20000);
    put(&mut bytes, 48 + 28, 3);
    put(&mut bytes, 80, 1);
    bytes[1152..].copy_from_slice(b"ABC");
    let mut cx = context();
    cx.parameters.insert(
        "malie/libp".into(),
        serde_json::json!({"data_alignment":64}),
    );
    let archive = open("malie/libp", bytes, &cx).unwrap();
    assert_eq!(archive.entries()[0].path, "a.txt");
    assert_eq!(archive.entries()[0].metadata["offset"], 1152);
    assert_eq!(
        read_all(archive.open(0, &cx).unwrap().as_ref(), 3).unwrap(),
        b"ABC"
    );
}

struct Mounted(ArchiveMount);
impl DependencyResolver for Mounted {
    fn open(&self, path: &str) -> anyhow::Result<Arc<dyn AssetReader>> {
        Ok(crysirror_core::vfs::VfsProvider::open(&self.0, path)?)
    }
}

#[test]
fn dzi_companion_failures_do_not_fall_back_on_access_errors() {
    struct Denied(AtomicU64);
    impl DependencyResolver for Denied {
        fn open(&self, _: &str) -> anyhow::Result<Arc<dyn AssetReader>> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into())
        }
    }
    let resolver = Arc::new(Denied(AtomicU64::new(0)));
    let mut cx = context();
    cx.source_name = Some("image.dzi".into());
    cx.resolver = Some(resolver.clone());
    let error = open("malie/dzi", b"DZI\n1,1\n1\n1,1\ntile\n".to_vec(), &cx)
        .err()
        .expect("denied tile must fail");
    assert!(error.downcast_ref::<std::io::Error>().is_some());
    assert_eq!(resolver.0.load(Ordering::Relaxed), 1);
}
#[test]
fn encrypted_mount_feeds_mgf_and_dzi_through_shared_resources() {
    let expected = crysirror_image::ImageSurface {
        width: 1,
        height: 1,
        rgba: vec![10, 20, 30, 40],
    };
    let mut mgf = crysirror_image::encode_png_uncompressed(&expected).unwrap();
    mgf[..8].copy_from_slice(b"MalieGF\0");
    let bytes = encrypt_cfi(libu(&[
        ("image.dzi", b"DZI\r\n1,1\r\n1\r\n1,1\r\ntile\r\n".to_vec()),
        ("tex/tile.mgf", mgf),
    ]));
    let cx = cipher_context("malie/libu");
    let mut registry = FormatRegistry::default();
    crysirror_formats_arc::register(&mut registry).unwrap();
    let (_, archive) = registry
        .open(memory(bytes), Some("malie/libu"), &cx)
        .unwrap();
    let mut foreign = context();
    foreign.client = ClientId(8);
    assert!(archive.open(0, &foreign).is_err());
    let mounted = Arc::new(Mounted(ArchiveMount::new(archive, cx.clone()).unwrap()));
    let mut dzi_context = cx.clone();
    dzi_context.source_name = Some("image.dzi".into());
    dzi_context.resolver = Some(mounted.clone());
    let dzi = mounted.open("image.dzi").unwrap();
    let archive = crysirror_formats_arc::legacy_archive("malie/dzi")
        .unwrap()
        .open(dzi, &dzi_context)
        .unwrap();
    let result = read_all(archive.open(0, &dzi_context).unwrap().as_ref(), 4096).unwrap();
    assert_eq!(crysirror_image::decode_standard(&result).unwrap(), expected);
}

#[test]
fn malformed_indexes_profiles_and_foreign_keys_fail_visibly() {
    let bytes = encrypt_cfi(libu(&[("a.txt", b"ABC".to_vec())]));
    let cx = cipher_context("malie/libu");
    let mut foreign = cx.clone();
    foreign.client = ClientId(8);
    assert!(open("malie/libu", bytes.clone(), &foreign).is_err());
    let mut invalid = cx.clone();
    invalid.parameters.get_mut("malie/libu").unwrap()["unknown"] = true.into();
    assert!(open("malie/libu", bytes, &invalid).is_err());
    assert!(
        open(
            "malie/libu",
            libu(&[("../escape.txt", vec![0])]),
            &context()
        )
        .is_err()
    );
    let mut cyclic = vec![0; 96];
    header(&mut cyclic, 0, b"LIBU", 1);
    unicode_entry(&mut cyclic, 16, "dir", 0, 96);
    assert!(open("malie/libu", cyclic, &context()).is_err());
    let mut truncated = libu(&[("a.txt", vec![0])]);
    truncated.truncate(70);
    assert!(open("malie/libu", truncated, &context()).is_err());
    let mut limited = context();
    limited.limits.max_index_size = 95;
    assert!(open("malie/libu", libu(&[("a.txt", vec![0])]), &limited).is_err());
}
