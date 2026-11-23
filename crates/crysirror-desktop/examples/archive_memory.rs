// SPDX-License-Identifier: MPL-2.0
//! Developer tool: live heap bytes of opened and mounted archive indexes.
//! Reads the user's local installation only; prints per-archive entry counts
//! and resident index bytes (archive entries, plans and the mount's path and
//! directory maps). Used for console memory budgets (docs/validation).
//!
//! cargo run --release -p crysirror-desktop --example archive_memory -- FORMAT ARCHIVE...
use anyhow::{Context, Result};
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{ArchiveFormat, ArchiveMount, DecodeContext};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs::File,
    sync::atomic::{AtomicIsize, Ordering},
    sync::Arc,
};

struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new: usize) -> *mut u8 {
        LIVE.fetch_add(new as isize - layout.size() as isize, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new) }
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;

struct Reader(File);
impl AssetReader for Reader {
    fn len(&self) -> crysirror_core::Result<u64> {
        self.0.metadata().map(|m| m.len()).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        std::os::unix::fs::FileExt::read_at(&self.0, out, at).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let id = args.get(1).context("FORMAT")?;
    let format: Arc<dyn ArchiveFormat> = if id == "kirikiri/xp3" {
        Arc::new(crysirror_formats_arc::Xp3)
    } else {
        crysirror_formats_arc::legacy_archive(id).with_context(|| format!("unknown format {id}"))?
    };
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = 1024 * 1024 * 1024;
    let mut mounts = Vec::new();
    let (mut entries, mut bytes) = (0usize, 0isize);
    for path in &args[2..] {
        let source: Arc<dyn AssetReader> = Arc::new(Reader(File::open(path)?));
        let before = LIVE.load(Ordering::Relaxed);
        let archive = format.open(source, &cx).with_context(|| format!("open {path}"))?;
        let count = archive.entries().len();
        let opened = LIVE.load(Ordering::Relaxed) - before;
        let mount = ArchiveMount::new(archive, cx.clone())?;
        let live = LIVE.load(Ordering::Relaxed) - before;
        let per = |n: isize| n / count.max(1) as isize;
        println!("{path}: {count} entries, {} KiB, {} B/entry (archive {}, mount maps {})", live / 1024, per(live), per(opened), per(live - opened));
        entries += count;
        bytes += live;
        mounts.push(mount);
    }
    println!("total: {entries} entries, {:.1} MiB, {} B/entry", bytes as f64 / 1048576.0, bytes / entries.max(1) as isize);
    Ok(())
}
