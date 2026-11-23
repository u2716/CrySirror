// SPDX-License-Identifier: MPL-2.0
use crysirror_core::{
    service::{AssetReader, MemoryAssetReader},
    state::ClientId,
    vfs::VfsProvider,
};
use crysirror_resource::{ArchiveMount, DecodeContext, FormatRegistry};
use flate2::{Compression, write::ZlibEncoder};
use std::{io::Write, sync::Arc};

fn record(dst: &mut Vec<u8>, hash: [u8; 16], index: u32, size: usize, at: usize) {
    dst.extend(hash);
    dst.extend(index.to_be_bytes());
    dst.extend(&(size as u64).to_be_bytes()[3..]);
    dst.extend(&(at as u64).to_be_bytes()[3..]);
}

fn fixture(name: &str) -> Vec<u8> {
    // Three source blocks exercise all branches: a raw full block, an
    // independently-compressed full block and a raw short final block.
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::fast());
    zlib.write_all(&vec![b'Z'; 65536]).unwrap();
    let z = zlib.finish().unwrap();
    let lengths = [name.len() as u16, 0, z.len() as u16, 4];
    let toc = 32 + 2 * 30 + lengths.len() * 2;
    let mut bytes = b"PSAR\0\x01\0\x03zlib".to_vec();
    for word in [toc as u32, 30, 2, 65536, 0] {
        bytes.extend(word.to_be_bytes());
    }
    record(&mut bytes, [0; 16], 0, name.len(), toc);
    record(
        &mut bytes,
        crysirror_resource::checksum::md5_legacy(name.as_bytes()),
        1,
        2 * 65536 + 4,
        toc + name.len(),
    );
    for len in lengths {
        bytes.extend(len.to_be_bytes());
    }
    bytes.extend(name.as_bytes());
    bytes.resize(bytes.len() + 65536, b'A');
    bytes.extend(z);
    bytes.extend(b"TAIL");
    bytes
}
fn source(bytes: Vec<u8>) -> Arc<dyn AssetReader> {
    Arc::new(MemoryAssetReader(bytes.into()))
}
fn registry() -> FormatRegistry {
    let mut r = FormatRegistry::default();
    crysirror_formats_arc::register(&mut r).unwrap();
    r
}
fn mount(bytes: Vec<u8>) -> anyhow::Result<ArchiveMount> {
    let c = DecodeContext::new(ClientId(1));
    let (_, a) = registry().open(source(bytes), Some("sony/psarc"), &c)?;
    ArchiveMount::new(a, c)
}
#[test]
fn mounted_seek_spans_all_block_types() {
    let bytes = fixture("maps/train.pak");
    let r = registry();
    let c = DecodeContext::new(ClientId(1));
    assert!(
        r.probe(source(bytes.clone()), &c)
            .unwrap()
            .iter()
            .any(|(id, _)| *id == "sony/psarc")
    );
    let m = mount(bytes).unwrap();
    assert_eq!(m.list_dir("maps").unwrap(), ["train.pak"]);
    let handle = m.open("maps/train.pak").unwrap();
    let mut buf = [0; 8];
    assert_eq!(handle.read_at(65532, &mut buf).unwrap(), 8);
    assert_eq!(&buf, b"AAAAZZZZ");
    assert_eq!(handle.read_at(131068, &mut buf).unwrap(), 8);
    assert_eq!(&buf, b"ZZZZTAIL");
    assert_eq!(handle.read_at(131074, &mut buf).unwrap(), 2);
    assert_eq!(&buf[..2], b"IL");
    assert_eq!(handle.read_at(u64::MAX, &mut buf).unwrap(), 0);
}
#[test]
fn rejects_wrong_profile_offsets_hash_paths_and_invalid_zlib_end() {
    let mut b = fixture("file");
    b[31] = 4;
    assert!(mount(b).is_err());
    let mut b = fixture("file");
    b[32 + 30] ^= 1;
    assert!(mount(b).is_err());
    let mut b = fixture("file");
    b[32 + 30 + 25] = 1;
    assert!(mount(b).is_err());
    let mut b = fixture("file");
    b.truncate(b.len() - 1);
    assert!(mount(b).is_err());
    assert!(mount(fixture("../escape")).is_err());
    let mut b = fixture("file");
    let corrupt = b.len() - 5;
    b[corrupt] ^= 1;
    let file = mount(b).unwrap().open("file").unwrap();
    assert!(file.read_at(65536, &mut [0; 1]).is_err());
}
