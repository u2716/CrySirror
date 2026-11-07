// SPDX-License-Identifier: GPL-3.0-or-later
//! ABM1/3 ordinal frame packs and native sequence tables, independent of a VM.
//! Layout: Tsukumo-2012 reader 440290; type1 starts with opaque BGR, type3
//! compresses every frame using the existing ABM stored-alpha byte runs.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{AssetReader, MemoryAssetReader},
    state::ClientId,
};
use crysirror_resource::{Archive, ArchiveEntry, ArchiveFormat, DecodeContext, Probe};
use std::sync::Arc;
fn word(b: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(at..at + 2)
            .context("ABM animation word")?
            .try_into()?,
    ))
}
fn dword(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4)
            .context("ABM animation dword")?
            .try_into()?,
    ))
}
/// Shared ABM32 byte-run decoder; stored output is BGRA, with no row flip.
pub(crate) fn rgba_runs(
    input: &[u8],
    pixels: usize,
    allow_padding: bool,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    let expected = pixels.checked_mul(4).context("ABM pixel overflow")?;
    ensure!(expected <= 256 * 1024 * 1024, "ABM decoded byte budget");
    let mut out = Vec::with_capacity(expected);
    let mut at = 0;
    let mut channel = 0;
    let mut packets = 0usize;
    while out.len() < expected {
        if packets & 1023 == 0 {
            check()?;
        }
        packets += 1;
        let control = *input.get(at).context("truncated ABM control")?;
        at += 1;
        let count = if matches!(control, 0 | 255) {
            let count = *input.get(at).context("truncated ABM count")? as usize;
            at += 1;
            count
        } else {
            1
        };
        for _ in 0..count {
            ensure!(out.len() < expected, "ABM run exceeds frame");
            let byte = if control == 0 {
                0
            } else {
                let b = *input.get(at).context("truncated ABM literal")?;
                at += 1;
                b
            };
            out.push(byte);
            channel += 1;
            if channel == 3 {
                out.push(control);
                channel = 0;
            }
        }
    }
    ensure!(
        channel == 0 && (allow_padding || at == input.len()),
        "ABM trailing/unaligned frame data"
    );
    check()?;
    Ok(out)
}
pub struct AbmFrames;
impl ArchiveFormat for AbmFrames {
    fn id(&self) -> &'static str {
        "lilim/abm-frames"
    }
    fn probe(&self, source: &dyn AssetReader) -> Result<Probe> {
        if source.len()? < 70 {
            return Ok(Probe { confidence: 0 });
        }
        let b = crate::common::bytes(source, 0, 70, 70)?;
        Ok(Probe {
            confidence: if &b[..2] == b"BM"
                && dword(&b, 14)? == 40
                && word(&b, 26)? == 1
                && matches!(word(&b, 28)?, 1 | 3)
                && word(&b, 56)? > 0
                && word(&b, 58)? > 0
            {
                100
            } else {
                0
            },
        })
    }
    fn open(&self, source: Arc<dyn AssetReader>, cx: &DecodeContext) -> Result<Box<dyn Archive>> {
        cx.check_cancelled()?;
        let b = crate::common::bytes(source.as_ref(), 0, source.len()?, cx.limits.max_entry_size)?;
        ensure!(
            b.len() >= 70 && &b[..2] == b"BM" && dword(&b, 14)? == 40 && word(&b, 26)? == 1,
            "ABM animation header"
        );
        let width = dword(&b, 18)?;
        let height = dword(&b, 22)?;
        ensure!(
            width > 0 && height > 0 && width <= 16384 && height <= 16384,
            "ABM animation dimensions"
        );
        let depth = word(&b, 28)?;
        ensure!(matches!(depth, 1 | 3), "ABM animation variant");
        let sequences = word(&b, 56)? as usize;
        let frames = word(&b, 58)? as usize;
        ensure!(
            sequences > 0 && frames > 0 && frames < cx.limits.max_entries && sequences <= 4096,
            "ABM animation count budget"
        );
        let pixels = (width as usize)
            .checked_mul(height as usize)
            .context("ABM frame pixels")?;
        let resident = (pixels as u64)
            .checked_mul(4)
            .and_then(|v| v.checked_mul(frames as u64))
            .context("ABM resident overflow")?;
        ensure!(
            resident <= cx.limits.max_resident_bytes
                && pixels as u64 * 4 <= cx.limits.max_entry_size,
            "ABM animation resident budget"
        );
        let program_size = dword(&b, 62)? as usize;
        let mut at = 66usize
            .checked_add(4 * frames)
            .context("ABM directory overflow")?;
        let program_end = at
            .checked_add(program_size)
            .context("ABM program overflow")?;
        ensure!(
            program_end <= b.len() && program_size as u64 <= cx.limits.max_index_size,
            "ABM program extent"
        );
        let mut programs = Vec::with_capacity(sequences);
        for _ in 0..sequences {
            cx.check_cancelled()?;
            let flags = word(&b, at)?;
            let count = word(&b, at + 2)? as usize;
            at += 4;
            ensure!(
                count > 0
                    && at
                        .checked_add(2 * count)
                        .is_some_and(|end| end <= program_end),
                "ABM sequence extent"
            );
            let mut indices = Vec::with_capacity(count);
            for _ in 0..count {
                let index = word(&b, at)?;
                at += 2;
                ensure!((index as usize) < frames, "ABM sequence frame ordinal");
                indices.push(index);
            }
            programs.push(serde_json::json!({"flags":flags,"frames":indices}));
        }
        ensure!(at == program_end, "ABM sequence table length");
        let mut offsets = (0..frames)
            .map(|i| dword(&b, 66 + 4 * i).map(|v| v as usize))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            offsets.iter().all(|v| *v >= program_end && *v < b.len())
                && offsets.windows(2).all(|v| v[0] < v[1]),
            "ABM frame offsets"
        );
        offsets.push(b.len());
        let metadata = serde_json::to_vec(
            &serde_json::json!({"format":self.id(),"size":[width,height],"depth":depth,"sequences":programs}),
        )?;
        let mut contents: Vec<Arc<[u8]>> = vec![metadata.into()];
        let mut entries = vec![ArchiveEntry {
            id: 0,
            path: "timeline.json".into(),
            size: contents[0].len() as u64,
            stored_size: program_size as u64,
            metadata: serde_json::json!({"format":self.id(),"kind":"timeline"}),
        }];
        let mut encoded_total = contents[0].len() as u64;
        for i in 0..frames {
            cx.check_cancelled()?;
            let data = &b[offsets[i]..offsets[i + 1]];
            let bgra = if depth == 1 && i == 0 {
                ensure!(data.len() >= 3 * pixels, "ABM opaque base extent");
                let mut out = Vec::with_capacity(4 * pixels);
                for row in data[..3 * pixels].chunks(3 * width as usize) {
                    cx.check_cancelled()?;
                    for p in row.chunks_exact(3) {
                        out.extend_from_slice(&[p[0], p[1], p[2], 255]);
                    }
                }
                out
            } else {
                rgba_runs(data, pixels, true, &|| cx.check_cancelled())?
            };
            let mut rgba = bgra;
            for row in rgba.chunks_mut(4 * width as usize) {
                cx.check_cancelled()?;
                for p in row.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
            }
            let png = crysirror_image::encode_png(&crysirror_image::ImageSurface {
                width,
                height,
                rgba,
            })?;
            ensure!(
                png.len() as u64 <= cx.limits.max_entry_size,
                "ABM exported frame budget"
            );
            encoded_total = encoded_total
                .checked_add(png.len() as u64)
                .context("ABM exported total overflow")?;
            ensure!(
                resident
                    .checked_add(encoded_total)
                    .is_some_and(|n| n <= cx.limits.max_resident_bytes),
                "ABM worker transient/resident budget"
            );
            let id = i as u32 + 1;
            entries.push(ArchiveEntry {
                id,
                path: format!("{i:06}.png"),
                size: png.len() as u64,
                stored_size: data.len() as u64,
                metadata: serde_json::json!({"format":self.id(),"frame":i,"size":[width,height]}),
            });
            contents.push(png.into());
        }
        cx.check_cancelled()?;
        Ok(Box::new(Frames {
            owner: cx.client,
            entries,
            contents,
        }))
    }
}
struct Frames {
    owner: ClientId,
    entries: Vec<ArchiveEntry>,
    contents: Vec<Arc<[u8]>>,
}
impl Archive for Frames {
    fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }
    fn resolve(&self, path: &str) -> Option<u32> {
        self.entries.iter().find(|e| e.path == path).map(|e| e.id)
    }
    fn open(&self, id: u32, cx: &DecodeContext) -> Result<Arc<dyn AssetReader>> {
        ensure!(cx.client == self.owner, "foreign ABM frame pack owner");
        cx.check_cancelled()?;
        let bytes = self.contents.get(id as usize).context("ABM frame ID")?;
        ensure!(
            bytes.len() as u64 <= cx.limits.max_entry_size,
            "ABM frame read budget"
        );
        Ok(Arc::new(MemoryAssetReader(bytes.clone())))
    }
}
