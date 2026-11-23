// SPDX-License-Identifier: MPL-2.0
use super::*;
use crysirror_core::service::MemoryAssetReader;
use std::cell::Cell;

fn header(thumbnail_enabled: bool) -> [u8; HEADER_BYTES] {
    let mut header = [0; HEADER_BYTES];
    header[..4].copy_from_slice(b"CSV2");
    put_word(&mut header, 4, HEADER_BYTES as u32);
    put_word(&mut header, 8, 1234);
    header[16..36].copy_from_slice(b"2026-11-19 12:34:56 ");
    header[272..279].copy_from_slice(b"fixture");
    put_word(&mut header, 528, 0x0102_0304);
    header[548] = u8::from(thumbnail_enabled);
    header
}

fn script() -> Vec<u8> {
    let words = [48, 2, 0x6eab_6fef, 1, 2, 4, 6, 0, 13, 16, 0x8765_4321];
    let mut expanded = Vec::from(*b"PS2A");
    for word in words {
        expanded.extend_from_slice(&u32::to_le_bytes(word));
    }
    expanded.extend_from_slice(b"\0\0\0\0\0\x20\x05\x06\x07\x08hello\0");
    expanded
}

fn unpack_bytes(bytes: Vec<u8>) -> Result<Slot> {
    unpack(&MemoryAssetReader(bytes.into()), || Ok(()))
}

#[test]
fn reads_synthetic_ranges_emitted_by_the_original_csv2_writer() {
    // The 20160201 writer at 0x140080b20 produced these sections from the
    // synthetic checkpoint, preview bytes and script constructed below.
    let packed = [
        0xbb, 0xc1, 0xcc, 0xc1, 0xdf, 0xb2, 0xa3, 0xb5, 0x1b, 0x20, 0xc9, 0x32, 0x3e, 0xe7, 0xda,
        0xfd, 0xd5, 0x3f, 0x42, 0x4d, 0x01, 0x02, 0x03, 0x04, 0x5f, 0x50, 0x53, 0x32, 0x41, 0x30,
        0xdc, 0xe1, 0x02, 0xdc, 0xe1, 0x5f, 0xef, 0x6f, 0xab, 0x6e, 0x01, 0xe4, 0xe5, 0x04, 0xdc,
        0xe1, 0xd5, 0x06, 0xd8, 0xe5, 0x0d, 0xdc, 0xe1, 0x10, 0xdc, 0xe1, 0x21, 0x43, 0xfb, 0x65,
        0x87, 0xda, 0xe3, 0x20, 0x05, 0x06, 0x07, 0x08, 0x3f, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x00,
        0xf9, 0x0a,
    ];
    let mut prefix = header(true);
    prefix[546..548].copy_from_slice(&1u16.to_le_bytes());
    for (offset, word) in [
        (540, 675),
        (560, 17),
        (564, 7),
        (568, 51),
        (580, 128),
        (584, 6),
        (588, 64),
    ] {
        put_word(&mut prefix, offset, word);
    }
    let mut file = prefix.to_vec();
    file.extend_from_slice(&packed);
    let slot = unpack_bytes(file).unwrap();
    assert_eq!(slot.checkpoint, b"\x00\x01\x04\x00abc\0".repeat(16));
    assert_eq!(
        slot.thumbnail.as_deref(),
        Some(b"BM\x01\x02\x03\x04".as_slice())
    );
    assert_eq!(
        slot.main.expanded_image(&[5, 6, 7, 8], b"hello\0").unwrap(),
        script()
    );
}

#[test]
fn checked_slot_keeps_native_header_and_expanded_main_image() {
    let raw = script();
    let main = Script::decode_saved(raw.clone(), ScriptFormat::Ps2a).unwrap();
    let live = main.expanded_image(b"data", b"saved\0").unwrap();
    assert_eq!(&live[..48], &raw[..48]);
    assert!(main.expanded_image(b"longer", b"saved\0").is_err());
    assert!(main.expanded_image(b"data", b"\0").is_err());

    let checkpoint = b"\x00\x01\x04\x00abc\0".repeat(80);
    let thumbnail = b"BM\x01\x02\x03\x04";
    let file = pack(header(true), &checkpoint, Some(thumbnail), &live, || Ok(())).unwrap();
    let trailer = file.len() - 2;
    let prefix = file.first_chunk::<HEADER_BYTES>().unwrap();
    let checkpoint_end = HEADER_BYTES + word(prefix, 560) as usize;
    assert_eq!(word(prefix, 540) as usize, trailer);
    assert_eq!(&file[546..550], &[1, 0, 1, 0]);
    let sum: u32 = file[HEADER_BYTES..checkpoint_end]
        .iter()
        .map(|&byte| u32::from(byte))
        .sum();
    assert_eq!(&file[trailer..], &(sum as u16).to_le_bytes());
    assert_eq!(&file[16..36], b"2026-11-19 12:34:56 ");
    assert_eq!(&file[272..279], b"fixture");

    let slot = unpack_bytes(file).unwrap();
    assert_eq!(word(&slot.header, 528), 0x0102_0304);
    assert_eq!(slot.checkpoint, checkpoint);
    assert_eq!(slot.thumbnail.as_deref(), Some(thumbnail.as_slice()));
    assert_eq!(slot.main.labels().collect::<Vec<_>>(), [0]);
    assert_eq!(slot.main.code(), [0, 0x20]);
    assert_eq!(slot.main.initial_data(), b"data");
    assert_eq!(slot.main.text_bytes(), b"saved\0");
    assert_eq!(slot.main.expanded_image(b"data", b"saved\0").unwrap(), live);
}

#[test]
fn older_untransformed_checkpoint_and_disabled_thumbnail_match_native_ranges() {
    let main = script();
    let preview = b"retained BMP";
    let checkpoint = b"\x00\x01\x07\x00record\0";
    let mut file = pack(header(false), checkpoint, Some(preview), &main, || Ok(())).unwrap();
    let prefix = file.first_chunk::<HEADER_BYTES>().unwrap();
    assert_eq!(word(prefix, 564), 0);
    assert_eq!(word(prefix, 584), preview.len() as u32);
    let packed_checkpoint = HEADER_BYTES..HEADER_BYTES + word(prefix, 560) as usize;
    for (offset, byte) in file[packed_checkpoint].iter_mut().enumerate() {
        let key = b"DAME_SAVE_IMAGE"[offset % b"DAME_SAVE_IMAGE".len()];
        *byte = (*byte ^ key).wrapping_sub(0x80);
    }
    file.truncate(file.len() - 2);
    let slot = unpack_bytes(file).unwrap();
    assert_eq!(slot.checkpoint, checkpoint);
    assert!(slot.thumbnail.is_none());
    assert_eq!(slot.main.text_bytes(), b"hello\0");
}

#[test]
fn invalid_sections_trailer_script_and_cancellation_do_not_return_state() {
    let main = script();
    let file = pack(header(true), b"abc", Some(b"BM"), &main, || Ok(())).unwrap();
    for (offset, value, expected) in [
        (0, 0, "whole-slot header"),
        (560, 0, "slot packed range"),
        (580, u32::MAX, "slot expanded range"),
        (540, 0, "packed ranges disagree"),
        (568, 1, "checksum offset"),
    ] {
        let mut invalid = file.clone();
        put_word(
            invalid.first_chunk_mut::<HEADER_BYTES>().unwrap(),
            offset,
            value,
        );
        let error = unpack_bytes(invalid).err().unwrap();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }
    let mut corrupted = file.clone();
    corrupted[HEADER_BYTES] ^= 1;
    let error = unpack_bytes(corrupted).err().unwrap();
    assert!(
        format!("{error:#}").contains("checkpoint checksum"),
        "{error:#}"
    );
    let mut invalid_main = main;
    invalid_main[48..52].copy_from_slice(&1u32.to_le_bytes());
    let invalid = pack(header(false), b"abc", None, &invalid_main, || Ok(())).unwrap();
    let error = unpack_bytes(invalid).err().unwrap();
    assert!(format!("{error:#}").contains("PS2A label"), "{error:#}");

    let cancel = Cell::new(0);
    let error = unpack(&MemoryAssetReader(file.into()), || {
        cancel.set(cancel.get() + 1);
        ensure!(cancel.get() < 4, "cancelled");
        Ok(())
    })
    .err()
    .unwrap();
    assert!(format!("{error:#}").contains("cancelled"), "{error:#}");
}

#[test]
fn csp1_uses_its_own_checkpoint_key_and_pss0_main_image() -> Result<()> {
    // Independently encoded literal-only LZSS packets for each section.
    let record = b"\x00\x01\x04\x00abc\0";
    let mut image = script();
    image[..4].copy_from_slice(b"PSS0");
    let literal = |data: &[u8]| -> Vec<u8> {
        data.chunks(8).flat_map(|chunk| std::iter::once(0xff).chain(chunk.iter().copied())).collect()
    };
    let mut packed = literal(record);
    for (i, b) in packed.iter_mut().enumerate() {
        *b = b.wrapping_add(128) ^ b"SAVE_IMAGE_ARC"[i % 14];
    }
    let sum: u16 = packed.iter().map(|&b| u16::from(b)).sum();
    let mut h = header(false);
    h[..4].copy_from_slice(b"CSP1");
    h[544..546].copy_from_slice(&3u16.to_le_bytes());
    put_word(&mut h, 560, packed.len() as u32);
    put_word(&mut h, 580, record.len() as u32);
    let main = literal(&image);
    put_word(&mut h, 568, main.len() as u32);
    put_word(&mut h, 588, image.len() as u32);
    put_word(&mut h, 540, (HEADER_BYTES + packed.len() + main.len()) as u32);
    let file = [h.as_slice(), &packed, &main, &sum.to_le_bytes()].concat();
    let reader = MemoryAssetReader(file.into());
    assert!(unpack(&reader, || Ok(())).is_err());
    let slot = unpack_native(SlotFormat::Csp1, &reader, || Ok(()))?;
    assert_eq!(slot.checkpoint, record);
    assert_eq!(slot.main.format(), ScriptFormat::Pss0);
    let written = pack_native(SlotFormat::Csp1, slot.header, record, None, &image, || Ok(()))?;
    let reopened = unpack_native(SlotFormat::Csp1, &MemoryAssetReader(written.into()), || Ok(()))?;
    assert_eq!(reopened.main.expanded_image(&[5,6,7,8], b"hello\0")?, image);
    Ok(())
}
